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

#[cfg(feature = "testing")]
fn wait_for_recovery(logs: &common::logs::Logs, message: &str) {
    let deadline = Instant::now() + TIMEOUT;
    while !logs
        .messages(tracing::Level::INFO)
        .iter()
        .any(|text| text == message)
    {
        assert!(Instant::now() < deadline, "did not receive {message}");
        std::thread::sleep(Duration::from_millis(1));
    }
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
        assert_eq!(
            next_call(&observer, "settings.setWalkSpeed")[1],
            Value::Number(30.0)
        );
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

fn assert_only_scripts_request_monster_drops(local_move: bool) {
    let host = testing::headless_host();
    let (mut session, observer) = testing::start_probe(
        &host,
        common::ready_movie()
            .recording("world.requestMonsterDrops")
            .recording("settings.setWalkSpeed"),
    );
    become_ready(&observer);
    let bot = Bot::start(session.attach().unwrap()).unwrap();
    next_call(&observer, "settings.setWalkSpeed");
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
    assert!(matches!(
        bot.wait(Duration::ZERO),
        Err(RuntimeError::Timeout)
    ));
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
            null;
        "#
            ),
        )
        .wait::<()>(TIMEOUT)
        .unwrap();
        wait_for_recovery(&logs, "settings policy recovered");
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
fn settings_policy_host_failures_retry_once_per_streak() {
    for boundary in ["read", "swf", "command"] {
        let logs = common::logs::Logs::default();
        let dispatch = logs.dispatch();
        let _guard = tracing::dispatcher::set_default(&dispatch);
        let (_host, mut session, observer) = start();
        let bot = Bot::start(session.attach().unwrap()).unwrap();
        next_call(&observer, "settings.setWalkSpeed");
        bot.eval(
            "fail-boundary.js",
            format!(
                r#"
                globalThis.failures = 0;
                const original = __host.{boundary};
                __host.{boundary} = (...args) => {{
                    if (++failures <= 3) throw new SyntaxError("host link unavailable");
                    return original(...args);
                }};
                new Promise(resolve => {{
                    const log = __host.log;
                    __host.log = (level, text) => {{
                        log(level, text);
                        if (level === "info" && text === "settings policy recovered") {{
                            __host.log = log;
                            resolve(null);
                        }}
                    }};
                }});
                "#
            ),
        )
        .wait::<()>(TIMEOUT)
        .unwrap();
        let warnings = logs.messages(tracing::Level::WARN);
        assert_eq!(
            warnings
                .iter()
                .filter(|text| text.contains("settings policy tick failed"))
                .count(),
            1,
            "{boundary}: {warnings:?}"
        );
        let expected_line = if boundary == "read" {
            "settings policy tick failed SyntaxError: host link unavailable"
        } else {
            "[cause]: SyntaxError: host link unavailable"
        };
        assert!(
            warnings[0].lines().any(|line| line.trim() == expected_line),
            "{warnings:?}"
        );
        let recovered = logs.messages(tracing::Level::INFO);
        assert_eq!(
            recovered
                .iter()
                .filter(|text| text.contains("settings policy recovered"))
                .count(),
            1,
            "{boundary}: {recovered:?}"
        );
        assert!(
            bot.eval("attempts.js", "failures")
                .wait::<u32>(TIMEOUT)
                .unwrap()
                > 3
        );
        drain(&observer);
        assert_eq!(
            next_call(&observer, "settings.setWalkSpeed")[1],
            Value::Number(30.0)
        );
        assert!(matches!(
            bot.wait(Duration::ZERO),
            Err(RuntimeError::Timeout)
        ));
    }
}

#[cfg(feature = "testing")]
fn assert_policy_retries(source: &str, expected_line: &str) {
    let logs = common::logs::Logs::default();
    let dispatch = logs.dispatch();
    let _guard = tracing::dispatcher::set_default(&dispatch);
    let (_host, mut session, observer) = start();
    let bot = Bot::start(session.attach().unwrap()).unwrap();
    next_call(&observer, "settings.setWalkSpeed");
    bot.eval("break-policy.js", source)
        .wait::<()>(TIMEOUT)
        .unwrap();
    bot.eval(
        "observe-failures.js",
        r#"new Promise(resolve => {
            const check = () => attempts >= 3 ? resolve(null) : setTimeout(check, 1);
            check();
        });"#,
    )
    .wait::<()>(TIMEOUT)
    .unwrap();
    let warnings = logs.messages(tracing::Level::WARN);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(
        warnings[0].lines().any(|line| line.trim() == expected_line),
        "expected {expected_line:?} in {warnings:?}"
    );
    assert!(warnings[0].lines().count() > 1, "{warnings:?}");
    assert!(logs.messages(tracing::Level::INFO).is_empty());
    assert!(
        bot.eval("attempts.js", "attempts")
            .wait::<u32>(TIMEOUT)
            .unwrap()
            >= 3
    );
    bot.eval("restore-policy.js", "restore(); null;")
        .wait::<()>(TIMEOUT)
        .unwrap();
    drain(&observer);
    assert_eq!(
        next_call(&observer, "settings.setWalkSpeed")[1],
        Value::Number(30.0)
    );
    wait_for_recovery(&logs, "settings policy recovered");
    assert_eq!(
        logs.messages(tracing::Level::INFO),
        vec!["settings policy recovered"]
    );
    assert_eq!(logs.messages(tracing::Level::WARN), warnings);
    assert!(matches!(
        bot.wait(Duration::ZERO),
        Err(RuntimeError::Timeout)
    ));
}

#[cfg(feature = "testing")]
#[test]
fn settings_policy_code_errors_warn_once_and_retry() {
    assert_policy_retries(
        r#"
        globalThis.attempts = 0;
        const command = __host.command;
        __host.command = undefined;
        const stringify = JSON.stringify;
        JSON.stringify = value => {
            if (value.kind === "refreshLoggedIn" || value.kind === "reapplySettings") attempts++;
            return stringify(value);
        };
        globalThis.restore = () => {
            __host.command = command;
            JSON.stringify = stringify;
        };
        null;
        "#,
        "settings policy tick failed Error: Failed refreshLoggedIn",
    );
    assert_policy_retries(
        r#"
        globalThis.attempts = 0;
        const stringify = JSON.stringify;
        JSON.stringify = value => {
            if (value.kind === "reapplySettings") {
                attempts++;
                throw new Error("broken policy serialization");
            }
            return stringify(value);
        };
        globalThis.restore = () => { JSON.stringify = stringify; };
        null;
        "#,
        "[cause]: Error: broken policy serialization",
    );
}

#[cfg(feature = "testing")]
#[test]
fn settings_policy_malformed_host_replies_warn_once_and_retry() {
    for (boundary, reply, expected_line) in [
        (
            "read",
            "not json",
            "settings policy tick failed SyntaxError: Unexpected token 'o', \"not json\" is not valid JSON",
        ),
        (
            "read",
            "{}",
            "settings policy tick failed TypeError: Cannot read properties of undefined (reading 'length')",
        ),
        (
            "swf",
            "not json",
            "[cause]: SyntaxError: Unexpected token 'o', \"not json\" is not valid JSON",
        ),
        ("swf", r#""true""#, "[cause]: SchemaError: Expected boolean"),
        (
            "command",
            "not json",
            "[cause]: SyntaxError: Unexpected token 'o', \"not json\" is not valid JSON",
        ),
    ] {
        assert_policy_retries(
            &format!(
                r#"
                globalThis.attempts = 0;
                const original = __host.{boundary};
                __host.{boundary} = (...args) => {{ attempts++; return {}; }};
                globalThis.restore = () => {{ __host.{boundary} = original; }};
                null;
                "#,
                serde_json::to_string(reply).unwrap()
            ),
            expected_line,
        );
    }
}

#[cfg(feature = "testing")]
#[test]
fn settings_policy_uses_player_readiness() {
    let (_host, mut session, observer) = start();
    let bot = Bot::start(session.attach().unwrap()).unwrap();
    next_call(&observer, "settings.setWalkSpeed");
    bot.eval(
        "not-logged-in.js",
        r#"
        globalThis.reapplications = 0;
        globalThis.readinessChecks = 0;
        const command = __host.command;
        __host.command = json => {
            const input = JSON.parse(json);
            if (input.kind === "refreshLoggedIn") { readinessChecks++; return "false"; }
            if (input.kind === "reapplySettings") reapplications++;
            return command(json);
        };
        new Promise(resolve => {
            const check = () => readinessChecks >= 3 ? resolve(null) : setTimeout(check, 1);
            check();
        });
    "#,
    )
    .wait::<()>(TIMEOUT)
    .unwrap();
    assert_eq!(
        bot.eval("count.js", "reapplications")
            .wait::<u32>(TIMEOUT)
            .unwrap(),
        0
    );
    bot.eval("ready-again.js", "__host.command = command; null;")
        .wait::<()>(TIMEOUT)
        .unwrap();
    drain(&observer);
    assert_eq!(
        next_call(&observer, "settings.setWalkSpeed")[1],
        Value::Number(30.0)
    );
}

#[cfg(feature = "testing")]
#[test]
fn settings_policy_recovery_waits_for_every_failing_loop() {
    let logs = common::logs::Logs::default();
    let dispatch = logs.dispatch();
    let _guard = tracing::dispatcher::set_default(&dispatch);
    let (_host, mut session, observer) = start();
    let bot = Bot::start(session.attach().unwrap()).unwrap();
    next_call(&observer, "settings.setWalkSpeed");
    bot.eval(
        "fail-full-ticks.js",
        r#"
        globalThis.fullFailures = 0;
        globalThis.actionFailures = 0;
        globalThis.actionsRecovered = false;
        const command = __host.command;
        __host.command = json => {
            const input = JSON.parse(json);
            if (input.kind === "reapplySettings" && !input.actionsOnly) {
                fullFailures++;
                throw new Error("full reapply unavailable");
            }
            if (input.kind === "reapplySettings" && !actionsRecovered) {
                actionFailures++;
                throw new Error("actions reapply unavailable");
            }
            return command(json);
        };
        new Promise(resolve => {
            const check = () => fullFailures >= 3 && actionFailures >= 3
                ? resolve(null) : setTimeout(check, 10);
            check();
        });
    "#,
    )
    .wait::<()>(TIMEOUT)
    .unwrap();
    assert_eq!(
        bot.eval("failures.js", "fullFailures")
            .wait::<u32>(TIMEOUT)
            .unwrap(),
        3
    );
    let warnings = logs.messages(tracing::Level::WARN);
    let recoveries = logs.messages(tracing::Level::INFO);
    assert_eq!(
        (
            warnings
                .iter()
                .filter(|text| text.contains("settings policy tick failed"))
                .count(),
            recoveries
                .iter()
                .filter(|text| text.contains("settings policy recovered"))
                .count()
        ),
        (1, 0),
        "warnings: {warnings:?}, recoveries: {recoveries:?}"
    );
    drain(&observer);
    bot.eval("recover-actions.js", "actionsRecovered = true; null;")
        .wait::<()>(TIMEOUT)
        .unwrap();
    next_call(&observer, "settings.enemyMagnet");
    assert_eq!(logs.messages(tracing::Level::WARN), warnings);
    assert!(logs.messages(tracing::Level::INFO).is_empty());
    bot.eval("restore-command.js", "__host.command = command; null;")
        .wait::<()>(TIMEOUT)
        .unwrap();
    drain(&observer);
    assert_eq!(
        next_call(&observer, "settings.setWalkSpeed")[1],
        Value::Number(30.0)
    );
    wait_for_recovery(&logs, "settings policy recovered");
    assert_eq!(
        logs.messages(tracing::Level::INFO)
            .iter()
            .filter(|text| text.contains("settings policy recovered"))
            .count(),
        1
    );
}

#[cfg(feature = "testing")]
#[test]
fn settings_policy_missing_loader_booleans_warn_once_and_retry() {
    for value in ["undefined", "null"] {
        assert_policy_retries(
            &format!(
                r#"
                globalThis.attempts = 0;
                const swf = __host.swf;
                __host.swf = () => {{ attempts++; return {value}; }};
                globalThis.restore = () => {{ __host.swf = swf; }};
                null;
                "#
            ),
            "[cause]: SchemaError: Expected boolean",
        );
    }
}

#[cfg(feature = "testing")]
#[test]
fn settings_policy_invalid_reapply_reply_warns_once_and_retries() {
    for reply in ["false", "{}"] {
        assert_policy_retries(
            &format!(
                r#"
                globalThis.attempts = 0;
                const command = __host.command;
                __host.command = json => {{
                    if (JSON.parse(json).kind === "reapplySettings") {{
                        attempts++;
                        return {reply};
                    }}
                    return command(json);
                }};
                globalThis.restore = () => {{ __host.command = command; }};
                null;
                "#,
                reply = serde_json::to_string(reply).unwrap()
            ),
            "settings policy tick failed Error: Invalid reapplySettings reply",
        );
    }
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
    wait_for_recovery(&logs, "auto zone host read recovered");
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
        wait_for_recovery(&logs, "auto zone host read recovered");
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
    wait_for_recovery(&logs, "auto zone recovered");
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

#[test]
fn set_enabled_returns_the_state_after_capture() {
    let host = testing::headless_host();
    let (mut session, observer) = testing::start_probe(
        &host,
        common::ready_movie()
            .constant("auth.getUsername", json!("player-a"))
            .constant("auth.getPassword", json!("secret")),
    );
    become_ready(&observer);
    let _bot = Bot::start(session.attach().unwrap()).unwrap();
    let run = common::start(
        &mut session,
        common::loose(&[(
            "enable.js",
            r#"
        import * as relogin from "lucent/autorelogin";
        export default function* () {
            const state = yield* relogin.setEnabled(true);
            if (!state.enabled || !state.captured) throw new Error(JSON.stringify(state));
        }
    "#,
        )]),
        Default::default(),
    );
    assert_eq!(
        run.wait(TIMEOUT).unwrap(),
        lucent_protocol::script::ScriptOutcome::Completed
    );
}

#[test]
fn relogin_setters_publish_sanitized_records_before_they_answer() {
    let (_host, mut session, observer) = start();
    let _bot = Bot::start(session.attach().unwrap()).unwrap();
    let run = common::start(
        &mut session,
        common::loose(&[(
            "configure.js",
            r#"
        import * as relogin from "lucent/autorelogin";
        export default function* () {
            yield* relogin.setDelay("7 seconds");
            yield* relogin.setServer("  Artix  ");
            for (let i = 0; i < 2; i++) {
                const state = yield* relogin.setEnabled(true);
                const read = yield* relogin.getState();
                if (!state.captured || JSON.stringify(state) !== JSON.stringify(read)) throw new Error(JSON.stringify({state, read}));
                if (state.username !== "player-a" || "password" in state) throw new Error("Captured credentials escaped");
            }
        }
    "#,
        )]),
        Default::default(),
    );
    assert_eq!(
        run.wait(TIMEOUT).unwrap(),
        lucent_protocol::script::ScriptOutcome::Completed
    );
    let records = session.auto_relogin();
    assert!(records.config.enabled);
    assert_eq!(records.config.delay_ms, 7000.0);
    assert_eq!(records.config.server.as_deref(), Some("Artix"));
    assert!(records.status.captured);
    let serialized = serde_json::to_string(&records).unwrap();
    assert!(serialized.contains("player-a"));
    assert!(!serialized.contains("password"));
    assert!(!serialized.contains("secret"));
    assert_eq!(
        observer
            .query(lucent_protocol::query::Query::AutoRelogin {})
            .unwrap(),
        serialized
    );
    assert_eq!(session.pending_ask_count(), 0);
}

#[cfg(feature = "testing")]
#[test]
fn set_enabled_captures_cached_credentials_when_refresh_fails() {
    let host = testing::headless_host();
    let (mut session, observer) = testing::start_probe(
        &host,
        common::ready_movie()
            .constant("auth.getUsername", json!("player-a"))
            .constant("auth.getPassword", json!("secret")),
    );
    become_ready(&observer);
    observer
        .command(Command::RefreshCredentials {})
        .recv_timeout(TIMEOUT)
        .unwrap();
    let bot = Bot::start(session.attach().unwrap()).unwrap();
    bot.eval(
        "fail-credential-refresh.js",
        r#"
        const command = __host.command;
        __host.command = json => {
            if (JSON.parse(json).kind === "refreshCredentials") throw new Error("player restarting");
            return command(json);
        };
        null;
    "#,
    )
    .wait::<()>(TIMEOUT)
    .unwrap();
    let run = common::start(
        &mut session,
        common::loose(&[(
            "enable.js",
            r#"
        import * as relogin from "lucent/autorelogin";
        export default function* () {
            const state = yield* relogin.setEnabled(true);
            if (state.enabled !== true || state.captured !== true || state.username !== "player-a"
                || state.attempting !== false || state.waitingDelay !== false || state.delayMs !== 3000) {
                throw new Error(JSON.stringify(state));
            }
            const read = yield* relogin.getState();
            if (JSON.stringify(read) !== JSON.stringify(state)) throw new Error(JSON.stringify({ state, read }));
        }
    "#,
        )]),
        Default::default(),
    );
    assert_eq!(
        run.wait(TIMEOUT).unwrap(),
        lucent_protocol::script::ScriptOutcome::Completed
    );
    let records = session.auto_relogin();
    assert!(records.config.enabled);
    assert!(records.status.captured);
    assert_eq!(records.status.username.as_deref(), Some("player-a"));
    assert_eq!(records.status.last_error, None);
    assert_eq!(session.pending_ask_count(), 0);
}

#[cfg(feature = "testing")]
#[test]
fn relogin_boundary_failures_fall_back_to_cached_credentials() {
    let logs = common::logs::Logs::default();
    let dispatch = logs.dispatch();
    let _guard = tracing::dispatcher::set_default(&dispatch);
    let (_host, mut session, observer) = start();
    let bot = Bot::start(session.attach().unwrap()).unwrap();
    next_call(&observer, "settings.setWalkSpeed");
    bot.eval("fail-capture.js", r#"
        const command = __host.command;
        let failures = 2;
        __host.command = json => {
            if (JSON.parse(json).kind === "refreshCredentials" && failures-- > 0) throw new Error("temporary capture failure");
            return command(json);
        };
        null;
    "#).wait::<()>(TIMEOUT).unwrap();
    for _ in 0..3 {
        let run = common::start(
            &mut session,
            common::loose(&[(
                "capture.js",
                r#"
        import * as relogin from "lucent/autorelogin";
        export default function* () {
            if (!(yield* relogin.setEnabled(true)).captured) throw new Error("capture did not recover");
        }
    "#,
            )]),
            Default::default(),
        );
        assert_eq!(
            run.wait(TIMEOUT).unwrap(),
            lucent_protocol::script::ScriptOutcome::Completed
        );
    }
    let warnings: Vec<_> = logs
        .messages(tracing::Level::WARN)
        .into_iter()
        .filter(|message| message.contains("auto relogin"))
        .collect();
    assert!(warnings.is_empty(), "{warnings:?}");
    assert!(matches!(
        bot.wait(Duration::ZERO),
        Err(RuntimeError::Timeout)
    ));
}

#[cfg(feature = "testing")]
#[test]
fn relogin_reply_errors_allow_the_next_capture() {
    for reply in ["{}", "{"] {
        let logs = common::logs::Logs::default();
        let dispatch = logs.dispatch();
        let _guard = tracing::dispatcher::set_default(&dispatch);
        let (_host, mut session, observer) = start();
        let bot = Bot::start(session.attach().unwrap()).unwrap();
        next_call(&observer, "settings.setWalkSpeed");
        bot.eval("malformed-capture.js", format!(r#"
            const command = __host.command;
            let failures = 2;
            __host.command = json => JSON.parse(json).kind === "refreshCredentials" && failures-- > 0 ? {reply} : command(json);
            null;
        "#, reply = serde_json::to_string(reply).unwrap())).wait::<()>(TIMEOUT).unwrap();
        for attempt in 0..3 {
            let run = common::start(
                &mut session,
                common::loose(&[(
                    "capture.js",
                    r#"
            import * as relogin from "lucent/autorelogin";
            export default function* () {
                    if (!(yield* relogin.setEnabled(true)).captured) throw new Error("capture did not recover");
            }
        "#,
                )]),
                Default::default(),
            );
            let outcome = run.wait(TIMEOUT).unwrap();
            if reply == "{}" && attempt < 2 {
                let lucent_protocol::script::ScriptOutcome::Failed { message, .. } = outcome else {
                    panic!("expected a failed ask, got {outcome:?}");
                };
                assert_eq!(message, "The bot request failed");
            } else {
                assert_eq!(outcome, lucent_protocol::script::ScriptOutcome::Completed);
            }
        }
        let warnings: Vec<_> = logs
            .messages(tracing::Level::WARN)
            .into_iter()
            .filter(|message| message.contains("auto relogin"))
            .collect();
        assert!(warnings.is_empty(), "{warnings:?}");
        assert!(matches!(
            bot.wait(Duration::ZERO),
            Err(RuntimeError::Timeout)
        ));
    }
}

#[cfg(feature = "testing")]
#[test]
fn relogin_code_failures_fail_the_ask_and_allow_the_next_setter() {
    let logs = common::logs::Logs::default();
    let dispatch = logs.dispatch();
    let _guard = tracing::dispatcher::set_default(&dispatch);
    let (_host, mut session, observer) = start();
    let bot = Bot::start(session.attach().unwrap()).unwrap();
    next_call(&observer, "settings.setWalkSpeed");
    bot.eval(
        "break-relogin.js",
        r#"
        const trim = String.prototype.trim;
        let failures = 2;
        String.prototype.trim = function () {
            if (this.toString() === "  Artix  " && failures-- > 0) throw new Error("relogin code failure");
            return trim.call(this);
        };
        null;
    "#,
    )
    .wait::<()>(TIMEOUT)
    .unwrap();
    for attempt in 0..3 {
        let run = common::start(
            &mut session,
            common::loose(&[(
                "server.js",
                r#"
        import * as relogin from "lucent/autorelogin";
        export default function* () {
            if ((yield* relogin.setServer("  Artix  ")).server !== "Artix") throw new Error("wrong server");
        }
    "#,
            )]),
            Default::default(),
        );
        let outcome = run.wait(TIMEOUT).unwrap();
        if attempt < 2 {
            assert!(
                matches!(
                    outcome,
                    lucent_protocol::script::ScriptOutcome::Failed { .. }
                ),
                "{outcome:?}"
            );
        } else {
            assert_eq!(outcome, lucent_protocol::script::ScriptOutcome::Completed);
        }
    }
    let warnings: Vec<_> = logs
        .messages(tracing::Level::WARN)
        .into_iter()
        .filter(|message| message.contains("auto relogin"))
        .collect();
    assert!(warnings.is_empty(), "{warnings:?}");
    assert!(matches!(
        bot.wait(Duration::ZERO),
        Err(RuntimeError::Timeout)
    ));
}

#[cfg(feature = "testing")]
fn assert_failed_recovery_command_backs_off(command: &str) {
    {
        let (_host, mut session, observer) = start();
        let bot = Bot::start(session.attach().unwrap()).unwrap();
        let run = common::start(
            &mut session,
            common::loose(&[(
                "enable.js",
                r#"
            import * as relogin from "lucent/autorelogin";
            export default function* () {
                yield* relogin.setDelay(0);
                yield* relogin.setServer("Artix");
                yield* relogin.setEnabled(true);
            }
        "#,
            )]),
            Default::default(),
        );
        assert_eq!(
            run.wait(TIMEOUT).unwrap(),
            lucent_protocol::script::ScriptOutcome::Completed
        );
        bot.eval("fail-recovery-command.js", format!(r#"
            const command = __host.command;
            const swf = __host.swf;
            __host.command = json => {{
                const input = JSON.parse(json);
                if (input.kind === "{command}") throw new Error("recovery command refused");
                if (input.kind === "login") return "true";
                if (input.kind === "refreshLoggedIn") return "false";
                return command(json);
            }};
            __host.swf = (method, args) => {{
                if (method === "world.isLoaded" || method === "player.isLoaded" || method === "auth.isLoggedIn") return "false";
                if (method === "flash.isNull") return "false";
                if (method === "auth.connectTo") return JSON.stringify({{ok:false,status:"blocked",reason:"full",message:"Server is full"}});
                return swf(method, args);
            }};
            new Promise(resolve => {{
                const publish = __host.publishAutoRelogin;
                __host.publishAutoRelogin = records => {{
                    publish(records);
                    if (JSON.parse(records).status.phase.tag === "backoff") {{
                        __host.publishAutoRelogin = publish;
                        resolve(null);
                    }}
                }};
                window.swf["probe.emit"]("onConnection", "OnConnectionLost");
            }});
        "#)).wait::<()>(TIMEOUT).unwrap();
        let records = session.auto_relogin();
        assert!(
            matches!(
                records.status.phase,
                lucent_protocol::auto_relogin::AutoReloginPhase::Backoff { .. }
            ),
            "{command}: {records:?}"
        );
        assert_eq!(records.status.attempts_remaining, Some(3.0));
        assert_eq!(
            records.status.last_error.as_deref(),
            Some(if command == "login" {
                "Login could not be submitted."
            } else {
                "Logout could not be submitted."
            })
        );
        drop(observer);
    }
}

#[cfg(feature = "testing")]
#[test]
fn invalid_bot_requests_keep_their_message() {
    let (_host, mut session, observer) = start();
    let _bot = Bot::start(session.attach().unwrap()).unwrap();
    let id = observer.ask(json!({"kind":"invalid"})).unwrap();
    let deadline = Instant::now() + TIMEOUT;
    loop {
        assert!(Instant::now() < deadline);
        if let Some(lucent_host::bridge::ScriptEvent::Answer {
            id: answered,
            answer,
        }) = observer.try_event().unwrap()
        {
            assert_eq!(answered, id);
            assert_eq!(answer.unwrap_err().to_string(), "Invalid bot request");
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[cfg(feature = "testing")]
#[test]
fn failed_login_commands_back_off() {
    assert_failed_recovery_command_backs_off("login");
}

#[cfg(feature = "testing")]
#[test]
fn failed_logout_commands_back_off() {
    assert_failed_recovery_command_backs_off("logout");
}

#[cfg(feature = "testing")]
fn assert_reconnect_uses_cached_auth(capture_on_reconnect: bool) {
    for cached_logged_in in [false, true] {
        let (_host, mut session, observer) = start();
        let bot = Bot::start(session.attach().unwrap()).unwrap();
        next_call(&observer, "settings.setWalkSpeed");
        bot.eval("prepare-reconnect.js", r#"
            const command = __host.command;
            const read = __host.read;
            const swf = __host.swf;
            globalThis.reconnecting = false;
            globalThis.ready = false;
            globalThis.refreshFailures = { credentials: 0, loggedIn: 0 };
            globalThis.recoveryCalls = { login: 0, connect: 0 };
            __host.command = json => {
                const input = JSON.parse(json);
                if (input.kind === "refreshCredentials") {
                    if (!reconnecting) return captureOnReconnect ? JSON.stringify({ username: "", password: "" }) : command(json);
                    refreshFailures.credentials++;
                    throw new Error("player not ready for credentials");
                }
                if (reconnecting && input.kind === "refreshLoggedIn") {
                    if (ready) return "true";
                    refreshFailures.loggedIn++;
                    throw new Error("logged-in refresh timed out");
                }
                if (input.kind === "login") { recoveryCalls.login++; return "true"; }
                return command(json);
            };
            __host.swf = (method, args) => {
                if (method === "world.isLoaded" || method === "player.isLoaded") return String(ready);
                if (method === "flash.isNull") return "false";
                if (method === "auth.connectTo") {
                    recoveryCalls.connect++;
                    ready = true;
                    return JSON.stringify({ ok: true, status: "selected", message: "Connected" });
                }
                return swf(method, args);
            };
            null;
        "#).wait::<()>(TIMEOUT).unwrap();
        bot.eval(
            "capture-mode.js",
            format!("globalThis.captureOnReconnect = {capture_on_reconnect}; null"),
        )
        .wait::<()>(TIMEOUT)
        .unwrap();
        let run = common::start(
            &mut session,
            common::loose(&[(
                "enable.js",
                r#"
                import * as relogin from "lucent/autorelogin";
                export default function* () {
                    yield* relogin.setDelay(0);
                    yield* relogin.setServer("Artix");
                    yield* relogin.setEnabled(true);
                }
            "#,
            )]),
            Default::default(),
        );
        assert_eq!(
            run.wait(TIMEOUT).unwrap(),
            lucent_protocol::script::ScriptOutcome::Completed
        );
        bot.eval(
            "reconnect.js",
            format!(
                r#"
            __host.read = json => {{
                const result = JSON.parse(read(json));
                if (JSON.parse(json).kind === "auth") {{
                    result.loggedIn = {cached_logged_in};
                    if (result.loggedIn && recoveryCalls.login > 0 && refreshFailures.loggedIn > 0) ready = true;
                }}
                if (JSON.parse(json).kind === "readiness" && ready) result.missing = [];
                return JSON.stringify(result);
            }};
            reconnecting = true;
            new Promise(resolve => {{
                const publish = __host.publishAutoRelogin;
                __host.publishAutoRelogin = records => {{
                    publish(records);
                    const status = JSON.parse(records).status;
                    if (recoveryCalls.login > 0 && status.phase.tag === "armed" && status.captured) {{
                        __host.publishAutoRelogin = publish;
                        resolve(null);
                    }}
                }};
                window.swf["probe.emit"]("onConnection", "OnConnection");
                window.swf["probe.emit"]("onConnection", "OnConnectionLost");
            }});
        "#
            ),
        )
        .wait::<()>(TIMEOUT)
        .unwrap();
        let records = session.auto_relogin();
        assert!(records.status.captured, "{records:?}");
        assert_eq!(
            records.status.phase,
            lucent_protocol::auto_relogin::AutoReloginPhase::Armed,
            "{records:?}"
        );
        assert_eq!(records.status.last_error, None);
        assert_eq!(
            session.auto_relogin().status.username.as_deref(),
            Some("player-a")
        );
        let calls = bot
            .eval(
                "recovery-calls.js",
                "({ ...recoveryCalls, ...refreshFailures })",
            )
            .wait::<serde_json::Value>(TIMEOUT)
            .unwrap();
        assert_eq!(calls["login"], 1);
        assert_eq!(calls["connect"], u32::from(!cached_logged_in));
        assert_eq!(
            calls["credentials"].as_u64().unwrap() >= 2,
            capture_on_reconnect,
            "{calls}"
        );
        assert!(calls["loggedIn"].as_u64().unwrap() >= 1, "{calls}");
    }
}

#[cfg(feature = "testing")]
#[test]
fn reconnect_reaches_armed_when_logged_in_refresh_keeps_throwing() {
    let (_host, mut session, observer) = start();
    let bot = Bot::start(session.attach().unwrap()).unwrap();
    next_call(&observer, "settings.setWalkSpeed");
    let run = common::start(
        &mut session,
        common::loose(&[(
            "enable.js",
            r#"
            import * as relogin from "lucent/autorelogin";
            export default function* () {
                yield* relogin.setDelay(0);
                yield* relogin.setServer("Artix");
                if (!(yield* relogin.setEnabled(true)).captured) throw new Error("credentials not captured");
            }
            "#,
        )]),
        Default::default(),
    );
    assert_eq!(
        run.wait(TIMEOUT).unwrap(),
        lucent_protocol::script::ScriptOutcome::Completed
    );
    bot.eval(
        "reconnect.js",
        r#"
        const command = __host.command;
        const read = __host.read;
        const swf = __host.swf;
        globalThis.ready = false;
        globalThis.refreshFailures = 0;
        globalThis.logouts = 0;
        __host.command = json => {
            const input = JSON.parse(json);
            if (input.kind === "refreshLoggedIn") {
                refreshFailures++;
                throw new Error("logged-in refresh timed out");
            }
            if (input.kind === "login") { ready = true; return "true"; }
            if (input.kind === "logout") logouts++;
            return command(json);
        };
        __host.read = json => {
            const result = JSON.parse(read(json));
            const input = JSON.parse(json);
            if (input.kind === "auth") result.loggedIn = true;
            if (input.kind === "readiness") result.missing = ready ? [] : ["player"];
            return JSON.stringify(result);
        };
        __host.swf = (method, args) => {
            if (method === "world.isLoaded" || method === "player.isLoaded") return String(ready);
            return swf(method, args);
        };
        new Promise(resolve => {
            const publish = __host.publishAutoRelogin;
            const finish = () => { __host.publishAutoRelogin = publish; resolve(null); };
            const deadline = setTimeout(finish, 2000);
            __host.publishAutoRelogin = records => {
                publish(records);
                if (ready && JSON.parse(records).status.phase.tag === "armed") {
                    clearTimeout(deadline);
                    finish();
                }
            };
            window.swf["probe.emit"]("onConnection", "OnConnectionLost");
        });
    "#,
    )
    .wait::<()>(TIMEOUT)
    .unwrap();
    let records = session.auto_relogin();
    assert_eq!(
        records.status.phase,
        lucent_protocol::auto_relogin::AutoReloginPhase::Armed,
        "{records:?}"
    );
    assert!(records.status.captured);
    assert_eq!(records.status.last_error, None);
    let calls = bot
        .eval("recovery-calls.js", "({ refreshFailures, logouts, ready })")
        .wait::<serde_json::Value>(TIMEOUT)
        .unwrap();
    assert_eq!(calls["ready"], true);
    assert!(calls["refreshFailures"].as_u64().unwrap() > 0, "{calls}");
    assert_eq!(calls["logouts"], 0);
}

#[cfg(feature = "testing")]
#[test]
fn reconnect_uses_cached_auth_when_logged_in_refresh_throws() {
    assert_reconnect_uses_cached_auth(false);
}

#[cfg(feature = "testing")]
#[test]
fn reconnect_captures_cached_credentials_when_refresh_throws() {
    assert_reconnect_uses_cached_auth(true);
}
