#![cfg(feature = "testing")]

mod common;
use common::logs;

use std::{thread, time::Duration};

use lucent_host::testing::{self, ProbeMovie};
use lucent_runtime::{Kill, Runtime, RuntimeError};

const TIMEOUT: Duration = Duration::from_secs(15);

fn warn_and_fail(message: &str) {
    let host = testing::headless_host();
    let (_session, link) = testing::start_probe(&host, ProbeMovie::new());
    let mut runtime = Runtime::start(link).unwrap();
    runtime
        .eval(
            "warning.js",
            format!(
                "console.warn({0}); console.info({0}); null",
                serde_json::to_string(message).unwrap()
            ),
        )
        .wait::<()>(TIMEOUT)
        .unwrap();
    assert!(matches!(
        runtime
            .eval("runaway.js", "while (true) {}")
            .wait::<()>(TIMEOUT),
        Err(RuntimeError::Killed(Kill::Watchdog))
    ));
    runtime.join_cpu_time();
}

#[test]
fn runtime_logs_are_captured_after_an_unobserved_runtime_and_stay_isolated() {
    let first = logs::Logs::default();
    let dispatch = first.dispatch();
    let _guard = tracing::dispatcher::set_default(&dispatch);
    thread::spawn(|| warn_and_fail("unobserved"))
        .join()
        .unwrap();
    assert!(first.messages(tracing::Level::WARN).is_empty());
    assert!(first.messages(tracing::Level::INFO).is_empty());
    assert!(first.messages(tracing::Level::ERROR).is_empty());

    warn_and_fail("first");
    assert_eq!(
        first.messages(tracing::Level::WARN),
        vec!["first", "script runtime killed"]
    );
    assert_eq!(first.messages(tracing::Level::INFO), vec!["first"]);
    assert_eq!(first.messages(tracing::Level::ERROR), Vec::<String>::new());

    let second = logs::Logs::default();
    let other = second.dispatch();
    thread::scope(|scope| {
        scope.spawn(|| {
            let _guard = tracing::dispatcher::set_default(&dispatch);
            warn_and_fail("first again");
        });
        scope.spawn(|| {
            let _guard = tracing::dispatcher::set_default(&other);
            warn_and_fail("second");
        });
    });
    assert_eq!(
        first.messages(tracing::Level::WARN),
        vec![
            "first",
            "script runtime killed",
            "first again",
            "script runtime killed"
        ]
    );
    assert_eq!(
        first.messages(tracing::Level::INFO),
        vec!["first", "first again"]
    );
    assert_eq!(first.messages(tracing::Level::ERROR), Vec::<String>::new());
    assert_eq!(
        second.messages(tracing::Level::WARN),
        vec!["second", "script runtime killed"]
    );
    assert_eq!(second.messages(tracing::Level::INFO), vec!["second"]);
    assert_eq!(second.messages(tracing::Level::ERROR), Vec::<String>::new());
}
