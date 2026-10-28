use serde::{Deserialize, Serialize};

use crate::{aura::AuraData, event::Point, monster_drop::MonsterDrop};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ts_rs::TS)]
#[ts(type = "typeof EntityState[keyof typeof EntityState]")]
pub enum EntityState {
    Dead,
    #[default]
    Idle,
    InCombat,
}

impl Serialize for EntityState {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_i64(i64::from(*self))
    }
}
impl<'de> Deserialize<'de> for EntityState {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::try_from(i64::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

impl TryFrom<i64> for EntityState {
    type Error = &'static str;
    fn try_from(value: i64) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Dead),
            1 => Ok(Self::Idle),
            2 => Ok(Self::InCombat),
            _ => Err("Unknown entity state"),
        }
    }
}
impl From<EntityState> for i64 {
    fn from(value: EntityState) -> Self {
        match value {
            EntityState::Dead => 0,
            EntityState::Idle => 1,
            EntityState::InCombat => 2,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct EntityData {
    pub cell: String,
    #[ts(type = "number")]
    pub hp: f64,
    #[ts(type = "number")]
    pub max_hp: f64,
    #[ts(type = "number")]
    pub max_mp: f64,
    #[ts(type = "number")]
    pub mp: f64,
    pub state: EntityState,
}

impl EntityData {
    pub fn alive(&self) -> bool {
        self.hp > 0.0 && self.state != EntityState::Dead
    }

    pub fn snapshot_fields(&self, auras: Vec<AuraData>) -> EntitySnapshotFields {
        EntitySnapshotFields {
            alive: self.alive(),
            auras,
            dead: !self.alive(),
            hp_percent: if self.max_hp <= 0.0 {
                0.0
            } else {
                self.hp / self.max_hp * 100.0
            },
            idle: self.state == EntityState::Idle,
            in_combat: self.state == EntityState::InCombat,
            mp_percent: if self.max_mp <= 0.0 {
                0.0
            } else {
                self.mp / self.max_mp * 100.0
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct EntitySnapshotFields {
    pub alive: bool,
    pub auras: Vec<AuraData>,
    pub dead: bool,
    pub hp_percent: f64,
    pub idle: bool,
    pub in_combat: bool,
    pub mp_percent: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, ts_rs::TS)]
pub struct EntitySnapshot {
    #[serde(flatten)]
    pub data: EntityData,
    #[serde(flatten)]
    pub fields: EntitySnapshotFields,
}

#[derive(Clone, Debug, PartialEq, Serialize, ts_rs::TS)]
pub struct PlayerSnapshot {
    #[serde(flatten)]
    pub data: PlayerData,
    #[serde(flatten)]
    pub fields: EntitySnapshotFields,
}

#[derive(Clone, Debug, PartialEq, Serialize, ts_rs::TS)]
pub struct MonsterSnapshot {
    #[serde(flatten)]
    pub data: MonsterData,
    #[serde(flatten)]
    pub fields: EntitySnapshotFields,
    pub drops: Vec<MonsterDrop>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct PlayerData {
    #[serde(flatten)]
    pub entity: EntityData,
    pub afk: bool,
    #[ts(type = "number")]
    pub entity_id: i64,
    pub entity_type: String,
    #[ts(type = "number")]
    pub level: f64,
    pub pad: String,
    pub position: Point,
    pub username: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct MonsterData {
    #[serde(flatten)]
    pub entity: EntityData,
    pub aggressive: bool,
    #[ts(type = "number")]
    pub level: f64,
    #[ts(type = "number")]
    pub monster_id: i64,
    #[ts(type = "number")]
    pub monster_map_id: i64,
    pub name: String,
    pub race: String,
}

#[derive(Clone, Debug, Default)]
pub struct EntityPatch {
    pub afk: Option<bool>,
    pub entity_id: Option<i64>,
    pub hp: Option<f64>,
    pub max_hp: Option<f64>,
    pub level: Option<f64>,
    pub mp: Option<f64>,
    pub max_mp: Option<f64>,
    pub state: Option<EntityState>,
    pub px: Option<f64>,
    pub py: Option<f64>,
    pub cell: Option<String>,
    pub pad: Option<String>,
    pub tx: Option<f64>,
    pub ty: Option<f64>,
}
