use indexmap::IndexSet;
use lucent_protocol::{
    Packet, PacketDirection,
    item::{ItemContainer, ItemContext},
    monster_drop::MonsterDrop,
};
use serde_json::{Map, Value};

use super::{
    ProjectionOutput, decode_item,
    quests::js_entries,
    wire::{trim_js, wire_bool, wire_i64, wire_int, wire_number},
};

#[derive(Debug)]
pub struct MonsterDropResponse {
    pub monster_map_id: Option<i64>,
    pub drops: Vec<MonsterDrop>,
    pub output: ProjectionOutput,
}

fn optional<'a, T>(
    object: &'a Map<String, Value>,
    key: &str,
    decode: impl FnOnce(&'a Value) -> Option<T>,
) -> Option<Option<T>> {
    object
        .get(key)
        .map_or(Some(None), |value| decode(value).map(Some))
}
fn strings(value: &Value) -> Option<Vec<String>> {
    value
        .as_array()?
        .iter()
        .map(|value| value.as_str().map(str::to_owned))
        .collect()
}

pub fn decode_monster_drop(value: &Value) -> Option<MonsterDrop> {
    let object = value.as_object()?;
    let stack = optional(object, "iStk", wire_int)?;
    let mut normalized = object.clone();
    normalized.remove("iStk");
    let mut item = decode_item(&Value::Object(normalized), ItemContainer::Inventory)?;
    item.max_stack = stack;
    if item.context != ItemContext::Bank {
        item.context = ItemContext::MonsterDrop;
    }
    let event_drop = optional(object, "bEventDrop", wire_bool)?.unwrap_or(false);
    let quest_gated = optional(object, "bQuestGated", wire_bool)?.unwrap_or(false);
    let variable_quantity = optional(object, "bVaryQty", wire_bool)?.unwrap_or(false);
    let rate = optional(object, "iRate", wire_number)?;
    let boosted = optional(object, "iRateBoosted", wire_number)?;
    let rarity = optional(object, "iRty", wire_int)?.unwrap_or(0.0);
    let required_quests = optional(object, "questGated", strings)?.unwrap_or_default();
    let quest_objectives = optional(object, "questObjective", strings)?.unwrap_or_default();
    let icon = optional(object, "sIcon", Value::as_str)?
        .unwrap_or("")
        .to_owned();
    let required = optional(object, "sReqQuests", Value::as_str)?.unwrap_or("");
    let required_quest_ids: IndexSet<i64> = required
        .split(',')
        .filter_map(|part| {
            let value = wire_number(&Value::String(trim_js(part).to_owned()))?;
            (value.fract() == 0.0 && value > 0.0 && value <= 9_007_199_254_740_991.0)
                .then_some(value as i64)
        })
        .collect();
    let divisor = if variable_quantity {
        item.quantity.max(1.0)
    } else {
        1.0
    };
    Some(MonsterDrop {
        event_drop,
        icon,
        item,
        quest_gated: quest_gated || !required_quests.is_empty(),
        quest_objectives,
        rarity,
        rarity_name: rarity_name(rarity).into(),
        rate_boost_percent: boosted.map(|rate| (rate / divisor).min(100.0)),
        rate_percent: rate.map(|rate| (rate / divisor).min(100.0)),
        required_quest_ids: required_quest_ids.into_iter().collect(),
        required_quests,
        stack_size: stack.unwrap_or(1.0).max(1.0),
        variable_quantity,
    })
}

pub fn decode_monster_drop_response(packet: &Packet) -> Option<MonsterDropResponse> {
    if packet.direction() != PacketDirection::Extension || packet.command() != "monsterDrops" {
        return None;
    }
    let output = ProjectionOutput::default();
    let data = packet.data().unwrap();
    let decoded = data.as_object().and_then(|object| {
        let id = wire_i64(object.get("MonMapID")?).filter(|id| *id > 0)?;
        Some((id, object.get("items")?.as_object()?))
    });
    let Some((monster_map_id, entries)) = decoded else {
        tracing::warn!(command = "monsterDrops", "invalid projection input");
        return Some(MonsterDropResponse {
            monster_map_id: None,
            drops: Vec::new(),
            output,
        });
    };
    let mut drops = Vec::new();
    let mut invalid = 0;
    for (_, value) in js_entries(entries) {
        if let Some(drop) = decode_monster_drop(value) {
            drops.push(drop);
        } else {
            invalid += 1;
        }
    }
    if invalid > 0 {
        tracing::warn!(
            command = "monsterDrops",
            rejected = invalid,
            "projection discarded invalid entries"
        );
    }
    Some(MonsterDropResponse {
        monster_map_id: Some(monster_map_id),
        drops,
        output,
    })
}

fn rarity_name(rarity: f64) -> &'static str {
    match rarity as i64 {
        1 => "Enhancement +0",
        2 => "Enhancement +1",
        3 => "Enhancement +2",
        4 => "Enhancement +3",
        5 => "Enhancement +4",
        6 => "Enhancement +5",
        7 => "Enhancement +6",
        8 => "Enhancement +100",
        9 => "Enhancement +200",
        11 => "Common",
        12 => "Weird",
        13 => "Awesome",
        14 => "1% Drop",
        15 => "5% Drop",
        16 => "Boss Drop",
        17 => "Hidden Secret",
        18 => "Junk",
        19 => "Impossible",
        20 => "Artifact",
        21 => "Limited Time Drop",
        23 => "Crazy",
        24 => "Expensive",
        25 => "Placeholder",
        30 => "Rare",
        35 => "Epic",
        36 => "Champion",
        40 => "Verification Shop",
        50 => "Seasonal",
        55 => "Seasonal Rare",
        60 => "Event",
        65 => "Event Rare",
        67 => "Infinity",
        68 => "New Collection Chest",
        70 => "Limited Rare",
        75 => "Collector",
        80 => "Promo",
        81 => "Founder",
        88 => "Benevolent",
        90 => "Ultra Rare",
        91 => "Achievement Tracker",
        92 => "Upgrade Pack",
        93 => "Limited Quantity",
        94 => "Frostval Gifting",
        95 => "Super Ultra Rare",
        96 => "Kickstarter Backer",
        99 => "Custom Item",
        100 => "Legendary",
        _ => "Unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lucent_protocol::PacketEncoding;
    use serde_json::json;

    #[test]
    fn displayed_rates_and_quest_metadata_follow_variable_quantity_rules() {
        let drop = decode_monster_drop(&json!({"ItemID":1,"iQty":4,"iStk":0,"bVaryQty":true,"iRate":300,"iRateBoosted":600,"iRty":30,"questGated":["Fixture quest"],"questObjective":["Collect"],"sReqQuests":"2, 1,2,0,1.5,0x3,9007199254740992"})).unwrap();
        assert_eq!(drop.rate_percent, Some(75.0));
        assert_eq!(drop.rate_boost_percent, Some(100.0));
        assert_eq!(drop.required_quest_ids, vec![2, 1, 3]);
        assert!(drop.quest_gated);
        assert_eq!(drop.rarity_name, "Rare");
        assert_eq!(drop.stack_size, 1.0);
        assert_eq!(drop.item.max_stack, Some(0.0));
        assert_eq!(drop.item.context, ItemContext::MonsterDrop);
    }

    #[test]
    fn response_preserves_numeric_key_order_and_valid_siblings() {
        let packet = Packet::Extension {
            command: "monsterDrops".into(),
            data: json!({"MonMapID":7,"items":{"9":{"ItemID":9},"2":{"ItemID":2},"bad":{"ItemID":0},"last":{"ItemID":3}}}),
            raw: String::new(),
            encoding: PacketEncoding::Json,
        };
        let result = decode_monster_drop_response(&packet).unwrap();
        assert_eq!(result.monster_map_id, Some(7));
        assert_eq!(
            result
                .drops
                .iter()
                .map(|drop| drop.item.item_id)
                .collect::<Vec<_>>(),
            vec![2, 9, 3]
        );
    }

    #[test]
    fn absent_rates_remain_null_and_optional_field_errors_reject_entries() {
        let drop = decode_monster_drop(&json!({"ItemID":1,"iStk":-3})).unwrap();
        assert_eq!(drop.rate_percent, None);
        assert_eq!(drop.rate_boost_percent, None);
        assert_eq!(drop.stack_size, 1.0);
        assert_eq!(drop.item.max_stack, Some(-3.0));
        assert!(decode_monster_drop(&json!({"ItemID":1,"questGated":null})).is_none());
        assert!(decode_monster_drop(&json!({"ItemID":1,"bVaryQty":"TRUE"})).is_none());
    }
}
