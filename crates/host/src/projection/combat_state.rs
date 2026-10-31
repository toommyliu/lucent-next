use std::collections::{BTreeMap, BTreeSet};
use std::time::{SystemTime, UNIX_EPOCH};

use lucent_protocol::combat_state::{
    CombatMonsterState, CombatSnapshot, CombatTarget, ConsumableReadiness, SkillCooldown,
};
use lucent_protocol::event::ProjectionEvent;
use lucent_protocol::query::MonsterSelector;
use lucent_protocol::world::EntityState;
use serde_json::Value;

use super::Projection;
use super::commands::decode_error;
use super::wire::{selector_id, trim_js, wire_number};
use super::world::{CellScope, MonsterLookup, monster_lookup};
use crate::bridge::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatScope {
    epoch: u64,
    area_revision: u64,
    packet_revision: u64,
}

impl CombatScope {
    pub(super) fn same_world(self, other: Self) -> bool {
        self.epoch == other.epoch && self.area_revision == other.area_revision
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CombatHydration {
    pub actions: Value,
    pub haste: Value,
    pub global_cooldown: Value,
    pub global_used_at: Value,
    pub potions_locked: Value,
    pub target: Value,
    pub available_monster_ids: Value,
}

#[derive(Debug)]
struct Skill {
    cooldown: f64,
    used_at: f64,
}

#[derive(Debug)]
struct LocalCombat {
    scope: CombatScope,
    observed_at: f64,
    skills: [Option<Skill>; 6],
    global: Skill,
    target: CombatTarget,
    consumable: Option<(i64, bool)>,
    available: BTreeSet<i64>,
    availability_scope: CellScope,
}

#[derive(Debug, Default)]
pub(super) struct CombatStore {
    local: Option<LocalCombat>,
    counters: BTreeMap<i64, f64>,
}

impl CombatStore {
    pub(super) fn observe(&mut self, events: &[ProjectionEvent], now_ms: f64) {
        for event in events {
            match event {
                ProjectionEvent::JoinMap { .. } => *self = Self::default(),
                ProjectionEvent::CounterAttackStart {
                    monster_map_id,
                    duration_ms,
                    ..
                } => {
                    let duration = duration_ms
                        .filter(|duration| duration.is_finite() && *duration > 0.0)
                        .unwrap_or(0.0);
                    let deadline = now_ms + duration + 7000.0;
                    self.counters
                        .entry(*monster_map_id)
                        .and_modify(|current| *current = current.max(deadline))
                        .or_insert(deadline);
                }
                ProjectionEvent::CounterAttackEnd { monster_map_id, .. }
                | ProjectionEvent::MonsterDeath { monster_map_id }
                | ProjectionEvent::MonsterRespawn { monster_map_id } => {
                    self.counters.remove(monster_map_id);
                }
                _ => {}
            }
        }
    }
}

fn invalid(reason: &str) -> Error {
    decode_error("refreshCombatState", reason)
}

fn number(value: &Value) -> Result<f64, Error> {
    wire_number(value)
        .filter(|value| value.abs() <= 9_007_199_254_740_991.0)
        .ok_or_else(|| invalid("expected a finite safe number"))
}

fn positive_id(value: &Value) -> Result<i64, Error> {
    selector_id(number(value)?)
        .filter(|id| *id > 0)
        .ok_or_else(|| invalid("expected a positive integral ID"))
}

fn truthy(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => false,
        Some(Value::Bool(value)) => *value,
        Some(Value::Number(value)) => value.as_f64().is_some_and(|value| value != 0.0),
        Some(Value::String(value)) => !value.is_empty(),
        Some(Value::Array(_) | Value::Object(_)) => true,
    }
}

impl Skill {
    fn remaining(&self, now_ms: f64) -> f64 {
        (self.cooldown - (now_ms - self.used_at)).max(0.0)
    }
}

impl LocalCombat {
    fn remaining(&self, slot: usize, now_ms: f64) -> f64 {
        self.skills[slot].as_ref().map_or(0.0, |skill| {
            skill.remaining(now_ms).max(self.global.remaining(now_ms))
        })
    }

    fn cooldown(&self, slot: usize, now_ms: f64) -> SkillCooldown {
        let remaining = self.remaining(slot, now_ms).trunc() as i64;
        if remaining <= 0 {
            SkillCooldown::Ready
        } else {
            SkillCooldown::Cooldown {
                remaining_ms: remaining,
            }
        }
    }
}

impl Projection {
    pub(crate) fn begin_combat_hydration(&mut self) -> CombatScope {
        self.combat.local = None;
        self.combat_scope()
    }

    pub fn combat_scope(&self) -> CombatScope {
        CombatScope {
            epoch: self.receipts.epoch(),
            area_revision: self.world.area_revision,
            packet_revision: self.packet_revision,
        }
    }

    pub fn hydrate_combat(
        &mut self,
        scope: CombatScope,
        input: CombatHydration,
        now_ms: f64,
    ) -> Result<(), Error> {
        if scope != self.combat_scope() {
            return Err(invalid(
                "combat hydration belongs to an earlier combat observation",
            ));
        }
        self.combat.local = None;
        let observed_at = number(&Value::from(now_ms))?;
        let haste = 1.0 - number(&input.haste)?.clamp(-1.0, 0.5);
        let actions = input
            .actions
            .as_array()
            .ok_or_else(|| invalid("expected an active action array"))?;
        let mut skills = std::array::from_fn(|_| None);
        for (slot, skill) in skills.iter_mut().enumerate() {
            let Some(action) = actions.get(slot).filter(|action| !action.is_null()) else {
                continue;
            };
            let cooldown = action
                .get("OldCD")
                .filter(|value| !value.is_null())
                .or_else(|| action.get("cd"))
                .ok_or_else(|| invalid("missing action cooldown"))?;
            let cooldown = number(cooldown)?;
            if !(0.0..=f64::from(i32::MAX) / 2.0).contains(&cooldown) {
                return Err(invalid("action cooldown exceeds the client integer range"));
            }
            *skill = Some(Skill {
                cooldown: (cooldown * haste + 0.5).floor(),
                used_at: number(
                    action
                        .get("ts")
                        .ok_or_else(|| invalid("missing action timestamp"))?,
                )?,
            });
        }
        let global = Skill {
            cooldown: number(&input.global_cooldown)?,
            used_at: number(&input.global_used_at)?,
        };
        let locked = truthy(Some(&input.potions_locked));
        let consumable = actions
            .get(5)
            .filter(|action| action.get("ref").and_then(Value::as_str) == Some("i1"))
            .and_then(|action| {
                action
                    .get("sArg1")
                    .and_then(|id| positive_id(id).ok())
                    .map(|id| {
                        (
                            id,
                            !locked
                                && truthy(action.get("isOK"))
                                && !truthy(action.get("skillLock"))
                                && !truthy(action.get("lock")),
                        )
                    })
            });
        let target = if input.target.is_null() {
            CombatTarget::None
        } else {
            match input.target.get("type").and_then(Value::as_str) {
                Some("monster") => {
                    let id = positive_id(
                        input
                            .target
                            .get("monsterMapId")
                            .ok_or_else(|| invalid("missing target ID"))?,
                    )?;
                    self.world
                        .get_monster(id)
                        .map_or(CombatTarget::Unavailable, |monster| CombatTarget::Monster {
                            monster_map_id: id,
                            generation: monster.generation(),
                        })
                }
                Some("player") => {
                    let entity_id = positive_id(
                        input
                            .target
                            .get("entityId")
                            .ok_or_else(|| invalid("missing target ID"))?,
                    )?;
                    let hp = number(
                        input
                            .target
                            .get("hp")
                            .ok_or_else(|| invalid("missing target HP"))?,
                    )?;
                    let state = number(
                        input
                            .target
                            .get("state")
                            .ok_or_else(|| invalid("missing target state"))?,
                    )?;
                    if ![0.0, 1.0, 2.0].contains(&state) {
                        return Err(invalid("invalid target state"));
                    }
                    CombatTarget::Player {
                        entity_id,
                        alive: hp > 0.0 && state > 0.0,
                    }
                }
                _ => return Err(invalid("expected a player or monster target")),
            }
        };
        let available = input
            .available_monster_ids
            .as_array()
            .ok_or_else(|| invalid("expected available monster IDs"))?
            .iter()
            .map(positive_id)
            .collect::<Result<BTreeSet<_>, _>>()?;
        self.combat.local = Some(LocalCombat {
            scope,
            observed_at,
            skills,
            global,
            target,
            consumable,
            available,
            availability_scope: self.world.cell_scope(),
        });
        Ok(())
    }

    pub fn combat_snapshot(
        &self,
        now_ms: f64,
        priorities: &[MonsterSelector],
        anti_counter_enabled: bool,
    ) -> CombatSnapshot {
        let scope = self.combat_scope();
        let local = self
            .combat
            .local
            .as_ref()
            .filter(|local| local.scope.same_world(scope));
        let availability =
            local.filter(|local| local.availability_scope == self.world.cell_scope());
        CombatSnapshot {
            epoch: scope.epoch,
            area_revision: scope.area_revision,
            observed_at_ms: local.map(|local| local.observed_at),
            skills: std::array::from_fn(|slot| {
                local.map_or(SkillCooldown::Unavailable, |local| {
                    local.cooldown(slot, now_ms)
                })
            }),
            selected_target: local.map_or(CombatTarget::Unavailable, |local| match local.target {
                CombatTarget::Player { entity_id, .. } => {
                    self.world.get_player_by_id(entity_id).map_or_else(
                        || local.target.clone(),
                        |player| CombatTarget::Player {
                            entity_id,
                            alive: player.data().entity.alive(),
                        },
                    )
                }
                _ => local.target.clone(),
            }),
            consumable: local.and_then(|local| {
                local
                    .consumable
                    .map(|(item_id, allowed)| ConsumableReadiness {
                        item_id,
                        ready: allowed && local.remaining(5, now_ms) <= 0.0,
                    })
            }),
            monsters: self
                .world
                .monsters()
                .into_iter()
                .map(|monster| {
                    let data = monster.data();
                    CombatMonsterState {
                        monster_map_id: data.monster_map_id,
                        generation: monster.generation(),
                        priority_rank: priorities.iter().position(|selector| match monster_lookup(
                            selector,
                        ) {
                            MonsterLookup::Id(id) => selector_id(id) == Some(data.monster_map_id),
                            MonsterLookup::Name(name) => {
                                let name = trim_js(name).to_lowercase();
                                name == "*"
                                    || (!name.is_empty()
                                        && name == trim_js(&data.name).to_lowercase())
                            }
                        }),
                        attack_blocked: anti_counter_enabled
                            && self
                                .combat
                                .counters
                                .get(&data.monster_map_id)
                                .is_some_and(|until| *until > now_ms),
                        available: availability.map(|local| {
                            local.available.contains(&data.monster_map_id)
                                && data.entity.state != EntityState::Dead
                        }),
                    }
                })
                .collect(),
        }
    }
}

pub(crate) fn now_ms() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
        * 1000.0
}
