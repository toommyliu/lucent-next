use lucent_host::projection::combat_state::CombatHydration;
use lucent_host::projection::{PacketProjection, Projection};
use lucent_protocol::{Packet, PacketEncoding, query::MonsterSelector};
use serde_json::{Value, json};

fn packet(state: &mut Projection, command: &str, data: Value, now: f64) {
    let input = Packet::Extension {
        command: command.into(),
        data,
        raw: String::new(),
        encoding: PacketEncoding::Json,
    };
    assert!(matches!(
        state.packet(&input, now, false),
        PacketProjection::Applied(_)
    ));
}

fn area(state: &mut Projection) {
    state.auth.username = "Hero".into();
    packet(
        state,
        "moveToArea",
        json!({
            "areaId":12,"areaName":"battleon-42",
            "uoBranch":[{"entID":10,"strUsername":"Hero","intHP":100,"strFrame":"Enter"}],
            "monBranch":[
                {"MonMapID":1,"MonID":5,"strMonName":" Slime ","intHP":100,"intHPMax":100,"intState":1,"strFrame":"Enter"},
                {"MonMapID":2,"MonID":6,"strMonName":"id.1","intHP":100,"intHPMax":100,"intState":1,"strFrame":"Enter"},
                {"MonMapID":3,"MonID":7,"strMonName":"Slime","intHP":100,"intHPMax":100,"intState":1,"strFrame":"Boss"}
            ]
        }),
        1000.0,
    );
}

fn snapshot(state: &Projection, now: f64) -> Value {
    serde_json::to_value(state.combat_snapshot(
        now,
        &[
            MonsterSelector::Name("ID:2".into()),
            MonsterSelector::Named {
                name: "id.1".into(),
            },
            MonsterSelector::Name("\u{feff}SLIME\u{a0}".into()),
        ],
        true,
    ))
    .unwrap()
}

fn input(actions: Value, target: Value, haste: f64, gcd: f64) -> CombatHydration {
    CombatHydration {
        actions,
        haste: json!(haste),
        global_cooldown: json!(gcd),
        global_used_at: json!(1000),
        potions_locked: json!(false),
        target,
        available_monster_ids: json!([1, 2]),
    }
}

fn hydrate(state: &mut Projection, actions: Value, target: Value, haste: f64, gcd: f64) {
    let scope = state.combat_scope();
    state
        .hydrate_combat(scope, input(actions, target, haste, gcd), 1000.0)
        .unwrap();
}

#[test]
fn cooldowns_use_old_cooldown_haste_and_the_global_deadline() {
    let mut state = Projection::default();
    area(&mut state);
    hydrate(
        &mut state,
        json!([
            {"cd":2000,"OldCD":1000,"ts":1000},
            {"cd":1000,"ts":800}, null, null, null,
            {"cd":1000,"ts":1000,"ref":"i1","sArg1":"42","isOK":true,"skillLock":false,"lock":false}
        ]),
        json!({"type":"monster","monsterMapId":1}),
        0.5,
        750.0,
    );
    let first = snapshot(&state, 1000.0);
    assert_eq!(
        first["skills"][0],
        json!({"status":"cooldown","remainingMs":750})
    );
    assert_eq!(first["consumable"], json!({"itemId":42,"ready":false}));
    assert_eq!(snapshot(&state, 1749.0)["skills"][0]["remainingMs"], 1);
    assert_eq!(
        snapshot(&state, 1750.0)["skills"][0],
        json!({"status":"ready"})
    );
    assert_eq!(
        snapshot(&state, 1750.0)["consumable"],
        json!({"itemId":42,"ready":true})
    );
    assert_eq!(first["skills"][2], json!({"status":"ready"}));
    hydrate(
        &mut state,
        json!([{"cd":1000,"ts":1000}]),
        Value::Null,
        -2.0,
        0.0,
    );
    assert_eq!(snapshot(&state, 1000.0)["skills"][0]["remainingMs"], 2000);
}

#[test]
fn generations_change_on_revival_and_selected_targets_keep_the_observed_generation() {
    let mut state = Projection::default();
    area(&mut state);
    hydrate(
        &mut state,
        json!([]),
        json!({"type":"monster","monsterMapId":1}),
        0.0,
        0.0,
    );
    let first = snapshot(&state, 1000.0);
    let generation = first["monsters"][0]["generation"].as_u64().unwrap();
    packet(
        &mut state,
        "cb",
        json!({"m":{"1":{"intHP":0,"intState":0}}}),
        1010.0,
    );
    assert_eq!(
        snapshot(&state, 1010.0)["monsters"][0]["generation"],
        generation
    );
    packet(
        &mut state,
        "respawnMon",
        json!(["xt", "respawnMon", "1"]),
        1020.0,
    );
    let next = snapshot(&state, 1020.0);
    assert_eq!(next["monsters"][0]["generation"], generation + 1);
    assert_eq!(next["selectedTarget"]["generation"], generation);
    packet(
        &mut state,
        "respawnMon",
        json!(["xt", "respawnMon", "1"]),
        1030.0,
    );
    assert_eq!(
        snapshot(&state, 1030.0)["monsters"][0]["generation"],
        generation + 1
    );
    assert_eq!(next["monsters"][0]["priorityRank"], 2);
    assert_eq!(next["monsters"][1]["priorityRank"], 0);
    assert_eq!(next["monsters"][2]["priorityRank"], 2);
    assert_eq!(next["monsters"][2]["available"], false);
}

#[test]
fn counter_protection_outlives_cast_time_and_ends_on_lifecycle_signals() {
    let mut state = Projection::default();
    area(&mut state);
    packet(
        &mut state,
        "cb",
        json!({"a":[{"cmd":"aura+","tInf":"m:1","aura":{"nam":"Counter Attack","t":"s","dur":2}}]}),
        1000.0,
    );
    assert_eq!(
        snapshot(&state, 9999.0)["monsters"][0]["attackBlocked"],
        true
    );
    assert_eq!(
        snapshot(&state, 10000.0)["monsters"][0]["attackBlocked"],
        false
    );
    packet(
        &mut state,
        "cb",
        json!({"a":[{"cmd":"aura-","tInf":"m:1","aura":{"nam":"Counter Attack"}}]}),
        1200.0,
    );
    assert_eq!(
        snapshot(&state, 1200.0)["monsters"][0]["attackBlocked"],
        false
    );
}

#[test]
fn connection_change_clears_every_local_observation_and_rejects_old_hydration() {
    for status in ["OnConnection", "OnConnectionLost", "OnConnectionFailed"] {
        let mut state = Projection::default();
        area(&mut state);
        hydrate(
            &mut state,
            json!([]),
            json!({"type":"player","entityId":10,"hp":100,"state":1}),
            0.0,
            0.0,
        );
        let scope = state.combat_scope();
        assert_eq!(
            snapshot(&state, 1000.0)["selectedTarget"],
            json!({"type":"player","entityId":10,"alive":true})
        );
        state.connection(status);
        let empty = snapshot(&state, 2000.0);
        assert_eq!(empty["selectedTarget"], json!({"type":"unavailable"}));
        assert_eq!(
            empty["skills"],
            json!(vec![json!({"status":"unavailable"}); 6])
        );
        assert_eq!(empty["consumable"], Value::Null);
        assert_eq!(empty["monsters"], json!([]));
        assert_eq!(empty["observedAtMs"], Value::Null);
        assert_ne!(state.combat_scope(), scope);
        assert!(
            state
                .hydrate_combat(scope, input(json!([]), Value::Null, 0.0, 0.0), 2000.0)
                .is_err()
        );
    }
}

#[test]
fn counter_windows_extend_without_shortening_and_obey_death_and_setting_changes() {
    let mut state = Projection::default();
    area(&mut state);
    let warning = json!({"anims":[{"cInf":"m:1","msg":"prepares a counter attack!"}]});
    packet(&mut state, "cb", warning.clone(), 1000.0);
    assert_eq!(
        snapshot(&state, 7999.0)["monsters"][0]["attackBlocked"],
        true
    );
    packet(&mut state, "cb", warning, 7000.0);
    assert_eq!(
        snapshot(&state, 13999.0)["monsters"][0]["attackBlocked"],
        true
    );
    assert_eq!(
        snapshot(&state, 14000.0)["monsters"][0]["attackBlocked"],
        false
    );
    assert!(!state.combat_snapshot(7000.0, &[], false).monsters[0].attack_blocked);
    packet(
        &mut state,
        "cb",
        json!({"m":{"1":{"intHP":0,"intState":0}}}),
        7010.0,
    );
    assert_eq!(
        snapshot(&state, 7010.0)["monsters"][0]["attackBlocked"],
        false
    );
    packet(
        &mut state,
        "mtls",
        json!({"id":1,"o":{"intHP":100,"intState":1}}),
        7020.0,
    );
    assert_eq!(snapshot(&state, 7020.0)["monsters"][0]["generation"], 1);
}

#[test]
fn changing_areas_discards_local_facts_and_rejects_inflight_hydration_even_for_the_same_map() {
    let mut state = Projection::default();
    area(&mut state);
    hydrate(&mut state, json!([]), Value::Null, 0.0, 0.0);
    let scope = state.combat_scope();
    packet(
        &mut state,
        "cb",
        json!({"a":[{"cmd":"aura+","tInf":"m:1","aura":{"nam":"Counter Attack"}}]}),
        1000.0,
    );
    area(&mut state);
    assert_ne!(scope, state.combat_scope());
    let next = snapshot(&state, 1100.0);
    assert_eq!(next["selectedTarget"], json!({"type":"unavailable"}));
    assert_eq!(next["observedAtMs"], Value::Null);
    assert_eq!(next["monsters"][0]["available"], Value::Null);
    assert_eq!(next["monsters"][0]["attackBlocked"], false);
    assert!(
        state
            .hydrate_combat(scope, input(json!([]), Value::Null, 0.0, 0.0), 1100.0)
            .is_err()
    );
}

#[test]
fn skill_rounding_and_consumable_locks_follow_the_client_formula() {
    let mut state = Projection::default();
    area(&mut state);
    let slot = json!({"cd":1001,"OldCD":null,"ts":1000,"ref":"i1","sArg1":42,"isOK":true});
    for (field, value) in [
        ("isOK", json!(false)),
        ("skillLock", json!(true)),
        ("lock", json!(true)),
        ("ref", json!("a1")),
        ("sArg1", Value::Null),
    ] {
        let mut action = slot.clone();
        action[field] = value;
        hydrate(
            &mut state,
            json!([null, null, null, null, null, action]),
            Value::Null,
            0.9,
            0.0,
        );
        let current = snapshot(&state, 2000.0);
        assert!(
            current["consumable"].is_null() || current["consumable"]["ready"] == false,
            "{field}"
        );
    }
    hydrate(
        &mut state,
        json!([null, null, null, null, null, slot.clone()]),
        Value::Null,
        0.9,
        0.0,
    );
    assert_eq!(snapshot(&state, 1000.0)["skills"][5]["remainingMs"], 501);
    assert_eq!(
        snapshot(&state, 1500.5)["skills"][5],
        json!({"status":"ready"})
    );
    assert_eq!(snapshot(&state, 1500.5)["consumable"]["ready"], false);
    assert_eq!(snapshot(&state, 1501.0)["consumable"]["ready"], true);
    let mut locked = input(
        json!([null, null, null, null, null, slot]),
        Value::Null,
        0.0,
        0.0,
    );
    locked.potions_locked = json!(true);
    state
        .hydrate_combat(state.combat_scope(), locked, 1000.0)
        .unwrap();
    assert_eq!(snapshot(&state, 5000.0)["consumable"]["ready"], false);
}

#[test]
fn invalid_hydration_discards_previous_readiness_instead_of_publishing_partial_facts() {
    let mut state = Projection::default();
    area(&mut state);
    for invalid in [
        json!({}),
        json!([{"cd":"bad","ts":1000}]),
        json!([{"cd":1e30,"ts":1000}]),
        json!([{"cd":1}]),
    ] {
        hydrate(&mut state, json!([]), Value::Null, 0.0, 0.0);
        assert!(
            state
                .hydrate_combat(
                    state.combat_scope(),
                    input(invalid, Value::Null, 0.0, 0.0),
                    1000.0
                )
                .is_err()
        );
        assert_eq!(
            snapshot(&state, 2000.0)["skills"][0],
            json!({"status":"unavailable"})
        );
        assert_eq!(snapshot(&state, 2000.0)["observedAtMs"], Value::Null);
    }
}

fn probe_query(link: &lucent_host::ScriptLink) -> Value {
    serde_json::from_str(
        &link
            .query(lucent_protocol::query::Query::CombatState { priorities: vec![] })
            .unwrap(),
    )
    .unwrap()
}

fn probe_emit(link: &lucent_host::ScriptLink, event: &str, data: Value) {
    use lucent_host::bridge::Value as LoaderValue;
    assert_eq!(
        link.call_loader(
            "probe.emit",
            vec![
                LoaderValue::String(event.into()),
                LoaderValue::String(data.to_string())
            ]
        )
        .recv_timeout(std::time::Duration::from_secs(5)),
        Ok(LoaderValue::Bool(true))
    );
}

fn combat_probe(events: Vec<(String, String)>) -> lucent_host::testing::ProbeMovie {
    use lucent_host::testing::ProbeMovie;
    ProbeMovie::new().responding("flash.getGameObject", [
        (json!(json!([null,null,null,null,null,{"cd":0,"ts":0,"ref":"i1","sArg1":42,"isOK":true}]).to_string()), vec![]), (json!("0"), vec![]),
        (json!("0"), events), (json!("0"), vec![]), (json!("false"), vec![]),
    ])
    .responding("combat.getTarget", [(json!({"type":"monster","monsterMapId":1}), vec![])])
    .constant("player.getUserId", json!(10))
    .responding("world.getAvailableMonsterMapIds", [(json!([1]), vec![])])
}

#[test]
fn typed_combat_refresh_commits_before_reply_and_reads_need_no_loader_callback() {
    use lucent_host::testing;
    use lucent_protocol::command::Command;
    let host = testing::headless_host();
    let (_session, link) = testing::start_probe(&host, combat_probe(vec![]));
    probe_emit(
        &link,
        "onExtensionResponse",
        json!({"type":"json","dataObj":{
            "cmd":"moveToArea","areaName":"battleon-42",
            "uoBranch":[{"entID":10,"uoName":"Hero","intHP":100,"strFrame":"Enter"}],
            "monBranch":[{"MonID":5,"MonMapID":1,"intHP":100,"intState":1,"strFrame":"Enter"}]
        }}),
    );
    let reply = link
        .command(Command::RefreshCombatState {})
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    let observed: Value = serde_json::from_str(&reply).unwrap();
    assert_eq!(
        observed["selectedTarget"],
        json!({"type":"monster","monsterMapId":1,"generation":0})
    );
    assert_eq!(observed["consumable"], json!({"itemId":42,"ready":true}));
    assert_eq!(observed["monsters"][0]["available"], true);
    assert_eq!(
        observed["skills"],
        json!(vec![json!({"status":"ready"}); 6])
    );
    assert!(observed["observedAtMs"].as_f64().unwrap() > 0.0);
    let mut calls = Vec::new();
    while let Some(event) = link.try_event().unwrap() {
        if let lucent_host::bridge::ScriptEvent::Loader { event, .. } = event
            && event.name == "onProbeCall"
        {
            calls.push(
                lucent_protocol::bridge::to_json(lucent_host::bridge::Value::List(event.args))
                    .unwrap(),
            );
        }
    }
    assert_eq!(
        calls,
        vec![
            json!(["flash.getGameObject", ["world.actions.active"]]),
            json!(["flash.getGameObject", ["world.myAvatar.dataLeaf.sta.$tha"]]),
            json!(["flash.getGameObject", ["world.GCD"]]),
            json!(["flash.getGameObject", ["world.GCDTS"]]),
            json!(["flash.getGameObject", ["world.lockdownPots"]]),
            json!(["combat.getTarget", []]),
            json!(["world.getAvailableMonsterMapIds", []]),
        ]
    );
    for _ in 0..3 {
        assert_eq!(probe_query(&link), observed);
        assert_eq!(link.try_event().unwrap(), None);
    }
}

#[test]
fn typed_combat_refresh_rejects_resets_during_its_loader_calls() {
    use lucent_host::testing;
    use lucent_protocol::command::Command;
    for event in [
        ("onConnection".into(), "OnConnectionLost".into()),
        (
            "onExtensionResponse".into(),
            json!({"type":"json","dataObj":{"cmd":"cb","p":{}}}).to_string(),
        ),
        (
            "onExtensionResponse".into(),
            json!({"type":"json","dataObj":{"cmd":"moveToArea","areaName":"battleon-42"}})
                .to_string(),
        ),
    ] {
        let host = testing::headless_host();
        let (_session, link) = testing::start_probe(&host, combat_probe(vec![event]));
        let reply = link
            .command(Command::RefreshCombatState {})
            .recv_timeout(std::time::Duration::from_secs(5));
        assert!(
            reply.is_err(),
            "a packet during hydration must reject the observation"
        );
        assert_eq!(probe_query(&link)["observedAtMs"], Value::Null);
        assert_eq!(
            probe_query(&link)["skills"][0],
            json!({"status":"unavailable"})
        );
    }
}

#[test]
fn combat_hydration_matches_the_cooldown_reference_corpus() {
    let fixtures: Vec<Value> =
        serde_json::from_str(include_str!("fixtures/projection/combat-state.json")).unwrap();
    for fixture in fixtures {
        let mut state = Projection::default();
        area(&mut state);
        state
            .hydrate_combat(
                state.combat_scope(),
                serde_json::from_value(fixture["hydration"].clone()).unwrap(),
                1000.0,
            )
            .unwrap();
        let current = snapshot(&state, fixture["nowMs"].as_f64().unwrap());
        assert_eq!(current["skills"], fixture["skills"], "{}", fixture["name"]);
        assert_eq!(
            current["consumable"], fixture["consumable"],
            "{}",
            fixture["name"]
        );
        assert_eq!(current["selectedTarget"], json!({"type":"none"}));
    }
}

#[test]
fn moving_cells_invalidates_display_availability_until_it_is_observed_again() {
    let mut state = Projection::default();
    area(&mut state);
    hydrate(&mut state, json!([]), Value::Null, 0.0, 0.0);
    assert_eq!(snapshot(&state, 1000.0)["monsters"][0]["available"], true);
    packet(
        &mut state,
        "uotls",
        json!({"unm":"Hero","o":{"strFrame":"Boss"}}),
        1100.0,
    );
    assert_eq!(
        snapshot(&state, 1100.0)["monsters"][0]["available"],
        Value::Null
    );
    let mut next = input(json!([]), Value::Null, 0.0, 0.0);
    next.available_monster_ids = json!([3]);
    state
        .hydrate_combat(state.combat_scope(), next, 1100.0)
        .unwrap();
    assert_eq!(snapshot(&state, 1100.0)["monsters"][0]["available"], false);
    assert_eq!(snapshot(&state, 1100.0)["monsters"][2]["available"], true);
}
