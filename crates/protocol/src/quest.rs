use serde::Serialize;
use ts_rs::TS;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum QuestCadence {
    Daily,
    Monthly,
    None,
    Weekly,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(optional_fields)]
pub struct QuestItem {
    pub item_id: i64,
    pub name: String,
    pub quantity: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_stack: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temporary_item: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(optional_fields)]
pub struct QuestReward {
    #[serde(flatten)]
    pub item: QuestItem,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drop_chance: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct QuestData {
    pub cadence: QuestCadence,
    pub id: f64,
    pub name: String,
    pub once: bool,
    pub requirements: Vec<QuestItem>,
    pub rewards: Vec<QuestReward>,
}
