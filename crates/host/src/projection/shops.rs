use std::sync::{Arc, RwLock};

use lucent_protocol::{Packet, PacketDirection, item::ItemContainer, shop::ShopData};
use serde_json::{Map, Value};

use super::{
    ItemStore, LiveItem, ProjectionOutput, decode_item,
    wire::{wire_bool, wire_i64},
};

#[derive(Debug)]
struct ShopState {
    house: bool,
    id: i64,
    items: Vec<LiveItem>,
    limited: bool,
    merge: bool,
    name: String,
}

#[derive(Clone, Debug)]
pub struct LiveShop(Arc<RwLock<ShopState>>);

impl LiveShop {
    pub fn data(&self) -> ShopData {
        let data = self.0.read().unwrap();
        ShopData {
            house: data.house,
            id: data.id,
            items: data.items.iter().map(|item| item.data().clone()).collect(),
            limited: data.limited,
            merge: data.merge,
            name: data.name.clone(),
        }
    }
    pub fn items(&self) -> Vec<LiveItem> {
        self.0.read().unwrap().items.clone()
    }
    pub fn snapshot(&self) -> Value {
        serde_json::to_value(self.data()).unwrap()
    }
    pub fn same_instance(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Debug, Default)]
pub struct ShopStore {
    current: Option<LiveShop>,
}

impl ShopStore {
    pub fn clear(&mut self) {
        self.current = None;
    }
    pub fn get(&self) -> Option<LiveShop> {
        self.current.clone()
    }
    pub fn set(&mut self, data: ShopData) -> LiveShop {
        let incoming = ShopState {
            house: data.house,
            id: data.id,
            items: data.items.into_iter().map(LiveItem::new).collect(),
            limited: data.limited,
            merge: data.merge,
            name: data.name,
        };
        if let Some(current) = &self.current
            && current.0.read().unwrap().id == incoming.id
        {
            *current.0.write().unwrap() = incoming;
            return current.clone();
        }
        let model = LiveShop(Arc::new(RwLock::new(incoming)));
        self.current = Some(model.clone());
        model
    }
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

fn decode_shop(value: &Value) -> Option<ShopData> {
    let object = value.as_object()?;
    let id = wire_i64(object.get("ShopID")?).filter(|value| *value > 0)?;
    let house = optional(object, "bHouse", wire_bool)?.unwrap_or(false);
    let limited = optional(object, "bLimited", wire_bool)?.unwrap_or(false);
    let merge = optional(object, "bMerge", wire_bool)?.unwrap_or(false);
    let shop_type = optional(object, "sType", Value::as_str)?;
    optional(object, "items", Value::as_array)?;
    let name = optional(object, "sName", Value::as_str)?
        .map_or_else(|| format!("Shop {id}"), str::to_owned);
    Some(ShopData {
        house,
        id,
        items: Vec::new(),
        limited,
        merge: merge || shop_type == Some("Merge"),
        name,
    })
}

pub fn project_shop(
    store: &mut ShopStore,
    items: &mut ItemStore,
    packet: &Packet,
) -> Option<ProjectionOutput> {
    if packet.direction() != PacketDirection::Extension || packet.command() != "loadShop" {
        return None;
    }
    let output = ProjectionOutput::default();
    let data = packet.data().unwrap();
    let Some((shop, payload)) = decode_shop(data).map(|shop| (shop, data)).or_else(|| {
        let wrapped = data.get("shopinfo")?;
        decode_shop(wrapped).map(|shop| (shop, wrapped))
    }) else {
        tracing::warn!(command = "loadShop", "invalid projection input");
        return Some(output);
    };
    let mut incoming = Vec::new();
    let mut invalid = 0;
    for value in payload
        .get("items")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if let Some(item) = decode_item(value, ItemContainer::Shop) {
            incoming.push(item);
        } else {
            invalid += 1;
        }
    }
    if invalid > 0 {
        tracing::warn!(
            command = "loadShop",
            rejected = invalid,
            "projection discarded invalid entries"
        );
    }
    let mut shop = shop;
    shop.items = incoming;
    let live = store.set(shop);
    items.replace_live(ItemContainer::Shop, live.items());
    Some(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lucent_protocol::PacketEncoding;
    use serde_json::json;

    fn packet(data: Value) -> Packet {
        Packet::Extension {
            command: "loadShop".into(),
            data,
            raw: String::new(),
            encoding: PacketEncoding::Json,
        }
    }

    #[test]
    fn same_shop_survives_reload_while_new_catalog_items_follow_reference_lifetimes() {
        let mut store = ShopStore::default();
        let mut items = ItemStore::default();
        project_shop(
            &mut store,
            &mut items,
            &packet(json!({"ShopID":1,"items":[{"ItemID":10,"ShopItemID":100,"iQty":2}]})),
        )
        .unwrap();
        let held = store.get().unwrap();
        let offer = items.by_shop_item_id(ItemContainer::Shop, 100).unwrap();
        assert!(held.items()[0].same_instance(&offer));
        offer.update(|item| item.quantity = 3.0);
        assert_eq!(held.snapshot()["items"][0]["quantity"].as_f64(), Some(3.0));
        project_shop(&mut store, &mut items, &packet(json!({"ShopID":1,"sName":"Reloaded","items":[{"ItemID":10,"ShopItemID":100,"iQty":7}]}))).unwrap();
        assert!(held.same_instance(&store.get().unwrap()));
        assert!(!held.items()[0].same_instance(&offer));
        assert_eq!(offer.data().quantity, 7.0);
        offer.update(|item| item.quantity = 9.0);
        assert_eq!(held.snapshot()["items"][0]["quantity"].as_f64(), Some(7.0));
        project_shop(&mut store, &mut items, &packet(json!({"ShopID":2}))).unwrap();
        assert!(!held.same_instance(&store.get().unwrap()));
        assert_eq!(held.data().name, "Reloaded");
    }

    #[test]
    fn shop_wrappers_partial_entries_and_merge_metadata_are_preserved() {
        let mut store = ShopStore::default();
        let mut items = ItemStore::default();
        let output = project_shop(&mut store, &mut items, &packet(json!({"shopinfo":{"ShopID":"2","sType":"Merge","bLimited":"1","bHouse":true,"items":[{"ItemID":3},{"ItemID":0}]}}))).unwrap();
        assert!(output.events.is_empty());
        let shop = store.get().unwrap().data();
        assert!(shop.merge && shop.house && shop.limited);
        assert_eq!(shop.name, "Shop 2");
        assert_eq!(shop.items.len(), 1);

        let held = store.get().unwrap();
        project_shop(
            &mut store,
            &mut items,
            &packet(json!({"ShopID":3,"items":null})),
        )
        .unwrap();
        assert!(held.same_instance(&store.get().unwrap()));
    }
}
