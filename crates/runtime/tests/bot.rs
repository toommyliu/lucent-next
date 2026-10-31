mod common;

use lucent_host::bridge::EventInterests;
use std::time::{Duration, Instant};

use lucent_host::bridge::Value;
use lucent_host::testing;
use lucent_host::{ScriptLink, Session};
use lucent_protocol::command::Command;
use lucent_runtime::{Bot, Kill, RuntimeError};
use serde_json::json;

use common::TIMEOUT;

#[cfg(feature = "testing")]
#[test]
fn bot_turns_charge_cpu_time_and_allow_waiting_off_cpu() {
    let host = testing::headless_host();
    let (_session, link) = testing::start_probe(&host, common::ready_movie());
    let mut bot = Bot::start(link).unwrap();
    assert_eq!(
        bot.eval(
            "wait.js",
            "Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 300)",
        )
        .wait::<String>(TIMEOUT)
        .unwrap(),
        "timed-out"
    );
    assert!(matches!(
        bot.eval(
            "runaway.js",
            "while (true) { try { while (true) {} } catch {} }"
        )
        .wait::<()>(TIMEOUT),
        Err(RuntimeError::Killed(Kill::Watchdog))
    ));
    assert!(matches!(
        bot.wait(TIMEOUT),
        Err(RuntimeError::Killed(Kill::Watchdog))
    ));
    let cpu = bot.join_cpu_time().unwrap();
    assert!(
        cpu >= Duration::from_millis(200),
        "stopped after {cpu:?} CPU"
    );
    assert!(
        cpu < Duration::from_millis(250),
        "stopped after {cpu:?} CPU"
    );
}

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
    let bot = Bot::start(session.attach(EventInterests::ALL).unwrap()).unwrap();
    #[cfg(feature = "testing")]
    bot.eval("identity.js", "globalThis.identity = { turns: 0 }; null")
        .wait::<()>(TIMEOUT)
        .unwrap();
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
    }
}

#[cfg(feature = "testing")]
#[test]
fn a_bot_kill_is_terminal_and_its_owner_can_start_another_bot() {
    for (source, expected) in [
        ("while (true) {}", Kill::Watchdog),
        (
            "const arrays = []; while (true) { arrays.push(new Array(100000).fill(1)); }",
            Kill::HeapLimit,
        ),
    ] {
        let logs = common::logs::Logs::default();
        let dispatch = logs.dispatch();
        let _guard = tracing::dispatcher::set_default(&dispatch);
        let (_host, mut session, observer) = start();
        let bot = Bot::start(session.attach(EventInterests::ALL).unwrap()).unwrap();
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
        assert_eq!(
            logs.messages(tracing::Level::ERROR),
            vec!["bot runtime failed"]
        );
        drain(&observer);
        let _replacement = Bot::start(session.attach(EventInterests::ALL).unwrap()).unwrap();
    }
}

#[test]
fn only_scripts_react_to_counter_attacks() {
    let host = testing::headless_host();
    let (mut session, observer) = testing::start_probe(
        &host,
        common::ready_movie()
            .constant(
                "combat.getTarget",
                json!({
                    "type":"monster","monsterMapId":1,"monsterId":1,"name":"Boss",
                    "cell":"Enter","hp":100,"maxHp":100,"level":1,"race":"None","state":1
                }),
            )
            .recording("combat.cancelAutoAttack")
            .recording("combat.cancelTarget")
            .recording("world.requestMonsterDrops")
            .recording("settings.setWalkSpeed"),
    );
    become_ready(&observer);
    update(&observer, json!({"antiCounterEnabled":true}));
    let _supervisor = lucent_runtime::Supervisor::start(&mut session, None).unwrap();
    drain(&observer);
    let counter = || {
        observer.call_loader("probe.emit", vec![
            Value::String("onExtensionResponse".into()),
            Value::String(json!({"type":"json","dataObj":{
                "cmd":"cb","anims":[{"msg":"Boss prepares a counter attack","animStr":"cast","cInf":"p:7","tInf":"m:1"}]
            }}).to_string()),
        ]).recv_timeout(TIMEOUT).unwrap();
    };
    counter();
    let until = Instant::now() + Duration::from_millis(1200);
    while Instant::now() < until {
        let call = common::await_event(&observer, "onProbeCall", None);
        let Some(Value::String(method)) = call.args.first() else {
            panic!("missing method")
        };
        assert!(!method.starts_with("combat."), "bot called {method}");
    }
    let run = common::yielding(&mut session);
    counter();
    assert_eq!(
        next_call(&observer, "combat.cancelAutoAttack")[0],
        Value::String("combat.cancelAutoAttack".into())
    );
    assert_eq!(
        next_call(&observer, "combat.cancelTarget")[0],
        Value::String("combat.cancelTarget".into())
    );
    run.stop_script();
    run.wait(TIMEOUT).unwrap();
}

fn assert_only_scripts_request_monster_drops(local_move: bool) {
    let host = testing::headless_host();
    let (mut session, observer) = testing::start_probe(
        &host,
        common::ready_movie()
            .recording("world.requestMonsterDrops")
            .recording("settings.setWalkSpeed"),
    );
    become_ready(&observer);
    let _supervisor = lucent_runtime::Supervisor::start(&mut session, None).unwrap();
    drain(&observer);
    let emit = |packet| {
        observer
            .call_loader(
                "probe.emit",
                vec![
                    Value::String("onExtensionResponse".into()),
                    Value::String(json!({"type":"json","dataObj":packet}).to_string()),
                ],
            )
            .recv_timeout(TIMEOUT)
            .unwrap();
    };
    let area = json!({
        "cmd":"moveToArea","areaId":5,"areaName":"testroom-1",
        "uoBranch":[{"entID":7,"uoName":"player-a","strFrame":"Enter",
            "intHP":100,"intHPMax":100}],
        "mondef":[{"MonID":1,"strMonName":"Frogzard","sRace":"Amphibian","iLvl":3}],
        "monBranch":[{"MonID":1,"MonMapID":2,"intHP":50,"intHPMax":50}],
        "monmap":[{"MonMapID":2,"strFrame":if local_move { "r2" } else { "Enter" }}]
    });
    let location = json!({"cmd":"uotls","unm":"player-a","o":{"strFrame":"r2"}});
    emit(area.clone());
    if local_move {
        emit(location.clone());
    }
    let until = Instant::now() + Duration::from_millis(1200);
    while Instant::now() < until {
        let call = common::await_event(&observer, "onProbeCall", None);
        assert_ne!(
            call.args.first(),
            Some(&Value::String("world.requestMonsterDrops".into())),
            "bot requested monster drops, local_move={local_move}"
        );
    }
    let run = common::yielding(&mut session);
    if local_move {
        emit(location);
    } else {
        emit(area);
    }
    assert_eq!(
        next_call(&observer, "world.requestMonsterDrops"),
        vec![
            Value::String("world.requestMonsterDrops".into()),
            Value::Number(2.0)
        ],
        "script did not request the current monster, local_move={local_move}"
    );
    run.stop_script();
    run.wait(TIMEOUT).unwrap();
}

#[test]
fn only_scripts_request_monster_drops_on_join_map() {
    assert_only_scripts_request_monster_drops(false);
}

#[test]
fn only_scripts_request_monster_drops_on_player_location() {
    assert_only_scripts_request_monster_drops(true);
}

#[cfg(feature = "testing")]
#[test]
fn a_throwing_bot_turn_is_terminal() {
    let (_host, mut session, _observer) = start();
    let bot = Bot::start(session.attach(EventInterests::ALL).unwrap()).unwrap();
    bot.eval(
        "break-turn.js",
        r#"
        Array.prototype.filter = () => { throw new Error("broken bot intrinsic"); };
        null;
    "#,
    )
    .wait::<()>(TIMEOUT)
    .unwrap();
    assert!(
        matches!(bot.wait(TIMEOUT), Err(RuntimeError::JavaScript(error)) if error.contains("broken bot intrinsic"))
    );
}

#[cfg(feature = "testing")]
#[test]
fn overflow_logs_identify_the_runtime() {
    let logs = common::logs::Logs::default();
    let dispatch = logs.dispatch();
    let _guard = tracing::dispatcher::set_default(&dispatch);
    let (_host, mut session, _observer) = start();
    let flood = r#"
        for (let i = 0; i < 1100; i++) window.swf["probe.emit"]("onDebug", String(i));
        new Promise(() => {});
    "#;
    let bot = Bot::start(session.attach(EventInterests::ALL).unwrap()).unwrap();
    assert!(matches!(
        bot.eval("flood.js", flood).wait::<()>(TIMEOUT),
        Err(RuntimeError::EventsLost)
    ));
    assert!(
        logs.messages(tracing::Level::ERROR)
            .iter()
            .any(|text| text == "bot events overflowed and were lost")
    );
    let runtime =
        lucent_runtime::Runtime::start(session.attach(EventInterests::ALL).unwrap()).unwrap();
    assert!(matches!(
        runtime.eval("flood.js", flood).wait::<()>(TIMEOUT),
        Err(RuntimeError::EventsLost)
    ));
    assert!(
        logs.messages(tracing::Level::ERROR)
            .iter()
            .any(|text| text == "script events overflowed and were lost")
    );
}
