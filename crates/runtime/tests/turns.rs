use std::time::Duration;

use lucent_host::testing::{self, ProbeMovie};
use lucent_runtime::{Runtime, RuntimeError};
use serde_json::{Value, json};

const TIMEOUT: Duration = Duration::from_secs(10);

#[test]
fn completion_promises_settle_without_failing_other_evaluations() {
    let host = testing::headless_host();
    let (_session, link) = testing::start_probe(&host, ProbeMovie::new());
    let runtime = Runtime::start(link).unwrap();
    let rejected = runtime.eval(
        "reject-later.js",
        "new Promise((_, reject) => { globalThis.rejectLater = reject; })",
    );
    let fulfilled = runtime.eval(
        "fulfill-later.js",
        "new Promise(resolve => { globalThis.fulfillLater = resolve; })",
    );
    assert_eq!(
        runtime
            .eval(
                "settle.js",
                "rejectLater(new Error('later fault')); fulfillLater({ value: 42 }); 7"
            )
            .wait::<i64>(TIMEOUT)
            .unwrap(),
        7
    );
    let RuntimeError::JavaScript(stack) = rejected.wait::<Value>(TIMEOUT).unwrap_err() else {
        panic!("completion rejection did not report a JavaScript error")
    };
    assert!(stack.contains("later fault"), "{stack}");
    assert!(stack.contains("settle.js"), "{stack}");
    assert_eq!(
        fulfilled.wait::<Value>(TIMEOUT).unwrap(),
        json!({"value":42})
    );
    let error = runtime
        .eval(
            "reject-now.js",
            "Promise.reject(new Error('immediate fault'))",
        )
        .wait::<Value>(TIMEOUT)
        .unwrap_err();
    assert!(matches!(error, RuntimeError::JavaScript(stack) if stack.contains("immediate fault")));
    runtime
        .eval("undefined.js", "Promise.resolve(undefined)")
        .wait::<()>(TIMEOUT)
        .unwrap();
}

#[test]
fn callback_faults_leave_later_events_and_timers_running() {
    let host = testing::headless_host();
    let (_session, link) = testing::start_probe(&host, ProbeMovie::new());
    let runtime = Runtime::start(link).unwrap();
    let seen: Value = runtime.eval("callbacks.js", r#"
        new Promise(resolve => {
            const seen = [];
            window.onExtensionResponse = () => { throw new Error('loader fault'); };
            window.onProjectionEvent = event => { seen.push(event.type); throw new Error('projection fault'); };
            window.onProjectionInvalidated = () => { seen.push('invalidated'); throw new Error('reset fault'); };
            window.onConnection = status => seen.push(status);
            window.onDebug = () => {
                setTimeout(() => { seen.push('timer'); throw new Error('timer fault'); }, 0);
                setTimeout(() => resolve(seen), 0);
            };
            window.swf['probe.emit']('onExtensionResponse', JSON.stringify({
                type: 'json', dataObj: { cmd: 'moveToArea', areaName: 'testroom-1', uoBranch: [] },
            }));
            window.swf['probe.emit']('onConnection', 'OnConnectionLost');
            window.swf['probe.emit']('onDebug', 'after');
        })
    "#).wait(TIMEOUT).unwrap();
    assert_eq!(
        seen,
        json!([
            "join-map",
            "players-changed",
            "OnConnectionLost",
            "invalidated",
            "timer"
        ])
    );
}

#[test]
fn a_due_timer_cleared_by_an_earlier_callback_does_not_fire() {
    let host = testing::headless_host();
    let (_session, link) = testing::start_probe(&host, ProbeMovie::new());
    let runtime = Runtime::start(link).unwrap();
    let fired: Value = runtime
        .eval(
            "cleared.js",
            r#"
            new Promise(resolve => {
                const fired = [];
                setTimeout(() => {
                    fired.push("first");
                    clearTimeout(second);
                    clearInterval(interval);
                }, 0);
                const second = setTimeout(() => fired.push("second"), 0);
                const interval = setInterval(() => fired.push("interval"), 0);
                setTimeout(() => resolve(fired), 20);
            })
            "#,
        )
        .wait(TIMEOUT)
        .unwrap();
    assert_eq!(fired, json!(["first"]));
}

#[test]
fn serializing_a_completion_can_settle_an_earlier_evaluation() {
    let host = testing::headless_host();
    let (_session, link) = testing::start_probe(&host, ProbeMovie::new());
    let runtime = Runtime::start(link).unwrap();
    let first = runtime.eval(
        "first.js",
        "new Promise(resolve => { globalThis.resolveFirst = resolve; })",
    );
    let second = runtime.eval(
        "second.js",
        "new Promise(resolve => { globalThis.resolveSecond = resolve; })",
    );
    runtime.eval("resolve.js", "resolveSecond({ toJSON() { Promise.resolve().then(() => resolveFirst(42)); return 7; } }); undefined")
        .wait::<()>(TIMEOUT).unwrap();
    assert_eq!(second.wait::<i64>(TIMEOUT).unwrap(), 7);
    assert_eq!(first.wait::<i64>(TIMEOUT).unwrap(), 42);
}
