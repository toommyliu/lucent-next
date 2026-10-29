use lucent_host::projection::{PacketProjection, Projection};
use lucent_protocol::{Packet, PacketEncoding};
use serde_json::{Value, json};

fn packet(state: &mut Projection, command: &str, data: Value, now: f64) -> Value {
    let packet = Packet::Extension {
        command: command.into(),
        data,
        raw: String::new(),
        encoding: PacketEncoding::Json,
    };
    project(state, &packet, now)
}

fn project(state: &mut Projection, packet: &Packet, now: f64) -> Value {
    let PacketProjection::Applied(output) = state.packet(packet, now, false) else {
        panic!("expected completed projection");
    };
    serde_json::to_value(output.events).unwrap()
}

fn enter_area() -> Projection {
    let mut state = Projection::default();
    state.auth.username = "Hero".into();
    packet(
        &mut state,
        "moveToArea",
        json!({
            "areaId": 12, "areaName": "battleon-42",
            "monBranch": [{"MonID": 5, "MonMapID": 1, "intHP": 100, "intHPMax": 100, "strMonName": "Slime"}],
            "uoBranch": [{"entID": 10, "intHP": 100, "intHPMax": 100, "strUsername": "Hero"}]
        }),
        1_000_000.0,
    );
    state
}

fn self_auras(state: &Projection) -> Value {
    state.world.get_me().unwrap().snapshot()["auras"].clone()
}

fn aura_names_and_kinds(auras: &Value) -> Value {
    auras
        .as_array()
        .unwrap()
        .iter()
        .map(|aura| json!([aura["name"], aura["kind"]]))
        .collect()
}

#[test]
fn aura_decay_refreshes_remaining_time_and_publishes_the_server_stack() {
    let mut state = enter_area();
    packet(
        &mut state,
        "cb",
        json!({"a": [{"cmd": "aura+", "tInf": "p:10", "aura": {
            "nam": "Styx Water", "stk": 4, "t": "s", "dur": 8
        }}]}),
        1_000_000.0,
    );
    assert_eq!(self_auras(&state)[0]["name"], "Styx Water");
    let events = packet(
        &mut state,
        "cb",
        json!({"a": [{"cmd": "aura-", "tInf": "p:10", "aura": {
            "nam": "Styx Water", "stk": 1, "dur": 7
        }}]}),
        1_003_000.0,
    );
    let auras = self_auras(&state);
    assert_eq!(auras[0]["stack"], 1.0);
    assert_eq!(auras[0]["duration"], 8.0);
    assert_eq!(auras[0]["expiresAt"], 1_010_000.0);
    assert_eq!(
        events,
        json!([{"type": "aura-updated", "kind": "active", "name": "Styx Water",
        "stack": 1.0, "duration": 8.0, "targetId": 10, "targetType": "player"}])
    );
}

#[test]
fn clearing_self_keeps_persistent_and_passive_auras_and_leaves_monsters_alone() {
    let mut state = enter_area();
    packet(
        &mut state,
        "cb",
        json!({"a": [
            {"cmd": "aura+", "tInf": "p:10", "auras": [
                {"nam": "Potent Battle Elixir", "t": "s", "dur": 900, "persist": true, "icon": "ice"},
                {"nam": "Empowered", "stk": 4}
            ]},
            {"cmd": "aura+p", "tInf": "p:10", "aura": {"nam": "Empowered"}},
            {"cmd": "aura+", "tInf": "m:1", "aura": {"nam": "Focus"}}
        ]}),
        1_000_000.0,
    );
    assert_eq!(self_auras(&state).as_array().unwrap().len(), 3);
    let events = packet(&mut state, "clearAuras", json!({}), 1_000_000.0);
    assert_eq!(
        aura_names_and_kinds(&self_auras(&state)),
        json!([["Potent Battle Elixir", "active"], ["Empowered", "passive"]])
    );
    assert_eq!(
        state.world.get_monster(1).unwrap().snapshot()["auras"][0]["name"],
        "Focus"
    );
    assert_eq!(
        events,
        json!([{"type": "aura-removed", "kind": "active", "name": "Empowered",
        "duration": 0.0, "targetId": 10, "targetType": "player"}])
    );
    assert_eq!(
        packet(&mut state, "clearAuras", json!({}), 1_000_000.0),
        json!([])
    );
}

#[test]
fn player_snapshots_reseed_active_auras_and_preserve_passives() {
    let mut state = enter_area();
    packet(
        &mut state,
        "cb",
        json!({"a": [
            {"cmd": "aura+", "tInf": "p:10", "aura": {"nam": "Empowered", "persist": true}},
            {"cmd": "aura+p", "tInf": "p:10", "aura": {"nam": "Brand of Chaos"}}
        ]}),
        1_000_000.0,
    );
    assert_eq!(self_auras(&state).as_array().unwrap().len(), 2);
    let events = packet(
        &mut state,
        "auSnap",
        json!({"unm": "Hero", "au": [["Inspired", "imr2,iihelm", 8, 1, 0, 12]]}),
        1_000_000.0,
    );
    assert_eq!(
        aura_names_and_kinds(&self_auras(&state)),
        json!([["Brand of Chaos", "passive"], ["Inspired", "active"]])
    );
    assert_eq!(
        self_auras(&state)[1],
        json!({"name": "Inspired", "kind": "active", "icon": "imr2,iihelm", "persistent": false,
        "stack": 1.0, "duration": 12.0, "expiresAt": 1_008_000.0})
    );
    assert_eq!(
        events,
        json!([
            {"type": "aura-removed", "kind": "active", "name": "Empowered", "duration": 0.0, "targetId": 10, "targetType": "player"},
            {"type": "aura-added", "kind": "active", "name": "Inspired", "duration": 12.0, "icon": "imr2,iihelm", "stack": 1.0, "targetId": 10, "targetType": "player"}
        ])
    );
}

fn add_same_named_auras(state: &mut Projection) -> Value {
    let mut events = packet(
        state,
        "aura+p",
        json!({"cmd": "aura+p", "tInf": "p:10", "auras": [
            {"nam": "Arcane Flux", "cat": "passive", "icon": "imr1"}
        ]}),
        1_000_000.0,
    )
    .as_array()
    .unwrap()
    .clone();
    events.extend(
        packet(
            state,
            "aura+",
            json!({"cmd": "aura+", "tInf": "p:10", "aura": {
                "nam": "Arcane Flux", "icon": "imr2", "t": "s", "dur": 6
            }}),
            1_000_000.0,
        )
        .as_array()
        .unwrap()
        .clone(),
    );
    json!(events)
}

#[test]
fn same_named_aura_events_identify_active_and_passive_kinds() {
    let mut state = enter_area();
    assert_eq!(
        add_same_named_auras(&mut state),
        json!([
            {"type": "aura-added", "kind": "passive", "name": "Arcane Flux", "duration": 0.0, "icon": "imr1", "stack": 1.0, "targetId": 10, "targetType": "player"},
            {"type": "aura-added", "kind": "active", "name": "Arcane Flux", "duration": 6.0, "icon": "imr2", "stack": 1.0, "targetId": 10, "targetType": "player"}
        ])
    );
    let events = packet(
        &mut state,
        "aura-",
        json!({"cmd": "aura-", "tInf": "p:10", "aura": {"nam": "Arcane Flux"}}),
        1_000_000.0,
    );
    assert_eq!(
        events,
        json!([{"type": "aura-removed", "kind": "active", "name": "Arcane Flux", "duration": 6.0,
        "icon": "imr2", "targetId": 10, "targetType": "player"}])
    );
    assert_eq!(
        self_auras(&state),
        json!([{"name": "Arcane Flux", "kind": "passive", "category": "passive", "icon": "imr1", "duration": 0.0, "stack": 1.0, "persistent": false}])
    );
}

fn assert_active_removal_keeps_passive(command: &str) {
    let mut state = enter_area();
    add_same_named_auras(&mut state);
    assert_eq!(
        aura_names_and_kinds(&self_auras(&state)),
        json!([["Arcane Flux", "passive"], ["Arcane Flux", "active"]])
    );
    let events = packet(
        &mut state,
        command,
        json!({"cmd": command, "tInf": "p:10", "aura": {"nam": "Arcane Flux"}}),
        1_000_000.0,
    );
    assert_eq!(
        aura_names_and_kinds(&self_auras(&state)),
        json!([["Arcane Flux", "passive"]])
    );
    assert_eq!(
        events,
        json!([{"type": "aura-removed", "kind": "active", "name": "Arcane Flux", "duration": 6.0,
        "icon": "imr2", "targetId": 10, "targetType": "player"}])
    );
}

#[test]
fn aura_minus_keeps_a_same_named_passive() {
    assert_active_removal_keeps_passive("aura-");
}

#[test]
fn aura_double_minus_keeps_a_same_named_passive() {
    assert_active_removal_keeps_passive("aura--");
}

#[test]
fn area_rebuild_carries_only_self_passives_across_an_entity_id_change() {
    let mut state = Projection::default();
    state.auth.username = "Hero".into();
    packet(
        &mut state,
        "initUserData",
        json!({"uid": 10, "data": {"strUsername": "Hero", "intHP": 100, "intHPMax": 100}}),
        1_000_000.0,
    );
    packet(
        &mut state,
        "aura+p",
        json!({"cmd": "aura+p", "tInf": "p:10", "auras": [
            {"nam": "Arcane Flux", "cat": "passive", "icon": "imr1", "val": 25, "dur": 3},
            {"nam": "Arcane Power", "cat": "buff", "icon": "imr2", "val": "mana"}
        ]}),
        1_000_000.0,
    );
    packet(
        &mut state,
        "aura+",
        json!({"cmd": "aura+", "tInf": "p:10", "aura": {"nam": "Old Active", "persist": true}}),
        1_000_000.0,
    );
    assert_eq!(
        aura_names_and_kinds(&self_auras(&state)),
        json!([
            ["Arcane Flux", "passive"],
            ["Arcane Power", "passive"],
            ["Old Active", "active"]
        ])
    );
    packet(
        &mut state,
        "uotls",
        json!({"unm": "Companion", "o": {"entID": 11, "intHP": 100}}),
        1_000_000.0,
    );
    packet(
        &mut state,
        "aura+p",
        json!({"cmd": "aura+p", "tInf": "p:11", "auras": [{"nam": "Other Passive"}]}),
        1_000_000.0,
    );
    assert_eq!(
        state.world.get_player("Companion").unwrap().snapshot()["auras"][0]["name"],
        "Other Passive"
    );
    packet(
        &mut state,
        "moveToArea",
        json!({"areaId": 13, "areaName": "yulgar-1", "uoBranch": [
            {"entID": 11, "uoName": "Companion", "au": [["Inspired", "ice", 0, 1, 0]]},
            {"entID": 20, "uoName": "hErO", "au": [["New Active", "imr3", 0, 2, 0]]}
        ]}),
        1_000_000.0,
    );
    assert_eq!(state.world.get_me().unwrap().data().entity_id, 20);
    let auras = self_auras(&state);
    assert_eq!(
        auras
            .as_array()
            .unwrap()
            .iter()
            .filter(|aura| aura["kind"] == "passive")
            .cloned()
            .collect::<Vec<_>>(),
        vec![
            json!({"name": "Arcane Flux", "kind": "passive", "category": "passive", "icon": "imr1", "value": 25.0, "duration": 3.0, "stack": 1.0, "persistent": false}),
            json!({"name": "Arcane Power", "kind": "passive", "category": "buff", "icon": "imr2", "value": "mana", "duration": 0.0, "stack": 1.0, "persistent": false})
        ]
    );
    assert_eq!(
        auras
            .as_array()
            .unwrap()
            .iter()
            .filter(|aura| aura["kind"] == "active")
            .map(|aura| json!([aura["name"], aura["stack"]]))
            .collect::<Vec<_>>(),
        vec![json!(["New Active", 2.0])]
    );
    assert_eq!(
        aura_names_and_kinds(&state.world.get_player("Companion").unwrap().snapshot()["auras"]),
        json!([["Inspired", "active"]])
    );
}

#[test]
fn embedded_auras_use_explicit_stacks_and_publish_self_removal_messages() {
    let mut state = enter_area();
    packet(
        &mut state,
        "cb",
        json!({"a": [
            {"cmd": "unsupported-aura", "tInf": "p:10"},
            {"cmd": "aura+", "tInf": "p:10", "auras": [{"nam": "Empowered", "t": "s", "dur": "10", "stk": 4, "isNew": true}]}
        ]}),
        1_000_000.0,
    );
    assert_eq!(self_auras(&state)[0]["name"], "Empowered");
    packet(
        &mut state,
        "cb",
        json!({"a": [{"cmd": "aura++", "tInf": "p:10", "auras": [{"nam": "Empowered", "dur": 10}]}]}),
        1_000_000.0,
    );
    assert_eq!(self_auras(&state)[0]["stack"], 1.0);
    packet(
        &mut state,
        "cb",
        json!({"a": [{"cmd": "aura--", "tInf": "p:10", "aura": {"nam": "Empowered"}}]}),
        1_000_000.0,
    );
    assert_eq!(self_auras(&state), json!([]));
    packet(
        &mut state,
        "cb",
        json!({"a": [{"cmd": "aura+", "tInf": "p:10", "aura": {"isNew": true, "nam": "Skill Locked", "val": "Ravenous"}}]}),
        1_000_000.0,
    );
    assert_eq!(self_auras(&state)[0]["value"], "Ravenous");
    let events = packet(
        &mut state,
        "cb",
        json!({"a": [{"cmd": "aura-", "tInf": "p:10", "aura": {
            "msgOff": "@Ravenous can now be used again!", "nam": "Skill Locked", "val": "Ravenous"
        }}]}),
        1_000_000.0,
    );
    assert_eq!(
        events,
        json!([
            {"type": "aura-removed", "kind": "active", "duration": 0.0, "name": "Skill Locked", "targetId": 10, "targetType": "player"},
            {"type": "update-message", "message": "Ravenous can now be used again!", "source": "aura"}
        ])
    );
    assert_eq!(self_auras(&state), json!([]));
}

fn assert_mtls_aura_lifecycle(string_encoding: bool) {
    let mut state = enter_area();
    packet(
        &mut state,
        "cb",
        json!({"a": [
            {"cmd": "aura+", "tInf": "m:1", "aura": {"nam": "Focus", "stk": 5, "persist": true}},
            {"cmd": "aura+p", "tInf": "m:1", "aura": {"nam": "Brand of Chaos"}}
        ]}),
        1_000_000.0,
    );
    let monster = state.world.get_monster(1).unwrap();
    assert_eq!(monster.snapshot()["auras"][0]["stack"], 5.0);
    let update = |state: &mut Projection, field: &str, value: i64| {
        let data = if string_encoding {
            json!(["mtls", "", "1", format!("{field}:{value}")])
        } else {
            json!({"id": 1, "o": {field: value}})
        };
        project(
            state,
            &Packet::Extension {
                command: "mtls".into(),
                data,
                raw: String::new(),
                encoding: if string_encoding {
                    PacketEncoding::String
                } else {
                    PacketEncoding::Json
                },
            },
            1_000_000.0,
        )
    };
    let names = || {
        monster.snapshot()["auras"]
            .as_array()
            .unwrap()
            .iter()
            .map(|aura| aura["name"].clone())
            .collect::<Vec<_>>()
    };
    update(&mut state, "intHP", 0);
    assert_eq!(monster.snapshot()["auras"][0]["name"], "Focus");
    update(&mut state, "intState", 2);
    assert_eq!(monster.snapshot()["auras"][0]["name"], "Focus");
    let events = update(&mut state, "intState", 1);
    assert_eq!(names(), vec![json!("Brand of Chaos")]);
    assert_eq!(
        events
            .as_array()
            .unwrap()
            .iter()
            .filter(|event| event["type"] == "aura-removed")
            .map(|event| event["name"].clone())
            .collect::<Vec<_>>(),
        vec![json!("Focus")]
    );
    packet(
        &mut state,
        "cb",
        json!({"a": [{"cmd": "aura+", "tInf": "m:1", "aura": {"nam": "Focus", "stk": 4}}]}),
        1_000_000.0,
    );
    assert_eq!(monster.snapshot()["auras"][1]["stack"], 4.0);
    update(&mut state, "intState", 1);
    assert_eq!(names(), vec![json!("Brand of Chaos")]);
    update(&mut state, "intState", 0);
    packet(
        &mut state,
        "cb",
        json!({"a": [{"cmd": "aura+", "tInf": "m:1", "aura": {"nam": "Focus", "stk": 5}}]}),
        1_000_000.0,
    );
    assert_eq!(names(), vec![json!("Brand of Chaos")]);
}

#[test]
fn json_monster_updates_apply_the_server_aura_lifecycle() {
    assert_mtls_aura_lifecycle(false);
}

#[test]
fn string_monster_updates_apply_the_server_aura_lifecycle() {
    assert_mtls_aura_lifecycle(true);
}

#[test]
fn invalid_snapshots_leave_known_auras_untouched_and_empty_snapshots_clear_them() {
    let mut state = enter_area();
    packet(
        &mut state,
        "cb",
        json!({"a": [{"cmd": "aura+", "tInf": "p:10", "aura": {"nam": "Focus", "stk": 5, "val": 7, "cat": "buff"}}]}),
        1_000_000.0,
    );
    let aura = state.world.get_me().unwrap().aura("Focus", None).unwrap();
    assert_eq!(aura.data().stack, 5.0);
    for data in [
        json!({"unm": "Unknown", "au": [["Inspired", "imr2,iihelm", 8, 1, 0, 12]]}),
        json!({"unm": "Hero"}),
        json!({"unm": "Hero", "au": null}),
        json!({"unm": "Hero", "au": {}}),
    ] {
        assert_eq!(packet(&mut state, "auSnap", data, 1_000_000.0), json!([]));
        assert!(aura.same_instance(&state.world.get_me().unwrap().aura("Focus", None).unwrap()));
        assert_eq!(aura.data().stack, 5.0);
    }
    assert!(state.world.get_player("Unknown").is_none());
    packet(
        &mut state,
        "auSnap",
        json!({"unm": "Hero", "au": [["Focus", "inver2", 6.4, 4, 0, 8]]}),
        1_000_000.0,
    );
    assert!(aura.same_instance(&state.world.get_me().unwrap().aura("Focus", None).unwrap()));
    assert_eq!(aura.data().stack, 4.0);
    assert_eq!(
        aura.data().value,
        Some(lucent_protocol::aura::AuraValue::Number(7.0))
    );
    assert_eq!(aura.data().category.as_deref(), Some("buff"));
    let events = packet(
        &mut state,
        "auSnap",
        json!({"unm": "Hero", "au": []}),
        1_000_000.0,
    );
    assert_eq!(self_auras(&state), json!([]));
    assert_eq!(
        events
            .as_array()
            .unwrap()
            .iter()
            .map(|event| json!([event["type"], event["name"]]))
            .collect::<Vec<_>>(),
        vec![json!(["aura-removed", "Focus"])]
    );
}

fn assert_player_counter_removal_has_no_counter_notice(command: &str) {
    let mut state = enter_area();
    let events = packet(
        &mut state,
        "aura+",
        json!({"cmd": "aura+", "tInf": "p:10", "aura": {"nam": "Counter Attack", "t": "s", "dur": 6}}),
        1_000_000.0,
    );
    assert_eq!(
        events,
        json!([{"type": "aura-added", "kind": "active", "duration": 6.0, "name": "Counter Attack", "stack": 1.0, "targetId": 10, "targetType": "player"}])
    );
    let data = if command == "auSnap" {
        json!({"unm": "Hero", "au": []})
    } else {
        json!({})
    };
    assert_eq!(
        packet(&mut state, command, data, 1_000_000.0),
        json!([{"type": "aura-removed", "kind": "active", "duration": 6.0, "name": "Counter Attack", "targetId": 10, "targetType": "player"}])
    );
}

#[test]
fn player_counter_snapshot_removal_emits_no_counter_notice() {
    assert_player_counter_removal_has_no_counter_notice("auSnap");
}

#[test]
fn clearing_player_counter_auras_emits_no_counter_notice() {
    assert_player_counter_removal_has_no_counter_notice("clearAuras");
}
