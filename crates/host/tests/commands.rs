use std::time::Duration;

use lucent_host::ScriptLink;
use lucent_host::bridge::Value as LoaderValue;
use lucent_host::testing::{self, ProbeMovie};
use lucent_protocol::command::Command;
use lucent_protocol::query::{Query, answer};
use serde_json::{Value, json};

fn command(link: &ScriptLink, command: Value) -> Result<Value, String> {
    let command: Command = serde_json::from_value(command).unwrap();
    link.command(command)
        .recv_timeout(Duration::from_secs(5))
        .map(|json: String| serde_json::from_str(&json).unwrap())
        .map_err(|error| error.to_string())
}

fn query(link: &ScriptLink, query: Value) -> Value {
    let query: Query = serde_json::from_value(query).unwrap();
    let answer = link
        .read(move |projection| answer(projection, query))
        .unwrap();
    serde_json::from_str(&answer).unwrap()
}

fn emit(link: &ScriptLink, name: &str, payload: String) {
    assert_eq!(
        link.call_loader(
            "probe.emit",
            vec![
                LoaderValue::String(name.into()),
                LoaderValue::String(payload)
            ],
        )
        .recv_timeout(Duration::from_secs(5)),
        Ok(LoaderValue::Bool(true))
    );
}

fn packet(link: &ScriptLink, data: Value) {
    emit(
        link,
        "onExtensionResponse",
        json!({"type": "json", "dataObj": data}).to_string(),
    );
}

fn start(movie: ProbeMovie) -> (lucent_host::Host, lucent_host::Session, ScriptLink) {
    let host = testing::headless_host();
    let (session, link) = testing::start_probe(&host, movie);
    (host, session, link)
}

fn enter_area(link: &ScriptLink, area: &str) {
    packet(
        link,
        json!({
            "cmd": "moveToArea",
            "areaName": area,
            "uoBranch": [{"entID": 7, "uoName": "player-a", "strFrame": "Enter", "intHP": 100}],
            "mondef": [{"MonID": 1, "strMonName": "Frogzard"}, {"MonID": 2, "strMonName": "Sneevil"}],
            "monBranch": [
                {"MonID": 2, "MonMapID": 3, "intHP": 80},
                {"MonID": 1, "MonMapID": 2, "intHP": 50}
            ]
        }),
    );
}

fn become_ready(link: &ScriptLink) {
    enter_area(link, "testroom-1");
    packet(
        link,
        json!({"cmd": "initInventory", "items": [{"ItemID": 1, "sName": "Sword"}]}),
    );
    packet(link, json!({"cmd": "loadHouseInventory", "items": []}));
    assert_eq!(
        query(link, json!({"kind": "readiness"}))["missing"],
        json!([])
    );
}

fn fields(value: &Value, keys: &[&str]) -> Value {
    match value {
        Value::Null => Value::Null,
        Value::Array(values) => values.iter().map(|value| fields(value, keys)).collect(),
        Value::Object(object) => Value::Object(
            keys.iter()
                .map(|key| {
                    let field = object
                        .get(*key)
                        .unwrap_or_else(|| panic!("{key} is missing from {value}"));
                    ((*key).to_owned(), field.clone())
                })
                .collect(),
        ),
        other => panic!("expected an object, an array or null, got {other}"),
    }
}

fn container_items(link: &ScriptLink, container: &str) -> Value {
    fields(
        &query(link, json!({"kind": "items", "container": container})),
        &["itemId", "name"],
    )
}

fn hydration(link: &ScriptLink, container: &str) -> Value {
    query(
        link,
        json!({"kind": "itemHydration", "container": container}),
    )
}

fn auth(link: &ScriptLink) -> Value {
    query(link, json!({"kind": "auth"}))
}

fn logged_out() -> Value {
    json!({"loggedIn": false, "username": "", "password": "", "servers": []})
}

fn monster_fields(monster: &Value) -> Value {
    fields(
        monster,
        &["monsterMapId", "monsterId", "name", "race", "hp", "maxHp"],
    )
}

fn load_shop(link: &ScriptLink) {
    packet(
        link,
        json!({
            "cmd": "loadShop",
            "ShopID": 3,
            "sName": "Armory",
            "items": [{"ItemID": 1, "ShopItemID": 10, "sName": "Sword"}]
        }),
    );
}

fn drop_frog_leg(link: &ScriptLink) {
    packet(
        link,
        json!({"cmd": "dropItem", "items": {"5": {"ItemID": 5, "sName": "Frog Leg"}}}),
    );
}

#[test]
fn refresh_bank_stores_decodable_non_class_items_and_counts_one_hydration() {
    let (_host, _session, link) = start(ProbeMovie::new().constant(
        "bank.getItems",
        json!([
            {"ItemID": 1, "sName": "Sword", "sType": "Sword"},
            {"ItemID": 2, "sName": "Warrior", "sType": "Class"},
            {"sName": "Broken"}
        ]),
    ));
    assert_eq!(hydration(&link, "bank"), json!(0));
    assert_eq!(
        command(
            &link,
            json!({"kind": "refreshContainer", "container": "bank"})
        ),
        Ok(json!(1))
    );
    assert_eq!(
        container_items(&link, "bank"),
        json!([{"itemId": 1, "name": "Sword"}])
    );
    assert_eq!(hydration(&link, "bank"), json!(1));
}

#[test]
fn refresh_inventory_replaces_the_baseline_and_bumps_hydration_again() {
    let (_host, _session, link) = start(ProbeMovie::new().constant(
        "inventory.getItems",
        json!([{"ItemID": 2, "sName": "Shield"}]),
    ));
    packet(
        &link,
        json!({"cmd": "initInventory", "items": [{"ItemID": 1, "sName": "Sword"}]}),
    );
    assert_eq!(
        container_items(&link, "inventory"),
        json!([{"itemId": 1, "name": "Sword"}])
    );
    assert_eq!(hydration(&link, "inventory"), json!(1));
    assert_eq!(
        command(
            &link,
            json!({"kind": "refreshContainer", "container": "inventory"})
        ),
        Ok(json!(1))
    );
    assert_eq!(
        container_items(&link, "inventory"),
        json!([{"itemId": 2, "name": "Shield"}])
    );
    assert_eq!(hydration(&link, "inventory"), json!(2));
}

#[test]
fn refresh_house_without_a_loader_callback_fails_and_keeps_the_house() {
    let (_host, _session, link) = start(ProbeMovie::new());
    packet(
        &link,
        json!({"cmd": "loadHouseInventory", "items": [{"ItemID": 3, "sName": "Chair"}]}),
    );
    assert_eq!(
        command(
            &link,
            json!({"kind": "refreshContainer", "container": "house"})
        ),
        Err("the loader has not registered callback house.getItems".into())
    );
    assert_eq!(
        container_items(&link, "house"),
        json!([{"itemId": 3, "name": "Chair"}])
    );
    assert_eq!(hydration(&link, "house"), json!(1));
}

#[test]
fn refresh_container_rejects_the_drop_container() {
    let parsed =
        serde_json::from_value::<Command>(json!({"kind": "refreshContainer", "container": "drop"}));
    assert_eq!(
        parsed.err().map(|error| error.to_string()),
        Some("unknown variant `drop`, expected one of `bank`, `house`, `inventory`".into())
    );
}

#[test]
fn confirm_shop_closed_clears_the_shop_when_the_loader_reports_it_closed() {
    let (_host, _session, link) = start(ProbeMovie::new().constant("shops.isOpen", json!(false)));
    load_shop(&link);
    assert_eq!(query(&link, json!({"kind": "shop"}))["id"], json!(3));
    assert_eq!(
        command(&link, json!({"kind": "confirmShopClosed", "shopId": 3})),
        Ok(json!(true))
    );
    assert_eq!(query(&link, json!({"kind": "shop"})), Value::Null);
    assert_eq!(container_items(&link, "shop"), json!([]));
}

#[test]
fn confirm_shop_closed_keeps_a_shop_the_loader_reports_open() {
    let (_host, _session, link) = start(ProbeMovie::new().constant("shops.isOpen", json!(true)));
    load_shop(&link);
    assert_eq!(
        command(&link, json!({"kind": "confirmShopClosed"})),
        Ok(json!(false))
    );
    assert_eq!(query(&link, json!({"kind": "shop"}))["id"], json!(3));
    assert_eq!(
        container_items(&link, "shop"),
        json!([{"itemId": 1, "name": "Sword"}])
    );
}

#[test]
fn reject_drop_removes_the_drop_when_the_loader_accepts() {
    let (_host, _session, link) = start(ProbeMovie::new().constant("drops.reject", json!(true)));
    drop_frog_leg(&link);
    assert_eq!(
        command(
            &link,
            json!({"kind": "rejectDrop", "selector": " frog LEG "})
        ),
        Ok(json!(true))
    );
    assert_eq!(container_items(&link, "drop"), json!([]));
}

#[test]
fn reject_drop_keeps_the_drop_when_the_loader_refuses() {
    let (_host, _session, link) = start(ProbeMovie::new().constant("drops.reject", json!(false)));
    drop_frog_leg(&link);
    assert_eq!(
        command(&link, json!({"kind": "rejectDrop", "selector": 5})),
        Ok(json!(false))
    );
    assert_eq!(
        container_items(&link, "drop"),
        json!([{"itemId": 5, "name": "Frog Leg"}])
    );
}

#[test]
fn reject_drop_of_an_item_not_dropped_is_false_without_a_loader_call() {
    let (_host, _session, link) = start(ProbeMovie::new());
    drop_frog_leg(&link);
    assert_eq!(
        command(
            &link,
            json!({"kind": "rejectDrop", "selector": "Dragon Scale"})
        ),
        Ok(json!(false))
    );
    assert_eq!(
        container_items(&link, "drop"),
        json!([{"itemId": 5, "name": "Frog Leg"}])
    );
}

#[test]
fn refresh_cells_stores_the_loaded_cells_until_the_area_changes() {
    let (_host, _session, link) = start(
        ProbeMovie::new()
            .constant("world.isLoaded", json!(true))
            .constant("world.getCells", json!(["Enter", "r2"])),
    );
    enter_area(&link, "testroom-1");
    assert_eq!(query(&link, json!({"kind": "cells"})), json!([]));
    assert_eq!(
        command(&link, json!({"kind": "refreshCells"})),
        Ok(json!(["Enter", "r2"]))
    );
    assert_eq!(
        query(&link, json!({"kind": "cells"})),
        json!(["Enter", "r2"])
    );
    enter_area(&link, "otherroom-2");
    assert_eq!(query(&link, json!({"kind": "cells"})), json!([]));
}

#[test]
fn refresh_cells_while_the_world_is_not_loaded_returns_no_cells_without_reading_them() {
    let (_host, _session, link) = start(ProbeMovie::new().constant("world.isLoaded", json!(false)));
    enter_area(&link, "testroom-1");
    assert_eq!(
        command(&link, json!({"kind": "refreshCells"})),
        Ok(json!([]))
    );
    assert_eq!(query(&link, json!({"kind": "cells"})), json!([]));
}

#[test]
fn refresh_cells_with_a_non_string_cell_fails_and_stores_nothing() {
    let (_host, _session, link) = start(
        ProbeMovie::new()
            .constant("world.isLoaded", json!(true))
            .constant("world.getCells", json!(["Enter", 3])),
    );
    enter_area(&link, "testroom-1");
    assert!(command(&link, json!({"kind": "refreshCells"})).is_err());
    assert_eq!(query(&link, json!({"kind": "cells"})), json!([]));
}

fn cell_pads_movie() -> ProbeMovie {
    ProbeMovie::new()
        .constant("world.isLoaded", json!(true))
        .constant("world.getCellPads", json!(["Left", "Right"]))
}

#[test]
fn refresh_cell_pads_stores_the_pads_until_self_moves_to_another_cell() {
    let (_host, _session, link) = start(cell_pads_movie());
    enter_area(&link, "testroom-1");
    assert_eq!(query(&link, json!({"kind": "cellPads"})), json!([]));
    assert_eq!(
        command(&link, json!({"kind": "refreshCellPads"})),
        Ok(json!(["Left", "Right"]))
    );
    assert_eq!(
        query(&link, json!({"kind": "cellPads"})),
        json!(["Left", "Right"])
    );
    packet(
        &link,
        json!({"cmd": "uotls", "unm": "player-a", "o": {"strFrame": "r2", "strPad": "Left"}}),
    );
    assert_eq!(query(&link, json!({"kind": "me"}))["cell"], json!("r2"));
    assert_eq!(query(&link, json!({"kind": "cellPads"})), json!([]));
}

#[test]
fn refresh_cell_pads_stores_the_pads_until_the_area_changes() {
    let (_host, _session, link) = start(cell_pads_movie());
    enter_area(&link, "testroom-1");
    assert_eq!(
        command(&link, json!({"kind": "refreshCellPads"})),
        Ok(json!(["Left", "Right"]))
    );
    assert_eq!(
        query(&link, json!({"kind": "cellPads"})),
        json!(["Left", "Right"])
    );
    enter_area(&link, "otherroom-2");
    assert_eq!(query(&link, json!({"kind": "cellPads"})), json!([]));
}

#[test]
fn hydrate_monster_returns_a_host_match_without_calling_the_loader() {
    let (_host, _session, link) = start(ProbeMovie::new());
    enter_area(&link, "testroom-1");
    assert_eq!(
        command(
            &link,
            json!({"kind": "hydrateMonster", "selector": "frogzard"})
        )
        .map(|monster| fields(&monster, &["monsterMapId", "name"])),
        Ok(json!({"monsterMapId": 2, "name": "Frogzard"}))
    );
}

#[test]
fn hydrate_monster_adds_a_flat_loader_payload_to_the_area() {
    let (_host, _session, link) = start(ProbeMovie::new().constant(
        "world.getMonster",
        json!({
            "MonMapID": 4, "MonID": 5, "strMonName": "Grizzly", "sRace": "Beast",
            "intHP": 10, "intHPMax": 10
        }),
    ));
    enter_area(&link, "testroom-1");
    assert_eq!(
        query(&link, json!({"kind": "monster", "selector": "grizzly"})),
        Value::Null
    );
    let grizzly = json!({
        "monsterMapId": 4, "monsterId": 5, "name": "Grizzly", "race": "Beast",
        "hp": 10.0, "maxHp": 10.0
    });
    assert_eq!(
        command(
            &link,
            json!({"kind": "hydrateMonster", "selector": "Grizzly"})
        )
        .map(|monster| monster_fields(&monster)),
        Ok(grizzly.clone())
    );
    assert_eq!(
        monster_fields(&query(
            &link,
            json!({"kind": "monster", "selector": "grizzly"})
        )),
        grizzly
    );
    assert_eq!(
        fields(
            &query(&link, json!({"kind": "monsters"})),
            &["monsterMapId"]
        ),
        json!([{"monsterMapId": 3}, {"monsterMapId": 2}, {"monsterMapId": 4}])
    );
}

#[test]
fn hydrate_monster_merges_a_nested_payload_with_data_leaf_winning() {
    let (_host, _session, link) = start(ProbeMovie::new().constant(
        "world.getMonster",
        json!({
            "dataLeaf": {"MonMapID": 4, "strMonName": "Grizzly", "intHP": 5},
            "objData": {
                "MonMapID": 4, "MonID": 5, "strMonName": "Bear", "sRace": "Beast",
                "intHP": 10, "intHPMax": 10
            }
        }),
    ));
    enter_area(&link, "testroom-1");
    let grizzly = json!({
        "monsterMapId": 4, "monsterId": 5, "name": "Grizzly", "race": "Beast",
        "hp": 5.0, "maxHp": 10.0
    });
    assert_eq!(
        command(
            &link,
            json!({"kind": "hydrateMonster", "selector": {"monsterMapId": 4}})
        )
        .map(|monster| monster_fields(&monster)),
        Ok(grizzly.clone())
    );
    assert_eq!(
        monster_fields(&query(&link, json!({"kind": "monster", "selector": 4}))),
        grizzly
    );
}

#[test]
fn hydrate_monster_stores_nothing_when_the_loader_returns_null() {
    let (_host, _session, link) =
        start(ProbeMovie::new().constant("world.getMonster", Value::Null));
    enter_area(&link, "testroom-1");
    assert_eq!(
        command(
            &link,
            json!({"kind": "hydrateMonster", "selector": "Grizzly"})
        ),
        Ok(Value::Null)
    );
    assert_eq!(
        fields(
            &query(&link, json!({"kind": "monsters"})),
            &["monsterMapId"]
        ),
        json!([{"monsterMapId": 3}, {"monsterMapId": 2}])
    );
}

#[test]
fn login_installs_trimmed_credentials_without_marking_logged_in() {
    let (_host, _session, link) = start(ProbeMovie::new().constant("auth.login", Value::Null));
    assert_eq!(
        command(
            &link,
            json!({"kind": "login", "username": "  player-a ", "password": "secret"})
        ),
        Ok(json!(true))
    );
    assert_eq!(
        auth(&link),
        json!({"loggedIn": false, "username": "player-a", "password": "secret", "servers": []})
    );
}

#[test]
fn login_credentials_bind_self_by_username_instead_of_the_loader_user_id() {
    let (_host, _session, link) = start(
        ProbeMovie::new()
            .constant("auth.login", Value::Null)
            .constant("player.getUserId", json!(99)),
    );
    assert_eq!(
        command(
            &link,
            json!({"kind": "login", "username": "player-a", "password": "secret"})
        ),
        Ok(json!(true))
    );
    packet(
        &link,
        json!({
            "cmd": "moveToArea",
            "areaName": "testroom-1",
            "uoBranch": [
                {"entID": 99, "uoName": "player-b"},
                {"entID": 8, "uoName": "player-a"}
            ]
        }),
    );
    assert_eq!(
        fields(
            &query(&link, json!({"kind": "me"})),
            &["entityId", "username"]
        ),
        json!({"entityId": 8, "username": "player-a"})
    );
}

#[test]
fn login_without_a_loader_callback_fails_and_keeps_the_previous_credentials() {
    let (_host, _session, link) = start(ProbeMovie::new());
    assert_eq!(
        command(
            &link,
            json!({"kind": "login", "username": "player-a", "password": "secret"})
        ),
        Err("the loader has not registered callback auth.login".into())
    );
    assert_eq!(auth(&link), logged_out());
}

#[test]
fn login_with_an_empty_password_or_blank_username_is_false_without_a_loader_call() {
    let (_host, _session, link) = start(ProbeMovie::new());
    assert_eq!(
        command(
            &link,
            json!({"kind": "login", "username": "player-a", "password": ""})
        ),
        Ok(json!(false))
    );
    assert_eq!(
        command(
            &link,
            json!({"kind": "login", "username": "   ", "password": "secret"})
        ),
        Ok(json!(false))
    );
    assert_eq!(auth(&link), logged_out());
}

#[test]
fn logout_clears_credentials_and_servers_even_when_the_loader_call_fails() {
    let (_host, _session, link) = start(
        ProbeMovie::new()
            .constant("auth.login", Value::Null)
            .constant("auth.getServers", json!([{"sName": "Artix"}])),
    );
    command(
        &link,
        json!({"kind": "login", "username": "player-a", "password": "secret"}),
    )
    .unwrap();
    command(&link, json!({"kind": "refreshServers"})).unwrap();
    assert_eq!(auth(&link)["servers"][0]["name"], json!("Artix"));
    let _ = command(&link, json!({"kind": "logout"}));
    assert_eq!(auth(&link), logged_out());
}

#[test]
fn logout_returns_null_when_the_loader_call_succeeds() {
    let (_host, _session, link) = start(
        ProbeMovie::new()
            .constant("auth.login", Value::Null)
            .constant("auth.logout", Value::Null),
    );
    command(
        &link,
        json!({"kind": "login", "username": "player-a", "password": "secret"}),
    )
    .unwrap();
    assert_eq!(command(&link, json!({"kind": "logout"})), Ok(Value::Null));
    assert_eq!(auth(&link), logged_out());
}

#[test]
fn logout_clears_credentials_when_the_disconnect_it_causes_resets_the_connection() {
    let (_host, _session, link) = start(
        ProbeMovie::new()
            .constant("auth.login", Value::Null)
            .raising("auth.logout", "onConnection", "OnConnectionLost"),
    );
    command(
        &link,
        json!({"kind": "login", "username": "player-a", "password": "secret"}),
    )
    .unwrap();
    assert_eq!(command(&link, json!({"kind": "logout"})), Ok(Value::Null));
    assert_eq!(auth(&link), logged_out());
}

#[test]
fn refresh_credentials_before_readiness_returns_stored_values_without_a_loader_call() {
    let (_host, _session, link) = start(ProbeMovie::new().constant("auth.login", Value::Null));
    command(
        &link,
        json!({"kind": "login", "username": "player-a", "password": "secret"}),
    )
    .unwrap();
    assert_eq!(
        command(&link, json!({"kind": "refreshCredentials"})),
        Ok(json!({"username": "player-a", "password": "secret"}))
    );
}

#[test]
fn refresh_credentials_when_ready_stores_the_trimmed_username_and_the_raw_password() {
    let (_host, _session, link) =
        start(ProbeMovie::new().constant("flash.getGameObjectS", json!("\"  player-c \"")));
    become_ready(&link);
    assert_eq!(
        command(&link, json!({"kind": "refreshCredentials"})),
        Ok(json!({"username": "player-c", "password": "  player-c "}))
    );
    assert_eq!(
        auth(&link),
        json!({"loggedIn": false, "username": "player-c", "password": "  player-c ", "servers": []})
    );
}

#[test]
fn refresh_servers_stores_the_list_with_full_derived_from_count_and_max() {
    let (_host, _session, link) = start(ProbeMovie::new().constant(
        "auth.getServers",
        json!([
            {"sName": "Artix", "iCount": 10, "iMax": 10},
            {"sName": "Yorumi", "iCount": 2, "iMax": 10, "sLang": "en", "bUpg": true}
        ]),
    ));
    let servers = json!([
        {
            "chat": 0.0, "count": 10.0, "language": "", "max": 10.0, "memberOnly": false,
            "name": "Artix", "online": true, "full": true
        },
        {
            "chat": 0.0, "count": 2.0, "language": "en", "max": 10.0, "memberOnly": true,
            "name": "Yorumi", "online": true, "full": false
        }
    ]);
    assert_eq!(
        command(&link, json!({"kind": "refreshServers"})),
        Ok(servers.clone())
    );
    assert_eq!(auth(&link)["servers"], servers);
}

#[test]
fn refresh_servers_answered_with_null_keeps_the_stored_servers() {
    let (_host, _session, link) = start(ProbeMovie::new().constant("auth.getServers", Value::Null));
    assert_eq!(
        command(&link, json!({"kind": "refreshServers"})),
        Ok(json!([]))
    );
    assert_eq!(auth(&link)["servers"], json!([]));
}

#[test]
fn refresh_logged_in_stores_the_loader_value_until_the_connection_is_lost() {
    let (_host, _session, link) = start(ProbeMovie::new().constant("auth.isLoggedIn", json!(true)));
    assert_eq!(auth(&link)["loggedIn"], json!(false));
    assert_eq!(
        command(&link, json!({"kind": "refreshLoggedIn"})),
        Ok(json!(true))
    );
    assert_eq!(auth(&link)["loggedIn"], json!(true));
    emit(&link, "onConnection", "OnConnectionLost".into());
    assert_eq!(auth(&link)["loggedIn"], json!(false));
}

#[test]
fn refresh_accepted_quests_returns_the_accepted_quests_with_metadata() {
    let (_host, _session, link) =
        start(ProbeMovie::new().constant("quests.getAccepted", json!([10])));
    packet(
        &link,
        json!({
            "cmd": "getQuests",
            "quests": {"10": {"sName": "Frog hunt"}, "12": {"sName": "Sneevil hunt"}}
        }),
    );
    assert_eq!(query(&link, json!({"kind": "acceptedQuests"})), json!([]));
    let frog_hunt = json!({
        "cadence": "none", "id": 10.0, "name": "Frog hunt", "once": false,
        "requirements": [], "rewards": []
    });
    assert_eq!(
        command(&link, json!({"kind": "refreshAcceptedQuests"})),
        Ok(json!([frog_hunt]))
    );
    assert_eq!(
        query(&link, json!({"kind": "acceptedQuests"})),
        json!([frog_hunt])
    );
}

fn load_consumables(link: &ScriptLink) {
    packet(
        link,
        json!({
            "cmd": "initInventory",
            "items": [
                {"ItemID": 1, "sName": "Sword", "sType": "Sword", "sES": "Weapon", "bEquip": true},
                {"ItemID": 2, "sName": "Potion A", "sType": "Item", "sLink": "potion-a", "bEquip": false},
                {"ItemID": 3, "sName": "Potion B", "sType": "Item", "sLink": "potion-b", "bEquip": true},
                {"ItemID": 4, "sName": "Token", "sType": "Item", "sLink": " None ", "bEquip": true}
            ]
        }),
    );
}

fn equipped(link: &ScriptLink) -> Value {
    fields(
        &query(link, json!({"kind": "items", "container": "inventory"})),
        &["itemId", "equipped"],
    )
}

#[test]
fn refresh_equipped_consumable_equips_the_slot_item_and_unequips_other_consumables() {
    let (_host, _session, link) =
        start(ProbeMovie::new().constant("combat.getConsumableSkillItem", json!({"itemId": 2})));
    load_consumables(&link);
    assert_eq!(
        command(&link, json!({"kind": "refreshEquippedConsumable"})),
        Ok(json!(2))
    );
    assert_eq!(
        equipped(&link),
        json!([
            {"itemId": 1, "equipped": true},
            {"itemId": 2, "equipped": true},
            {"itemId": 3, "equipped": false},
            {"itemId": 4, "equipped": true}
        ])
    );
}

#[test]
fn refresh_equipped_consumable_with_an_empty_slot_unequips_every_consumable() {
    let (_host, _session, link) =
        start(ProbeMovie::new().constant("combat.getConsumableSkillItem", Value::Null));
    load_consumables(&link);
    assert_eq!(
        command(&link, json!({"kind": "refreshEquippedConsumable"})),
        Ok(Value::Null)
    );
    assert_eq!(
        equipped(&link),
        json!([
            {"itemId": 1, "equipped": true},
            {"itemId": 2, "equipped": false},
            {"itemId": 3, "equipped": false},
            {"itemId": 4, "equipped": true}
        ])
    );
}

#[test]
fn commands_before_the_viewport_are_not_ready_and_stop_with_the_session() {
    let host = testing::headless_host();
    let mut session = host.start_session(testing::headless_surface()).unwrap();
    let link = session.take_script_link().unwrap();
    assert_eq!(
        link.command(Command::RefreshLoggedIn {})
            .recv_timeout(Duration::from_secs(5)),
        Err(lucent_host::bridge::Error::NotReady)
    );
    drop(session);
    assert_eq!(
        link.command(Command::RefreshLoggedIn {})
            .recv_timeout(Duration::from_secs(5)),
        Err(lucent_host::bridge::Error::Stopped)
    );
}
