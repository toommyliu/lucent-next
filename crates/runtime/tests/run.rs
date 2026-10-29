use std::time::Duration;

use lucent_host::testing::{self, ProbeMovie};
use lucent_runtime::Runtime;
use serde_json::{Value, json};

const TIMEOUT: Duration = Duration::from_secs(10);

#[test]
fn a_failed_program_rejects_its_run_and_the_next_run_still_answers() {
    let host = testing::headless_host();
    let (_session, link) = testing::start_probe(&host, ProbeMovie::new());
    let runtime = Runtime::start(link).unwrap();

    let observed: Value = runtime
        .eval(
            "run.js",
            r#"
            (async () => {
                const rejected = await __lucent
                    .run(() => {
                        throw new Error("broken program");
                    })
                    .then(() => null, (error) => String(error));
                return { rejected, map: await __lucent.run((api) => api.map.getName()) };
            })()
            "#,
        )
        .wait(TIMEOUT)
        .unwrap();

    assert_eq!(
        observed,
        json!({"rejected": "Error: broken program", "map": ""})
    );
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
