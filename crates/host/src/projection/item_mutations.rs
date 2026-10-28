use lucent_protocol::event::ProjectionEvent;
use lucent_protocol::item::{Enhancement, ItemContainer, ItemContext, ItemData};
use lucent_protocol::{Packet, PacketDirection};
use serde_json::{Map, Value};

use super::ProjectionOutput;
use super::item::{decode_item, decode_sparse_item};
use super::items::ItemStore;
use super::wire::{trim_js, wire_bool, wire_i64, wire_int, wire_number};

struct Record<'a>(&'a Map<String, Value>);

impl<'a> Record<'a> {
    fn new(value: &'a Value) -> Option<Self> {
        value.as_object().map(Self)
    }
    fn optional<T>(
        &self,
        key: &str,
        decode: impl FnOnce(&Value) -> Option<T>,
    ) -> Option<Option<T>> {
        self.0
            .get(key)
            .map_or(Some(None), |value| decode(value).map(Some))
    }
    fn quantity(&self, key: &str) -> Option<Option<f64>> {
        self.optional(key, wire_int)
    }
    fn positive(&self, key: &str) -> Option<i64> {
        positive(self.0.get(key)?)
    }
    fn boolean(&self, key: &str) -> Option<Option<bool>> {
        self.optional(key, wire_bool)
    }
    fn text(&self, key: &str) -> Option<Option<&'a str>> {
        match self.0.get(key) {
            None => Some(None),
            Some(value) => value.as_str().map(Some),
        }
    }
}

fn positive(value: &Value) -> Option<i64> {
    wire_i64(value).filter(|value| *value > 0)
}
fn nonnegative(value: &Value) -> Option<f64> {
    wire_int(value).filter(|value| *value >= 0.0)
}

pub(crate) fn js_entries(object: &Map<String, Value>) -> Vec<(&str, &Value)> {
    let mut indexed = Vec::new();
    let mut named = Vec::new();
    for (key, value) in object {
        match key
            .parse::<u32>()
            .ok()
            .filter(|index| *index != u32::MAX && index.to_string() == *key)
        {
            Some(index) => indexed.push((index, key.as_str(), value)),
            None => named.push((key.as_str(), value)),
        }
    }
    indexed.sort_unstable_by_key(|entry| entry.0);
    indexed
        .into_iter()
        .map(|(_, key, value)| (key, value))
        .chain(named)
        .collect()
}

struct ItemMutation {
    id: i64,
    bank: Option<bool>,
    house: Option<bool>,
    success: Option<bool>,
    quantity: Option<f64>,
    quantity_now: Option<f64>,
}

fn mutation(data: &Value) -> Option<ItemMutation> {
    let data = Record::new(data)?;
    Some(ItemMutation {
        id: data.positive("ItemID")?,
        bank: data.boolean("bBank")?,
        house: data.boolean("bHouse")?,
        success: data.boolean("bSuccess")?,
        quantity: data.quantity("iQty")?,
        quantity_now: data.quantity("iQtyNow")?,
    })
}

fn equip_house(store: &ItemStore, id: i64) {
    let Some(selected) = store.get(ItemContainer::House, id) else {
        return;
    };
    if selected.data().category != "House" {
        return;
    }
    for item in store.all(ItemContainer::House) {
        item.update(|item| {
            if item.category == "House" {
                item.equipped = item.item_id == id;
            }
        });
    }
}

fn deposit(store: &mut ItemStore, id: i64) {
    let Some(item) = store
        .remove(ItemContainer::Inventory, id)
        .or_else(|| store.remove(ItemContainer::House, id))
    else {
        return;
    };
    item.update(|item| item.context = ItemContext::Bank);
    store.upsert_model(ItemContainer::Bank, item);
}

fn withdraw(store: &mut ItemStore, id: i64) {
    let Some(item) = store.remove(ItemContainer::Bank, id) else {
        return;
    };
    let target = if item.data().house_item {
        ItemContainer::House
    } else {
        ItemContainer::Inventory
    };
    item.update(|item| item.context = target.into());
    store.upsert_model(target, item);
}

fn find_owned(store: &ItemStore, id: i64) -> Option<super::LiveItem> {
    [
        ItemContainer::Temporary,
        ItemContainer::Inventory,
        ItemContainer::House,
        ItemContainer::Bank,
    ]
    .into_iter()
    .find_map(|container| store.get(container, id))
}

fn owned_container(item: &ItemData) -> ItemContainer {
    if item.context == ItemContext::Bank {
        ItemContainer::Bank
    } else if item.house_item {
        ItemContainer::House
    } else if item.temporary_item {
        ItemContainer::Temporary
    } else {
        ItemContainer::Inventory
    }
}

fn purchase(store: &mut ItemStore, value: &Value) {
    let Some(record) = Record::new(value) else {
        tracing::warn!(command = "buyItem", "invalid projection input");
        return;
    };
    let Some(success) = record.0.get("bitSuccess").and_then(wire_bool) else {
        tracing::warn!(command = "buyItem", "invalid projection input");
        return;
    };
    if !success {
        return;
    }
    let decoded = (|| {
        Some((
            record.positive("CharItemID")?,
            record.positive("ItemID")?,
            wire_bool(record.0.get("bBank")?)?,
            record.positive("iQty")?,
        ))
    })();
    let Some((char_id, id, bank, quantity)) = decoded else {
        tracing::warn!(command = "buyItem", "invalid projection input");
        return;
    };
    let Some(shop_item) = store.get(ItemContainer::Shop, id) else {
        return;
    };
    let mut data = shop_item.data().clone();
    let target = if bank && data.category != "Class" {
        ItemContainer::Bank
    } else if data.house_item {
        ItemContainer::House
    } else {
        ItemContainer::Inventory
    };
    let current = store.get(target, id);
    let auto_equip = target == ItemContainer::House
        && data.category == "House"
        && !store.all(ItemContainer::House).iter().any(|item| {
            let item = item.data();
            item.category == "House" && item.equipped
        });
    data.char_item_id = Some(char_id);
    data.context = target.into();
    data.equipped = current
        .as_ref()
        .map_or(auto_equip, |item| item.data().equipped);
    data.quantity = current.as_ref().map_or(0.0, |item| item.data().quantity) + quantity as f64;
    data.worn = current.as_ref().is_some_and(|item| item.data().worn);
    store.upsert(target, data);
}

fn wheel_reward(
    store: &mut ItemStore,
    value: &Value,
    overrides: impl IntoIterator<Item = (&'static str, Value)>,
) {
    let value = match value.as_object() {
        Some(object) => {
            let mut object = object.clone();
            for (key, value) in overrides {
                object.insert(key.into(), value);
            }
            Value::Object(object)
        }
        None => value.clone(),
    };
    let Some(mut reward) = decode_item(&value, ItemContainer::Inventory) else {
        return;
    };
    let target = owned_container(&reward);
    reward.quantity += store
        .get(target, reward.item_id)
        .map_or(0.0, |item| item.data().quantity);
    reward.context = target.into();
    store.upsert(target, reward);
}

pub fn project_items(
    store: &mut ItemStore,
    packet: &Packet,
    self_entity_id: Option<i64>,
) -> Option<ProjectionOutput> {
    let mut output = ProjectionOutput::default();
    let command = packet.command();
    if let Packet::Client { params, .. } = packet {
        if command != "equipItem" {
            return None;
        }
        if let Some(id) = params
            .get(4)
            .and_then(|value| positive(&Value::String(value.clone())))
        {
            equip_house(store, id);
        } else {
            tracing::warn!(command = "equipItem", "invalid projection input");
        }
        return Some(output);
    }
    if packet.direction() != PacketDirection::Extension {
        return None;
    }
    let data = packet.data()?;
    match command {
        "bankFromInv" | "bankToInv" => {
            let Some(decoded) = mutation(data) else {
                tracing::warn!(command = command, "invalid projection input");
                return Some(output);
            };
            if decoded.success != Some(false) {
                if command == "bankFromInv" {
                    deposit(store, decoded.id);
                } else {
                    withdraw(store, decoded.id);
                }
            }
        }
        "bankSwapInv" => {
            let decoded = (|| {
                let record = Record::new(data)?;
                Some((
                    record.positive("invItemID")?,
                    record.positive("bankItemID")?,
                ))
            })();
            let Some((inventory_id, bank_id)) = decoded else {
                tracing::warn!(command = "bankSwapInv", "invalid projection input");
                return Some(output);
            };
            let inventory = store.get(ItemContainer::Inventory, inventory_id);
            let source = if inventory.is_some() {
                ItemContainer::Inventory
            } else {
                ItemContainer::House
            };
            let outgoing = inventory.or_else(|| store.get(ItemContainer::House, inventory_id));
            if let (Some(outgoing), Some(incoming)) =
                (outgoing, store.get(ItemContainer::Bank, bank_id))
            {
                store.remove(source, inventory_id);
                store.remove(ItemContainer::Bank, bank_id);
                outgoing.update(|item| item.context = ItemContext::Bank);
                store.upsert_model(ItemContainer::Bank, outgoing);
                let target = if incoming.data().house_item {
                    ItemContainer::House
                } else {
                    ItemContainer::Inventory
                };
                incoming.update(|item| item.context = target.into());
                store.upsert_model(target, incoming);
            }
        }
        "dropItem" => {
            let Some(items) = data
                .as_object()
                .and_then(|record| record.get("items"))
                .and_then(Value::as_object)
            else {
                tracing::warn!(command = "dropItem", "invalid projection input");
                return Some(output);
            };
            let values = js_entries(items);
            let decoded: Vec<_> = values
                .iter()
                .filter_map(|(_, item)| decode_item(item, ItemContainer::Drop))
                .collect();
            let invalid = values.len() - decoded.len();
            if invalid > 0 {
                tracing::warn!(
                    command = "dropItem",
                    rejected = invalid,
                    "projection discarded invalid entries"
                );
            }
            for item in decoded {
                store.upsert(ItemContainer::Drop, item.clone());
                output.events.push(ProjectionEvent::ItemDrop {
                    item: item.snapshot_data().into(),
                });
            }
        }
        "buyItem" => purchase(store, data),
        "equipItem" | "wearItem" | "unequipItem" | "unwearItem" => {
            let decoded = (|| {
                let record = Record::new(data)?;
                Some((
                    record.positive("ItemID")?,
                    record.text("sES")?,
                    record.text("strES")?,
                    record.boolean("success")?,
                    record.optional("uid", positive)?,
                ))
            })();
            let Some((id, slot, alternate_slot, success, user_id)) = decoded else {
                tracing::warn!(command = command, "invalid projection input");
                return Some(output);
            };
            if success != Some(false) && (user_id.is_none() || user_id == self_entity_id) {
                let wear = command == "wearItem" || command == "unwearItem";
                let activating = command == "equipItem" || command == "wearItem";
                let containers: &[ItemContainer] = if wear {
                    &[ItemContainer::Inventory]
                } else {
                    &[
                        ItemContainer::Inventory,
                        ItemContainer::Temporary,
                        ItemContainer::House,
                    ]
                };
                for container in containers {
                    let Some(item) = store.get(*container, id) else {
                        continue;
                    };
                    if let Some(slot) = slot.or(alternate_slot).filter(|_| activating) {
                        for current in store.all(*container) {
                            current.update(|current| {
                                if current.equipment_slot == slot {
                                    if wear {
                                        current.worn = false;
                                    } else {
                                        current.equipped = false;
                                    }
                                }
                            });
                        }
                    }
                    item.update(|item| {
                        if wear {
                            item.worn = activating
                        } else {
                            item.equipped = activating
                        }
                    });
                    break;
                }
            }
        }
        "enhanceItemShop" => {
            let decoded = (|| {
                let record = Record::new(data)?;
                let ids = record
                    .0
                    .get("ItemIDs")?
                    .as_array()?
                    .iter()
                    .map(positive)
                    .collect::<Option<Vec<_>>>()?;
                let enhancement = Enhancement {
                    dps: record.optional("EnhDPS", wire_number)?,
                    id: record.optional("EnhID", nonnegative)?,
                    level: record.optional("EnhLvl", nonnegative)?,
                    pattern_id: record.optional("EnhPID", nonnegative)?,
                    proc_id: record.optional("ProcID", nonnegative)?,
                    range: record.optional("EnhRng", wire_number)?,
                    rarity: record.optional("EnhRty", wire_number)?,
                };
                Some((ids, enhancement))
            })();
            let Some((ids, enhancement)) = decoded else {
                tracing::warn!(command = "enhanceItemShop", "invalid projection input");
                return Some(output);
            };
            for id in ids {
                if let Some(item) = store.get(ItemContainer::Inventory, id) {
                    item.update(|item| item.enhancement = Some(enhancement.clone()));
                }
            }
        }
        "Wheel" => {
            let decoded = (|| {
                let record = Record::new(data)?;
                Some((
                    record.0.get("dropItems")?.as_object()?,
                    record.optional("CharItemID", positive)?,
                    record.optional("charItem1", positive)?,
                    record.optional("charItem2", positive)?,
                    record.optional("dropQty", nonnegative)?,
                    record.0.get("Item"),
                ))
            })();
            let Some((drops, main_id, first_id, second_id, quantity, main)) = decoded else {
                tracing::warn!(command = "Wheel", "invalid projection input");
                return Some(output);
            };
            for (key, char_id, quantity) in [
                ("18927", first_id, quantity.unwrap_or(1.0)),
                ("19189", second_id, 1.0),
            ] {
                if let Some(value) = drops.get(key) {
                    let mut overrides = vec![("iQty", Value::from(quantity))];
                    if let Some(id) = char_id {
                        overrides.push(("CharItemID", Value::from(id)));
                    }
                    wheel_reward(store, value, overrides);
                }
            }
            if let Some(main) = main {
                let mut overrides = vec![("bBank", Value::Bool(false)), ("iQty", Value::from(1))];
                if let Some(id) = main_id {
                    overrides.push(("CharItemID", Value::from(id)));
                }
                wheel_reward(store, main, overrides);
            }
        }
        "getDrop" => {
            let Some(decoded) = mutation(data) else {
                tracing::warn!(command = "getDrop", "invalid projection input");
                return Some(output);
            };
            if decoded.success == Some(true)
                && let Some(drop) = store.remove(ItemContainer::Drop, decoded.id)
            {
                let mut accepted = drop.data().clone();
                let target = if decoded.bank == Some(true) && accepted.category != "Class" {
                    ItemContainer::Bank
                } else if decoded.house == Some(true) || accepted.house_item {
                    ItemContainer::House
                } else {
                    ItemContainer::Inventory
                };
                accepted.context = target.into();
                accepted.quantity = decoded.quantity_now.unwrap_or_else(|| {
                    store
                        .get(target, decoded.id)
                        .map_or(0.0, |item| item.data().quantity)
                        + decoded.quantity.unwrap_or(accepted.quantity)
                });
                store.upsert(target, accepted);
            }
        }
        "addItems" | "forceAddItem" => {
            let Some(items) = data
                .as_object()
                .and_then(|record| record.get("items"))
                .and_then(Value::as_object)
            else {
                return Some(output);
            };
            for (key, value) in js_entries(items) {
                let mut value = value.clone();
                if let Some(record) = value
                    .as_object_mut()
                    .filter(|record| !record.contains_key("ItemID"))
                    && let Some(id) = strict_positive(key)
                {
                    record.insert("ItemID".into(), Value::from(id));
                }
                let Some(decoded) = decode_sparse_item(&value, None) else {
                    continue;
                };
                let existing = find_owned(store, decoded.item_id);
                let defaults = existing.as_ref().map(|item| item.data().clone());
                let Some(item) = decode_sparse_item(&value, defaults.as_ref()) else {
                    continue;
                };
                let target = owned_container(&item);
                let current = if existing
                    .as_ref()
                    .is_some_and(|existing| existing.data().context == ItemContext::from(target))
                {
                    existing
                } else {
                    store.get(target, item.item_id)
                };
                if let Some(current) = current {
                    let now = value.get("iQtyNow").and_then(wire_int);
                    current.update(|current| {
                        current.quantity = now.unwrap_or(current.quantity + item.quantity)
                    });
                } else {
                    store.upsert(target, item);
                }
            }
        }
        "turnIn" => {
            let decoded = Record::new(data).and_then(|record| {
                record.optional("sItems", |value| {
                    if value.is_null() {
                        Some(None)
                    } else {
                        value.as_str().map(|value| Some(value.to_owned()))
                    }
                })
            });
            let Some(value) = decoded else {
                tracing::warn!(command = "turnIn", "invalid projection input");
                return Some(output);
            };
            if let Some(Some(value)) = value {
                let mut invalid = 0;
                let mut items = Vec::new();
                for entry in value.split(',').filter(|entry| !trim_js(entry).is_empty()) {
                    let parts: Vec<_> = entry.split(':').collect();
                    match parts.as_slice() {
                        [id, quantity] => match (strict_positive(id), strict_positive(quantity)) {
                            (Some(id), Some(quantity)) => items.push((id, quantity)),
                            _ => invalid += 1,
                        },
                        _ => invalid += 1,
                    }
                }
                if invalid > 0 {
                    tracing::warn!(
                        command = "turnIn",
                        rejected = invalid,
                        "projection discarded invalid entries"
                    );
                }
                for (id, amount) in items {
                    let temporary = store.get(ItemContainer::Temporary, id);
                    let container = if temporary.is_some() {
                        ItemContainer::Temporary
                    } else {
                        ItemContainer::Inventory
                    };
                    if let Some(current) =
                        temporary.or_else(|| store.get(ItemContainer::Inventory, id))
                    {
                        let quantity = current.data().quantity - amount as f64;
                        if quantity <= 0.0 {
                            store.remove(container, id);
                        } else {
                            current.update(|current| current.quantity = quantity);
                        }
                    }
                }
            }
        }
        "removeItem" | "sellItem" => {
            let decoded = (|| {
                let record = Record::new(data)?;
                Some((
                    record.positive("CharItemID")?,
                    record.boolean("bBank")?,
                    record.boolean("bSuccess")?,
                    record.quantity("iQty")?,
                    record.quantity("iQtyNow")?,
                ))
            })();
            let Some((id, bank, success, quantity, now)) = decoded else {
                return Some(output);
            };
            if success != Some(false) {
                let inventory = if bank == Some(true) {
                    store.by_char_item_id(ItemContainer::Inventory, id)
                } else {
                    None
                };
                let containers: &[ItemContainer] = if bank == Some(true)
                    && !inventory
                        .as_ref()
                        .is_some_and(|item| item.data().category == "Class")
                {
                    &[ItemContainer::Bank]
                } else {
                    &[ItemContainer::Inventory, ItemContainer::House]
                };
                for container in containers {
                    if let Some(current) = store.by_char_item_id(*container, id) {
                        let data = current.data();
                        let quantity = now.unwrap_or(data.quantity - quantity.unwrap_or(1.0));
                        let remove = data.category == "Class" || quantity <= 0.0;
                        let item_id = data.item_id;
                        drop(data);
                        if remove {
                            store.remove(*container, item_id);
                        } else {
                            current.update(|current| current.quantity = quantity);
                        }
                        break;
                    }
                }
            }
        }
        "removeTempItem" => {
            let Some(decoded) = mutation(data) else {
                return Some(output);
            };
            if let Some(current) = store.get(ItemContainer::Temporary, decoded.id) {
                let quantity = decoded
                    .quantity_now
                    .unwrap_or(current.data().quantity - decoded.quantity.unwrap_or(1.0));
                if quantity <= 0.0 {
                    store.remove(ItemContainer::Temporary, decoded.id);
                } else {
                    current.update(|current| current.quantity = quantity);
                }
            }
        }
        _ => return None,
    }
    Some(output)
}

fn strict_positive(value: &str) -> Option<i64> {
    let number = wire_number(&Value::String(value.into()))?;
    (number.fract() == 0.0 && number > 0.0 && number <= 9_007_199_254_740_991.0)
        .then_some(number as i64)
}
#[cfg(test)]
mod tests {
    use super::*;
    use lucent_protocol::PacketEncoding;
    use serde_json::json;

    fn packet(command: &str, data: Value) -> Packet {
        Packet::Extension {
            command: command.into(),
            data,
            raw: String::new(),
            encoding: PacketEncoding::Json,
        }
    }

    fn insert(
        store: &mut ItemStore,
        container: ItemContainer,
        value: Value,
    ) -> super::super::LiveItem {
        store.upsert(container, decode_item(&value, container).unwrap())
    }

    #[test]
    fn bank_transfers_move_the_retained_object_and_keep_class_sale_routing() {
        let mut store = ItemStore::default();
        let held = insert(
            &mut store,
            ItemContainer::Inventory,
            json!({"ItemID":1,"CharItemID":11,"sName":"Resource","iQty":5}),
        );
        project_items(
            &mut store,
            &packet("bankFromInv", json!({"ItemID":1,"bSuccess":true})),
            None,
        )
        .unwrap();
        assert!(held.same_instance(&store.get(ItemContainer::Bank, 1).unwrap()));
        assert_eq!(held.data().context, ItemContext::Bank);
        project_items(&mut store, &packet("bankToInv", json!({"ItemID":1})), None).unwrap();
        assert!(held.same_instance(&store.get(ItemContainer::Inventory, 1).unwrap()));
        insert(
            &mut store,
            ItemContainer::Inventory,
            json!({"ItemID":2,"CharItemID":22,"sType":"Class","iQty":100}),
        );
        project_items(
            &mut store,
            &packet(
                "sellItem",
                json!({"CharItemID":22,"bBank":true,"iQtyNow":99}),
            ),
            None,
        )
        .unwrap();
        assert!(store.get(ItemContainer::Inventory, 2).is_none());
    }

    #[test]
    fn sparse_additions_keep_existing_metadata_and_turnins_prefer_temporary_items() {
        let mut store = ItemStore::default();
        let held = insert(
            &mut store,
            ItemContainer::Temporary,
            json!({"ItemID":1,"sName":"Token","bTemp":true,"iQty":5}),
        );
        insert(
            &mut store,
            ItemContainer::Inventory,
            json!({"ItemID":1,"iQty":9}),
        );
        project_items(
            &mut store,
            &packet("addItems", json!({"items":{"1":{"iQty":2}}})),
            None,
        )
        .unwrap();
        assert_eq!(held.data().quantity, 7.0);
        assert_eq!(held.data().name, "Token");
        project_items(
            &mut store,
            &packet("turnIn", json!({"sItems":"1:3,invalid,1.1:2"})),
            None,
        )
        .unwrap();
        assert_eq!(held.data().quantity, 4.0);
        assert_eq!(
            store
                .get(ItemContainer::Inventory, 1)
                .unwrap()
                .data()
                .quantity,
            9.0
        );
    }

    #[test]
    fn accepting_a_drop_does_not_reuse_its_live_identity_or_mutate_its_event() {
        let mut store = ItemStore::default();
        let dropped = project_items(
            &mut store,
            &packet("dropItem", json!({"items":{"1":{"ItemID":1,"iQty":2}}})),
            None,
        )
        .unwrap();
        let held = store.get(ItemContainer::Drop, 1).unwrap();
        let event = serde_json::to_value(&dropped.events[0]).unwrap();
        project_items(
            &mut store,
            &packet("getDrop", json!({"ItemID":1,"bSuccess":true,"iQtyNow":5})),
            None,
        )
        .unwrap();
        assert!(!held.same_instance(&store.get(ItemContainer::Inventory, 1).unwrap()));
        assert_eq!(held.data().quantity, 2.0);
        assert_eq!(event["item"]["quantity"].as_f64(), Some(2.0));
    }

    #[test]
    fn equipment_and_wear_are_separate_and_other_players_cannot_change_self() {
        let mut store = ItemStore::default();
        let first = insert(
            &mut store,
            ItemContainer::Inventory,
            json!({"ItemID":1,"sES":"co","bEquip":true,"bWear":true}),
        );
        let second = insert(
            &mut store,
            ItemContainer::Inventory,
            json!({"ItemID":2,"sES":"co"}),
        );
        project_items(
            &mut store,
            &packet("wearItem", json!({"ItemID":2,"sES":"co","uid":8})),
            Some(7),
        )
        .unwrap();
        assert!(!second.data().worn);
        project_items(
            &mut store,
            &packet("wearItem", json!({"ItemID":2,"sES":"co","uid":7})),
            Some(7),
        )
        .unwrap();
        assert!(first.data().equipped);
        assert!(!first.data().worn);
        assert!(second.data().worn);
        assert!(!second.data().equipped);
    }
    #[test]
    fn purchases_accumulate_into_the_correct_container_and_auto_equip_only_the_first_house() {
        let mut store = ItemStore::default();
        insert(
            &mut store,
            ItemContainer::Shop,
            json!({"ItemID":1,"ShopItemID":101,"sName":"House A","sType":"House","iQty":1}),
        );
        insert(
            &mut store,
            ItemContainer::Shop,
            json!({"ItemID":2,"ShopItemID":102,"sName":"House B","sType":"House","iQty":1}),
        );
        insert(
            &mut store,
            ItemContainer::Shop,
            json!({"ItemID":3,"ShopItemID":103,"sName":"Class A","sType":"Class","iQty":1}),
        );
        for (id, bank) in [(1, false), (2, false), (3, true)] {
            project_items(
                &mut store,
                &packet(
                    "buyItem",
                    json!({"bitSuccess":true,"CharItemID":id+10,"ItemID":id,"bBank":bank,"iQty":2}),
                ),
                None,
            )
            .unwrap();
        }
        assert!(store.get(ItemContainer::House, 1).unwrap().data().equipped);
        assert!(!store.get(ItemContainer::House, 2).unwrap().data().equipped);
        assert!(store.get(ItemContainer::Bank, 3).is_none());
        assert_eq!(
            store
                .get(ItemContainer::Inventory, 3)
                .unwrap()
                .data()
                .quantity,
            2.0
        );
        project_items(
            &mut store,
            &packet(
                "buyItem",
                json!({"bitSuccess":true,"CharItemID":14,"ItemID":1,"bBank":false,"iQty":3}),
            ),
            None,
        )
        .unwrap();
        let house = store.get(ItemContainer::House, 1).unwrap();
        assert_eq!(house.data().quantity, 5.0);
        assert_eq!(house.data().char_item_id, Some(14));
    }

    #[test]
    fn swap_is_all_or_nothing_and_withdraws_house_items_into_house_inventory() {
        let mut store = ItemStore::default();
        let house = insert(
            &mut store,
            ItemContainer::Bank,
            json!({"ItemID":1,"sType":"House"}),
        );
        let outgoing = insert(
            &mut store,
            ItemContainer::Inventory,
            json!({"ItemID":2,"sType":"Resource"}),
        );
        project_items(
            &mut store,
            &packet("bankSwapInv", json!({"invItemID":9,"bankItemID":1})),
            None,
        )
        .unwrap();
        assert!(store.get(ItemContainer::Bank, 1).is_some());
        project_items(
            &mut store,
            &packet("bankSwapInv", json!({"invItemID":2,"bankItemID":1})),
            None,
        )
        .unwrap();
        assert!(house.same_instance(&store.get(ItemContainer::House, 1).unwrap()));
        assert_eq!(house.data().context, ItemContext::House);
        assert!(outgoing.same_instance(&store.get(ItemContainer::Bank, 2).unwrap()));
        assert!(store.get(ItemContainer::Inventory, 2).is_none());
    }

    #[test]
    fn enhancement_response_replaces_the_whole_enhancement_even_with_zero_or_empty_fields() {
        let mut store = ItemStore::default();
        let held = insert(
            &mut store,
            ItemContainer::Inventory,
            json!({"ItemID":1,"EnhID":9,"EnhLvl":100,"EnhPID":32,"ProcID":14}),
        );
        project_items(
            &mut store,
            &packet(
                "enhanceItemShop",
                json!({"ItemIDs":[1],"EnhID":0,"EnhPID":2}),
            ),
            None,
        )
        .unwrap();
        let enhancement = held.data().enhancement.clone().unwrap();
        assert_eq!(enhancement.id, Some(0.0));
        assert_eq!(enhancement.pattern_id, Some(2.0));
        assert_eq!(enhancement.level, None);
        assert_eq!(enhancement.proc_id, None);
        project_items(
            &mut store,
            &packet("enhanceItemShop", json!({"ItemIDs":[1]})),
            None,
        )
        .unwrap();
        assert!(held.data().enhancement.as_ref().unwrap().is_empty());
    }

    #[test]
    fn wheel_rewards_use_overrides_and_add_to_existing_quantities() {
        let mut store = ItemStore::default();
        insert(
            &mut store,
            ItemContainer::Inventory,
            json!({"ItemID":18927,"iQty":5}),
        );
        project_items(&mut store, &packet("Wheel", json!({
            "charItem1":10,"charItem2":11,"CharItemID":12,"dropQty":3,
            "dropItems":{"18927":{"ItemID":18927,"iQty":99},"19189":{"ItemID":19189,"bBank":true}},
            "Item":{"ItemID":20,"bBank":true,"bTemp":true}
        })), None).unwrap();
        assert_eq!(
            store
                .get(ItemContainer::Inventory, 18927)
                .unwrap()
                .data()
                .quantity,
            8.0
        );
        assert_eq!(
            store
                .get(ItemContainer::Inventory, 18927)
                .unwrap()
                .data()
                .char_item_id,
            Some(10)
        );
        assert_eq!(
            store
                .get(ItemContainer::Bank, 19189)
                .unwrap()
                .data()
                .char_item_id,
            Some(11)
        );
        assert!(store.get(ItemContainer::Bank, 20).is_none());
        assert_eq!(
            store
                .get(ItemContainer::Temporary, 20)
                .unwrap()
                .data()
                .char_item_id,
            Some(12)
        );
    }

    #[test]
    fn malformed_optional_fields_prevent_mutation_and_temp_removal_keeps_its_existing_success_semantics()
     {
        let mut store = ItemStore::default();
        let held = insert(
            &mut store,
            ItemContainer::Temporary,
            json!({"ItemID":1,"bTemp":true,"iQty":5}),
        );
        let invalid = project_items(
            &mut store,
            &packet("bankFromInv", json!({"ItemID":1,"bHouse":null})),
            None,
        )
        .unwrap();
        assert!(invalid.events.is_empty());

        project_items(
            &mut store,
            &packet(
                "removeTempItem",
                json!({"ItemID":1,"bSuccess":false,"iQty":2}),
            ),
            None,
        )
        .unwrap();
        assert_eq!(held.data().quantity, 3.0);
        project_items(
            &mut store,
            &packet("removeTempItem", json!({"ItemID":1,"iQtyNow":0})),
            None,
        )
        .unwrap();
        assert!(store.get(ItemContainer::Temporary, 1).is_none());
        assert_eq!(held.data().quantity, 3.0);
    }

    #[test]
    fn drop_event_order_follows_javascript_numeric_property_order() {
        let mut store = ItemStore::default();
        let value: Value = serde_json::from_str(
            r#"{"items":{"10":{"ItemID":10},"2":{"ItemID":2},"named":{"ItemID":3}}}"#,
        )
        .unwrap();
        let events = project_items(&mut store, &packet("dropItem", value), None)
            .unwrap()
            .events;
        let ids: Vec<_> = events
            .iter()
            .map(|event| match event {
                ProjectionEvent::ItemDrop { item } => item.data.item_id,
                _ => panic!("unexpected event"),
            })
            .collect();
        assert_eq!(ids, [2, 10, 3]);
    }
}
