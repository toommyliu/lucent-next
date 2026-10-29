mod common;

use common::TIMEOUT;
use lucent_host::testing::{self, ProbeMovie};
use lucent_protocol::script::ScriptOutcome;
use lucent_runtime::Runtime;

#[test]
fn a_failed_script_reports_its_error_and_the_next_run_completes() {
    let host = testing::headless_host();
    let (mut session, link) = testing::start_probe(&host, common::ready_movie());
    let runtime = Runtime::start(link).unwrap();
    common::ready(&runtime);
    match common::start(
        &mut session,
        common::loose(&[(
            "run.js",
            "export default function* () { throw new Error(\"broken program\"); }",
        )]),
        Default::default(),
    )
    .wait(TIMEOUT)
    .unwrap()
    {
        ScriptOutcome::Failed { message, .. } => assert_eq!(message, "run.js:1:37: broken program"),
        other => panic!("{other:?}"),
    }
    common::completed(&mut session);
}

#[test]
fn the_api_layer_listens_for_projection_events_before_the_first_turn() {
    let host = testing::headless_host();
    let (_session, link) = testing::start_probe(&host, ProbeMovie::new());
    let runtime = Runtime::start(link).unwrap();

    let listening: String = runtime
        .eval("first.js", "typeof window.onProjectionEvent")
        .wait(TIMEOUT)
        .unwrap();

    assert_eq!(listening, "function");
}
