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
    let _bot = Bot::start(session.attach(EventInterests::ALL).unwrap()).unwrap();
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
    let _bot = Bot::start(session.attach(EventInterests::ALL).unwrap()).unwrap();
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
    let bot = Bot::start(session.attach(EventInterests::ALL).unwrap()).unwrap();
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
    let (_host, mut session, _observer) = start();
    let bot = Bot::start(session.attach(EventInterests::ALL).unwrap()).unwrap();
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
        let (_host, mut session, _observer) = start();
        let bot = Bot::start(session.attach(EventInterests::ALL).unwrap()).unwrap();
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
    let (_host, mut session, _observer) = start();
    let bot = Bot::start(session.attach(EventInterests::ALL).unwrap()).unwrap();
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
        let bot = Bot::start(session.attach(EventInterests::ALL).unwrap()).unwrap();
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
    let _bot = Bot::start(session.attach(EventInterests::ALL).unwrap()).unwrap();
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
        let (_host, mut session, _observer) = start();
        let bot = Bot::start(session.attach(EventInterests::ALL).unwrap()).unwrap();
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
    let (_host, mut session, _observer) = start();
    let bot = Bot::start(session.attach(EventInterests::ALL).unwrap()).unwrap();
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
