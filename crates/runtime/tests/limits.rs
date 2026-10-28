use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use lucent_host::Session;
use lucent_host::testing::{self, ProbeMovie};
use lucent_protocol::query::{Query, answer};
use lucent_runtime::{Kill, PendingEval, Runtime, RuntimeError};
use serde_json::{Value, json};

const TIMEOUT: Duration = Duration::from_secs(10);
const STOP_LIMIT: Duration = Duration::from_millis(250);

fn stall_player(session: &Session) -> mpsc::Sender<()> {
    let (entered, stalled) = mpsc::channel();
    let (release, held) = mpsc::channel::<()>();
    let _ = session.read_projection(move |_| {
        let _ = entered.send(());
        let _ = held.recv();
    });
    stalled
        .recv_timeout(TIMEOUT)
        .expect("the player did not stall");
    release
}

// Nothing public reports that a script is inside a loader call. The assertions that follow
// prove the call reached the player before the script stopped.
fn let_the_script_reach_the_player() {
    thread::sleep(Duration::from_millis(100));
}

fn listen(runtime: &Runtime) -> PendingEval {
    runtime.eval(
        "listen.js",
        r#"
        new Promise(resolve => {
            const heard = [typeof globalThis.marker];
            window.onDebug = message => {
                heard.push(message);
                if (message === "after") resolve(heard);
            };
        })
        "#,
    )
}

fn emit_after(runtime: &Runtime) {
    assert!(
        runtime
            .eval(
                "after.js",
                r#"window.swf["probe.emit"]("onDebug", "after")"#
            )
            .wait::<bool>(TIMEOUT)
            .unwrap()
    );
}

#[test]
fn a_runaway_turn_is_killed_within_250_ms_and_the_runtime_keeps_running() {
    let host = testing::headless_host();
    let (_session, link) = testing::start_probe(&host, ProbeMovie::new());
    let runtime = Runtime::start(link).unwrap();
    for source in [
        "while (true) {}",
        "while (true) { try { while (true) {} } catch {} finally { while (true) {} } }",
        "new Promise(() => setTimeout(() => { while (true) {} }, 0))",
        "(function again() { Promise.resolve().then(again); })()",
    ] {
        let waiting = runtime.eval("wait.js", "globalThis.marker = 1; new Promise(() => {})");
        let started = Instant::now();
        let error = runtime
            .eval("runaway.js", source)
            .wait::<Value>(TIMEOUT)
            .unwrap_err();
        let elapsed = started.elapsed();
        assert!(
            matches!(error, RuntimeError::Killed(Kill::Watchdog)),
            "{source}: {error}"
        );
        assert!(elapsed < STOP_LIMIT, "{source}: stopped after {elapsed:?}");
        let error = waiting.wait::<Value>(TIMEOUT).unwrap_err();
        assert!(
            matches!(error, RuntimeError::Killed(Kill::Watchdog)),
            "{source}: {error}"
        );

        let heard = listen(&runtime);
        emit_after(&runtime);
        assert_eq!(
            heard.wait::<Value>(TIMEOUT).unwrap(),
            json!(["undefined", "after"]),
            "{source}"
        );
    }
}

#[test]
fn a_script_blocked_in_a_loader_call_is_killed_without_waiting_for_the_reply() {
    let host = testing::headless_host();
    let (session, link) = testing::start_probe(&host, ProbeMovie::new());
    let runtime = Runtime::start(link).unwrap();
    let release = stall_player(&session);
    let blocked = runtime.eval(
        "blocked.js",
        r#"globalThis.marker = 1; window.swf["probe.emit"]("onDebug", "issued")"#,
    );
    let_the_script_reach_the_player();

    let killed = Instant::now();
    runtime.kill();
    let error = blocked.wait::<Value>(TIMEOUT).unwrap_err();
    let elapsed = killed.elapsed();
    assert!(
        matches!(error, RuntimeError::Killed(Kill::Requested)),
        "{error}"
    );
    assert!(elapsed < STOP_LIMIT, "stopped after {elapsed:?}");

    let heard = listen(&runtime);
    drop(release);
    emit_after(&runtime);
    assert_eq!(
        heard.wait::<Value>(TIMEOUT).unwrap(),
        json!(["undefined", "issued", "after"])
    );
}

#[test]
fn a_script_that_catches_out_of_memory_is_killed_at_the_heap_limit() {
    let host = testing::headless_host();
    let (_session, link) = testing::start_probe(&host, ProbeMovie::new());
    let runtime = Runtime::start(link).unwrap();
    let error = runtime
        .eval(
            "hoard.js",
            r#"
            globalThis.marker = 1;
            const hoard = [];
            for (;;) {
                try { hoard.push(new Array(100000).fill(1.5)); } catch {}
            }
            "#,
        )
        .wait::<Value>(TIMEOUT)
        .unwrap_err();
    assert!(
        matches!(error, RuntimeError::Killed(Kill::HeapLimit)),
        "{error}"
    );

    let heard = listen(&runtime);
    emit_after(&runtime);
    assert_eq!(
        heard.wait::<Value>(TIMEOUT).unwrap(),
        json!(["undefined", "after"])
    );
}

#[test]
fn dropping_a_runtime_blocked_in_a_loader_call_returns_promptly() {
    let host = testing::headless_host();
    let (session, link) = testing::start_probe(&host, ProbeMovie::new());
    let runtime = Runtime::start(link).unwrap();
    let release = stall_player(&session);
    let blocked = runtime.eval(
        "blocked.js",
        r#"
        window.swf["probe.emit"]("onExtensionResponse", JSON.stringify({
            type: "json",
            dataObj: { cmd: "moveToArea", areaName: "issued-1", uoBranch: [] },
        }))
        "#,
    );
    let_the_script_reach_the_player();

    let dropped = Instant::now();
    drop(runtime);
    let elapsed = dropped.elapsed();
    assert!(elapsed < STOP_LIMIT, "dropped after {elapsed:?}");
    assert!(matches!(
        blocked.wait::<Value>(TIMEOUT),
        Err(RuntimeError::Stopped)
    ));

    drop(release);
    let map: Value = serde_json::from_str(
        &session
            .read_projection(|projection| answer(projection, Query::Map {}))
            .recv_timeout(TIMEOUT)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(map["name"], "issued");
}
