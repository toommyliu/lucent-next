#![cfg(any(target_os = "macos", target_os = "linux"))]

mod common;

use common::*;
use lucent_host::Supervisor;
use lucent_host::{bridge::Value, testing};

#[test]
fn only_an_active_script_owns_a_js_worker_and_watchdog() {
    let logs = logs::Logs::default();
    let dispatch = logs.dispatch();
    let _guard = tracing::dispatcher::set_default(&dispatch);
    let host = testing::headless_host();
    let (mut session, observer) =
        testing::start_probe(&host, ready_movie().recording("settings.setWalkSpeed"));
    log_in(&observer);
    let mut supervisor = Supervisor::start(&mut session, None).unwrap();
    for _ in 0..2 {
        while await_event(&observer, "onProbeCall", None).args.first()
            != Some(&Value::String("settings.setWalkSpeed".into()))
        {}
    }
    assert_eq!(js_threads(), Vec::<String>::new());
    let controls = supervisor
        .start_script(
            &mut session,
            loose(&[(
                "wait.js",
                r#"
        import * as api from "lucent/api";
        import * as script from "lucent/script";
        export default function* () {
            yield* api.packet.sendToClient("started");
            yield* script.sleep("1 hour");
        }
    "#,
            )]),
        )
        .unwrap();
    await_event(&observer, "onDebug", None);
    #[cfg(target_os = "macos")]
    let expected = vec!["lucent-js-1", "lucent-js-watchdog-1"];
    #[cfg(target_os = "linux")]
    let expected = vec!["lucent-js-1", "lucent-js-watch"];
    assert_eq!(js_threads(), expected);
    controls.stop();
    logs.wait_for(tracing::Level::DEBUG, "session 1: script worker reaped");
    assert_eq!(js_threads(), Vec::<String>::new());
}
