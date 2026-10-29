mod common;

use std::time::{Duration, Instant};

use lucent_host::bridge::{ScriptEvent, Value};
use lucent_host::testing;
use lucent_host::{ScriptLink, Session};
use lucent_protocol::command::Command;
use lucent_runtime::{Bot, Kill, RuntimeError};
use serde_json::json;

use common::TIMEOUT;

fn start() -> (lucent_host::Host, Session, ScriptLink) {
    let host = testing::headless_host();
    let (session, link) = testing::start_probe(
        &host,
        common::ready_movie()
            .recording("settings.setWalkSpeed")
            .recording("settings.enemyMagnet"),
    );
    for packet in [
        json!({"cmd":"loadInventoryBig","items":[],"hitems":[]}),
        json!({"cmd":"moveToArea","areaId":1,"areaName":"battleon-1",
            "uoBranch":[{"entID":7,"uoName":"player-a","intHP":100,"intHPMax":100}]}),
    ] {
        link.call_loader(
            "probe.emit",
            vec![
                Value::String("onExtensionResponse".into()),
                Value::String(json!({"type":"json","dataObj":packet}).to_string()),
            ],
        )
        .recv_timeout(TIMEOUT)
        .unwrap();
    }
    update(&link, json!({"walkSpeed":30,"enemyMagnetEnabled":true}));
    (host, session, link)
}

fn update(link: &ScriptLink, patch: serde_json::Value) {
    let command: Command =
        serde_json::from_value(json!({"kind":"updateSettings","patch":patch})).unwrap();
    link.command(command).recv_timeout(TIMEOUT).unwrap();
}

fn next_call(link: &ScriptLink, method: &str) -> Vec<Value> {
    loop {
        let event = common::await_event(link, "onProbeCall", None);
        if event.args.first() == Some(&Value::String(method.into())) {
            return event.args;
        }
    }
}

fn drain(link: &ScriptLink) {
    while link.try_event().unwrap().is_some() {}
}

#[test]
fn script_kills_never_restart_the_bot() {
    let (_host, mut session, observer) = start();
    drain(&observer);
    let bot = Bot::start(session.attach().unwrap()).unwrap();
    assert_eq!(
        next_call(&observer, "settings.setWalkSpeed")[1],
        Value::Number(30.0)
    );
    for _ in 0..3 {
        let run = common::start(
            &mut session,
            common::loose(&[("loop.js", "export default function* () { while (true) {} }")]),
            Default::default(),
        );
        assert!(matches!(
            run.wait(TIMEOUT),
            Err(RuntimeError::Killed(Kill::Watchdog))
        ));
        assert!(matches!(
            bot.wait(Duration::ZERO),
            Err(RuntimeError::Timeout)
        ));
        drain(&observer);
        assert_eq!(
            next_call(&observer, "settings.setWalkSpeed")[1],
            Value::Number(30.0)
        );
    }
}

#[test]
fn the_settings_policy_reapplies_through_the_host() {
    let (_host, mut session, observer) = start();
    drain(&observer);
    let _bot = Bot::start(session.attach().unwrap()).unwrap();
    assert_eq!(
        next_call(&observer, "settings.setWalkSpeed")[1],
        Value::Number(30.0)
    );
    update(&observer, json!({"walkSpeed":45}));
    drain(&observer);
    let began = Instant::now();
    assert_eq!(next_call(&observer, "settings.enemyMagnet")[1], Value::Null);
    assert!(began.elapsed() < Duration::from_millis(1500));
    assert_eq!(
        next_call(&observer, "settings.setWalkSpeed")[1],
        Value::Number(45.0)
    );
    update(&observer, json!({"enemyMagnetEnabled":false}));
    drain(&observer);
    let until = Instant::now() + Duration::from_millis(1200);
    while Instant::now() < until {
        let event = Some(ScriptEvent::Loader {
            event: common::await_event(&observer, "onProbeCall", None),
            projected: vec![],
        });
        if let ScriptEvent::Loader { event, .. } = event {
            assert_ne!(
                event.args.first(),
                Some(&Value::String("settings.enemyMagnet".into()))
            );
        }
    }
}
