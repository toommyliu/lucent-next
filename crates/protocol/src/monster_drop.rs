use serde::{Serialize, Serializer, ser::SerializeStruct};
use ts_rs::TS;

use crate::item::ItemData;

#[derive(Clone, Debug, PartialEq, TS)]
#[ts(rename_all = "camelCase")]
pub struct MonsterDrop {
    pub event_drop: bool,
    pub icon: String,
    #[ts(as = "crate::item::ItemSnapshot")]
    pub item: ItemData,
    pub quest_gated: bool,
    pub quest_objectives: Vec<String>,
    pub rarity: f64,
    pub rarity_name: String,
    pub rate_boost_percent: Option<f64>,
    pub rate_percent: Option<f64>,
    pub required_quest_ids: Vec<i64>,
    pub required_quests: Vec<String>,
    pub stack_size: f64,
    pub variable_quantity: bool,
}

impl Serialize for MonsterDrop {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut fields = serializer.serialize_struct("MonsterDrop", 13)?;
        fields.serialize_field("eventDrop", &self.event_drop)?;
        fields.serialize_field("icon", &self.icon)?;
        fields.serialize_field("item", &self.item.snapshot())?;
        fields.serialize_field("questGated", &self.quest_gated)?;
        fields.serialize_field("questObjectives", &self.quest_objectives)?;
        fields.serialize_field("rarity", &self.rarity)?;
        fields.serialize_field("rarityName", &self.rarity_name)?;
        fields.serialize_field("rateBoostPercent", &self.rate_boost_percent)?;
        fields.serialize_field("ratePercent", &self.rate_percent)?;
        fields.serialize_field("requiredQuestIds", &self.required_quest_ids)?;
        fields.serialize_field("requiredQuests", &self.required_quests)?;
        fields.serialize_field("stackSize", &self.stack_size)?;
        fields.serialize_field("variableQuantity", &self.variable_quantity)?;
        fields.end()
    }
}
