use std::sync::{Arc, RwLock, RwLockReadGuard};

use indexmap::{IndexMap, IndexSet};
use lucent_protocol::item::{ItemContainer, ItemData};

use super::wire::{selector_id, trim_js};
use lucent_protocol::query::ItemSelector;

#[derive(Clone, Debug)]
pub struct LiveItem(Arc<RwLock<ItemData>>);

impl LiveItem {
    pub fn new(data: ItemData) -> Self {
        Self(Arc::new(RwLock::new(data)))
    }

    pub fn data(&self) -> RwLockReadGuard<'_, ItemData> {
        self.0.read().unwrap()
    }
    pub(crate) fn update(&self, apply: impl FnOnce(&mut ItemData)) {
        apply(&mut self.0.write().unwrap());
    }

    pub fn same_instance(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Default, Debug)]
struct Container {
    values: IndexMap<i64, LiveItem>,
    item_ids: IndexMap<i64, IndexSet<i64>>,
    names: IndexMap<String, IndexSet<i64>>,
    char_item_ids: IndexMap<i64, i64>,
    hydration: u64,
}

fn normalized_name(name: &str) -> String {
    trim_js(name).to_lowercase()
}

impl Container {
    fn remove_indexes(&mut self, key: i64, item: &ItemData) {
        let name = normalized_name(&item.name);
        if let Some(keys) = self.names.get_mut(&name) {
            keys.shift_remove(&key);
            if keys.is_empty() {
                self.names.shift_remove(&name);
            }
        }
        if let Some(keys) = self.item_ids.get_mut(&item.item_id) {
            keys.shift_remove(&key);
            if keys.is_empty() {
                self.item_ids.shift_remove(&item.item_id);
            }
        }
        if let Some(id) = item.char_item_id {
            self.char_item_ids.shift_remove(&id);
        }
    }

    fn add_indexes(&mut self, key: i64, item: &ItemData) {
        self.names
            .entry(normalized_name(&item.name))
            .or_default()
            .insert(key);
        self.item_ids.entry(item.item_id).or_default().insert(key);
        if let Some(id) = item.char_item_id {
            self.char_item_ids.insert(id, key);
        }
    }
}

#[derive(Debug)]
pub struct ItemStore {
    containers: IndexMap<ItemContainer, Container>,
}

impl Default for ItemStore {
    fn default() -> Self {
        Self {
            containers: ItemContainer::ALL
                .into_iter()
                .map(|container| (container, Container::default()))
                .collect(),
        }
    }
}

impl ItemStore {
    pub fn resolve(&self, kind: ItemContainer, selector: &ItemSelector) -> Option<LiveItem> {
        match selector {
            ItemSelector::Id(id) | ItemSelector::ItemId { item_id: id } => {
                self.get(kind, selector_id(*id)?)
            }
            ItemSelector::Name(name) | ItemSelector::Named { name } => self.by_name(kind, name),
            ItemSelector::ShopItemId { shop_item_id } => {
                self.by_shop_item_id(kind, selector_id(*shop_item_id)?)
            }
        }
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn replace(&mut self, kind: ItemContainer, items: Vec<ItemData>) {
        self.replace_live(kind, items.into_iter().map(LiveItem::new).collect());
    }

    pub fn replace_live(&mut self, kind: ItemContainer, items: Vec<LiveItem>) {
        let next: IndexSet<i64> = items
            .iter()
            .map(|item| {
                let item = item.data();
                if kind == ItemContainer::Shop {
                    item.shop_item_id.unwrap_or(item.item_id)
                } else {
                    item.item_id
                }
            })
            .collect();
        let container = self.containers.get_mut(&kind).unwrap();
        let removed: Vec<i64> = container
            .values
            .keys()
            .copied()
            .filter(|key| !next.contains(key))
            .collect();
        for key in removed {
            let old = container.values.shift_remove(&key).unwrap();
            container.remove_indexes(key, &old.data());
        }
        container.hydration += 1;
        for item in items {
            self.upsert_model(kind, item);
        }
    }

    pub fn upsert(&mut self, kind: ItemContainer, data: ItemData) -> LiveItem {
        self.upsert_model(kind, LiveItem::new(data))
    }

    pub(crate) fn upsert_model(&mut self, kind: ItemContainer, incoming: LiveItem) -> LiveItem {
        let data = incoming.data().clone();
        let key = if kind == ItemContainer::Shop {
            data.shop_item_id.unwrap_or(data.item_id)
        } else {
            data.item_id
        };
        let container = self.containers.get_mut(&kind).unwrap();
        if let Some(current) = container.values.get(&key).cloned() {
            container.remove_indexes(key, &current.data());
            container.add_indexes(key, &data);
            if !current.same_instance(&incoming) {
                current.update(|current| *current = data);
            }
            current
        } else {
            container.add_indexes(key, &data);
            container.values.insert(key, incoming.clone());
            incoming
        }
    }

    pub fn remove(&mut self, kind: ItemContainer, key: i64) -> Option<LiveItem> {
        let container = self.containers.get_mut(&kind).unwrap();
        let current = container.values.shift_remove(&key)?;
        container.remove_indexes(key, &current.data());
        Some(current)
    }

    pub fn all(&self, kind: ItemContainer) -> Vec<LiveItem> {
        self.containers[&kind].values.values().cloned().collect()
    }
    pub fn hydration(&self, kind: ItemContainer) -> u64 {
        self.containers[&kind].hydration
    }

    pub fn get(&self, kind: ItemContainer, item_id: i64) -> Option<LiveItem> {
        let container = &self.containers[&kind];
        container
            .item_ids
            .get(&item_id)
            .and_then(|keys| keys.first())
            .and_then(|key| container.values.get(key))
            .cloned()
    }

    pub fn by_name(&self, kind: ItemContainer, name: &str) -> Option<LiveItem> {
        let container = &self.containers[&kind];
        container
            .names
            .get(&normalized_name(name))
            .and_then(|keys| keys.first())
            .and_then(|key| container.values.get(key))
            .cloned()
    }

    pub fn by_char_item_id(&self, kind: ItemContainer, id: i64) -> Option<LiveItem> {
        let container = &self.containers[&kind];
        container
            .char_item_ids
            .get(&id)
            .and_then(|key| container.values.get(key))
            .cloned()
    }

    pub fn by_shop_item_id(&self, kind: ItemContainer, id: i64) -> Option<LiveItem> {
        self.containers[&kind].values.get(&id).cloned()
    }
}
