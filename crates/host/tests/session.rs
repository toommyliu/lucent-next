use std::time::Duration;

use lucent_host::scripting::Runtime;
use lucent_host::testing::{self, ProbeMovie};
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

#[derive(Debug, Deserialize, PartialEq)]
struct Refreshed {
    stored: u64,
    names: Vec<String>,
}

#[test]
fn a_script_refreshes_the_bank_through_a_host_command_and_reads_the_stored_items() {
    let host = testing::headless_host();
    let (_session, link) = testing::start_probe(
        &host,
        ProbeMovie::new().constant(
            "bank.getItems",
            serde_json::json!([
                {"ItemID": 1, "sName": "Sword", "sType": "Sword"},
                {"ItemID": 2, "sName": "Shield", "sType": "Shield"}
            ]),
        ),
    );
    let runtime = Runtime::start(link).unwrap();

    let refreshed: Refreshed = runtime
        .eval(
            "bank.js",
            r#"
            const stored = JSON.parse(__host.command(JSON.stringify({ kind: "refreshContainer", container: "bank" })));
            const items = JSON.parse(__host.read(JSON.stringify({ kind: "items", container: "bank" })));
            ({ stored, names: items.map((item) => item.name) });
        "#,
        )
        .wait(Duration::from_secs(5))
        .unwrap();

    assert_eq!(
        refreshed,
        Refreshed {
            stored: 2,
            names: vec!["Sword".into(), "Shield".into()]
        }
    );
}
