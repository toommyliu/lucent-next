use serde::{Deserialize, Serialize};

use crate::item::ItemSnapshot;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct MapState {
    pub id: i64,
    pub name: String,
    pub room_number: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "lowercase")]
pub enum EntityKind {
    Player,
    Monster,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "lowercase")]
pub enum AuraKind {
    Active,
    Passive,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "lowercase")]
pub enum CounterSource {
    Aura,
    Message,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "lowercase")]
pub enum MessageSource {
    Animation,
    Aura,
}

#[derive(Clone, Debug, PartialEq, Serialize, ts_rs::TS)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum LocationChange {
    Cell,
    Position,
    Walk { destination: Point },
}

#[derive(Clone, Debug, PartialEq, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct AuraEvent {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub duration: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub icon: Option<String>,
    pub kind: AuraKind,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub source_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub source_type: Option<EntityKind>,
    pub target_id: i64,
    pub target_type: EntityKind,
}

#[derive(Clone, Debug, PartialEq, Serialize, ts_rs::TS)]
#[serde(
    tag = "type",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum ProjectionEvent {
    Login,
    Logout,
    PlayerAfk {
        afk: bool,
        entity_id: i64,
        username: String,
    },
    JoinMap {
        map: MapState,
    },
    ItemDrop {
        item: Box<ItemSnapshot>,
    },
    QuestComplete {
        quest_id: i64,
    },
    MonsterDeath {
        monster_map_id: i64,
    },
    MonsterRespawn {
        monster_map_id: i64,
    },
    PlayerDeath {
        entity_id: i64,
        username: String,
    },
    PlayersChanged,
    AuraAdded {
        #[serde(flatten)]
        aura: AuraEvent,
        stack: f64,
    },
    AuraUpdated {
        #[serde(flatten)]
        aura: AuraEvent,
        stack: f64,
    },
    AuraRemoved {
        #[serde(flatten)]
        aura: AuraEvent,
    },
    CounterAttackStart {
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        duration_ms: Option<f64>,
        monster_map_id: i64,
        source: CounterSource,
        trigger_id: String,
        trigger_text: String,
    },
    CounterAttackEnd {
        monster_map_id: i64,
        source: CounterSource,
        trigger_id: String,
        trigger_text: String,
    },
    PlayerLocation {
        cell: String,
        entity_id: i64,
        pad: String,
        position: Point,
        username: String,
        #[serde(flatten)]
        change: LocationChange,
    },
    UpdateMessage {
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        animation: Option<String>,
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        monster_map_id: Option<i64>,
        source: MessageSource,
    },
    Zone {
        map: String,
        zone: String,
    },
}
