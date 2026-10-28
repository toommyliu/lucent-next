use std::sync::LazyLock;

use lucent_protocol::Packet;
use lucent_protocol::aura::{
    AuraApplication, AuraDelta, AuraMutation, AuraSeed, AuraTiming, AuraValue, AuraWithdrawal,
    PassiveAuraInput, PassiveMode, SeedTimer,
};
use lucent_protocol::event::{
    AuraEvent, AuraKind, CounterSource, EntityKind, MessageSource, ProjectionEvent,
};
use regex::Regex;
use serde_json::{Map, Value};

use super::ProjectionOutput;
use super::entity::{LiveMonster, LivePlayer};
use super::wire::{trim_js, wire_number};
use super::world::WorldStore;

struct AntiCounterTrigger {
    aura_names: &'static [&'static str],
    aura_patterns: Vec<Regex>,
    message_patterns: Vec<Regex>,
}

static ANTI_COUNTER_TRIGGERS: LazyLock<Vec<AntiCounterTrigger>> = LazyLock::new(|| {
    vec![AntiCounterTrigger {
        aura_names: &["Counter Attack"],
        aura_patterns: vec![
            Regex::new(r"(?i)(?-u:\b)counter(?-u:\b).*(?-u:\b)attack(?-u:\b)").unwrap(),
        ],
        message_patterns: vec![Regex::new(r"(?i-u:prepares +a +counter +attack)").unwrap()],
    }]
});

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AuraTarget {
    pub kind: EntityKind,
    pub id: i64,
}

pub fn parse_entity_references(value: &str) -> Vec<AuraTarget> {
    value
        .split(',')
        .filter_map(|token| {
            let token = trim_js(token.rsplit_once('>').map_or(token, |(_, token)| token));
            let mut fields = token.split(':');
            let kind = match fields.next()? {
                "m" => EntityKind::Monster,
                "p" => EntityKind::Player,
                _ => return None,
            };
            let id = wire_number(&Value::String(fields.next()?.into()))?;
            (id.fract() == 0.0 && id > 0.0 && id <= i64::MAX as f64).then_some(AuraTarget {
                kind,
                id: id as i64,
            })
        })
        .collect()
}

fn scalar(value: &Value) -> bool {
    value.is_null() || value.is_string() || value.is_boolean() || value.is_number()
}
fn js_number(value: Option<&Value>) -> f64 {
    match value {
        Some(Value::Null) => 0.0,
        Some(Value::Bool(value)) => {
            if *value {
                1.0
            } else {
                0.0
            }
        }
        Some(Value::String(value)) if trim_js(value).is_empty() => 0.0,
        Some(value) => wire_number(value).unwrap_or(f64::NAN),
        None => f64::NAN,
    }
}
fn js_int(value: Option<&Value>) -> f64 {
    let value = js_number(value);
    if !value.is_finite() || value == 0.0 {
        return 0.0;
    }
    let modulo = value.trunc().rem_euclid(4294967296.0);
    if modulo >= 2147483648.0 {
        modulo - 4294967296.0
    } else {
        modulo
    }
}
fn js_string(value: Option<&Value>) -> String {
    match value {
        None => "undefined".into(),
        Some(Value::Null) => "null".into(),
        Some(Value::String(value)) => value.clone(),
        Some(Value::Number(number)) => {
            let number = number.as_f64().unwrap_or(f64::NAN);
            if number == 0.0 {
                "0".into()
            } else if number.abs() >= 1e21 || number.abs() < 1e-6 {
                let scientific = format!("{number:e}");
                if scientific.contains("e-") {
                    scientific
                } else {
                    scientific.replace('e', "e+")
                }
            } else {
                number.to_string()
            }
        }
        Some(value) => value.to_string(),
    }
}
fn js_truthy(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => false,
        Some(Value::Bool(value)) => *value,
        Some(Value::Number(value)) => value
            .as_f64()
            .is_some_and(|value| value != 0.0 && !value.is_nan()),
        Some(Value::String(value)) => !value.is_empty(),
        Some(_) => true,
    }
}

pub struct SeedResult {
    pub entries: Vec<AuraSeed>,
    pub rejected: usize,
}
pub fn decode_aura_seed(value: Option<&Value>) -> SeedResult {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return SeedResult {
            entries: vec![],
            rejected: 0,
        };
    };
    let Some(values) = value.as_array() else {
        return SeedResult {
            entries: vec![],
            rejected: 1,
        };
    };
    let mut entries = Vec::new();
    let mut rejected = 0;
    for value in values {
        let Some(slots) = value.as_array().filter(|slots| {
            slots.len() >= 5 && !slots[0].is_null() && slots.iter().take(6).all(scalar)
        }) else {
            rejected += 1;
            continue;
        };
        let remaining = js_number(slots.get(2));
        let full = js_number(slots.get(5));
        entries.push(AuraSeed {
            name: js_string(slots.first()),
            icon: js_string(slots.get(1)),
            stack: js_int(slots.get(3)),
            persistent: js_int(slots.get(4)) == 1.0,
            timer: if remaining > 0.0 && remaining.is_finite() {
                SeedTimer::Timed {
                    remaining_seconds: remaining,
                    full_seconds: if full.is_finite() && full > remaining {
                        full
                    } else {
                        remaining
                    },
                }
            } else {
                SeedTimer::Untimed
            },
        });
    }
    SeedResult { entries, rejected }
}

pub trait AuraEntity {
    fn apply_auras(&self, mutation: AuraMutation, now_ms: f64) -> Vec<AuraDelta>;
}
impl AuraEntity for LivePlayer {
    fn apply_auras(&self, mutation: AuraMutation, now_ms: f64) -> Vec<AuraDelta> {
        self.project_auras(mutation, now_ms)
    }
}
impl AuraEntity for LiveMonster {
    fn apply_auras(&self, mutation: AuraMutation, now_ms: f64) -> Vec<AuraDelta> {
        self.project_auras(mutation, now_ms)
    }
}
pub struct AppliedSeed {
    pub changes: Vec<AuraDelta>,
    pub rejected: usize,
}
pub fn seed_auras(entity: &impl AuraEntity, value: Option<&Value>, now_ms: f64) -> AppliedSeed {
    let decoded = decode_aura_seed(value);
    AppliedSeed {
        changes: entity.apply_auras(AuraMutation::Seed(decoded.entries), now_ms),
        rejected: decoded.rejected,
    }
}

pub fn message_text(value: Option<&Value>) -> Option<String> {
    let value = match value? {
        Value::Null => return None,
        Value::String(value) => value.clone(),
        Value::Array(values) => values
            .iter()
            .map(Value::as_str)
            .collect::<Option<Vec<_>>>()?
            .join(" "),
        _ => return None,
    };
    let value = trim_js(&value);
    (!value.is_empty()).then(|| value.into())
}

fn normalize_message(value: &str) -> String {
    value.split(|value| matches!(value, '\u{0009}'..='\u{000d}' | '\u{0020}' | '\u{00a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}')).filter(|value| !value.is_empty()).collect::<Vec<_>>().join(" ").to_lowercase()
}

pub fn match_anti_counter_aura(name: &str) -> bool {
    let normalized = normalize_message(name);
    ANTI_COUNTER_TRIGGERS.iter().any(|trigger| {
        trigger
            .aura_names
            .iter()
            .any(|name| normalize_message(name) == normalized)
            || trigger
                .aura_patterns
                .iter()
                .any(|pattern| pattern.is_match(&normalized))
    })
}
pub fn match_anti_counter_message(message: &str) -> bool {
    let normalized = normalize_message(message);
    ANTI_COUNTER_TRIGGERS.iter().any(|trigger| {
        trigger
            .message_patterns
            .iter()
            .any(|pattern| pattern.is_match(&normalized))
    })
}

pub fn aura_events(
    target_kind: EntityKind,
    target_id: i64,
    changes: &[AuraDelta],
    source: Option<AuraTarget>,
    applied_at_ms: Option<f64>,
) -> Vec<ProjectionEvent> {
    let mut events = Vec::new();
    for change in changes {
        let (aura, removed) = match change {
            AuraDelta::Added { after } | AuraDelta::Updated { after, .. } => (after, false),
            AuraDelta::Removed { before } => (before, true),
        };
        if target_kind == EntityKind::Monster
            && aura.kind == AuraKind::Active
            && match_anti_counter_aura(&aura.name)
        {
            if removed {
                events.push(ProjectionEvent::CounterAttackEnd {
                    monster_map_id: target_id,
                    source: CounterSource::Aura,
                    trigger_id: "anti-counter".into(),
                    trigger_text: aura.name.clone(),
                });
            } else if let Some(now_ms) = applied_at_ms {
                events.push(ProjectionEvent::CounterAttackStart {
                    duration_ms: aura.expires_at.map(|expiry| expiry - now_ms),
                    monster_map_id: target_id,
                    source: CounterSource::Aura,
                    trigger_id: "anti-counter".into(),
                    trigger_text: aura.name.clone(),
                });
            }
        }
        let aura = AuraEvent {
            duration: Some(aura.duration),
            icon: aura.icon.clone(),
            kind: aura.kind,
            name: aura.name.clone(),
            source_id: source.map(|source| source.id),
            source_type: source.map(|source| source.kind),
            target_id,
            target_type: target_kind,
        };
        events.push(match change {
            AuraDelta::Added { after } => ProjectionEvent::AuraAdded {
                aura,
                stack: after.stack,
            },
            AuraDelta::Updated { after, .. } => ProjectionEvent::AuraUpdated {
                aura,
                stack: after.stack,
            },
            AuraDelta::Removed { .. } => ProjectionEvent::AuraRemoved { aura },
        });
    }
    events
}

fn optional_nullable<T>(
    object: &Map<String, Value>,
    key: &str,
    decode: impl FnOnce(&Value) -> Option<T>,
) -> Option<Option<T>> {
    object
        .get(key)
        .filter(|value| !value.is_null())
        .map_or(Some(None), |value| decode(value).map(Some))
}
fn nullable_string(object: &Map<String, Value>, key: &str) -> Option<Option<String>> {
    optional_nullable(object, key, |value| value.as_str().map(str::to_owned))
}
fn optional_scalar<'a>(object: &'a Map<String, Value>, key: &str) -> Option<Option<&'a Value>> {
    match object.get(key) {
        None => Some(None),
        Some(value) if scalar(value) => Some(Some(value)),
        _ => None,
    }
}
fn valid_message(value: &Value) -> bool {
    value.is_string()
        || value
            .as_array()
            .is_some_and(|values| values.iter().all(Value::is_string))
}

struct AuraPayload {
    name: String,
    category: Option<String>,
    duration: Option<f64>,
    icon: Option<String>,
    is_new: bool,
    persistent: bool,
    stack: Option<f64>,
    timer_type: Option<String>,
    value: Option<AuraValue>,
    message_on: Option<Value>,
    message_off: Option<Value>,
}
fn decode_payload(value: &Value) -> Option<AuraPayload> {
    let object = value.as_object()?;
    let name = object.get("nam")?.as_str()?.into();
    let category = nullable_string(object, "cat")?;
    let duration = optional_nullable(object, "dur", wire_number)?;
    let icon = nullable_string(object, "icon")?;
    let is_new = js_truthy(optional_scalar(object, "isNew")?);
    let persistent = js_number(optional_scalar(object, "persist")?) == 1.0;
    let stack = optional_scalar(object, "stk")?.map(|value| js_int(Some(value)));
    let timer_type = nullable_string(object, "t")?;
    let value = optional_nullable(object, "val", |value| {
        wire_number(value)
            .map(AuraValue::Number)
            .or_else(|| value.as_str().map(|value| AuraValue::Text(value.into())))
    })?;
    let message_on = optional_nullable(object, "msgOn", |value| {
        valid_message(value).then(|| value.clone())
    })?;
    let message_off = optional_nullable(object, "msgOff", |value| {
        valid_message(value).then(|| value.clone())
    })?;
    Some(AuraPayload {
        name,
        category,
        duration,
        icon,
        is_new,
        persistent,
        stack,
        timer_type,
        value,
        message_on,
        message_off,
    })
}

pub fn project_aura_events(
    world: &mut WorldStore,
    values: &[Value],
    passive_mode: PassiveMode,
    now_ms: f64,
) -> ProjectionOutput {
    let mut output = ProjectionOutput::default();
    for value in values {
        let decoded = value.as_object().and_then(|object| {
            let command = object.get("cmd")?.as_str()?;
            if !matches!(
                command,
                "aura+" | "aura++" | "aura-" | "aura--" | "aura=" | "aura+p" | "aura*"
            ) {
                return None;
            }
            let target = object.get("tInf")?.as_str()?;
            let source = nullable_string(object, "cInf")?;
            let auras = optional_nullable(object, "auras", |value| value.as_array().cloned())?;
            Some((command, target, source, auras, object.get("aura")))
        });
        let Some((command, target, source, auras, aura)) = decoded else {
            tracing::warn!(input = "aura_change", "invalid projection input");
            continue;
        };
        if command == "aura*" {
            continue;
        }
        let raw_payloads = auras.unwrap_or_else(|| {
            aura.filter(|aura| !aura.is_null())
                .cloned()
                .into_iter()
                .collect()
        });
        let mut payloads = Vec::new();
        for value in raw_payloads {
            if let Some(payload) = decode_payload(&value) {
                payloads.push(payload);
            } else {
                tracing::warn!(input = "aura_payload", "invalid projection input");
            }
        }
        static TARGET: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[mp]:[0-9]+$").unwrap());
        let Some(target) = TARGET
            .is_match(target)
            .then(|| parse_entity_references(target))
            .and_then(|targets| targets.first().copied())
        else {
            continue;
        };
        let source = source
            .as_deref()
            .map(parse_entity_references)
            .and_then(|sources| sources.first().copied());
        if command == "aura+p" {
            let entries = payloads
                .into_iter()
                .map(|payload| PassiveAuraInput {
                    name: payload.name,
                    category: payload.category,
                    icon: payload.icon,
                    value: payload.value,
                    duration: payload.duration.unwrap_or(0.0),
                })
                .collect();
            let changes = world.project_auras(
                target.kind,
                target.id,
                AuraMutation::Passives {
                    mode: passive_mode,
                    entries,
                },
                now_ms,
            );
            output
                .events
                .extend(aura_events(target.kind, target.id, &changes, source, None));
            continue;
        }
        let adding = command == "aura+" || command == "aura++";
        for payload in payloads {
            let mutation = if adding {
                AuraMutation::Apply(vec![AuraApplication {
                    name: payload.name.clone(),
                    stack: payload.stack.unwrap_or(1.0),
                    timing: if payload.timer_type.as_deref() == Some("s")
                        && payload.duration.is_some_and(|duration| duration > 0.0)
                    {
                        AuraTiming::Seconds(payload.duration.unwrap())
                    } else {
                        AuraTiming::Untimed
                    },
                    persistent: payload.persistent,
                    restart_duration: payload.is_new,
                    icon: payload.icon.clone(),
                    category: payload.category.clone(),
                    value: payload.value.clone(),
                }])
            } else if command == "aura=" {
                AuraMutation::SetStack(
                    payload
                        .stack
                        .map(|stack| (payload.name.clone(), stack))
                        .into_iter()
                        .collect(),
                )
            } else {
                AuraMutation::Withdraw(vec![match payload.stack {
                    Some(stack) if stack > 0.0 => AuraWithdrawal::Decay {
                        name: payload.name.clone(),
                        stack,
                        refresh_seconds: payload.duration,
                    },
                    _ => AuraWithdrawal::Remove {
                        name: payload.name.clone(),
                    },
                }])
            };
            let changes = world.project_auras(target.kind, target.id, mutation, now_ms);
            output.events.extend(aura_events(
                target.kind,
                target.id,
                &changes,
                source,
                adding.then_some(now_ms),
            ));
            if command == "aura=" {
                continue;
            }
            let Some(message) = message_text(if adding {
                payload.message_on.as_ref()
            } else {
                payload.message_off.as_ref()
            }) else {
                continue;
            };
            let self_only = message.starts_with('@');
            if self_only
                && (target.kind != EntityKind::Player
                    || world
                        .get_me()
                        .is_none_or(|player| player.data().entity_id != target.id))
            {
                continue;
            }
            let message = if self_only {
                trim_js(&message[1..]).into()
            } else {
                message
            };
            if !message.is_empty() {
                output.events.push(ProjectionEvent::UpdateMessage {
                    animation: None,
                    message,
                    monster_map_id: (target.kind == EntityKind::Monster).then_some(target.id),
                    source: MessageSource::Aura,
                });
            }
        }
    }
    output
}

pub fn project_standalone_auras(
    world: &mut WorldStore,
    packet: &Packet,
    now_ms: f64,
) -> Option<ProjectionOutput> {
    if packet.direction() != lucent_protocol::PacketDirection::Extension {
        return None;
    }
    if !matches!(
        packet.command(),
        "aura+" | "aura++" | "aura-" | "aura--" | "aura+p" | "aura*" | "auSnap"
    ) {
        return None;
    }
    let data = packet.data()?;
    if packet.command() != "auSnap" {
        return Some(project_aura_events(
            world,
            std::slice::from_ref(data),
            PassiveMode::Replace,
            now_ms,
        ));
    }
    let mut output = ProjectionOutput::default();
    let Some((username, values)) = data
        .get("unm")
        .and_then(Value::as_str)
        .zip(data.get("au").and_then(Value::as_array))
    else {
        return Some(output);
    };
    let Some(player) = world.get_player(username) else {
        return Some(output);
    };
    let seed = seed_auras(&player, Some(&Value::Array(values.clone())), now_ms);
    if seed.rejected > 0 {
        tracing::warn!(
            command = "auSnap",
            rejected = seed.rejected,
            "projection discarded invalid entries"
        );
    }
    output.events.extend(aura_events(
        EntityKind::Player,
        player.data().entity_id,
        &seed.changes,
        None,
        None,
    ));
    Some(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn counter_triggers_match_reference_names_and_messages() {
        for name in [
            "Counter Attack",
            "Empowered Counter Attack",
            "Counter\nAttack",
            "éCounter Attack",
        ] {
            assert!(match_anti_counter_aura(name), "{name:?}");
        }
        assert!(!match_anti_counter_aura("Focus"));
        assert!(!match_anti_counter_aura("Counterattack"));
        assert!(match_anti_counter_message(
            "  Boss prepares   a counter attack!  "
        ));
        assert!(!match_anti_counter_message("Boss prepares an attack"));
    }

    #[test]
    fn seeds_use_as3_integer_conversion_and_server_owned_expiry() {
        let seed = decode_aura_seed(Some(&json!([
            ["Focus", "icon", "3", "4294967297", true, "5"],
            [null, "x", 1, 1, 0],
            ["bad", {}, 1, 1, 0]
        ])));
        assert_eq!(seed.entries.len(), 1);
        assert_eq!(seed.rejected, 2);
        assert_eq!(seed.entries[0].stack, 1.0);
        assert!(seed.entries[0].persistent);
        assert_eq!(
            seed.entries[0].timer,
            SeedTimer::Timed {
                remaining_seconds: 3.0,
                full_seconds: 5.0
            }
        );
    }
    #[test]
    fn empty_messages_and_entity_reference_syntax_follow_the_reference() {
        assert_eq!(
            message_text(Some(&json!(["  first ", "second "]))),
            Some("first  second".into())
        );
        assert_eq!(message_text(Some(&json!([" "]))), None);
        assert_eq!(
            parse_entity_references("actor>m:3, p:5, m:0, p:no, unknown:4"),
            vec![
                AuraTarget {
                    kind: EntityKind::Monster,
                    id: 3
                },
                AuraTarget {
                    kind: EntityKind::Player,
                    id: 5
                }
            ]
        );
    }
    #[test]
    fn counter_aura_events_precede_deltas_and_server_state_removes_them() {
        use lucent_protocol::world::{EntityData, EntityPatch, EntityState, MonsterData};
        let mut world = WorldStore::default();
        world.put_monster(LiveMonster::new(
            MonsterData {
                entity: EntityData {
                    hp: 100.0,
                    max_hp: 100.0,
                    state: EntityState::InCombat,
                    ..EntityData::default()
                },
                monster_map_id: 5,
                name: "Training Target".into(),
                ..MonsterData::default()
            },
            true,
        ));
        let output = project_aura_events(
            &mut world,
            &[
                json!({"cmd":"aura+","tInf":"m:5","cInf":"p:1","aura":{"nam":"Counter Attack","t":"s","dur":10,"stk":2}}),
            ],
            PassiveMode::Replace,
            1000.0,
        );
        assert!(matches!(
            output.events[0],
            ProjectionEvent::CounterAttackStart {
                duration_ms: Some(10000.0),
                monster_map_id: 5,
                ..
            }
        ));
        assert!(
            matches!(&output.events[1], ProjectionEvent::AuraAdded { aura, stack:2.0 } if aura.source_id == Some(1) && aura.target_id == 5)
        );
        let monster = world.get_monster(5).unwrap();
        let held = monster.aura("counter attack", None).unwrap();
        let output = project_aura_events(
            &mut world,
            &[json!({"cmd":"aura--","tInf":"m:5","aura":{"nam":"Counter Attack","stk":1,"dur":2}})],
            PassiveMode::Replace,
            5000.0,
        );
        assert!(matches!(
            output.events[0],
            ProjectionEvent::AuraUpdated { stack: 1.0, .. }
        ));
        assert_eq!(held.data().duration, 10.0);
        assert_eq!(held.data().expires_at, Some(7000.0));
        let changes = world
            .write_monster(
                5,
                &EntityPatch {
                    state: Some(EntityState::Dead),
                    ..EntityPatch::default()
                },
            )
            .unwrap()
            .aura_changes;
        let events = aura_events(EntityKind::Monster, 5, &changes, None, None);
        assert!(matches!(
            events[0],
            ProjectionEvent::CounterAttackEnd {
                monster_map_id: 5,
                ..
            }
        ));
        assert!(matches!(events[1], ProjectionEvent::AuraRemoved { .. }));
        assert!(monster.auras().is_empty());
        assert_eq!(held.data().stack, 1.0);
    }

    #[test]
    fn passive_replace_and_merge_have_distinct_membership_and_identity() {
        use lucent_protocol::world::PlayerData;
        let mut world = WorldStore::default();
        let player = world.put_player(LivePlayer::new(PlayerData {
            entity_id: 1,
            username: "player-a".into(),
            ..PlayerData::default()
        }));
        let first = json!({"cmd":"aura+p","tInf":"p:1","auras":[{"nam":"First","val":"2"}]});
        project_aura_events(&mut world, &[first], PassiveMode::Replace, 0.0);
        let held = player.aura("First", None).unwrap();
        assert_eq!(held.data().value, Some(AuraValue::Number(2.0)));
        let second = json!({"cmd":"aura+p","tInf":"p:1","auras":[{"nam":"Second","val":"text"}]});
        project_aura_events(
            &mut world,
            std::slice::from_ref(&second),
            PassiveMode::Merge,
            0.0,
        );
        assert_eq!(player.auras().len(), 2);
        project_aura_events(&mut world, &[second], PassiveMode::Replace, 0.0);
        assert_eq!(player.auras().len(), 1);
        assert!(player.aura("First", None).is_none());
        assert_eq!(held.data().value, Some(AuraValue::Number(2.0)));
    }
}
