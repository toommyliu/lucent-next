mod common;

use common::TIMEOUT;
use lucent_host::{
    Session,
    testing::{self, ProbeMovie},
};
use lucent_protocol::{
    bridge::Value as BridgeValue,
    script::{ModulePath, Script, ScriptOutcome},
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, time::Duration};

fn assert_script_result(session: &mut Session, source: &str, expected: Value) {
    let outcome = common::start(
        session,
        common::loose(&[("ordering.js", source)]),
        Default::default(),
    )
    .wait(TIMEOUT)
    .unwrap();
    let ScriptOutcome::StoppedByScript {
        reason: Some(reason),
    } = outcome
    else {
        panic!("expected a script result: {outcome:?}");
    };
    assert_eq!(serde_json::from_str::<Value>(&reason).unwrap(), expected);
}

#[test]
fn quest_completion_observes_an_event_raised_during_dispatch() {
    let host = testing::headless_host();
    let (mut session, link) = testing::start_probe(&host, quest_completion_movie());
    common::log_in(&link);
    assert_script_result(
        &mut session,
        r#"
        import * as api from "lucent/api";
        import * as script from "lucent/script";
        export default function* () {
            const event = yield* api.events.once(
                { type: "quest-complete", questId: 802 },
                { trigger: api.quests.complete(802, { turnIns: 1 }), timeout: "5 seconds" },
            );
            yield* script.stop(JSON.stringify(event));
        }
        "#,
        json!({"type":"quest-complete", "questId":802}),
    );
}

fn quest_completion_movie() -> ProbeMovie {
    common::ready_movie()
        .constant("quests.isInProgress", json!(true))
        .constant("quests.canComplete", json!(true))
        .constant("world.isActionAvailable", json!(true))
        .raising(
            "quests.complete",
            "onExtensionResponse",
            json!({"type":"json","dataObj":{
                "cmd":"ccqr", "QuestID":802, "bSuccess":1
            }})
            .to_string(),
        )
}

#[test]
fn a_packet_wait_subscribes_before_the_trigger_waits_for_its_reply() {
    let host = testing::headless_host();
    let (mut session, link) = testing::start_probe(&host, quest_completion_movie());
    common::log_in(&link);
    assert_script_result(
        &mut session,
        r#"
        import * as api from "lucent/api";
        import * as script from "lucent/script";
        export default function* () {
            const packet = yield* api.packet.once(
                { direction: "extension", command: "ccqr", encoding: "json" },
                { trigger: api.quests.complete(802, { turnIns: 1 }), timeout: "5 seconds" },
            );
            yield* script.stop(JSON.stringify(packet === null ? null : {
                direction: packet.direction,
                command: packet.command,
                questId: packet.data.QuestID,
                success: packet.data.bSuccess,
            }));
        }
        "#,
        json!({"direction":"extension", "command":"ccqr", "questId":802, "success":1}),
    );
}

#[test]
fn quest_loading_observes_a_packet_raised_during_dispatch() {
    let host = testing::headless_host();
    let (mut session, link) = testing::start_probe(&host, common::ready_movie());
    common::log_in(&link);
    assert_script_result(
        &mut session,
        r#"
        import * as api from "lucent/api";
        import * as script from "lucent/script";
        export default function* () {
            const packet = yield* api.packet.once(
                { direction: "extension", command: "getQuests", encoding: "json" },
                { trigger: api.quests.load(802, true), timeout: "5 seconds" },
            );
            const quest = yield* api.quests.get(802);
            yield* script.stop(JSON.stringify({
                command: packet?.command,
                direction: packet?.direction,
                quest: quest === null ? null : [quest.id, quest.name],
            }));
        }
        "#,
        json!({
            "command":"getQuests", "direction":"extension",
            "quest":[802, "Elders Blood"]
        }),
    );
}

fn emit_packet(link: &lucent_host::ScriptLink, data: Value) {
    link.call_loader(
        "probe.emit",
        vec![
            BridgeValue::String("onExtensionResponse".into()),
            BridgeValue::String(json!({"type":"json", "dataObj":data}).to_string()),
        ],
    )
    .recv_timeout(TIMEOUT)
    .unwrap();
}

#[test]
fn a_monster_death_and_drop_in_one_batch_are_visible_to_the_kill_continuation() {
    let host = testing::headless_host();
    let (mut session, link) = testing::start_probe(
        &host,
        common::ready_movie().raising_sequence(
            "flash.sendClientPacket",
            [
                (
                    "packetFromServer".into(),
                    json!({"t":"xt","b":{"o":{
                        "cmd":"ct", "m":{"2":{"intHP":0,"intState":0}}
                    }}})
                    .to_string(),
                ),
                (
                    "onExtensionResponse".into(),
                    json!({"type":"json","dataObj":{
                        "cmd":"dropItem", "items":{"70":{
                            "ItemID":70,"sName":"Frogzard Scale","iQty":3
                        }}
                    }})
                    .to_string(),
                ),
            ],
        ),
    );
    common::log_in(&link);
    emit_packet(
        &link,
        json!({
            "cmd":"moveToArea", "areaId":5,"areaName":"battleon-1",
            "uoBranch":[{"entID":7,"uoName":"player-a","strFrame":"Enter","intHP":100,"intHPMax":100}],
            "monBranch":[{"MonID":1,"MonMapID":2,"strMonName":"Frogzard","strFrame":"r2","intHP":50,"intHPMax":50,"intState":1}]
        }),
    );
    assert_script_result(
        &mut session,
        r#"
        import * as api from "lucent/api";
        import * as script from "lucent/script";
        export default function* () {
            const before = yield* api.drops.get(70);
            const death = yield* api.events.once(
                { type: "monster-death", monsterMapId: 2 },
                { trigger: api.packet.sendToClient("kill"), timeout: "5 seconds" },
            );
            const drop = yield* api.drops.get(70);
            const monster = yield* api.monsters.get(2);
            yield* script.stop(JSON.stringify({
                before,
                death,
                drop: drop === null ? null : [drop.itemId, drop.name, drop.quantity],
                monster: monster === null ? null : [monster.monsterMapId, monster.hp, monster.alive],
            }));
        }
        "#,
        json!({
            "before":null,
            "death":{"type":"monster-death","monsterMapId":2},
            "drop":[70,"Frogzard Scale",3],
            "monster":[2,0,false]
        }),
    );
}

#[test]
fn accepting_a_drop_updates_host_state_before_the_action_completes() {
    let host = testing::headless_host();
    let (mut session, link) = testing::start_probe(
        &host,
        common::ready_movie().accepting_drop(
            Duration::from_millis(50),
            json!({"type":"json","dataObj":{
                "cmd":"getDrop","ItemID":70,"bSuccess":1,"iQtyNow":3
            }})
            .to_string(),
        ),
    );
    common::log_in(&link);
    emit_packet(
        &link,
        json!({"cmd":"dropItem","items":{
            "70":{"ItemID":70,"sName":"Frogzard Scale","iQty":3}
        }}),
    );
    assert_script_result(
        &mut session,
        r#"
        import * as api from "lucent/api";
        import * as script from "lucent/script";
        export default function* () {
            const before = yield* api.inventory.get(70);
            const accepted = yield* api.drops.accept(70);
            const item = yield* api.inventory.get(70);
            yield* script.stop(JSON.stringify({
                before,
                accepted,
                item: item === null ? null : [item.itemId, item.name, item.quantity],
                drop: yield* api.drops.get(70),
            }));
        }
        "#,
        json!({"before":null,"accepted":true,"item":[70,"Frogzard Scale",3],"drop":null}),
    );
}

#[test]
fn loader_exceptions_return_false_through_the_public_api() {
    for (movie, expected) in [
        (
            common::ready_movie().constant("world.isMonsterAvailable", json!(true)),
            json!(true),
        ),
        (
            common::ready_movie().throwing("world.isMonsterAvailable", "monster lookup exploded"),
            json!(false),
        ),
    ] {
        let host = testing::headless_host();
        let (mut session, link) = testing::start_probe(&host, movie);
        common::log_in(&link);
        emit_packet(
            &link,
            json!({
            "cmd":"moveToArea","areaId":5,"areaName":"battleon-1",
            "uoBranch":[{"entID":7,"uoName":"player-a","intHP":100,"intHPMax":100}],
            "monBranch":[{"MonID":1,"MonMapID":2,"strMonName":"Frogzard","intHP":50,"intHPMax":50}]
            }),
        );
        assert_script_result(
            &mut session,
            r#"
        import * as api from "lucent/api";
        import * as script from "lucent/script";
        export default function* () {
            yield* script.stop(JSON.stringify(yield* api.monsters.isAvailable(2)));
        }
        "#,
            expected,
        );
    }
}

#[test]
fn a_failed_host_command_returns_false_and_leaves_the_drop_available() {
    for (movie, expected) in [
        (
            common::ready_movie().constant("drops.reject", json!(true)),
            json!({"rejected":true,"drop":null}),
        ),
        (
            common::ready_movie().throwing("drops.reject", "drop rejection exploded"),
            json!({"rejected":false,"drop":[70,"Frogzard Scale",3]}),
        ),
    ] {
        let host = testing::headless_host();
        let (mut session, link) = testing::start_probe(&host, movie);
        common::log_in(&link);
        emit_packet(
            &link,
            json!({"cmd":"dropItem","items":{
            "70":{"ItemID":70,"sName":"Frogzard Scale","iQty":3}
            }}),
        );
        assert_script_result(
            &mut session,
            r#"
        import { Effect } from "effect";
        import * as api from "lucent/api";
        import * as script from "lucent/script";
        export default function* () {
            const rejected = yield* api.drops.reject(70).pipe(
                Effect.catch((error) => Effect.succeed(String(error))),
            );
            const drop = yield* api.drops.get(70);
            yield* script.stop(JSON.stringify({
                rejected,
                drop: drop === null ? null : [drop.itemId, drop.name, drop.quantity],
            }));
        }
        "#,
            expected,
        );
    }
}

#[test]
fn a_host_command_validation_error_keeps_its_original_message() {
    let host = testing::headless_host();
    let (mut session, link) = testing::start_probe(&host, common::ready_movie());
    common::log_in(&link);
    let outcome = common::start(
        &mut session,
        common::loose(&[(
            "command-error.js",
            r#"import * as autozone from "lucent/autozone";
export default function* () { yield* autozone.setEnabled("yes"); }"#,
        )]),
        Default::default(),
    )
    .wait(TIMEOUT)
    .unwrap();
    let ScriptOutcome::Failed { message, .. } = outcome else {
        panic!("accepted an invalid host command: {outcome:?}");
    };
    assert_eq!(message, "invalid type: string \"yes\", expected a boolean");
}

#[test]
fn package_modules_cannot_import_internal_packages_or_escape_their_package() {
    let host = testing::headless_host();
    let (mut session, link) = testing::start_probe(&host, common::ready_movie());
    common::log_in(&link);
    for specifier in [
        "@lucent/runtime",
        "../secret.js",
        "lucent/host",
        "effect/Effect",
    ] {
        let entry = ModulePath::in_package("ordering-fixture", "entry.js").unwrap();
        let script = Script::new(
            entry.clone(),
            BTreeMap::from([
                (
                    entry,
                    format!("import {specifier:?};\nexport default function* () {{}}"),
                ),
                (
                    ModulePath::loose("secret.js").unwrap(),
                    "throw new Error('escaped the package');".into(),
                ),
            ]),
            BTreeMap::new(),
        )
        .unwrap();
        let outcome = common::start(&mut session, script, Default::default())
            .wait(TIMEOUT)
            .unwrap();
        let ScriptOutcome::Failed { message, .. } = outcome else {
            panic!("imported {specifier}: {outcome:?}");
        };
        assert_eq!(
            message,
            format!("ordering-fixture/entry.js:1:8: Cannot find module {specifier:?}")
        );
    }
}

#[test]
fn public_modules_do_not_export_private_host_bindings() {
    let host = testing::headless_host();
    let (mut session, link) = testing::start_probe(&host, common::ready_movie());
    common::log_in(&link);
    let outcome = common::start(
        &mut session,
        common::loose(&[(
            "private.js",
            "import { __host } from 'lucent/api';\nexport default function* () {}",
        )]),
        Default::default(),
    )
    .wait(TIMEOUT)
    .unwrap();
    let ScriptOutcome::Failed { message, .. } = outcome else {
        panic!("imported a private binding: {outcome:?}");
    };
    assert_eq!(
        message,
        "private.js:1:10: SyntaxError: The requested module 'lucent/api' does not provide an export named '__host'"
    );
}
