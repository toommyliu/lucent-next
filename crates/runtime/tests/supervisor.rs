mod common;

use common::{logs::Logs, *};
use lucent_host::{ViewEvent, Viewport, testing};
use lucent_runtime::{ScriptRequest, Supervisor};
const INFO: tracing::Level = tracing::Level::INFO;
const WARN: tracing::Level = tracing::Level::WARN;

fn session_lines(logs: &Logs, level: tracing::Level) -> Vec<String> {
    logs.messages(level)
        .into_iter()
        .filter(|s| s.starts_with("session "))
        .collect()
}

const WAIT: &str = r#"
    import * as api from "lucent/api";
    import * as script from "lucent/script";
    export default function* () {
        yield* api.packet.sendToClient("started");
        yield* script.sleep("1 hour");
    }
"#;

#[test]
fn the_daily_script_runs_after_login_and_the_bot_keeps_running() {
    let logs = Logs::default();
    let dispatch = logs.dispatch();
    let _guard = tracing::dispatcher::set_default(&dispatch);
    let host = testing::headless_host();
    let (mut session, observer) = testing::start_probe(&host, ready_movie());
    let script = lucent_host::library::load_script(
        "@lucent/dailies/scripts/farm-elders-blood.js",
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
    )
    .unwrap();
    let (request, _) = ScriptRequest::new(script);
    let _supervisor = Supervisor::start(&mut session, Some(request)).unwrap();
    logs.wait_for(
        INFO,
        "session 1: @lucent/dailies/scripts/farm-elders-blood.js is waiting for login",
    );
    assert_eq!(session_lines(&logs, INFO).len(), 2);
    log_in(&observer);
    logs.wait_for(INFO, "session 1: bot 1 is running");
    assert_eq!(
        session_lines(&logs, INFO),
        [
            "session 1: bot 1 started",
            "session 1: @lucent/dailies/scripts/farm-elders-blood.js is waiting for login",
            "session 1: @lucent/dailies/scripts/farm-elders-blood.js started",
            "session 1: @lucent/dailies/scripts/farm-elders-blood.js completed",
            "session 1: bot 1 is running",
        ]
    );
    assert_eq!(session_lines(&logs, WARN), Vec::<String>::new());
}

#[test]
fn a_watchdog_kill_leaves_the_same_bot_running() {
    let logs = Logs::default();
    let dispatch = logs.dispatch();
    let _guard = tracing::dispatcher::set_default(&dispatch);
    let host = testing::headless_host();
    let (mut session, observer) = testing::start_probe(&host, ready_movie());
    let (request, controls) = ScriptRequest::new(loose(&[(
        "loop.js",
        "export default function* () { while (true) {} }",
    )]));
    let _supervisor = Supervisor::start(&mut session, Some(request)).unwrap();
    logs.wait_for(INFO, "session 1: loop.js is waiting for login");
    log_in(&observer);
    logs.wait_for(INFO, "session 1: bot 1 is running");
    assert_eq!(
        session_lines(&logs, WARN),
        ["session 1: loop.js was killed: a turn ran for more than 200 ms"]
    );
    let first = logs.bot_turns()[0];
    let deadline = std::time::Instant::now() + TIMEOUT;
    let mut count = 0;
    loop {
        assert!(
            std::time::Instant::now() < deadline,
            "bot turns did not rise"
        );
        observer
            .call_loader(
                "probe.emit",
                vec![
                    lucent_protocol::bridge::Value::String("onDebug".into()),
                    lucent_protocol::bridge::Value::String("turn".into()),
                ],
            )
            .recv_timeout(TIMEOUT)
            .unwrap();
        controls.status();
        count += 1;
        logs.wait_for_count(INFO, "session 1: no script is running", count);
        if *logs.bot_turns().last().unwrap() > first {
            break;
        }
    }
    let info = session_lines(&logs, INFO);
    assert_eq!(
        &info[info.len() - 2..],
        [
            "session 1: bot 1 is running",
            "session 1: no script is running"
        ]
    );
    assert!(
        !session_lines(&logs, INFO)
            .iter()
            .any(|s| s == "session 1: bot 2 started")
    );
}

#[test]
fn stop_and_kill_reach_the_run_from_the_controls() {
    for kill in [false, true] {
        let logs = Logs::default();
        let dispatch = logs.dispatch();
        let _guard = tracing::dispatcher::set_default(&dispatch);
        let host = testing::headless_host();
        let (mut session, observer) = testing::start_probe(&host, ready_movie());
        let (request, controls) = ScriptRequest::new(loose(&[("wait.js", WAIT)]));
        let _supervisor = Supervisor::start(&mut session, Some(request)).unwrap();
        logs.wait_for(INFO, "session 1: wait.js is waiting for login");
        log_in(&observer);
        assert_eq!(
            await_event(&observer, "onDebug", None).args,
            vec![lucent_protocol::bridge::Value::String("started".into())]
        );
        if kill {
            controls.kill();
            logs.wait_for(WARN, "session 1: wait.js was killed");
            assert!(session_lines(&logs, INFO).contains(&"session 1: killing wait.js".into()));
        } else {
            controls.stop();
            logs.wait_for(INFO, "session 1: wait.js was stopped");
            assert!(session_lines(&logs, INFO).contains(&"session 1: stopping wait.js".into()));
        }
        logs.wait_for(INFO, "session 1: bot 1 is running");
    }
}

#[test]
fn a_restart_ends_the_run_starts_a_new_bot_and_does_not_rerun() {
    let logs = Logs::default();
    let dispatch = logs.dispatch();
    let _guard = tracing::dispatcher::set_default(&dispatch);
    let host = testing::headless_host();
    let (mut session, observer) = testing::start_probe(&host, ready_movie());
    let (request, controls) = ScriptRequest::new(loose(&[("wait.js", WAIT)]));
    let mut supervisor = Supervisor::start(&mut session, Some(request)).unwrap();
    logs.wait_for(INFO, "session 1: wait.js is waiting for login");
    log_in(&observer);
    await_event(&observer, "onDebug", None);
    supervisor
        .restart_player(&mut session, testing::headless_surface())
        .unwrap();
    logs.wait_for(INFO, "session 1: bot 2 started");
    let observer = session.attach().unwrap();
    session.send(ViewEvent::Resized(
        Viewport::new(100.0, 100.0, 1.0).unwrap(),
    ));
    testing::wait_loaded(&observer);
    log_in(&observer);
    controls.status();
    logs.wait_for(INFO, "session 1: no script is running");
    assert_eq!(
        session_lines(&logs, WARN),
        ["session 1: wait.js ended because its player stopped"]
    );
    let info = session_lines(&logs, INFO);
    assert_eq!(
        info.iter()
            .filter(|s| s.as_str() == "session 1: wait.js started")
            .count(),
        1
    );
    assert!(
        info.iter()
            .position(|s| s == "session 1: bot 1 stopped with its player")
            .unwrap()
            < info
                .iter()
                .position(|s| s == "session 1: bot 2 started")
                .unwrap()
    );
    assert_eq!(
        &info[info.len() - 2..],
        [
            "session 1: bot 2 is running",
            "session 1: no script is running"
        ]
    );
}

#[test]
fn a_script_waiting_for_login_waits_again_after_a_restart() {
    let logs = Logs::default();
    let dispatch = logs.dispatch();
    let _guard = tracing::dispatcher::set_default(&dispatch);
    let host = testing::headless_host();
    let (mut session, _) = testing::start_probe(&host, ready_movie());
    let (request, _) = ScriptRequest::new(loose(&[("done.js", "export default function* () {}")]));
    let mut supervisor = Supervisor::start(&mut session, Some(request)).unwrap();
    logs.wait_for(INFO, "session 1: done.js is waiting for login");
    supervisor
        .restart_player(&mut session, testing::headless_surface())
        .unwrap();
    logs.wait_for(INFO, "session 1: bot 2 started");
    let observer = session.attach().unwrap();
    session.send(ViewEvent::Resized(
        Viewport::new(100.0, 100.0, 1.0).unwrap(),
    ));
    testing::wait_loaded(&observer);
    log_in(&observer);
    logs.wait_for(INFO, "session 1: bot 2 is running");
    assert_eq!(
        session_lines(&logs, INFO),
        [
            "session 1: bot 1 started",
            "session 1: done.js is waiting for login",
            "session 1: bot 1 stopped with its player",
            "session 1: bot 2 started",
            "session 1: done.js is waiting for login",
            "session 1: done.js started",
            "session 1: done.js completed",
            "session 1: bot 2 is running",
        ]
    );
}

#[test]
fn controls_cancel_a_waiting_script_without_replaying_it_after_login() {
    for kill in [false, true] {
        let logs = Logs::default();
        let dispatch = logs.dispatch();
        let _guard = tracing::dispatcher::set_default(&dispatch);
        let host = testing::headless_host();
        let (mut session, observer) = testing::start_probe(&host, ready_movie());
        let (request, controls) =
            ScriptRequest::new(loose(&[("done.js", "export default function* () {}")]));
        if kill {
            controls.kill();
        } else {
            controls.stop();
        }
        let _supervisor = Supervisor::start(&mut session, Some(request)).unwrap();
        logs.wait_for(INFO, "session 1: done.js was cancelled before it started");
        log_in(&observer);
        controls.status();
        logs.wait_for(INFO, "session 1: no script is running");
        assert_eq!(
            session_lines(&logs, INFO),
            [
                "session 1: bot 1 started",
                "session 1: done.js is waiting for login",
                "session 1: done.js was cancelled before it started",
                "session 1: bot 1 is running",
                "session 1: bot 1 is running",
                "session 1: no script is running",
            ]
        );
    }
}

#[test]
fn an_already_logged_in_player_runs_without_another_login_event() {
    let logs = Logs::default();
    let dispatch = logs.dispatch();
    let _guard = tracing::dispatcher::set_default(&dispatch);
    let host = testing::headless_host();
    let (mut session, observer) = testing::start_probe(&host, ready_movie());
    log_in(&observer);
    let (request, _) = ScriptRequest::new(loose(&[("done.js", "export default function* () {}")]));
    let _supervisor = Supervisor::start(&mut session, Some(request)).unwrap();
    logs.wait_for(INFO, "session 1: bot 1 is running");
    assert_eq!(
        session_lines(&logs, INFO),
        [
            "session 1: bot 1 started",
            "session 1: done.js is waiting for login",
            "session 1: done.js started",
            "session 1: done.js completed",
            "session 1: bot 1 is running",
        ]
    );
}

#[test]
fn a_session_without_a_script_starts_a_bot_for_each_player() {
    let logs = Logs::default();
    let dispatch = logs.dispatch();
    let _guard = tracing::dispatcher::set_default(&dispatch);
    let host = testing::headless_host();
    let (mut session, _) = testing::start_probe_without_viewport(&host, ready_movie());
    let mut supervisor = Supervisor::start(&mut session, None).unwrap();
    logs.wait_for(INFO, "session 1: bot 1 started");
    supervisor
        .restart_player(&mut session, testing::headless_surface())
        .unwrap();
    logs.wait_for(INFO, "session 1: bot 2 started");
    assert_eq!(
        session_lines(&logs, INFO),
        [
            "session 1: bot 1 started",
            "session 1: bot 1 stopped with its player",
            "session 1: bot 2 started",
        ]
    );
}
