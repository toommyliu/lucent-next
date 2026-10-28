use std::time::Duration;

use lucent_host::testing::{self, ProbeMovie};
use lucent_host::{ViewEvent, Viewport};
use lucent_runtime::{Runtime, RuntimeError};
use serde_json::{Value, json};

const TIMEOUT: Duration = Duration::from_secs(10);

#[test]
fn a_loader_callback_reaches_scripts_before_the_projection_events_it_produced() {
    let host = testing::headless_host();
    let (_session, link) = testing::start_probe(&host, ProbeMovie::new());
    let runtime = Runtime::start(link).unwrap();

    let seen: Value = runtime
        .eval(
            "order.js",
            r#"
            new Promise(resolve => {
                const seen = [];
                window.onExtensionResponse = raw => seen.push(["onExtensionResponse", JSON.parse(raw).dataObj.cmd]);
                window.onProjectionEvent = event => seen.push([event.type]);
                window.onDebug = message => {
                    seen.push(["onDebug", message]);
                    resolve(seen);
                };
                window.swf["probe.emit"]("onExtensionResponse", JSON.stringify({
                    type: "json",
                    dataObj: { cmd: "moveToArea", areaName: "testroom-1", uoBranch: [{ entID: 7, uoName: "player-a", intHP: 100 }] },
                }));
                window.swf["probe.emit"]("onDebug", "after");
            })
            "#,
        )
        .wait(TIMEOUT)
        .unwrap();

    assert_eq!(
        seen,
        json!([
            ["onExtensionResponse", "moveToArea"],
            ["join-map"],
            ["players-changed"],
            ["onDebug", "after"],
        ])
    );
}

#[test]
fn event_overflow_fails_the_active_script_and_the_runtime_keeps_running() {
    let host = testing::headless_host();
    let (_session, link) = testing::start_probe(&host, ProbeMovie::new());
    let runtime = Runtime::start(link).unwrap();

    let error = runtime
        .eval(
            "flood.js",
            r#"
            for (let index = 0; index < 1100; index++) {
                window.swf["probe.emit"]("onDebug", String(index));
            }
            new Promise(() => {});
            "#,
        )
        .wait::<Value>(TIMEOUT)
        .unwrap_err();
    assert!(matches!(error, RuntimeError::EventsLost), "{error}");

    let delivered: String = runtime
        .eval(
            "after.js",
            r#"
            new Promise(resolve => {
                window.onDebug = resolve;
                window.swf["probe.emit"]("onDebug", "after");
            })
            "#,
        )
        .wait(TIMEOUT)
        .unwrap();
    assert_eq!(delivered, "after");
}

#[test]
fn restart_stops_the_runtime_on_the_previous_link() {
    let host = testing::headless_host();
    let (mut session, link) = testing::start_probe(&host, ProbeMovie::new());
    let runtime = Runtime::start(link).unwrap();
    let active = runtime.eval("wait.js", "new Promise(() => {})");

    session.restart(testing::headless_surface()).unwrap();

    assert!(matches!(
        active.wait::<Value>(TIMEOUT),
        Err(RuntimeError::Stopped)
    ));
    assert!(matches!(
        runtime.eval("after.js", "1").wait::<i64>(TIMEOUT),
        Err(RuntimeError::Stopped)
    ));

    let runtime = Runtime::start(session.take_script_link().unwrap()).unwrap();
    let echoed = runtime.eval(
        "loaded.js",
        r#"new Promise(resolve => { window.onLoaded = () => resolve(window.swf["probe.echo"](42)); })"#,
    );
    session.send(ViewEvent::Resized(
        Viewport::new(100.0, 100.0, 1.0).unwrap(),
    ));
    assert_eq!(echoed.wait::<i64>(TIMEOUT).unwrap(), 42);
}

#[test]
fn timers_fire_in_deadline_order_without_starving_commands() {
    let host = testing::headless_host();
    let (_session, link) = testing::start_probe(&host, ProbeMovie::new());
    let runtime = Runtime::start(link).unwrap();
    runtime
        .eval(
            "spin.js",
            "globalThis.spins = 0; setInterval(() => spins++, 0); undefined",
        )
        .wait::<()>(TIMEOUT)
        .unwrap();

    let observed: Value = runtime
        .eval(
            "timers.js",
            r#"
            new Promise(resolve => {
                const fired = [];
                const started = __host.now();
                setTimeout(() => fired.push("late"), 40);
                const cancelled = setTimeout(() => fired.push("cancelled"), 10);
                setTimeout(label => fired.push(label), 10, "early");
                clearTimeout(cancelled);
                setTimeout(() => resolve({ fired, waited: __host.now() - started >= 60, spun: spins > 0 }), 60);
            })
            "#,
        )
        .wait(TIMEOUT)
        .unwrap();

    assert_eq!(
        observed,
        json!({ "fired": ["early", "late"], "waited": true, "spun": true })
    );
}

#[test]
fn a_script_settled_after_events_were_lost_still_fails() {
    let host = testing::headless_host();
    let (_session, link) = testing::start_probe(&host, ProbeMovie::new());
    let runtime = Runtime::start(link).unwrap();
    let waiting = runtime.eval(
        "wait.js",
        "new Promise(resolve => { globalThis.finish = resolve; })",
    );
    runtime
        .eval(
            "flood.js",
            r#"
            for (let index = 0; index < 1100; index++) {
                window.swf["probe.emit"]("onDebug", String(index));
            }
            finish("done");
            "#,
        )
        .wait::<()>(TIMEOUT)
        .unwrap();
    let error = waiting.wait::<Value>(TIMEOUT).unwrap_err();
    assert!(matches!(error, RuntimeError::EventsLost), "{error}");
}
