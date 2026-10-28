use std::time::Duration;

use lucent_host::testing::{self, ProbeMovie};
use lucent_runtime::Runtime;
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
struct Observed {
    emitted: bool,
    map: String,
    me: Option<i64>,
}

#[test]
fn a_script_calls_the_loader_and_reads_projected_state() {
    let host = testing::headless_host();
    let (_session, link) = testing::start_probe(&host, ProbeMovie::new());
    let runtime = Runtime::start(link).unwrap();

    let observed: Observed = runtime
        .eval(
            "probe.js",
            r#"
            const read = (query) => JSON.parse(__host.read(JSON.stringify(query)));
            const emitted = window.swf["probe.emit"]("onExtensionResponse", JSON.stringify({
                type: "json",
                dataObj: { cmd: "moveToArea", areaName: "testroom-1", uoBranch: [{ entID: 7, uoName: "player-a", intHP: 100 }] },
            }));
            ({ emitted, map: read({ kind: "map" }).name, me: read({ kind: "me" })?.entityId ?? null });
        "#,
        )
        .wait(Duration::from_secs(5))
        .unwrap();

    assert_eq!(
        observed,
        Observed {
            emitted: true,
            map: "testroom".into(),
            me: Some(7)
        }
    );
}
