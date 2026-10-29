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
    assert_eq!(
        next_call(&observer, "settings.setWalkSpeed")[1],
        Value::Number(45.0)
    );
    assert!(began.elapsed() < Duration::from_millis(1500));
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
    let _bot = Bot::start(session.attach().unwrap()).unwrap();
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

#[cfg(feature = "testing")]
#[test]
fn failed_settings_ticks_warn_and_retry() {
    for boundary in ["command", "read"] {
        let logs = common::logs::Logs::default();
        let dispatch = logs.dispatch();
        let _guard = tracing::dispatcher::set_default(&dispatch);
        let (_host, mut session, observer) = start();
        let bot = Bot::start(session.attach().unwrap()).unwrap();
        next_call(&observer, "settings.setWalkSpeed");
        bot.eval(
            "fail-host.js",
            format!(
                r#"
            globalThis.attempts = [];
            const original = __host.{boundary};
            let failures = 2;
            __host.{boundary} = function (json) {{
                const input = JSON.parse(json);
                if (input.kind === "readiness" || input.kind === "reapplySettings") {{
                    attempts.push(input);
                    if (failures-- > 0) throw new Error("temporary host failure");
                }}
                return original(json);
            }};
            new Promise(resolve => setTimeout(() => resolve(null), 2300));
        "#
            ),
        )
        .wait::<()>(TIMEOUT)
        .unwrap();
        let warnings = logs.messages(tracing::Level::WARN);
        assert!(
            warnings
                .iter()
                .any(|text| text.contains("settings") && text.contains("temporary host failure")),
            "{boundary}: {warnings:?}"
        );
        assert_eq!(
            next_call(&observer, "settings.setWalkSpeed")[1],
            Value::Number(30.0)
        );
        let attempts: usize = bot
            .eval("attempts.js", "attempts.length")
            .wait(TIMEOUT)
            .unwrap();
        assert!(attempts > 2, "{boundary}: only {attempts} attempts");
        assert!(matches!(
            bot.wait(Duration::ZERO),
            Err(RuntimeError::Timeout)
        ));
    }
}

#[cfg(feature = "testing")]
#[test]
fn a_throwing_bot_turn_is_terminal() {
    let (_host, mut session, _observer) = start();
    let bot = Bot::start(session.attach().unwrap()).unwrap();
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
        new Promise(resolve => setTimeout(() => resolve(null), 10));
    "#;
    let bot = Bot::start(session.attach().unwrap()).unwrap();
    assert!(matches!(
        bot.eval("flood.js", flood).wait::<()>(TIMEOUT),
        Err(RuntimeError::EventsLost)
    ));
    assert!(
        logs.messages(tracing::Level::ERROR)
            .iter()
            .any(|text| text == "bot events overflowed and were lost")
    );
    let runtime = lucent_runtime::Runtime::start(session.attach().unwrap()).unwrap();
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

#[test]
fn walk_speed_resends_never_go_backwards() {
    let (_host, mut session, observer) = start();
    update(&observer, json!({"enemyMagnetEnabled":false}));
    drain(&observer);
    let _bot = Bot::start(session.attach().unwrap()).unwrap();
    next_call(&observer, "settings.setWalkSpeed");
    let writer = session.attach().unwrap();
    std::thread::scope(|scope| {
        scope.spawn(move || {
            for speed in 31..=530 {
                update(&writer, json!({"walkSpeed":speed}));
                drain(&writer);
                std::thread::sleep(Duration::from_millis(4));
            }
        });
        let mut previous = 30.0;
        let mut resends = 0;
        while previous < 530.0 || resends == 0 {
            let call = next_call(&observer, "settings.setWalkSpeed");
            let Value::Number(speed) = call[1] else {
                panic!("missing walk speed")
            };
            assert!(
                speed >= previous,
                "walk speed went backwards from {previous} to {speed}"
            );
            resends += usize::from(speed == previous);
            previous = speed;
        }
        assert_eq!(previous, 530.0);
    });
}

#[cfg(feature = "testing")]
#[test]
fn a_policy_defect_ends_the_bot_instead_of_silencing_a_tick() {
    let (_host, mut session, observer) = start();
    let bot = Bot::start(session.attach().unwrap()).unwrap();
    next_call(&observer, "settings.setWalkSpeed");
    bot.eval(
        "malformed-readiness.js",
        r#"
        const read = __host.read;
        __host.read = query => JSON.parse(query).kind === "readiness" ? "{}" : read(query);
        null;
    "#,
    )
    .wait::<()>(TIMEOUT)
    .unwrap();
    assert!(
        matches!(bot.wait(Duration::from_secs(2)), Err(RuntimeError::JavaScript(error)) if error.contains("length"))
    );
}
