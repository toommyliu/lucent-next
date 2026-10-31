use lucent_host::bridge::EventInterests;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use lucent_host::scripting::{Kill, Runtime, RuntimeError};
use lucent_host::testing::{self, ProbeMovie};
use lucent_protocol::item::ItemContainer;
use serde_json::{Value, json};

const TIMEOUT: Duration = Duration::from_secs(5);

#[test]
fn invalid_commands_and_unavailable_callbacks_throw_errors_and_leave_the_runtime_usable() {
    let host = testing::headless_host();
    let (_session, link) = testing::start_probe(&host, ProbeMovie::new());
    let runtime = Runtime::start(link).unwrap();
    let errors = runtime
        .eval(
            "commands.js",
            r#"
        [
            'invalid',
            '{"kind":"missing"}',
            '{"kind":"refreshContainer","container":"drop"}',
            '{"kind":"refreshContainer","container":"bank"}'
        ].map(json => {
            try { __host.command(json); return null; }
            catch (error) { return error instanceof Error ? error.message : null; }
        })
    "#,
        )
        .wait::<Vec<String>>(TIMEOUT)
        .unwrap();
    assert!(errors[0].contains("expected value"), "{}", errors[0]);
    assert!(
        errors[1].contains("unknown variant `missing`"),
        "{}",
        errors[1]
    );
    assert_eq!(
        errors[2],
        "unknown variant `drop`, expected one of `bank`, `house`, `inventory`"
    );
    assert_eq!(
        errors[3],
        "the loader has not registered callback bank.getItems"
    );
    assert_eq!(
        runtime
            .eval(
                "after.js",
                "JSON.parse(__host.read('{\"kind\":\"itemHydration\",\"container\":\"bank\"}'))"
            )
            .wait::<u64>(TIMEOUT)
            .unwrap(),
        0
    );
}

#[test]
fn a_script_waiting_for_a_host_command_is_killed_without_waiting_for_the_player() {
    let host = testing::headless_host();
    let (mut session, link) = testing::start_probe(
        &host,
        ProbeMovie::new().constant("bank.getItems", json!([{"ItemID": 1, "sName": "Sword"}])),
    );
    let runtime = Runtime::start(link).unwrap();
    let (entered, stalled) = mpsc::channel();
    let (release, held) = mpsc::channel::<()>();
    let _ = session.read_projection(move |_| {
        entered.send(()).unwrap();
        let _ = held.recv();
    });
    stalled.recv_timeout(TIMEOUT).unwrap();
    let blocked = runtime.eval(
        "blocked.js",
        r#"
        globalThis.marker = 1;
        __host.command('{"kind":"refreshContainer","container":"bank"}')
    "#,
    );
    thread::sleep(Duration::from_millis(100));
    let started = Instant::now();
    runtime.kill();
    let error = blocked.wait::<Value>(TIMEOUT).unwrap_err();
    assert!(
        matches!(error, RuntimeError::Killed(Kill::Requested)),
        "{error}"
    );
    assert!(started.elapsed() < Duration::from_millis(250));
    drop(release);
    assert_eq!(
        session
            .read_projection(|projection| projection.items.hydration(ItemContainer::Bank))
            .recv_timeout(TIMEOUT)
            .unwrap(),
        1
    );
    drop(runtime);
    let runtime = Runtime::start(session.attach(EventInterests::ALL).unwrap()).unwrap();
    assert_eq!(
        runtime
            .eval("after.js", "typeof globalThis.marker")
            .wait::<String>(TIMEOUT)
            .unwrap(),
        "undefined"
    );
}
