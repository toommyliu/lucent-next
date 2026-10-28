use serde::{Deserialize, Serialize};

use crate::event::Point;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ts_rs::TS)]
#[ts(type = "0 | 1 | 2")]
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
    #[ts(type = "0 | 1 | 2")]
    pub state: EntityState,
}

impl EntityData {
    pub fn alive(&self) -> bool {
        self.hp > 0.0 && self.state != EntityState::Dead
    }
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
