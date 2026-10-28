use lucent_protocol::aura::PassiveMode;
use lucent_protocol::event::{CounterSource, EntityKind, MessageSource, ProjectionEvent};
use lucent_protocol::world::EntityPatch;
use lucent_protocol::{Packet, PacketDirection};
use serde_json::{Map, Value};

use super::ProjectionOutput;
use super::auras::{
    aura_events, match_anti_counter_message, message_text, parse_entity_references,
    project_aura_events,
};
use super::wire::wire_number;
use super::world::{WorldStore, decode_entity_patch};

fn entries(object: &Map<String, Value>) -> Vec<(&str, &Value)> {
    let mut entries: Vec<_> = object
        .iter()
        .map(|(key, value)| (key.as_str(), value))
        .collect();
    entries.sort_by_key(|(key, _)| {
        let index = key
            .parse::<u32>()
            .ok()
            .filter(|index| *index != u32::MAX && index.to_string() == *key);
        (index.is_none(), index.unwrap_or(0))
    });
    entries
}

fn optional_text<'a>(object: &'a Map<String, Value>, key: &str) -> Option<Option<&'a str>> {
    match object.get(key) {
        None => Some(None),
        Some(value) => value.as_str().map(Some),
    }
}

fn entity_patch(patch: EntityPatch) -> EntityPatch {
    EntityPatch {
        hp: patch.hp,
        max_hp: patch.max_hp,
        mp: patch.mp,
        max_mp: patch.max_mp,
        cell: patch.cell,
        state: patch.state,
        ..EntityPatch::default()
    }
}

fn counter_duration(value: Option<&Value>, monster_map_id: i64) -> Option<f64> {
    let mut duration: Option<f64> = None;
    for entry in value.and_then(Value::as_array).into_iter().flatten() {
        let Some(result) = entry
            .as_object()
            .and_then(|entry| entry.get("actionResult"))
            .and_then(Value::as_object)
        else {
            continue;
        };
        let Some(source) = optional_text(result, "cInf") else {
            continue;
        };
        let cast_time = match result.get("ct") {
            None => None,
            Some(value) => match wire_number(value) {
                Some(value) => Some(value),
                None => continue,
            },
        };
        let Some(cast_time) = cast_time.filter(|duration| *duration > 0.0) else {
            continue;
        };
        if parse_entity_references(source.unwrap_or_default())
            .iter()
            .any(|entity| entity.kind == EntityKind::Monster && entity.id == monster_map_id)
        {
            duration = Some(duration.unwrap_or(0.0).max(cast_time));
        }
    }
    duration
}

fn animation_events(
    world: &WorldStore,
    value: &Value,
    sara: Option<&Value>,
    output: &mut ProjectionOutput,
) {
    let Some(animation) = value.as_object() else {
        return;
    };
    let Some(animation_name) = optional_text(animation, "animStr") else {
        return;
    };
    let Some(source) = optional_text(animation, "cInf") else {
        return;
    };
    let Some(target) = optional_text(animation, "tInf") else {
        return;
    };
    let Some(raw_message) = animation.get("msg").filter(|value| {
        value.is_string()
            || value
                .as_array()
                .is_some_and(|values| values.iter().all(Value::is_string))
    }) else {
        return;
    };
    let Some(message) = message_text(Some(raw_message)) else {
        return;
    };
    let monster_map_id = parse_entity_references(source.unwrap_or_default())
        .into_iter()
        .chain(parse_entity_references(target.unwrap_or_default()))
        .find(|entity| entity.kind == EntityKind::Monster)
        .map(|entity| entity.id);
    let message = monster_map_id
        .and_then(|id| world.get_monster(id))
        .map_or_else(
            || message.clone(),
            |monster| message.replace("<mon>", &monster.data().name),
        );
    if let Some(monster_map_id) = monster_map_id.filter(|_| match_anti_counter_message(&message)) {
        output.events.push(ProjectionEvent::CounterAttackStart {
            duration_ms: counter_duration(sara, monster_map_id),
            monster_map_id,
            source: CounterSource::Message,
            trigger_id: "anti-counter".into(),
            trigger_text: message.clone(),
        });
    }
    output.events.push(ProjectionEvent::UpdateMessage {
        animation: animation_name.map(str::to_owned),
        message,
        monster_map_id,
        source: MessageSource::Animation,
    });
}

pub fn project_combat(
    world: &mut WorldStore,
    packet: &Packet,
    now_ms: f64,
) -> Option<ProjectionOutput> {
    if !matches!(
        (packet.direction(), packet.command()),
        (PacketDirection::Server, "ct") | (PacketDirection::Extension, "cb")
    ) {
        return None;
    }
    let mut output = ProjectionOutput::default();
    let Some(data) = packet.data().and_then(Value::as_object).filter(|data| {
        ["a", "anims"].iter().all(|key| {
            data.get(*key)
                .is_none_or(|value| value.is_null() || value.is_array())
        }) && ["m", "p"].iter().all(|key| {
            data.get(*key)
                .is_none_or(|value| value.is_null() || value.is_object())
        })
    }) else {
        tracing::warn!(command = packet.command(), "invalid projection input");
        return Some(output);
    };
    if let Some(players) = data.get("p").and_then(Value::as_object) {
        for (username, value) in entries(players) {
            let Some(mut patch) = decode_entity_patch(value) else {
                tracing::warn!(
                    command = packet.command(),
                    entity_kind = "player",
                    "invalid projection input"
                );
                continue;
            };
            if world.get_player(username).is_none() {
                tracing::warn!(
                    command = packet.command(),
                    entity_kind = "player",
                    "projection input references an unknown entity"
                );
                continue;
            }
            patch.entity_id = None;
            patch.px = None;
            patch.py = None;
            if let Some(result) = world
                .patch_player(username, &patch)
                .filter(|result| result.became_dead)
            {
                let player = result.player.data();
                output.events.push(ProjectionEvent::PlayerDeath {
                    entity_id: player.entity_id,
                    username: player.username,
                });
            }
        }
    }
    if let Some(monsters) = data.get("m").and_then(Value::as_object) {
        for (raw_id, value) in entries(monsters) {
            let Some(id) = wire_number(&Value::String(raw_id.into()))
                .filter(|id| id.fract() == 0.0 && *id > 0.0)
            else {
                continue;
            };
            let Some(patch) = decode_entity_patch(value) else {
                tracing::warn!(
                    command = packet.command(),
                    entity_kind = "monster",
                    "invalid projection input"
                );
                continue;
            };
            let id = id as i64;
            let Some(result) = world.write_monster(id, &entity_patch(patch)) else {
                tracing::warn!(
                    command = packet.command(),
                    entity_kind = "monster",
                    entity_id = id,
                    "projection input references an unknown entity"
                );
                continue;
            };
            if result.became_dead {
                output
                    .events
                    .push(ProjectionEvent::MonsterDeath { monster_map_id: id });
            }
            output.events.extend(aura_events(
                EntityKind::Monster,
                id,
                &result.aura_changes,
                None,
                None,
            ));
        }
    }
    for animation in data
        .get("anims")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        animation_events(world, animation, data.get("sara"), &mut output);
    }
    if let Some(values) = data
        .get("a")
        .and_then(Value::as_array)
        .filter(|values| !values.is_empty())
    {
        let aura_output = project_aura_events(world, values, PassiveMode::Merge, now_ms);
        output.events.extend(aura_output.events);
    }
    Some(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projection::{PacketProjection, Projection};
    use lucent_protocol::PacketEncoding;
    use lucent_protocol::aura::{AuraMutation, AuraSeed, PassiveAuraInput, SeedTimer};
    use lucent_protocol::event::{AuraKind, Point};
    use serde_json::json;

    fn packet(direction: PacketDirection, command: &str, data: Value) -> Packet {
        match direction {
            PacketDirection::Server => Packet::Server {
                command: command.into(),
                data,
                raw: String::new(),
                encoding: PacketEncoding::Json,
            },
            PacketDirection::Extension => Packet::Extension {
                command: command.into(),
                data,
                raw: String::new(),
                encoding: PacketEncoding::Json,
            },
            PacketDirection::Client => unreachable!(),
        }
    }

    fn world() -> WorldStore {
        let mut projection = Projection::default();
        projection.auth.username = "player-a".into();
        let baseline = packet(
            PacketDirection::Extension,
            "moveToArea",
            json!({
                "areaId":1,"areaName":"training-2",
                "uoBranch":[{"entID":1,"strUsername":"player-a","intHP":100,"intHPMax":100,"intMP":20,"intMPMax":20,"intState":2,"tx":1,"ty":2}],
                "monBranch":[{"MonMapID":1,"MonID":10,"strMonName":"Sentinel","intHP":50,"intHPMax":50,"intState":2},{"MonMapID":3,"MonID":11,"strMonName":"Guardian","intHP":40,"intHPMax":40,"intState":2}]
            }),
        );
        assert!(matches!(
            projection.packet(&baseline, 1000.0, false),
            PacketProjection::Applied(_)
        ));
        projection.world
    }

    fn events(output: &ProjectionOutput) -> Value {
        serde_json::to_value(&output.events).unwrap()
    }

    #[test]
    fn combat_updates_only_the_fields_the_combat_contract_owns() {
        let mut world = world();
        let player = world.get_player("player-a").unwrap();
        let output = project_combat(&mut world, &packet(PacketDirection::Server,"ct",json!({
            "p":{"player-a":{"intHP":0,"intState":0,"afk":true,"intLevel":12,"entID":99,"px":90,"py":80,"tx":7}},
            "m":{"1":{"intHP":25,"intLevel":99,"entID":88,"strPad":"ignored","tx":90}}
        })),1000.0).unwrap();
        assert_eq!(
            events(&output),
            json!([{"type":"player-death","entityId":1,"username":"player-a"}])
        );
        assert_eq!(player.data().entity_id, 1);
        assert_eq!(player.data().position, Point { x: 7.0, y: 2.0 });
        assert!(player.data().afk);
        assert_eq!(player.data().level, 12.0);
        assert_eq!(world.get_monster(1).unwrap().data().level, 0.0);
        assert!(player.same_instance(&world.get_player("player-a").unwrap()));
        let repeated = project_combat(
            &mut world,
            &packet(
                PacketDirection::Extension,
                "cb",
                json!({"p":{"player-a":{"intHP":0}}}),
            ),
            1000.0,
        )
        .unwrap();
        assert!(repeated.events.is_empty());
    }

    #[test]
    fn monster_death_precedes_server_aura_removal_and_numeric_key_order_is_preserved() {
        let mut world = world();
        let monster = world.get_monster(1).unwrap();
        monster.project_auras(
            AuraMutation::Seed(vec![AuraSeed {
                name: "Counter Attack".into(),
                icon: "counter".into(),
                stack: 1.0,
                persistent: false,
                timer: SeedTimer::Timed {
                    remaining_seconds: 3.0,
                    full_seconds: 3.0,
                },
            }]),
            1000.0,
        );
        let output = project_combat(
            &mut world,
            &packet(
                PacketDirection::Server,
                "ct",
                json!({"m":{"3":{"intHP":0,"intState":0},"1":{"intHP":0,"intState":0}}}),
            ),
            2000.0,
        )
        .unwrap();
        let value = events(&output);
        assert_eq!(value[0], json!({"type":"monster-death","monsterMapId":1}));
        assert_eq!(
            value[1],
            json!({"type":"counter-attack-end","monsterMapId":1,"source":"aura","triggerId":"anti-counter","triggerText":"Counter Attack"})
        );
        assert_eq!(
            value[2],
            json!({"type":"aura-removed","duration":3.0,"icon":"counter","kind":"active","name":"Counter Attack","targetId":1,"targetType":"monster"})
        );
        assert_eq!(value[3], json!({"type":"monster-death","monsterMapId":3}));
        assert!(monster.auras().is_empty());
    }

    #[test]
    fn malformed_siblings_do_not_suppress_valid_updates() {
        let mut world = world();
        let output = project_combat(
            &mut world,
            &packet(
                PacketDirection::Extension,
                "cb",
                json!({
                    "p":{"player-a":{"intHP":70},"missing":{"intHP":1},"player-b":{"intHP":null}},
                    "m":{"1":{"intHP":30},"8":{},"9":{"intHP":"bad"},"0":{},"not-id":{}}
                }),
            ),
            1000.0,
        )
        .unwrap();
        assert_eq!(world.get_player("player-a").unwrap().data().entity.hp, 70.0);
        assert_eq!(world.get_monster(1).unwrap().data().entity.hp, 30.0);

        assert!(output.events.is_empty());
    }

    #[test]
    fn messages_substitute_names_and_counter_windows_use_the_longest_matching_cast() {
        let mut world = world();
        let output = project_combat(&mut world,&packet(PacketDirection::Server,"ct",json!({
            "anims":[{"msg":["<mon> prepares","a counter attack"],"animStr":"cast","cInf":"p:1","tInf":"m:1"},{"msg":"ignored","cInf":null},{"msg":"   "}],
            "sara":[{"actionResult":{"cInf":"m:1","ct":600}},{"actionResult":{"cInf":"prefix>m:1,p:1","ct":"900"}},{"actionResult":{"cInf":"m:3","ct":5000}},{"actionResult":{"cInf":null,"ct":10000}},{"actionResult":{"cInf":"m:1","ct":"bad"}}]
        })),1000.0).unwrap();
        assert_eq!(
            events(&output),
            json!([
                {"type":"counter-attack-start","monsterMapId":1,"source":"message","triggerId":"anti-counter","triggerText":"Sentinel prepares a counter attack","durationMs":900.0},
                {"type":"update-message","animation":"cast","message":"Sentinel prepares a counter attack","monsterMapId":1,"source":"animation"}
            ])
        );
    }

    #[test]
    fn embedded_passives_merge_after_animation_events() {
        let mut world = world();
        let player = world.get_player("player-a").unwrap();
        player.project_auras(
            AuraMutation::Passives {
                mode: PassiveMode::Replace,
                entries: vec![PassiveAuraInput {
                    name: "Existing".into(),
                    category: None,
                    icon: None,
                    value: None,
                    duration: 0.0,
                }],
            },
            1000.0,
        );
        let output = project_combat(
            &mut world,
            &packet(
                PacketDirection::Extension,
                "cb",
                json!({
                    "anims":[{"msg":"Notice"}],
                    "a":[{"cmd":"aura+p","tInf":"p:1","auras":[{"nam":"Added","dur":4}]}]
                }),
            ),
            2000.0,
        )
        .unwrap();
        assert_eq!(
            events(&output)[0],
            json!({"type":"update-message","message":"Notice","source":"animation"})
        );
        assert_eq!(
            events(&output)[1],
            json!({"type":"aura-added","duration":4.0,"kind":"passive","name":"Added","stack":1.0,"targetId":1,"targetType":"player"})
        );
        assert!(player.aura("Existing", Some(AuraKind::Passive)).is_some());
        assert!(player.aura("Added", Some(AuraKind::Passive)).is_some());
    }

    #[test]
    fn only_the_combat_direction_routes_apply_and_top_level_invalidity_is_atomic() {
        let mut world = world();
        assert!(
            project_combat(
                &mut world,
                &packet(
                    PacketDirection::Extension,
                    "ct",
                    json!({"p":{"player-a":{"intHP":0}}})
                ),
                0.0
            )
            .is_none()
        );
        assert!(
            project_combat(
                &mut world,
                &packet(
                    PacketDirection::Server,
                    "cb",
                    json!({"p":{"player-a":{"intHP":0}}})
                ),
                0.0
            )
            .is_none()
        );
        let output = project_combat(
            &mut world,
            &packet(
                PacketDirection::Server,
                "ct",
                json!({"p":{"player-a":{"intHP":0}},"anims":false}),
            ),
            0.0,
        )
        .unwrap();
        assert_eq!(
            world.get_player("player-a").unwrap().data().entity.hp,
            100.0
        );
        assert!(output.events.is_empty());
    }
}
