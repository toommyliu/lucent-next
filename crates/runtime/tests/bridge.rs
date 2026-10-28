use std::time::Duration;

use lucent_host::testing::{self, ProbeMovie};
use lucent_runtime::{Runtime, RuntimeError};
use serde_json::{Value, json};

const TIMEOUT: Duration = Duration::from_secs(5);

#[test]
fn loader_values_round_trip_and_invalid_host_requests_throw_errors() {
    let host = testing::headless_host();
    let mut session = testing::start_probe(&host, ProbeMovie::new());
    let runtime = Runtime::start(session.take_script_link().unwrap()).unwrap();
    let observed: Value = runtime
        .eval(
            "bridge.js",
            r#"
            (() => {
                const value = { nested: [null, true, 3.5, "text", { key: 7 }] };
                const errors = [
                    () => __host.swf("probe.echo", "invalid"),
                    () => __host.swf("probe.echo", "{}"),
                    () => __host.read('{"kind":"missing"}'),
                    () => window.swf["missing"](),
                ].map(call => {
                    try { call(); return false; }
                    catch (error) { return error instanceof Error && error.message.length > 0; }
                });
                return {
                    value: window.swf["probe.echo"](value),
                    undefined: window.swf["probe.echo"]() === undefined,
                    me: JSON.parse(__host.read('{"kind":"me"}')),
                    errors,
                };
            })()
            "#,
        )
        .wait(TIMEOUT)
        .unwrap();
    assert_eq!(
        observed,
        json!({
            "value": {"nested":[null,true,3.5,"text",{"key":7}]},
            "undefined": true,
            "me": null,
            "errors": [true,true,true,true],
        })
    );
}

#[test]
fn evaluations_checkpoint_microtasks_and_report_faults_without_poisoning_the_context() {
    let host = testing::headless_host();
    let mut session = testing::start_probe(&host, ProbeMovie::new());
    let runtime = Runtime::start(session.take_script_link().unwrap()).unwrap();
    runtime
        .eval(
            "jobs.js",
            "globalThis.job = 0; Promise.resolve().then(() => { job = 7; }); undefined",
        )
        .wait::<()>(TIMEOUT)
        .unwrap();
    assert_eq!(
        runtime.eval("read.js", "job").wait::<i64>(TIMEOUT).unwrap(),
        7
    );
    assert_eq!(
        runtime.eval("completion.js", "const completion = { value: 0 }; Promise.resolve().then(() => completion.value = 9); completion")
            .wait::<Value>(TIMEOUT).unwrap(),
        json!({"value":9})
    );
    for source in [
        "undefined",
        "(() => {})",
        "Symbol('value')",
        "({ toJSON() {} })",
    ] {
        runtime
            .eval("undefined.js", source)
            .wait::<()>(TIMEOUT)
            .unwrap();
    }
    for (name, source, expected) in [
        ("throw.js", "throw new Error('sync fault')", "sync fault"),
        (
            "reject.js",
            "Promise.resolve().then(() => { throw new Error('async fault'); }); undefined",
            "async fault",
        ),
        (
            "serialize.js",
            "({ toJSON() { Promise.resolve().then(() => { throw new Error('serialization fault'); }); return 7; } })",
            "serialization fault",
        ),
        ("syntax.js", "const =", "SyntaxError"),
        (
            "cycle.js",
            "const cycle = {}; cycle.self = cycle; cycle",
            "circular",
        ),
    ] {
        let error = runtime
            .eval(name, source)
            .wait::<Value>(TIMEOUT)
            .unwrap_err();
        let RuntimeError::JavaScript(stack) = error else {
            panic!("unexpected error: {error}")
        };
        assert!(stack.contains(expected), "{stack}");
        if name == "throw.js" || name == "reject.js" {
            assert!(stack.contains(name), "{stack}");
        }
    }
    runtime
        .eval(
            "handled.js",
            "Promise.reject(new Error('handled')).catch(() => {}); undefined",
        )
        .wait::<()>(TIMEOUT)
        .unwrap();
    assert_eq!(
        runtime
            .eval("alive.js", "6 * 7")
            .wait::<i64>(TIMEOUT)
            .unwrap(),
        42
    );
    assert!(matches!(
        runtime.eval("decode.js", "'text'").wait::<i64>(TIMEOUT),
        Err(RuntimeError::Decode(_))
    ));
}
