use std::sync::{Arc, RwLock, RwLockReadGuard};

use indexmap::{IndexMap, IndexSet};
use lucent_protocol::quest::{QuestCadence, QuestData, QuestItem, QuestReward};
use lucent_protocol::{Packet, PacketDirection, event::ProjectionEvent};
use serde_json::{Map, Value};

use super::{
    ProjectionOutput,
    wire::{wire_bool, wire_i64, wire_number},
};

#[derive(Clone, Debug)]
pub struct LiveQuest(Arc<RwLock<QuestData>>);

impl LiveQuest {
    pub fn data(&self) -> RwLockReadGuard<'_, QuestData> {
        self.0.read().unwrap()
    }
    pub fn same_instance(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Debug, Default)]
pub struct QuestStore {
    quests: IndexMap<u64, LiveQuest>,
    accepted: IndexSet<u64>,
}

impl QuestStore {
    pub fn clear(&mut self) {
        self.quests.clear();
        self.accepted.clear();
    }
    pub fn get(&self, id: f64) -> Option<LiveQuest> {
        self.quests.get(&id.to_bits()).cloned()
    }
    pub fn all(&self) -> Vec<LiveQuest> {
        self.quests.values().cloned().collect()
    }
    pub fn accepted(&self) -> Vec<LiveQuest> {
        self.accepted
            .iter()
            .filter_map(|id| self.quests.get(id).cloned())
            .collect()
    }
    pub fn accepted_ids(&self) -> Vec<f64> {
        self.accepted.iter().map(|id| f64::from_bits(*id)).collect()
    }
    pub fn set_accepted(&mut self, ids: &[f64]) {
        self.accepted = ids.iter().map(|id| id.to_bits()).collect();
    }
    pub fn upsert(&mut self, incoming: QuestData) -> LiveQuest {
        let key = incoming.id.to_bits();
        if let Some(current) = self.quests.get(&key) {
            *current.0.write().unwrap() = incoming;
            current.clone()
        } else {
            let model = LiveQuest(Arc::new(RwLock::new(incoming)));
            self.quests.insert(key, model.clone());
            model
        }
    }
}

pub(crate) fn js_entries(object: &Map<String, Value>) -> Vec<(&String, &Value)> {
    let mut entries: Vec<_> = object.iter().collect();
    entries.sort_by_key(|(key, _)| {
        let index = key
            .parse::<u32>()
            .ok()
            .filter(|index| *index != u32::MAX && index.to_string() == **key);
        (index.is_none(), index.unwrap_or(0))
    });
    entries
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
fn positive(value: &Value) -> Option<i64> {
    wire_i64(value).filter(|value| *value > 0)
}

#[derive(Clone, Default)]
struct ItemPayload {
    item_id: i64,
    name: Option<String>,
    quantity: Option<i64>,
    max_stack: Option<i64>,
    temporary: Option<bool>,
    drop_chance: Option<f64>,
    rate: Option<f64>,
}

impl ItemPayload {
    fn merge(&mut self, newer: Self) {
        if newer.name.is_some() {
            self.name = newer.name;
        }
        if newer.quantity.is_some() {
            self.quantity = newer.quantity;
        }
        if newer.max_stack.is_some() {
            self.max_stack = newer.max_stack;
        }
        if newer.temporary.is_some() {
            self.temporary = newer.temporary;
        }
        if newer.drop_chance.is_some() {
            self.drop_chance = newer.drop_chance;
        }
        if newer.rate.is_some() {
            self.rate = newer.rate;
        }
    }
    fn item(&self, quantity_override: Option<i64>) -> QuestItem {
        QuestItem {
            item_id: self.item_id,
            name: self
                .name
                .clone()
                .unwrap_or_else(|| format!("Item {}", self.item_id)),
            quantity: quantity_override.or(self.quantity).unwrap_or(1).max(1),
            max_stack: self.max_stack,
            temporary_item: self.temporary,
        }
    }
}

fn decode_quest_item(value: &Value) -> Option<ItemPayload> {
    let object = value.as_object()?;
    Some(ItemPayload {
        item_id: positive(object.get("ItemID")?)?,
        name: optional(object, "sName", |value| value.as_str().map(str::to_owned))?,
        quantity: optional(object, "iQty", wire_i64)?,
        max_stack: optional(object, "iStk", positive)?,
        temporary: optional(object, "bTemp", wire_bool)?,
        drop_chance: optional(object, "DropChance", wire_number)?,
        rate: optional(object, "iRate", wire_number)?,
    })
}

fn collect_items(value: &Value, items: &mut IndexMap<i64, ItemPayload>) {
    if let Some(incoming) = decode_quest_item(value) {
        if let Some(current) = items.get_mut(&incoming.item_id) {
            current.merge(incoming);
        } else {
            items.insert(incoming.item_id, incoming);
        }
    } else if let Some(array) = value.as_array() {
        for value in array {
            collect_items(value, items);
        }
    } else if let Some(object) = value.as_object() {
        for (_, value) in js_entries(object) {
            collect_items(value, items);
        }
    }
}

pub fn decode_quest(id: f64, value: &Value) -> Option<QuestData> {
    let object = value.as_object()?;
    optional(object, "QuestID", positive)?;
    for key in ["Rewards", "Rewards2", "RequiredItems", "reward", "turnin"] {
        optional(object, key, Value::as_array)?;
    }
    for key in ["oItems", "oReqd", "oRewards"] {
        optional(object, key, Value::as_object)?;
    }
    let once = optional(object, "bOnce", wire_bool)?.unwrap_or(false);
    let field = optional(object, "sField", Value::as_str)?;
    let name = optional(object, "sName", Value::as_str)?
        .map_or_else(|| format!("Quest {id}"), str::to_owned);
    let mut turnin = IndexMap::new();
    let mut rates = IndexMap::new();
    let mut requirements = IndexMap::new();
    let mut rewards = IndexMap::new();
    if let Some(value) = object.get("turnin") {
        collect_items(value, &mut turnin);
    }
    if let Some(value) = object.get("reward") {
        collect_items(value, &mut rates);
    }
    if let Some(value) = object.get("RequiredItems") {
        collect_items(value, &mut requirements);
    }
    if let Some(value) = object.get("oItems").or_else(|| object.get("oReqd")) {
        collect_items(value, &mut requirements);
    }
    if let Some(value) = object.get("Rewards").or_else(|| object.get("Rewards2")) {
        collect_items(value, &mut rewards);
    }
    if let Some(value) = object.get("oRewards") {
        collect_items(value, &mut rewards);
    }
    Some(QuestData {
        cadence: match field {
            Some("id0") => QuestCadence::Daily,
            Some("iw0") => QuestCadence::Weekly,
            Some("im0") => QuestCadence::Monthly,
            _ => QuestCadence::None,
        },
        id,
        name,
        once,
        requirements: requirements
            .into_values()
            .map(|item| {
                let quantity = turnin.get(&item.item_id).and_then(|value| value.quantity);
                let mut merged = turnin.get(&item.item_id).cloned().unwrap_or_default();
                merged.item_id = item.item_id;
                merged.merge(item);
                merged.item(quantity)
            })
            .collect(),
        rewards: rewards
            .into_values()
            .map(|item| QuestReward {
                drop_chance: rates
                    .get(&item.item_id)
                    .and_then(|value| value.rate)
                    .or(item.drop_chance)
                    .or(item.rate),
                item: item.item(None),
            })
            .collect(),
    })
}

pub fn project_quests(store: &mut QuestStore, packet: &Packet) -> Option<ProjectionOutput> {
    if packet.direction() != PacketDirection::Extension
        || !matches!(packet.command(), "getQuests" | "getQuests2" | "ccqr")
    {
        return None;
    }
    let mut output = ProjectionOutput::default();
    let data = packet.data().unwrap();
    if packet.command() == "ccqr" {
        let decoded = data.as_object().and_then(|object| {
            let id = optional(object, "QuestID", positive)?;
            Some((id.unwrap_or(0), wire_bool(object.get("bSuccess")?)?))
        });
        match decoded {
            None => tracing::warn!(command = "ccqr", "invalid projection input"),
            Some((quest_id, true)) => output
                .events
                .push(ProjectionEvent::QuestComplete { quest_id }),
            Some((_, false)) => {}
        }
        return Some(output);
    }
    let Some(quests) = data.get("quests").and_then(Value::as_object) else {
        tracing::warn!(command = packet.command(), "invalid projection input");
        return Some(output);
    };
    let mut invalid = 0;
    for (raw_id, value) in js_entries(quests) {
        let id = wire_number(&Value::String(raw_id.clone()));
        if let Some(quest) = id
            .filter(|id| id.fract() == 0.0 && *id > 0.0)
            .and_then(|id| decode_quest(id, value))
        {
            store.upsert(quest);
        } else {
            invalid += 1;
        }
    }
    if invalid > 0 {
        tracing::warn!(
            command = packet.command(),
            rejected = invalid,
            "projection discarded invalid entries"
        );
    }
    Some(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn metadata_merges_nested_requirements_and_rate_overrides_in_js_order() {
        let quest = decode_quest(
            7.0,
            &json!({
                "sField":"id0", "bOnce":"true", "sName":"Fixture quest",
                "RequiredItems":[{"ItemID":5,"iQty":1,"sName":"Token"}],
                "oItems":{"9":{"ItemID":9},"2":{"ItemID":2},"nested":{"ItemID":5,"iStk":20}},
                "turnin":[{"ItemID":5,"iQty":4,"bTemp":true}],
                "Rewards":[{"ItemID":6,"DropChance":10,"iQty":0}],
                "reward":[{"ItemID":6,"iRate":25}]
            }),
        )
        .unwrap();
        assert_eq!(quest.cadence, QuestCadence::Daily);
        assert!(quest.once);
        assert_eq!(
            quest
                .requirements
                .iter()
                .map(|item| item.item_id)
                .collect::<Vec<_>>(),
            vec![5, 2, 9]
        );
        assert_eq!(
            quest.requirements[0],
            QuestItem {
                item_id: 5,
                name: "Token".into(),
                quantity: 4,
                max_stack: Some(20),
                temporary_item: Some(true)
            }
        );
        assert_eq!(quest.rewards[0].drop_chance, Some(25.0));
        assert_eq!(quest.rewards[0].item.quantity, 1);
    }

    #[test]
    fn accepted_quest_order_filters_missing_metadata_and_updates_held_models() {
        let mut store = QuestStore::default();
        store.set_accepted(&[2.0, 1.0, 2.0, 3.0]);
        let held = store.upsert(decode_quest(1.0, &json!({"sName":"Before"})).unwrap());
        store.upsert(decode_quest(2.0, &json!({})).unwrap());
        assert_eq!(
            store
                .accepted()
                .iter()
                .map(|quest| quest.data().id)
                .collect::<Vec<_>>(),
            vec![2.0, 1.0]
        );
        store.upsert(decode_quest(1.0, &json!({"sName":"After"})).unwrap());
        assert!(held.same_instance(&store.get(1.0).unwrap()));
        assert_eq!(held.data().name, "After");
        store.clear();
        store.upsert(decode_quest(1.0, &json!({"sName":"New"})).unwrap());
        assert!(!held.same_instance(&store.get(1.0).unwrap()));
        assert_eq!(held.data().name, "After");
        assert!(store.accepted().is_empty());
    }

    #[test]
    fn quest_packets_merge_numeric_keys_and_publish_only_successful_completions() {
        use lucent_protocol::PacketEncoding;
        let extension = |command: &str, data| Packet::Extension {
            command: command.into(),
            data,
            raw: String::new(),
            encoding: PacketEncoding::Json,
        };
        let mut store = QuestStore::default();
        let output = project_quests(&mut store, &extension("getQuests", json!({"quests":{"2":{},"bad":{},"1":{"sName":"First"},"1.0":{"sName":"Updated"},"3":{}}}))).unwrap();
        assert!(output.events.is_empty());
        assert_eq!(
            store
                .all()
                .iter()
                .map(|quest| quest.data().id)
                .collect::<Vec<_>>(),
            vec![1.0, 2.0, 3.0]
        );
        assert_eq!(store.get(1.0).unwrap().data().name, "Updated");
        assert_eq!(store.get(2.0).unwrap().data().name, "Quest 2");

        let output =
            project_quests(&mut store, &extension("ccqr", json!({"bSuccess":"1"}))).unwrap();
        assert_eq!(
            output.events,
            vec![ProjectionEvent::QuestComplete { quest_id: 0 }]
        );
        assert!(
            project_quests(&mut store, &extension("ccqr", json!({"bSuccess":false})))
                .unwrap()
                .events
                .is_empty()
        );
        let invalid = project_quests(
            &mut store,
            &extension("ccqr", json!({"bSuccess":true,"QuestID":0})),
        )
        .unwrap();
        assert!(invalid.events.is_empty());

        let invalid =
            project_quests(&mut store, &extension("getQuests2", json!({"quests":null}))).unwrap();
        assert!(invalid.events.is_empty());

        assert_eq!(store.all().len(), 3);
    }

    #[test]
    fn invalid_optional_metadata_is_rejected_but_invalid_nested_items_are_skipped() {
        assert!(decode_quest(1.0, &json!({"Rewards":null})).is_none());
        let quest = decode_quest(
            1.0,
            &json!({"RequiredItems":[{"ItemID":0},{"ItemID":2,"sName":null},{"ItemID":3}]}),
        )
        .unwrap();
        assert_eq!(
            quest
                .requirements
                .iter()
                .map(|item| item.item_id)
                .collect::<Vec<_>>(),
            vec![3]
        );
    }
}
