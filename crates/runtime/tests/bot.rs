mod common;

use std::time::{Duration, Instant};

use lucent_host::bridge::Value;
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
    become_ready(&link);
    update(&link, json!({"walkSpeed":30,"enemyMagnetEnabled":true}));
    (host, session, link)
}

fn become_ready(link: &ScriptLink) {
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
}

fn update(link: &ScriptLink, patch: serde_json::Value) {
    let command: Command =
        serde_json::from_value(json!({"kind":"updateSettings","patch":patch})).unwrap();
    link.command(command).recv_timeout(TIMEOUT).unwrap();
}

fn next_call(link: &ScriptLink, method: &str) -> Vec<Value> {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        assert!(Instant::now() < deadline, "did not receive {method}");
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
    #[cfg(feature = "testing")]
    bot.eval("identity.js", "globalThis.identity = { turns: 0 }; null")
        .wait::<()>(TIMEOUT)
        .unwrap();
    assert_eq!(
        next_call(&observer, "settings.setWalkSpeed")[1],
        Value::Number(30.0)
    );
    for _index in 0..3 {
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
        #[cfg(feature = "testing")]
        assert_eq!(
            bot.eval("identity.js", "++identity.turns")
                .wait::<u32>(TIMEOUT)
                .unwrap(),
            _index + 1
        );
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
    assert_eq!(
        common::await_event(&observer, "onProbeCall", None).args,
        vec![
            Value::String("settings.enemyMagnet".into()),
            Value::Undefined
        ]
    );
    assert!(began.elapsed() < Duration::from_millis(1500));
    assert_eq!(
        next_call(&observer, "settings.setWalkSpeed")[1],
        Value::Number(45.0)
    );
    update(&observer, json!({"enemyMagnetEnabled":false}));
    drain(&observer);
    let until = Instant::now() + Duration::from_millis(1200);
    while Instant::now() < until {
        let event = common::await_event(&observer, "onProbeCall", None);
        assert_ne!(
            event.args.first(),
            Some(&Value::String("settings.enemyMagnet".into()))
        );
    }
}

#[test]
fn a_player_restart_stops_the_bot_and_a_new_bot_reads_the_host_settings() {
    let (_host, mut session, observer) = start();
    let bot = Bot::start(session.attach().unwrap()).unwrap();
    session.restart(testing::headless_surface()).unwrap();
    assert!(matches!(bot.wait(TIMEOUT), Err(RuntimeError::Stopped)));
    drop(bot);
    drop(observer);
    let observer = session.attach().unwrap();
    session.send(lucent_host::ViewEvent::Resized(
        lucent_host::Viewport::new(100.0, 100.0, 1.0).unwrap(),
    ));
    testing::wait_loaded(&observer);
    become_ready(&observer);
    drain(&observer);
    let bot = Bot::start(session.attach().unwrap()).unwrap();
    assert!(matches!(
        bot.wait(Duration::ZERO),
        Err(RuntimeError::Timeout)
    ));
    assert_eq!(
        next_call(&observer, "settings.setWalkSpeed")[1],
        Value::Number(30.0)
    );
}

#[cfg(feature = "testing")]
#[test]
fn a_bot_kill_is_terminal_and_its_owner_can_start_another_bot() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use tracing_subscriber::prelude::*;
    struct Errors(Arc<AtomicUsize>);
    impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for Errors {
        fn on_event(
            &self,
            event: &tracing::Event<'_>,
            _: tracing_subscriber::layer::Context<'_, S>,
        ) {
            if *event.metadata().level() == tracing::Level::ERROR {
                self.0.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
    for (source, expected) in [
        ("while (true) {}", Kill::Watchdog),
        (
            "const arrays = []; while (true) { arrays.push(new Array(100000).fill(1)); }",
            Kill::HeapLimit,
        ),
    ] {
        let errors = Arc::new(AtomicUsize::new(0));
        let dispatch =
            tracing::Dispatch::new(tracing_subscriber::registry().with(Errors(errors.clone())));
        let _guard = tracing::dispatcher::set_default(&dispatch);
        let (_host, mut session, observer) = start();
        let bot = Bot::start(session.attach().unwrap()).unwrap();
        assert_eq!(
            bot.eval(
                "exports.js",
                "[typeof __lucent.start, typeof __lucent.stop]"
            )
            .wait::<Vec<String>>(TIMEOUT)
            .unwrap(),
            vec!["undefined", "undefined"]
        );
        let pending = bot.eval("bad-automation.js", source);
        assert!(
            matches!(pending.wait::<()>(TIMEOUT), Err(RuntimeError::Killed(kill)) if kill == expected)
        );
        assert!(matches!(bot.wait(TIMEOUT), Err(RuntimeError::Killed(kill)) if kill == expected));
        assert!(matches!(
            bot.eval("dead.js", "42").wait::<u32>(TIMEOUT),
            Err(RuntimeError::Stopped)
        ));
        drop(bot);
        assert_eq!(errors.load(Ordering::Relaxed), 1);
        drain(&observer);
        let _replacement = Bot::start(session.attach().unwrap()).unwrap();
        assert_eq!(
            next_call(&observer, "settings.setWalkSpeed")[1],
            Value::Number(30.0)
        );
    }
}
