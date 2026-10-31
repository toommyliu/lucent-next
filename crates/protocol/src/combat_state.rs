use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Serialize, ts_rs::TS)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SkillCooldown {
    Ready,
    Cooldown { remaining_ms: i64 },
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, ts_rs::TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum CombatTarget {
    #[default]
    None,
    Monster {
        monster_map_id: i64,
    },
    Player {
        entity_id: i64,
        alive: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct ConsumableReadiness {
    pub item_id: i64,
    pub ready: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct CombatMonsterState {
    pub monster_map_id: i64,
    pub generation: u64,
    pub priority_rank: Option<usize>,
    pub attack_blocked: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct CombatSnapshot {
    pub observed_at_ms: f64,
    pub skills: [SkillCooldown; 6],
    pub selected_target: CombatTarget,
    pub consumable: Option<ConsumableReadiness>,
    pub available_monster_ids: Vec<i64>,
}
