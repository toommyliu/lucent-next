mod common;

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

#[cfg(feature = "testing")]
#[test]
fn malformed_packets_do_not_stop_bot_event_delivery() {
    let host = testing::headless_host();
    let (mut session, observer) =
        testing::start_probe(&host, common::ready_movie().recording("player.walkTo"));
    become_ready(&observer);
    enter_map(&observer, "ledgermayne");
    observer
        .command(
            serde_json::from_value(json!({
                "kind":"patchAutoZone","patch":{"enabled":true,"map":"ledgermayne"}
            }))
            .unwrap(),
        )
        .recv_timeout(TIMEOUT)
        .unwrap();
    let bot = Bot::start(session.attach().unwrap()).unwrap();
    bot.eval(
        "observe-walks.js",
        r#"
        globalThis.walks = [];
        const swf = __host.swf;
        __host.swf = (method, args) => {
            if (method === "player.walkTo") walks.push(JSON.parse(args));
            return swf(method, args);
        };
        null;
        "#,
    )
    .wait::<()>(TIMEOUT)
    .unwrap();
    for (zone, expected_range) in [("A", 147..=276), ("B", 727..=852)] {
        if zone == "B" {
            observer
                .call_loader(
                    "probe.emit",
                    vec![
                        Value::String("onExtensionResponse".into()),
                        Value::String("not json".into()),
                    ],
                )
                .recv_timeout(TIMEOUT)
                .unwrap();
        }
        packet(&observer, json!({"cmd":"event","args":{"zoneSet":zone}}));
        next_call(&observer, "player.walkTo");
        let x = bot
            .eval("observed-walk.js", "walks.shift()?.[0] ?? null")
            .wait::<Option<i32>>(TIMEOUT)
            .unwrap();
        assert!(
            x.is_some_and(|x| expected_range.contains(&x)),
            "{zone}: {x:?}"
        );
    }
    assert!(matches!(
        bot.wait(Duration::ZERO),
        Err(RuntimeError::Timeout)
    ));
}

fn walks_for(link: &ScriptLink, duration: Duration) -> usize {
    let deadline = Instant::now() + duration;
    let mut walks = 0;
    while Instant::now() < deadline {
        if let Some(lucent_host::bridge::ScriptEvent::Loader { event, .. }) =
            link.try_event().unwrap()
        {
            walks += usize::from(
                event.name == "onProbeCall"
                    && event.args.first() == Some(&Value::String("player.walkTo".into())),
            );
        } else {
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    walks
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
        assert_eq!(
            logs.messages(tracing::Level::ERROR),
            vec!["bot runtime failed"]
        );
        drain(&observer);
        let _replacement = Bot::start(session.attach().unwrap()).unwrap();
    }
}

fn packet(link: &ScriptLink, data: serde_json::Value) {
    link.call_loader(
        "probe.emit",
        vec![
            Value::String("onExtensionResponse".into()),
            Value::String(json!({"type":"json","dataObj":data}).to_string()),
        ],
    )
    .recv_timeout(TIMEOUT)
    .unwrap();
}

fn enter_map(link: &ScriptLink, map: &str) {
    packet(
        link,
        json!({
            "cmd":"moveToArea","areaName":format!("{map}-1"),
            "uoBranch":[{"entID":7,"uoName":"player-a","intHP":100}],
        }),
    );
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
        new Promise(() => {});
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

#[cfg(feature = "testing")]
#[test]
fn failed_auto_zone_walks_leave_the_bot_ready_for_the_next_zone() {
    let logs = common::logs::Logs::default();
    let dispatch = logs.dispatch();
    let _guard = tracing::dispatcher::set_default(&dispatch);
    let host = testing::headless_host();
    let (mut session, observer) =
        testing::start_probe(&host, common::ready_movie().recording("player.walkTo"));
    become_ready(&observer);
    enter_map(&observer, "ledgermayne");
    observer
        .command(
            serde_json::from_value(json!({
                "kind":"patchAutoZone","patch":{"enabled":true,"map":"ledgermayne"}
            }))
            .unwrap(),
        )
        .recv_timeout(TIMEOUT)
        .unwrap();
    let bot = Bot::start(session.attach().unwrap()).unwrap();
    bot.eval(
        "fail-walk.js",
        r#"
        const swf = __host.swf;
        globalThis.walkAttempts = 0;
        __host.swf = (method, args) => {
            if (method === "player.walkTo") {
                walkAttempts++;
                throw new Error("temporary walk failure");
            }
            return swf(method, args);
        };
        globalThis.restoreWalk = () => { __host.swf = swf; };
        null;
        "#,
    )
    .wait::<()>(TIMEOUT)
    .unwrap();
    packet(&observer, json!({"cmd":"event","args":{"zoneSet":"A"}}));
    let attempts = bot
        .eval(
            "wait-for-walk.js",
            r#"new Promise(resolve => {
                const check = () => walkAttempts > 0
                    ? setTimeout(() => resolve(walkAttempts), 700) : setTimeout(check, 1);
                check();
            });"#,
        )
        .wait::<usize>(TIMEOUT)
        .unwrap();
    assert!(matches!(
        bot.wait(Duration::ZERO),
        Err(RuntimeError::Timeout)
    ));
    bot.eval("restore-walk.js", "restoreWalk(); null;")
        .wait::<()>(TIMEOUT)
        .unwrap();
    drain(&observer);
    packet(&observer, json!({"cmd":"event","args":{"zoneSet":"B"}}));
    let call = next_call(&observer, "player.walkTo");
    assert!(matches!(call.get(1), Some(Value::Number(x)) if (727.0..=852.0).contains(x)));
    assert!(matches!(
        bot.wait(Duration::ZERO),
        Err(RuntimeError::Timeout)
    ));
    assert_eq!(
        attempts, 1,
        "a failed loader walk must finish without retrying"
    );
    assert!(logs.messages(tracing::Level::WARN).is_empty());
}

#[test]
fn auto_zone_follows_a_script_toggle_through_kills() {
    let host = testing::headless_host();
    let (mut session, observer) =
        testing::start_probe(&host, common::ready_movie().recording("player.walkTo"));
    become_ready(&observer);
    enter_map(&observer, "ledgermayne");
    let bot = Bot::start(session.attach().unwrap()).unwrap();
    let run = common::started_run(
        &mut session,
        r#"
        import * as autozone from "lucent/autozone";
        import * as api from "lucent/api";
        export default function* () {
            yield* autozone.setMap("ledgermayne");
            const state = yield* autozone.setEnabled(true);
            if (!state.enabled || state.map !== "ledgermayne") throw new Error("wrong toggle reply");
            yield* api.packet.sendToClient("started");
            while (true) {}
        }
    "#,
    );
    assert!(matches!(
        run.wait(TIMEOUT),
        Err(RuntimeError::Killed(Kill::Watchdog))
    ));
    common::start(
        &mut session,
        common::loose(&[(
            "read.js",
            r#"
        import * as autozone from "lucent/autozone";
        export default function* () {
            const state = yield* autozone.getState();
            if (!state.enabled || state.map !== "ledgermayne") throw new Error("lost auto zone");
        }
    "#,
        )]),
        Default::default(),
    )
    .wait(TIMEOUT)
    .map(|outcome| {
        assert_eq!(outcome, lucent_protocol::script::ScriptOutcome::Completed);
    })
    .unwrap();
    drain(&observer);
    packet(&observer, json!({"cmd":"event","args":{"zoneSet":"A"}}));
    let call = next_call(&observer, "player.walkTo");
    assert!(matches!(call.get(1), Some(Value::Number(x)) if (147.0..=276.0).contains(x)));
    assert!(matches!(
        bot.wait(Duration::ZERO),
        Err(RuntimeError::Timeout)
    ));
}

#[test]
fn a_new_bot_reads_the_auto_zone_config_and_scripts_can_clear_it() {
    let host = testing::headless_host();
    let (mut session, observer) =
        testing::start_probe(&host, common::ready_movie().recording("player.walkTo"));
    become_ready(&observer);
    let run = common::start(
        &mut session,
        common::loose(&[(
            "config.js",
            r#"
        import * as autozone from "lucent/autozone";
        export default function* () {
            yield* autozone.setMap("ledgermayne");
            yield* autozone.setEnabled(true);
            const state = yield* autozone.getState();
            if (!state.enabled || state.map !== "ledgermayne") throw new Error("stale auto zone read");
        }
    "#,
        )]),
        Default::default(),
    );
    assert_eq!(
        run.wait(TIMEOUT).unwrap(),
        lucent_protocol::script::ScriptOutcome::Completed
    );
    let bot = Bot::start(session.attach().unwrap()).unwrap();
    enter_map(&observer, "ledgermayne");
    drain(&observer);
    packet(&observer, json!({"cmd":"event","args":{"zoneSet":"B"}}));
    let call = next_call(&observer, "player.walkTo");
    assert!(matches!(call.get(1), Some(Value::Number(x)) if (727.0..=852.0).contains(x)));
    assert!(matches!(
        bot.wait(Duration::ZERO),
        Err(RuntimeError::Timeout)
    ));
    let run = common::start(
        &mut session,
        common::loose(&[(
            "clear.js",
            r#"
        import * as autozone from "lucent/autozone";
        export default function* () {
            const state = yield* autozone.setMap(undefined);
            if (!state.enabled || state.map !== undefined || (yield* autozone.getMap()) !== undefined) throw new Error("map not cleared");
        }
    "#,
        )]),
        Default::default(),
    );
    assert_eq!(
        run.wait(TIMEOUT).unwrap(),
        lucent_protocol::script::ScriptOutcome::Completed
    );
    drain(&observer);
    packet(&observer, json!({"cmd":"event","args":{"zoneSet":"A"}}));
    assert_eq!(
        walks_for(&observer, Duration::from_millis(1200)),
        0,
        "the cleared map still moved the player"
    );
    assert!(matches!(
        bot.wait(Duration::ZERO),
        Err(RuntimeError::Timeout)
    ));
}

#[cfg(feature = "testing")]
#[test]
fn auto_zone_rereads_the_host_record_after_the_bot_link_overflows() {
    let host = testing::headless_host();
    let (mut session, observer) = testing::start_probe(
        &host,
        common::ready_movie()
            .recording("player.walkTo")
            .raising_many("probe.overflow", "onDebug", "noise", 100),
    );
    become_ready(&observer);
    observer
        .command(
            serde_json::from_value(json!({
                "kind":"patchAutoZone","patch":{"enabled":true,"map":"ledgermayne"}
            }))
            .unwrap(),
        )
        .recv_timeout(TIMEOUT)
        .unwrap();
    let bot = Bot::start(session.attach().unwrap()).unwrap();
    bot.eval(
        "overflow.js",
        r#"
        for (let i = 0; i < 11; i++) __host.swf("probe.overflow", "[]");
        __host.command(JSON.stringify({kind:"patchAutoZone",patch:{map:"moreskulls"}}));
        null;
    "#,
    )
    .wait::<()>(TIMEOUT)
    .unwrap();
    assert!(
        observer.overflowed(),
        "the burst did not overflow the links"
    );
    assert_eq!(
        observer.try_event(),
        Err(lucent_host::bridge::Error::Overflow)
    );
    bot.eval("barrier.js", "null").wait::<()>(TIMEOUT).unwrap();
    enter_map(&observer, "moreskulls");
    drain(&observer);
    packet(&observer, json!({"cmd":"event","args":{"zoneSet":"A"}}));
    let call = next_call(&observer, "player.walkTo");
    assert!(matches!(call.get(1), Some(Value::Number(x)) if (696.0..=802.0).contains(x)));
}

#[cfg(feature = "testing")]
#[test]
fn auto_zone_overflow_host_failures_retry_and_warn_once_per_streak() {
    let logs = common::logs::Logs::default();
    let dispatch = logs.dispatch();
    let _guard = tracing::dispatcher::set_default(&dispatch);
    let host = testing::headless_host();
    let (mut session, observer) = testing::start_probe(
        &host,
        common::ready_movie()
            .recording("player.walkTo")
            .raising_many("probe.overflow", "onDebug", "noise", 100),
    );
    become_ready(&observer);
    let bot = Bot::start(session.attach().unwrap()).unwrap();
    bot.eval("overflow-read-failure.js", r#"
        const original = __host.read;
        globalThis.autoZoneReads = 0;
        __host.read = json => {
            if (JSON.parse(json).kind === "autoZone" && ++autoZoneReads <= 2) {
                throw new Error("temporary auto zone host failure");
            }
            return original(json);
        };
        for (let i = 0; i < 11; i++) __host.swf("probe.overflow", "[]");
        __host.command(JSON.stringify({kind:"patchAutoZone",patch:{enabled:true,map:"moreskulls"}}));
        null;
    "#).wait::<()>(TIMEOUT).unwrap();
    logs.wait_for(tracing::Level::INFO, "auto zone host read recovered");
    assert_eq!(
        bot.eval("read-attempts.js", "autoZoneReads")
            .wait::<usize>(TIMEOUT)
            .unwrap(),
        3
    );
    let warnings = logs.messages(tracing::Level::WARN);
    assert_eq!(
        warnings
            .iter()
            .filter(|message| message.contains("auto zone") && message.contains("failed"))
            .count(),
        1,
        "{warnings:?}"
    );
    let recoveries = logs.messages(tracing::Level::INFO);
    assert_eq!(
        recoveries
            .iter()
            .filter(|message| message.contains("auto zone") && message.contains("recovered"))
            .count(),
        1,
        "{recoveries:?}"
    );
    assert_eq!(
        observer.try_event(),
        Err(lucent_host::bridge::Error::Overflow)
    );
    enter_map(&observer, "moreskulls");
    drain(&observer);
    packet(&observer, json!({"cmd":"event","args":{"zoneSet":"A"}}));
    let call = next_call(&observer, "player.walkTo");
    assert!(matches!(call.get(1), Some(Value::Number(x)) if (696.0..=802.0).contains(x)));
}

#[cfg(feature = "testing")]
#[test]
fn malformed_auto_zone_overflow_replies_retry_and_recover() {
    for reply in ["{", "{}", r#"{"enabled":"yes","map":null}"#] {
        let logs = common::logs::Logs::default();
        let dispatch = logs.dispatch();
        let _guard = tracing::dispatcher::set_default(&dispatch);
        let host = testing::headless_host();
        let (mut session, observer) = testing::start_probe(
            &host,
            common::ready_movie()
                .recording("player.walkTo")
                .raising_many("probe.overflow", "onDebug", "noise", 100),
        );
        become_ready(&observer);
        let bot = Bot::start(session.attach().unwrap()).unwrap();
        bot.eval(
            "malformed-overflow-read.js",
            format!(
                r#"
            const original = __host.read;
            globalThis.autoZoneReads = 0;
            __host.read = json => JSON.parse(json).kind === "autoZone" && ++autoZoneReads <= 2
                ? {reply} : original(json);
            for (let i = 0; i < 11; i++) __host.swf("probe.overflow", "[]");
            __host.command(JSON.stringify({{kind:"patchAutoZone",patch:{{enabled:true,map:"moreskulls"}}}}));
            null;
        "#,
                reply = serde_json::to_string(reply).unwrap()
            ),
        )
        .wait::<()>(TIMEOUT)
        .unwrap();
        logs.wait_for(tracing::Level::INFO, "auto zone host read recovered");
        assert_eq!(
            bot.eval("attempts.js", "autoZoneReads")
                .wait::<u32>(TIMEOUT)
                .unwrap(),
            3
        );
        let warnings = logs.messages(tracing::Level::WARN);
        assert_eq!(
            warnings
                .iter()
                .filter(|message| message.contains("auto zone") && message.contains("failed"))
                .count(),
            1,
            "{warnings:?}"
        );
        assert!(
            warnings
                .iter()
                .any(|message| message.contains("SyntaxError") || message.contains("SchemaError")),
            "{warnings:?}"
        );
        assert_eq!(
            logs.messages(tracing::Level::INFO)
                .iter()
                .filter(|message| message.contains("auto zone") && message.contains("recovered"))
                .count(),
            1
        );
        assert_eq!(
            observer.try_event(),
            Err(lucent_host::bridge::Error::Overflow)
        );
        enter_map(&observer, "moreskulls");
        drain(&observer);
        packet(&observer, json!({"cmd":"event","args":{"zoneSet":"A"}}));
        let call = next_call(&observer, "player.walkTo");
        assert!(matches!(call.get(1), Some(Value::Number(x)) if (696.0..=802.0).contains(x)));
        assert!(matches!(
            bot.wait(Duration::ZERO),
            Err(RuntimeError::Timeout)
        ));
    }
}

#[cfg(feature = "testing")]
#[test]
fn auto_zone_policy_defects_retry_and_recover() {
    let logs = common::logs::Logs::default();
    let dispatch = logs.dispatch();
    let _guard = tracing::dispatcher::set_default(&dispatch);
    let (_host, mut session, _observer) = start();
    let bot = Bot::start(session.attach().unwrap()).unwrap();
    bot.eval(
        "auto-zone-defect.js",
        r#"
        globalThis.configAttempts = 0;
        setTimeout(() => window.onAutoZoneChanged({
            get enabled() {
                if (++configAttempts <= 2) throw new Error("auto zone policy defect");
                return false;
            },
            map: null
        }), 10);
        null;
    "#,
    )
    .wait::<()>(TIMEOUT)
    .unwrap();
    logs.wait_for(tracing::Level::INFO, "auto zone recovered");
    assert_eq!(
        bot.eval("attempts.js", "configAttempts")
            .wait::<u32>(TIMEOUT)
            .unwrap(),
        3
    );
    let warnings = logs.messages(tracing::Level::WARN);
    assert_eq!(
        warnings
            .iter()
            .filter(|message| message.contains("auto zone") && message.contains("failed"))
            .count(),
        1,
        "{warnings:?}"
    );
    assert!(
        warnings
            .iter()
            .any(|message| message.contains("auto zone policy defect")),
        "{warnings:?}"
    );
    assert_eq!(
        logs.messages(tracing::Level::INFO)
            .iter()
            .filter(|message| message.contains("auto zone") && message.contains("recovered"))
            .count(),
        1
    );
    assert!(matches!(
        bot.wait(Duration::ZERO),
        Err(RuntimeError::Timeout)
    ));
}

#[test]
fn auto_zone_moves_once_while_a_script_is_running() {
    let host = testing::headless_host();
    let (mut session, observer) =
        testing::start_probe(&host, common::ready_movie().recording("player.walkTo"));
    become_ready(&observer);
    enter_map(&observer, "ledgermayne");
    observer
        .command(
            serde_json::from_value(json!({
                "kind":"patchAutoZone","patch":{"enabled":true,"map":"ledgermayne"}
            }))
            .unwrap(),
        )
        .recv_timeout(TIMEOUT)
        .unwrap();
    let bot = Bot::start(session.attach().unwrap()).unwrap();
    let run = common::yielding(&mut session);
    drain(&observer);
    packet(&observer, json!({"cmd":"event","args":{"zoneSet":"A"}}));
    let call = next_call(&observer, "player.walkTo");
    assert!(matches!(call.get(1), Some(Value::Number(x)) if (147.0..=276.0).contains(x)));
    assert_eq!(
        1 + walks_for(&observer, Duration::from_millis(1200)),
        1,
        "auto zone moved in more than one runtime"
    );
    assert!(matches!(
        run.wait(Duration::ZERO),
        Err(RuntimeError::Timeout)
    ));
    assert!(matches!(
        bot.wait(Duration::ZERO),
        Err(RuntimeError::Timeout)
    ));
    run.stop_script();
    run.wait(TIMEOUT).unwrap();
}
