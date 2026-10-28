use lucent_host::projection::{Projection, ReadinessKey, decode_item};
use lucent_protocol::item::ItemContainer;
use serde_json::json;

#[test]
fn retained_items_follow_replacement_but_not_reused_ids() {
    let mut state = Projection::default();
    let item = decode_item(
        &json!({"ItemID": 1, "CharItemID": 10, "sName": "A", "iQty": 2}),
        ItemContainer::Inventory,
    )
    .unwrap();
    state.items.replace(ItemContainer::Inventory, vec![item]);
    let held = state.items.get(ItemContainer::Inventory, 1).unwrap();
    let replacement = decode_item(
        &json!({"ItemID": 1, "sName": "B", "iQty": 7}),
        ItemContainer::Inventory,
    )
    .unwrap();
    state
        .items
        .replace(ItemContainer::Inventory, vec![replacement]);
    assert!(held.same_instance(&state.items.get(ItemContainer::Inventory, 1).unwrap()));
    assert_eq!(held.data().quantity, 7.0);
    assert_eq!(held.data().char_item_id, None);
    assert!(state.items.by_name(ItemContainer::Inventory, "a").is_none());
    state.items.replace(ItemContainer::Inventory, vec![]);
    let reinserted =
        decode_item(&json!({"ItemID": 1, "iQty": 9}), ItemContainer::Inventory).unwrap();
    state
        .items
        .replace(ItemContainer::Inventory, vec![reinserted]);
    assert!(!held.same_instance(&state.items.get(ItemContainer::Inventory, 1).unwrap()));
    assert_eq!(held.data().quantity, 7.0);
}

#[test]
fn readiness_requires_receipts_and_actual_baselines() {
    let mut state = Projection::default();
    for key in ReadinessKey::ALL {
        state.receipts.complete(key);
    }
    assert_eq!(
        state.readiness("battleon", true).missing,
        vec![ReadinessKey::HouseInventory, ReadinessKey::Inventory]
    );
    state.items.replace(ItemContainer::House, vec![]);
    state.items.replace(ItemContainer::Inventory, vec![]);
    assert!(state.readiness("battleon", true).missing.is_empty());
    assert_eq!(
        state.readiness("", false).missing,
        vec![ReadinessKey::Map, ReadinessKey::Player]
    );
    state.connection("OnConnection");
    assert_eq!(state.readiness("battleon", true).missing, ReadinessKey::ALL);
}

#[test]
fn connections_reset_receipts_and_items_but_preserve_credentials() {
    let mut state = Projection::default();
    state.auth.username = "player-a".into();
    state.auth.password = "synthetic-password".into();
    state.auth.logged_in = true;
    state
        .receipts
        .fail(ReadinessKey::Inventory, "bad baseline".into());
    state.items.replace(ItemContainer::House, vec![]);
    state.connection("OnConnection");
    assert!(state.auth.logged_in);
    assert_eq!(state.receipts.epoch(), 1);
    assert_eq!(state.items.hydration(ItemContainer::House), 0);
    assert!(state.receipts.failures().is_empty());
    state.connection("OnConnectionLost");
    assert!(!state.auth.logged_in);
    assert_eq!(state.auth.username, "player-a");
    assert_eq!(state.auth.password, "synthetic-password");
    assert_eq!(state.receipts.epoch(), 2);
    state.connection("other");
    assert_eq!(state.receipts.epoch(), 2);
}

#[test]
fn reconnect_invalidates_pending_hydration_without_rebinding_old_entities() {
    use lucent_host::projection::{PacketProjection, world::HydrationResult};
    use lucent_protocol::{Packet, PacketEncoding};
    let mut state = Projection::default();
    let packet = Packet::Extension {
        command: "moveToArea".into(),
        data: json!({"areaName":"testroom-1","uoBranch":[{"entID":7,"uoName":"player-a"}]}),
        raw: String::new(),
        encoding: PacketEncoding::Json,
    };
    let PacketProjection::NeedHydration(pending) = state.packet(&packet, 1000.0, true) else {
        panic!("expected self hydration");
    };
    assert_eq!(pending.request.method.name(), "player.getUserId");
    assert_eq!(state.world.map.name, "testroom");
    state.connection("OnConnectionLost");
    assert!(
        state
            .resume_hydration(pending, HydrationResult::Value(json!(7)), 2000.0)
            .is_err()
    );
    assert!(state.world.get_me().is_none());
    assert!(state.world.players().is_empty());
    assert_eq!(state.current_readiness().missing, ReadinessKey::ALL);
}

#[test]
fn server_refresh_preserves_live_handles_and_connection_reset_keeps_the_cache() {
    use lucent_host::projection::auth::decode_servers;
    let mut state = Projection::default();
    state
        .auth
        .set_servers(decode_servers(&json!([{"sName":"server-a","iCount":2,"iMax":5}])).unwrap());
    let held = state.auth.servers().remove(0);
    assert!(!held.snapshot().full);
    state.auth.set_servers(
        decode_servers(&json!([{"sName":"server-b"},{"sName":"server-a","iCount":5,"iMax":5}]))
            .unwrap(),
    );
    assert!(held.same_instance(&state.auth.servers()[0]));
    assert!(held.snapshot().full);
    state.connection("OnConnectionLost");
    assert_eq!(state.auth.servers().len(), 2);
    state
        .auth
        .set_servers(decode_servers(&json!([{"sName":"server-b"}])).unwrap());
    state
        .auth
        .set_servers(decode_servers(&json!([{"sName":"server-a","iCount":1}])).unwrap());
    assert!(!held.same_instance(&state.auth.servers()[0]));
    assert_eq!(held.snapshot().data.count, 5.0);
}

fn apply(
    state: &mut Projection,
    command: &str,
    data: serde_json::Value,
) -> Vec<lucent_protocol::event::ProjectionEvent> {
    use lucent_host::projection::PacketProjection;
    use lucent_protocol::{Packet, PacketEncoding};
    let packet = Packet::Extension {
        command: command.into(),
        data,
        raw: String::new(),
        encoding: PacketEncoding::Json,
    };
    let PacketProjection::Applied(output) = state.packet(&packet, 1000.0, false) else {
        panic!("expected completed projection");
    };
    output.events
}

fn world_with_self() -> Projection {
    let mut state = Projection::default();
    state.auth.username = "player-a".into();
    apply(
        &mut state,
        "moveToArea",
        json!({
            "areaName":"testroom-1", "uoBranch":[
                {"entID":7,"uoName":"player-a"}, {"entID":8,"uoName":"player-b"}
            ]
        }),
    );
    state
}

#[test]
fn null_aura_stack_is_coerced_before_apply_and_sync() {
    use lucent_protocol::event::ProjectionEvent;
    let mut state = world_with_self();
    let events = apply(
        &mut state,
        "cb",
        json!({"a":[{"cmd":"aura+","tInf":"p:7","aura":{"nam":"Focus","stk":null}}]}),
    );
    assert!(matches!(
        events.as_slice(),
        [ProjectionEvent::AuraAdded { stack: 0.0, .. }]
    ));
    let held = state.world.get_me().unwrap().aura("Focus", None).unwrap();
    assert_eq!(held.data().stack, 0.0);
    apply(
        &mut state,
        "cb",
        json!({"a":[{"cmd":"aura+","tInf":"p:7","aura":{"nam":"Focus","stk":3}}]}),
    );
    let events = apply(
        &mut state,
        "cb",
        json!({"a":[{"cmd":"aura=","tInf":"p:7","aura":{"nam":"Focus","stk":null}}]}),
    );
    assert!(matches!(
        events.as_slice(),
        [ProjectionEvent::AuraUpdated { stack: 1.0, .. }]
    ));
    assert_eq!(held.data().stack, 1.0);
}

#[test]
fn clearing_auras_resolves_the_current_id_through_cached_membership() {
    use lucent_protocol::event::ProjectionEvent;
    let mut state = world_with_self();
    apply(
        &mut state,
        "cb",
        json!({"a":[
            {"cmd":"aura+","tInf":"p:7","aura":{"nam":"Focus"}},
            {"cmd":"aura+","tInf":"p:8","aura":{"nam":"Other Focus"}}
        ]}),
    );
    let held = state.world.get_me().unwrap().aura("Focus", None).unwrap();
    apply(
        &mut state,
        "uotls",
        json!({"unm":"player-a","o":{"entID":9}}),
    );
    assert!(apply(&mut state, "clearAuras", json!({})).is_empty());
    assert!(
        state
            .world
            .get_me()
            .unwrap()
            .aura("Focus", None)
            .unwrap()
            .same_instance(&held)
    );
    apply(
        &mut state,
        "uotls",
        json!({"unm":"player-a","o":{"entID":8}}),
    );
    let events = apply(&mut state, "clearAuras", json!({}));
    assert!(
        matches!(events.as_slice(), [ProjectionEvent::AuraRemoved { aura }] if aura.target_id == 8 && aura.name == "Other Focus")
    );
    assert!(
        state
            .world
            .get_player("player-b")
            .unwrap()
            .aura("Other Focus", None)
            .is_none()
    );
    assert!(state.world.get_me().unwrap().aura("Focus", None).is_some());
}

#[test]
fn accepted_integer_values_preserve_signed_zero_in_live_numeric_fields() {
    use lucent_host::projection::{auth::decode_servers, monster_drops::decode_monster_drop};
    let mut state = world_with_self();
    let normalized =
        decode_item(&json!({"ItemID":1,"iQty":-0.9}), ItemContainer::Inventory).unwrap();
    assert_eq!(normalized.quantity.to_bits(), 0.0_f64.to_bits());
    apply(
        &mut state,
        "uotls",
        json!({"unm":"player-a","o":{"intHP":-0.9,"intLevel":-0.9}}),
    );
    let player = state.world.get_me().unwrap().snapshot();
    assert!(player["hp"].as_f64().unwrap().is_sign_negative());
    assert!(player["level"].as_f64().unwrap().is_sign_negative());
    let item = decode_item(&json!({"ItemID":1}), ItemContainer::Inventory).unwrap();
    state.items.replace(ItemContainer::Inventory, vec![item]);
    apply(
        &mut state,
        "enhanceItemShop",
        json!({"ItemIDs":[1],"EnhID":-0.9,"EnhLvl":-0.9,"EnhPID":-0.9,"ProcID":-0.9}),
    );
    let item = state
        .items
        .get(ItemContainer::Inventory, 1)
        .unwrap()
        .data()
        .snapshot();
    for key in ["id", "level", "patternId", "procId"] {
        assert!(
            item["enhancement"][key]
                .as_f64()
                .unwrap()
                .is_sign_negative()
        );
    }
    apply(&mut state, "dropItem", json!({"items":{"1":{"ItemID":1}}}));
    apply(
        &mut state,
        "getDrop",
        json!({"ItemID":1,"bSuccess":true,"iQtyNow":-0.9}),
    );
    assert!(
        state
            .items
            .get(ItemContainer::Inventory, 1)
            .unwrap()
            .data()
            .quantity
            .is_sign_negative()
    );
    let drop = serde_json::to_value(
        decode_monster_drop(&json!({"ItemID":1,"iStk":-0.9,"iRty":-0.9})).unwrap(),
    )
    .unwrap();
    assert!(
        drop["item"]["maxStack"]
            .as_f64()
            .unwrap()
            .is_sign_negative()
    );
    assert!(drop["rarity"].as_f64().unwrap().is_sign_negative());
    let servers =
        serde_json::to_value(decode_servers(&json!([{"sName":"server-a","iCount":-0.9}])).unwrap())
            .unwrap();
    assert!(servers[0]["count"].as_f64().unwrap().is_sign_negative());
}
