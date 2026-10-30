mod common;

use common::TIMEOUT;
use lucent_host::bridge::ScriptEvent;
use lucent_host::{ScriptLink, Session, testing};
use lucent_protocol::script::{Script, ScriptOutcome};
use serde_json::{Value, json};

fn response(value: Value, packets: Vec<Value>) -> (Value, Vec<(String, String)>) {
    (
        value,
        packets
            .into_iter()
            .map(|data| {
                (
                    "onExtensionResponse".into(),
                    json!({"type":"json", "dataObj":data}).to_string(),
                )
            })
            .collect(),
    )
}

fn map(name: &str) -> Value {
    json!({"cmd":"moveToArea", "areaId":1, "areaName":format!("{name}-1"),
        "uoBranch":[{"entID":7,"uoName":"player-a","intHP":100,"intHPMax":100,"intState":1}]})
}

fn calls(link: &ScriptLink) -> Vec<Value> {
    let mut calls = Vec::new();
    while let Some(event) = link.try_event().unwrap() {
        if let ScriptEvent::Loader { event, .. } = event
            && event.name == "onProbeCall"
        {
            calls.push(
                lucent_protocol::bridge::to_json(lucent_protocol::bridge::Value::List(event.args))
                    .unwrap(),
            );
        }
    }
    calls
}

fn run(session: &mut Session, script: Script, inputs: Value) -> ScriptOutcome {
    common::start(session, script, serde_json::from_value(inputs).unwrap())
        .wait(TIMEOUT)
        .unwrap()
}

fn consumables() -> Script {
    common::package(
        "lucent-consumables",
        "tests/consumables.js",
        &[(
            "tests/consumables.js",
            r#"
        import run from "../scripts/get-consumables";
        import * as script from "lucent/script";
        export { inputs } from "../scripts/get-consumables";
        export default function* () {
            yield* script.options.update({ roomPolicy: { kind: "public" } });
            yield* run();
        }
        "#,
        )],
    )
}

fn wheel() -> Script {
    common::package(
        "lucent-dailies",
        "tests/wheel.js",
        &[(
            "tests/wheel.js",
            r#"
        import run from "../scripts/spin-wheel-of-doom";
        import * as script from "lucent/script";
        export { inputs } from "../scripts/spin-wheel-of-doom";
        export default function* () {
            yield* script.options.update({ roomPolicy: { kind: "public" } });
            yield* script.stop(JSON.stringify(yield* run()));
        }
    "#,
        )],
    )
}

fn assert_result(outcome: ScriptOutcome, expected: Value) {
    let ScriptOutcome::StoppedByScript {
        reason: Some(reason),
    } = outcome
    else {
        panic!("expected a script result: {outcome:?}");
    };
    assert_eq!(serde_json::from_str::<Value>(&reason).unwrap(), expected);
}

#[test]
fn consumables_buys_the_missing_potions() {
    let item = json!({"ItemID":900,"ShopItemID":1900,"sName":"Endurance Draught",
        "sType":"Item","iQty":1,"iStk":300,"iCost":10});
    let movie = common::ready_movie()
        .constant("world.isActionAvailable", json!(true))
        .constant("player.getGold", json!(1000))
        .constant("shops.canBuyItem", json!(true))
        .responding(
            "player.joinMap",
            [response(Value::Null, vec![map("alchemyacademy")])],
        )
        .responding(
            "shops.load",
            [response(
                Value::Null,
                vec![json!({
                    "cmd":"loadShop","ShopID":2036,"items":[item]
                })],
            )],
        )
        .responding(
            "shops.buy",
            [response(
                Value::Null,
                vec![json!({
                    "cmd":"buyItem","ItemID":900,"CharItemID":2900,"bitSuccess":1,"bBank":0,"iQty":3
                })],
            )],
        )
        .constant(
            "inventory.getItems",
            json!([{ "ItemID":900,"sName":"Endurance Draught",
            "sType":"Item","iQty":5,"iStk":300 }]),
        );
    let host = testing::headless_host();
    let (mut session, link) = testing::start_probe(&host, movie);
    common::log_in(&link);
    emit(&link, map("buyhouse"));
    emit(
        &link,
        json!({"cmd":"addItems","items":{"900":{
            "ItemID":900,"sName":"Endurance Draught","iQty":2
        }}}),
    );
    assert_eq!(
        run(
            &mut session,
            consumables(),
            json!({
                "potions":["Endurance Draught"],"scrolls":[],"quantity":5
            })
        ),
        ScriptOutcome::Completed
    );
    assert_eq!(
        owned(&link, lucent_protocol::item::ItemContainer::Inventory, 900),
        Some(5.0)
    );
    assert_eq!(
        calls(&link),
        vec![
            json!(["player.joinMap", ["alchemyacademy"]]),
            json!(["shops.load", [2036.0]]),
            json!(["shops.buy",[{"shopItemId":1900.0},3.0]]),
        ]
    );
}

#[test]
fn wheel_completes_the_daily_and_weekly_spins_with_default_banking() {
    let movie = common::ready_movie()
        .constant("player.isMember", json!(true))
        .constant("world.isActionAvailable", json!(true))
        .constant("quests.isAvailable", json!(true))
        .constant("quests.canComplete", json!(true))
        .constant("quests.getMaxTurnIns", json!(1))
        .responding("quests.getMultiple", [
            response(Value::Null, vec![json!({"cmd":"getQuests", "quests":{
                "3075":{"QuestID":3075,"sName":"Daily Wheel"}
            }})]),
            response(Value::Null, vec![json!({"cmd":"getQuests", "quests":{
                "3076":{"QuestID":3076,"sName":"Weekly Wheel"}
            }})]),
        ])
        .responding("quests.isInProgress", [false,false,true,true,false,false,true,true]
            .map(|active| response(json!(active), vec![])))
        .responding("player.joinMap", [response(Value::Null, vec![map("doom")])])
        .responding("quests.accept", [response(json!(true), vec![])])
        .responding("quests.complete", [3075,3076].map(|id| {
            let mut packets = vec![
                json!({"cmd":"ccqr","QuestID":id,"bSuccess":1}),
                json!({"cmd":"Wheel","dropItems":{},"Item":{"ItemID":901,"sName":"Wheel Prize","iQty":1}}),
            ];
            if id == 3076 {
                packets.push(json!({"cmd":"turnIn","sItems":"45739:3"}));
            }
            response(Value::Null, packets)
        }));
    let host = testing::headless_host();
    let (mut session, link) = testing::start_probe(&host, movie);
    common::log_in(&link);
    emit(&link, map("buyhouse"));
    emit(
        &link,
        json!({"cmd":"addItems","items":{"45739":{
            "ItemID":45739,"sName":"Gear of Doom","iQty":3
        }}}),
    );
    assert_result(
        run(&mut session, wheel(), json!({})),
        json!({
            "daily":{"status":"completed","banking":"not-requested"},
            "weekly":{"status":"completed","banking":"not-requested"}
        }),
    );
    assert_eq!(
        owned(
            &link,
            lucent_protocol::item::ItemContainer::Inventory,
            45739
        ),
        None
    );
    let actions: Vec<_> = calls(&link)
        .into_iter()
        .filter(|call| call[0] != "quests.isInProgress")
        .collect();
    assert_eq!(
        actions,
        vec![
            json!(["quests.getMultiple", ["3075"]]),
            json!(["player.joinMap", ["doom"]]),
            json!(["quests.accept", [3075.0, false]]),
            json!(["quests.complete", [3075.0, 1.0, -1.0, false]]),
            json!(["quests.getMultiple", ["3076"]]),
            json!(["quests.accept", [3076.0, false]]),
            json!(["quests.complete", [3076.0, 1.0, -1.0, false]]),
        ]
    );
}

fn emit(link: &ScriptLink, data: Value) {
    link.call_loader(
        "probe.emit",
        vec![
            lucent_protocol::bridge::Value::String("onExtensionResponse".into()),
            lucent_protocol::bridge::Value::String(
                json!({"type":"json","dataObj":data}).to_string(),
            ),
        ],
    )
    .recv_timeout(TIMEOUT)
    .unwrap();
}

fn assert_failed(outcome: ScriptOutcome, expected: &str) {
    let ScriptOutcome::Failed { message, .. } = outcome else {
        panic!("expected failure: {outcome:?}");
    };
    assert_eq!(message, expected);
}

#[test]
fn consumables_leaves_an_owned_potion_stack_alone() {
    let host = testing::headless_host();
    let (mut session, link) = testing::start_probe(
        &host,
        common::ready_movie()
            .constant("player.getGold", json!(0))
            .responding("shops.buy", [response(Value::Null, vec![])])
            .responding("player.joinMap", [response(Value::Null, vec![])]),
    );
    common::log_in(&link);
    emit(&link, map("buyhouse"));
    emit(
        &link,
        json!({"cmd":"addItems","items":{"900":{
            "ItemID":900,"sName":"Endurance Draught","iQty":300,"iStk":300
        }}}),
    );
    assert_eq!(
        run(
            &mut session,
            consumables(),
            json!({
                "potions":["Endurance Draught"],"scrolls":[],"quantity":300
            })
        ),
        ScriptOutcome::Completed
    );
    assert_eq!(calls(&link), Vec::<Value>::new());
}

#[test]
fn consumables_reports_a_missing_shop_recipe_before_buying() {
    let host = testing::headless_host();
    let movie = common::ready_movie()
        .constant("world.isActionAvailable", json!(true))
        .responding(
            "player.joinMap",
            [response(Value::Null, vec![map("alchemyacademy")])],
        )
        .responding(
            "shops.load",
            [response(
                Value::Null,
                vec![json!({
                    "cmd":"loadShop","ShopID":2036,"items":[]
                })],
            )],
        )
        .responding("shops.buy", [response(Value::Null, vec![])]);
    let (mut session, link) = testing::start_probe(&host, movie);
    common::log_in(&link);
    emit(&link, map("buyhouse"));
    assert_failed(
        run(
            &mut session,
            consumables(),
            json!({
                "potions":["Endurance Draught"],"scrolls":[],"quantity":1
            }),
        ),
        "@lucent/consumables/lib/supplies.js:151:17: Could not read the purchase recipe for Endurance Draught in shop 2036.",
    );
    assert_eq!(
        calls(&link),
        vec![
            json!(["player.joinMap", ["alchemyacademy"]]),
            json!(["shops.load", [2036.0]]),
        ]
    );
}

#[test]
fn wheel_skips_spins_that_are_already_unavailable() {
    let host = testing::headless_host();
    let movie = common::ready_movie()
        .constant("player.isMember", json!(true))
        .responding(
            "quests.getMultiple",
            [response(
                Value::Null,
                vec![json!({
                    "cmd":"getQuests","quests":{"3075":{"QuestID":3075,"sName":"Daily Wheel"}}
                })],
            )],
        )
        .constant("quests.isAvailable", json!(false))
        .constant("quests.isInProgress", json!(false))
        .responding("quests.accept", [response(json!(true), vec![])])
        .responding("quests.complete", [response(Value::Null, vec![])])
        .responding("player.joinMap", [response(Value::Null, vec![])]);
    let (mut session, link) = testing::start_probe(&host, movie);
    common::log_in(&link);
    emit(&link, map("buyhouse"));
    assert_eq!(
        run(
            &mut session,
            common::package("lucent-dailies", "scripts/spin-wheel-of-doom.js", &[]),
            json!({}),
        ),
        ScriptOutcome::Completed
    );
    assert_result(
        run(&mut session, wheel(), json!({})),
        json!({
            "daily":{"status":"unavailable"},"weekly":{"status":"unavailable"}
        }),
    );
    assert_eq!(calls(&link), vec![json!(["quests.getMultiple", ["3075"]])]);
}

#[test]
fn wheel_reports_an_unfinishable_daily_quest() {
    let host = testing::headless_host();
    let movie = common::ready_movie()
        .constant("player.isMember", json!(true))
        .constant("world.isActionAvailable", json!(true))
        .constant("quests.isAvailable", json!(true))
        .constant("quests.canComplete", json!(false))
        .responding(
            "quests.getMultiple",
            [response(
                Value::Null,
                vec![json!({
                    "cmd":"getQuests","quests":{"3075":{"QuestID":3075,"sName":"Daily Wheel"}}
                })],
            )],
        )
        .responding(
            "quests.isInProgress",
            [false, false, true].map(|active| response(json!(active), vec![])),
        )
        .responding("player.joinMap", [response(Value::Null, vec![map("doom")])])
        .responding("quests.accept", [response(json!(true), vec![])])
        .responding("quests.complete", [response(Value::Null, vec![])]);
    let (mut session, link) = testing::start_probe(&host, movie);
    common::log_in(&link);
    emit(&link, map("buyhouse"));
    assert_result(
        run(&mut session, wheel(), json!({})),
        json!({
            "daily":{"status":"failed"},"weekly":{"status":"unavailable"}
        }),
    );
    let actions: Vec<_> = calls(&link)
        .into_iter()
        .filter(|call| call[0] != "quests.isInProgress")
        .collect();
    assert_eq!(
        actions,
        vec![
            json!(["quests.getMultiple", ["3075"]]),
            json!(["player.joinMap", ["doom"]]),
            json!(["quests.accept", [3075.0, false]]),
        ]
    );
}

#[test]
fn consumables_rejects_a_nonpositive_quantity() {
    let host = testing::headless_host();
    let (mut session, link) = testing::start_probe(
        &host,
        common::ready_movie().responding("bank.loadItems", [response(Value::Null, vec![])]),
    );
    common::log_in(&link);
    emit(&link, map("buyhouse"));
    assert_failed(
        run(&mut session, consumables(), json!({"quantity":0})),
        "@lucent/consumables/scripts/get-consumables.js:21:18: $.quantity: Use a number of 1 or more.",
    );
    assert_eq!(calls(&link), Vec::<Value>::new());
}

#[test]
fn wheel_uses_default_banking_for_a_nonboolean_input() {
    let host = testing::headless_host();
    let (mut session, link) = testing::start_probe(
        &host,
        common::ready_movie().responding("quests.getMultiple", [response(Value::Null, vec![])]),
    );
    common::log_in(&link);
    emit(&link, map("buyhouse"));
    assert_result(
        run(&mut session, wheel(), json!({"bankRewards":"yes"})),
        json!({"daily":{"status":"unavailable"},"weekly":{"status":"unavailable"}}),
    );
    assert_eq!(calls(&link), Vec::<Value>::new());
}

fn owned(
    link: &ScriptLink,
    container: lucent_protocol::item::ItemContainer,
    id: i64,
) -> Option<f64> {
    link.read(|state| {
        state
            .items
            .get(container, id)
            .map(|item| item.data().quantity)
    })
    .unwrap()
}

#[test]
fn consumables_uses_the_default_scroll_and_quantity_to_finish_its_quest() {
    let movie = common::ready_movie()
        .constant("world.isActionAvailable", json!(true))
        .constant("player.getGold", json!(0))
        .constant("quests.isAvailable", json!(true))
        .constant("quests.canComplete", json!(true))
        .responding("quests.getMultiple", [response(Value::Null, vec![json!({
            "cmd":"getQuests","quests":{"2330":{
                "QuestID":2330,"sName":"Scroll of Enrage",
                "Rewards":[{"ItemID":903,"sName":"Scroll of Enrage","iQty":1,"iStk":100}],
                "RequiredItems":[{"ItemID":904,"sName":"Enrage Ink","iQty":1}]
            }}
        })])])
        .responding("quests.isInProgress", [false,false,false,true,true,false].map(|active| response(json!(active), vec![])))
        .responding("quests.accept", [response(json!(true), vec![])])
        .responding("player.joinMap", [response(Value::Null, vec![map("spellcraft")])])
        .responding("quests.complete", [response(Value::Null, vec![
            json!({"cmd":"ccqr","QuestID":2330,"bSuccess":1}),
            json!({"cmd":"turnIn","sItems":"904:1"}),
            json!({"cmd":"addItems","items":{"903":{"ItemID":903,"sName":"Scroll of Enrage","iQty":1}}}),
        ])])
        .responding("shops.buy", [response(Value::Null, vec![])]);
    let host = testing::headless_host();
    let (mut session, link) = testing::start_probe(&host, movie);
    common::log_in(&link);
    emit(&link, map("buyhouse"));
    emit(
        &link,
        json!({"cmd":"addItems","items":{
            "903":{"ItemID":903,"sName":"Scroll of Enrage","iQty":99,"iStk":100},
            "904":{"ItemID":904,"sName":"Enrage Ink","iQty":1}
        }}),
    );
    assert_eq!(
        run(&mut session, consumables(), json!({})),
        ScriptOutcome::Completed
    );
    assert_eq!(
        owned(&link, lucent_protocol::item::ItemContainer::Inventory, 903),
        Some(100.0)
    );
    assert_eq!(
        owned(&link, lucent_protocol::item::ItemContainer::Inventory, 904),
        None
    );
    let actions: Vec<_> = calls(&link)
        .into_iter()
        .filter(|call| call[0] != "quests.isInProgress")
        .collect();
    assert_eq!(
        actions,
        vec![
            json!(["quests.getMultiple", ["2330"]]),
            json!(["player.joinMap", ["spellcraft"]]),
            json!(["quests.accept", [2330.0, true]]),
            json!(["quests.complete", [2330.0, 1.0, -1.0, false]]),
        ]
    );
}

#[test]
fn consumables_crafts_a_potion_and_consumes_its_reagents() {
    let movie = common::ready_movie()
        .constant("world.isActionAvailable", json!(true))
        .constant("player.getGold", json!(0))
        .constant("player.getFactions", json!([{"FactionID":1,"sName":"Alchemy","iRank":10}]))
        .responding("player.joinMap", [response(Value::Null, vec![map("alchemy")])])
        .responding("flash.callGameFunction", [
            response(json!(""), vec![json!({"cmd":"alchOnStart"})]),
            response(json!(""), vec![
                json!({"cmd":"alchComplete"}),
                json!({"cmd":"turnIn","sItems":"11473:1,11474:1,905:1"}),
                json!({"cmd":"addItems","items":{"902":{"ItemID":902,"sName":"Sage Tonic","iQty":1}}}),
            ]),
        ])
        .responding("shops.buy", [response(Value::Null, vec![])]);
    let host = testing::headless_host();
    let (mut session, link) = testing::start_probe(&host, movie);
    common::log_in(&link);
    emit(&link, map("buyhouse"));
    emit(
        &link,
        json!({"cmd":"addItems","items":{
            "11473":{"ItemID":11473,"sName":"Arashtite Ore","iQty":1},
            "11474":{"ItemID":11474,"sName":"Dried Slime","iQty":1},
            "905":{"ItemID":905,"sName":"Dragon Runestone","iQty":1}
        }}),
    );
    assert_eq!(
        run(
            &mut session,
            consumables(),
            json!({
                "potions":["Sage Tonic"],"scrolls":[],"quantity":1,"potionMethod":"Craft"
            })
        ),
        ScriptOutcome::Completed
    );
    use lucent_protocol::item::ItemContainer::Inventory;
    assert_eq!(owned(&link, Inventory, 902), Some(1.0));
    for id in [11473, 11474, 905] {
        assert_eq!(owned(&link, Inventory, id), None);
    }
    assert_eq!(
        calls(&link),
        vec![
            json!(["player.joinMap", ["alchemy"]]),
            json!([
                "flash.callGameFunction",
                [
                    "sfc.sendString",
                    "%xt%zm%crafting%1%getAlchWait%11473%11474%true%Ready to Mix%Arashtite Ore%Dried Slime%Gebo%Int%"
                ]
            ]),
            json!([
                "flash.callGameFunction",
                [
                    "sfc.sendString",
                    "%xt%zm%crafting%1%checkAlchComplete%11473%11474%true%Mix Complete%Arashtite Ore%Dried Slime%Gebo%Int%"
                ]
            ]),
        ]
    );
}

#[test]
fn wheel_banks_the_boost_and_prize_reported_by_its_spin() {
    let movie = common::ready_movie()
        .constant("player.isMember", json!(true))
        .constant("world.isActionAvailable", json!(true))
        .constant("quests.isAvailable", json!(true))
        .constant("quests.canComplete", json!(true))
        .constant("quests.getMaxTurnIns", json!(1))
        .constant("bank.isOpen", json!(true))
        .constant("bank.getSlots", json!(20))
        .constant("bank.getUsedSlots", json!(0))
        .responding(
            "quests.getMultiple",
            [response(
                Value::Null,
                vec![json!({
                    "cmd":"getQuests","quests":{"3075":{"QuestID":3075,"sName":"Daily Wheel"}}
                })],
            )],
        )
        .responding(
            "quests.isInProgress",
            [false, false, true, true].map(|active| response(json!(active), vec![])),
        )
        .responding("player.joinMap", [response(Value::Null, vec![map("doom")])])
        .responding("quests.accept", [response(json!(true), vec![])])
        .responding(
            "quests.complete",
            [response(
                Value::Null,
                vec![
                    json!({"cmd":"ccqr","QuestID":3075,"bSuccess":1}),
                    json!({"cmd":"Wheel","dropItems":{"19189":{
                "ItemID":19189,"sName":"XP Boost","sType":"Item","iQty":1
            }},"Item":{"ItemID":901,"sName":"Wheel Prize","sType":"Item","iQty":1}}),
                ],
            )],
        )
        .responding(
            "bank.deposit",
            [19189, 901].map(|id| {
                response(
                    json!(true),
                    vec![json!({"cmd":"bankFromInv","ItemID":id,"bSuccess":1})],
                )
            }),
        );
    let host = testing::headless_host();
    let (mut session, link) = testing::start_probe(&host, movie);
    common::log_in(&link);
    emit(&link, map("buyhouse"));
    assert_result(
        run(&mut session, wheel(), json!({"bankRewards":true})),
        json!({
            "daily":{"status":"completed","banking":"completed"},"weekly":{"status":"unavailable"}
        }),
    );
    use lucent_protocol::item::ItemContainer::{Bank, Inventory};
    for id in [19189, 901] {
        assert_eq!(owned(&link, Inventory, id), None);
        assert_eq!(owned(&link, Bank, id), Some(1.0));
    }
    let actions: Vec<_> = calls(&link)
        .into_iter()
        .filter(|call| call[0] != "quests.isInProgress")
        .collect();
    assert_eq!(
        actions,
        vec![
            json!(["quests.getMultiple", ["3075"]]),
            json!(["player.joinMap", ["doom"]]),
            json!(["quests.accept", [3075.0, false]]),
            json!(["quests.complete", [3075.0, 1.0, -1.0, false]]),
            json!(["bank.deposit",[{"name":"XP Boost"}]]),
            json!(["bank.deposit",[{"name":"Wheel Prize"}]]),
        ]
    );
}

#[test]
fn wheel_rejects_an_invalid_public_banking_option() {
    let host = testing::headless_host();
    let (mut session, link) = testing::start_probe(
        &host,
        common::ready_movie().responding("quests.getMultiple", [response(Value::Null, vec![])]),
    );
    common::log_in(&link);
    emit(&link, map("buyhouse"));
    let script = common::package(
        "lucent-dailies",
        "tests/invalid-wheel.js",
        &[(
            "tests/invalid-wheel.js",
            r#"
        import { spinWheelOfDoom } from "@lucent/dailies";
        export default function* () {
            yield* spinWheelOfDoom({ bankRewards: "yes" });
        }
        "#,
        )],
    );
    assert_failed(
        run(&mut session, script, json!({})),
        "@lucent/dailies/lib/wheel-of-doom.js:74:41: $.bankRewards: Expected boolean",
    );
    assert_eq!(calls(&link), Vec::<Value>::new());
}
