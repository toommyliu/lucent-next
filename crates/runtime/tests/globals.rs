use std::time::Duration;

use lucent_host::testing::{self, ProbeMovie};
use lucent_runtime::Runtime;
use serde_json::{Value, json};

#[test]
fn abort_signals_and_console_work_in_the_runtime() {
    let host = testing::headless_host();
    let (_session, link) = testing::start_probe(&host, ProbeMovie::new());
    let runtime = Runtime::start(link).unwrap();
    let observed: Value = runtime
        .eval(
            "globals.js",
            r#"
            (() => {
                const controller = new AbortController();
                const calls = [];
                controller.signal.addEventListener("abort", () => calls.push("first"));
                controller.signal.onabort = () => calls.push("onabort");
                controller.signal.addEventListener("abort", () => calls.push("last"), { once: true });
                controller.abort();
                controller.abort("ignored");
                let thrown;
                try { controller.signal.throwIfAborted(); } catch (error) { thrown = error.message; }
                const cycle = {};
                cycle.self = cycle;
                console.log({ count: 2 }, new Error("broken"), cycle);
                return {
                    aborted: controller.signal.aborted,
                    reason: { name: controller.signal.reason.name, message: controller.signal.reason.message },
                    calls,
                    thrown,
                    staticReason: AbortSignal.abort("finished").reason,
                    encoded: [...new TextEncoder().encode("é")],
                    logged: true,
                };
            })()
            "#,
        )
        .wait(Duration::from_secs(10))
        .unwrap();
    assert_eq!(
        observed,
        json!({
            "aborted": true,
            "reason": { "name": "AbortError", "message": "This operation was aborted" },
            "calls": ["first", "onabort", "last"],
            "thrown": "This operation was aborted",
            "staticReason": "finished",
            "encoded": [195, 169],
            "logged": true,
        })
    );
}
