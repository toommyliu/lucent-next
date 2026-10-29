#![allow(dead_code)]

use lucent_host::testing::{self, ProbeMovie};
use lucent_host::{Host, Session};
use lucent_protocol::script::{ModulePath, Script};
use lucent_runtime::Runtime;
use serde_json::json;
use std::{collections::BTreeMap, path::Path, time::Duration};

pub const TIMEOUT: Duration = Duration::from_secs(15);

pub fn loose(modules: &[(&str, &str)]) -> Script {
    Script::new(
        ModulePath::loose(modules[0].0).unwrap(),
        modules
            .iter()
            .map(|(path, source)| (ModulePath::loose(*path).unwrap(), (*source).into()))
            .collect(),
        BTreeMap::new(),
    )
    .unwrap()
}

pub fn package(dir: &str, entry: &str, extra: &[(&str, &str)]) -> Script {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../script-packages")
        .join(dir);
    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("package.json")).unwrap()).unwrap();
    let name = manifest["name"].as_str().unwrap();
    let mut modules = BTreeMap::new();
    fn read(root: &Path, dir: &Path, name: &str, modules: &mut BTreeMap<ModulePath, String>) {
        for file in std::fs::read_dir(dir).unwrap() {
            let path = file.unwrap().path();
            if path.is_dir() {
                read(root, &path, name, modules);
            } else if path.extension().is_some_and(|ext| ext == "js") {
                modules.insert(
                    ModulePath::in_package(
                        name,
                        path.strip_prefix(root)
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .replace('\\', "/"),
                    )
                    .unwrap(),
                    std::fs::read_to_string(path).unwrap(),
                );
            }
        }
    }
    read(&dir, &dir, name, &mut modules);
    for (path, source) in extra {
        modules.insert(
            ModulePath::in_package(name, *path).unwrap(),
            (*source).into(),
        );
    }
    Script::new(
        ModulePath::in_package(name, entry).unwrap(),
        modules,
        [(
            name.into(),
            ModulePath::in_package(name, manifest["main"].as_str().unwrap()).unwrap(),
        )]
        .into(),
    )
    .unwrap()
}

pub fn ready_movie() -> ProbeMovie {
    ProbeMovie::new()
        .constant("auth.isLoggedIn", json!(true))
        .constant("flash.getGameObjectS", json!("\"player-a\""))
        .constant("flash.callGameFunction0", json!("true"))
        .constant("player.isLoaded", json!(true))
        .constant("world.isLoaded", json!(true))
        .constant("bank.loadItems", json!(null))
        .constant("bank.isLoaded", json!(true))
        .constant("bank.getItems", json!([]))
        .raising(
            "quests.getMultiple",
            "onExtensionResponse",
            json!({
                "type": "json",
                "dataObj": {
                    "cmd": "getQuests",
                    "quests": {
                        "802": {
                            "QuestID": 802,
                            "sName": "Elders Blood",
                            "Rewards": [{"ItemID": 5586, "sName": "Elders Blood", "iStk": 20}],
                        },
                    },
                },
            })
            .to_string(),
        )
}

pub fn ready(runtime: &Runtime) {
    runtime
        .eval(
            "ready.js",
            r#"
        window.swf["probe.emit"]("onExtensionResponse", JSON.stringify({
            type: "json",
            dataObj: {
                cmd: "loadInventoryBig",
                items: [{ ItemID: 5586, sName: "Elders Blood", sType: "Item", iQty: 20, iStk: 20 }],
                hitems: [{ ItemID: 2, sName: "Home", sType: "House", bEquip: 1 }],
            },
        }));
        window.swf["probe.emit"]("onExtensionResponse", JSON.stringify({
            type: "json",
            dataObj: {
                cmd: "moveToArea",
                areaId: 1,
                areaName: "buyhouse-1",
                uoBranch: [{ entID: 7, uoName: "player-a", intHP: 100, intHPMax: 100 }],
            },
        }));
        null;
    "#,
        )
        .wait::<()>(TIMEOUT)
        .unwrap();
}

pub fn yielding(runtime: &Runtime) -> lucent_runtime::ScriptRun {
    let started = runtime.eval(
        "listen.js",
        "new Promise(resolve => { globalThis.started = resolve; })",
    );
    let script = loose(&[(
        "wait.js",
        r#"
        import * as script from "lucent/script";
        export default function* () {
            globalThis.started("running");
            yield* script.sleep("1 hour");
        }
    "#,
    )]);
    let run = runtime.run_script(script, BTreeMap::new());
    assert_eq!(started.wait::<String>(TIMEOUT).unwrap(), "running");
    run
}

pub fn completed(runtime: &Runtime) {
    assert_eq!(
        runtime
            .run_script(
                loose(&[("done.js", "export default function* () {}")]),
                BTreeMap::new()
            )
            .wait(TIMEOUT)
            .unwrap(),
        lucent_protocol::script::ScriptOutcome::Completed
    );
}

pub fn ready_runtime() -> (Host, Session, Runtime) {
    let host = testing::headless_host();
    let (session, link) = testing::start_probe(&host, ready_movie());
    let runtime = Runtime::start(link).unwrap();
    ready(&runtime);
    (host, session, runtime)
}
