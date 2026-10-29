use std::time::Duration;

use lucent_host::testing::{self, ProbeMovie};
use lucent_runtime::Runtime;
use serde_json::{Value, json};

const TIMEOUT: Duration = Duration::from_secs(10);

fn area() -> String {
    json!({
        "type": "json",
        "dataObj": {
            "cmd": "moveToArea",
            "areaId": 5,
            "areaName": "testroom-1",
            "uoBranch": [
                {"entID": 8, "uoName": "player-b", "strFrame": "Enter", "intHP": 100, "intHPMax": 100},
                {"entID": 7, "uoName": "player-a", "strFrame": "Enter", "intHP": 100, "intHPMax": 100}
            ],
            "mondef": [
                {"MonID": 1, "strMonName": "Frogzard", "sRace": "Amphibian", "iLvl": 3}
            ],
            "monBranch": [
                {"MonID": 1, "MonMapID": 2, "intHP": 50, "intHPMax": 50}
            ],
            "monmap": [
                {"MonMapID": 2, "strFrame": "r2"}
            ]
        }
    })
    .to_string()
}

#[test]
fn the_api_layer_reads_and_changes_host_state_through_its_services() {
    let host = testing::headless_host();
    let (_session, link) = testing::start_probe(
        &host,
        ProbeMovie::new()
            .raising("flash.sendClientPacket", "onExtensionResponse", area())
            .constant("auth.isLoggedIn", json!(true))
            .constant("bank.loadItems", Value::Null)
            .constant("bank.isLoaded", json!(true))
            .constant(
                "bank.getItems",
                json!([
                    {"ItemID": 1, "sName": "Sword", "sType": "Sword"},
                    {"ItemID": 2, "sName": "Rogue", "sType": "Class"},
                    {"ItemID": 3, "sName": "Shield", "sType": "Shield"}
                ]),
            )
            .constant(
                "world.getMonster",
                json!({"MonMapID": 9, "MonID": 4, "strMonName": "Ghost", "intHP": 10, "intHPMax": 10}),
            )
            .constant(
                "auth.getServers",
                json!([
                    {"sName": "Artix", "iCount": 10, "iMax": 10, "bOnline": 1},
                    {"sName": "Yorumi", "iCount": 3, "iMax": 10, "bOnline": 1}
                ]),
            ),
    );
    let runtime = Runtime::start(link).unwrap();

    let observed: Value = runtime
        .eval(
            "api.js",
            r#"
            (async () => {
                const run = __lucent.run;
                const joined = await run((api) =>
                    api.wait.forEvent(
                        { type: "join-map" },
                        { trigger: api.packet.sendToClient("join"), timeout: "5 seconds" },
                    ),
                );
                const me = await run((api) => api.player.get());
                const players = await run((api) => api.players.getAll());
                const frogzard = await run((api) => api.monsters.get("frogzard"));
                const ghost = await run((api) => api.monsters.get("id.9"));
                const loaded = await run((api) => api.bank.load());
                const bank = await run((api) => api.bank.getAll());
                const servers = await run((api) => api.auth.getServers());
                return {
                    joined,
                    map: await run((api) => api.map.getName()),
                    me: me === null ? null : [me.entityId, me.username, me.alive],
                    players: players.map((player) => player.username),
                    frogzard: frogzard === null ? null : [frogzard.monsterMapId, frogzard.cell, frogzard.hpPercent],
                    ghost: ghost === null ? null : [ghost.monsterMapId, ghost.name],
                    loggedIn: await run((api) => api.auth.isLoggedIn()),
                    loaded,
                    bank: bank.map((item) => [item.itemId, item.name, item.banked]),
                    servers: servers.map((server) => [server.name, server.full]),
                };
            })()
            "#,
        )
        .wait(TIMEOUT)
        .unwrap();

    assert_eq!(
        observed,
        json!({
            "joined": {"type": "join-map", "map": {"id": 5, "name": "testroom", "roomNumber": 1}},
            "map": "testroom",
            "me": [7, "player-a", true],
            "players": ["player-b", "player-a"],
            "frogzard": [2, "r2", 100],
            "ghost": [9, "Ghost"],
            "loggedIn": true,
            "loaded": true,
            "bank": [[1, "Sword", true], [3, "Shield", true]],
            "servers": [["Artix", true], ["Yorumi", false]],
        })
    );
}
