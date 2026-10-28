use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize, ts_rs::TS)]
#[serde(rename_all = "lowercase")]
pub enum ItemContainer {
    Bank,
    Drop,
    House,
    Inventory,
    Shop,
    Temporary,
}

impl ItemContainer {
    pub const ALL: [Self; 6] = [
        Self::Bank,
        Self::Drop,
        Self::House,
        Self::Inventory,
        Self::Shop,
        Self::Temporary,
    ];
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, ts_rs::TS)]
#[serde(rename_all = "kebab-case")]
pub enum ItemContext {
    Bank,
    Drop,
    House,
    Inventory,
    MonsterDrop,
    Shop,
    Temporary,
}

impl From<ItemContainer> for ItemContext {
    fn from(value: ItemContainer) -> Self {
        match value {
            ItemContainer::Bank => Self::Bank,
            ItemContainer::Drop => Self::Drop,
            ItemContainer::House => Self::House,
            ItemContainer::Inventory => Self::Inventory,
            ItemContainer::Shop => Self::Shop,
            ItemContainer::Temporary => Self::Temporary,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct Enhancement {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub dps: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub id: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub level: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub pattern_id: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub proc_id: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub range: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub rarity: Option<f64>,
}

impl Enhancement {
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }

    fn snapshot_data(&self) -> EnhancementSnapshot {
        let (name, pattern_special) = match self.pattern_id.filter(|id| *id > 0.0) {
            None => (None, None),
            Some(id) => {
                let (name, special) = match id as i64 {
                    1 => ("Adventurer", None),
                    2 => ("Fighter", None),
                    3 => ("Thief", None),
                    4 => ("Armsman", None),
                    5 => ("Hybrid", None),
                    6 => ("Wizard", None),
                    7 => ("Healer", None),
                    8 => ("Spellbreaker", None),
                    9 => ("Lucky", None),
                    10 => ("Forge", None),
                    11 => ("Forge", Some("Absolution")),
                    12 => ("Forge", Some("Avarice")),
                    23 => ("Depths", None),
                    24 => ("Forge", Some("Vainglory")),
                    25 => ("Vim", Some("Ether")),
                    26 => ("Examen", Some("Ether")),
                    27 => ("Pneuma", Some("Clairvoyance")),
                    28 => ("Anima", Some("Clairvoyance")),
                    29 => ("Forge", Some("Penitence")),
                    30 => ("Forge", Some("Lament")),
                    32 => ("Grimskull", Some("Hearty")),
                    _ => ("", None),
                };
                (
                    Some(if name.is_empty() {
                        format!("Pattern {id}")
                    } else {
                        name.to_owned()
                    }),
                    special.map(str::to_owned),
                )
            }
        };
        let special = match self.proc_id.filter(|id| *id > 0.0) {
            None => pattern_special,
            Some(id) => Some(match id as i64 {
                2 => "Spiral Carve".into(),
                3 => "Awe Blast".into(),
                4 => "Health Vamp".into(),
                5 => "Mana Vamp".into(),
                6 => "Powerword DIE".into(),
                7 => "Lacerate".into(),
                8 => "Smite".into(),
                9 => "Valiance".into(),
                10 => "Arcana's Concerto".into(),
                11 => "Acheron".into(),
                12 => "Elysium".into(),
                13 => "Praxis".into(),
                14 => "Dauntless".into(),
                15 => "Ravenous".into(),
                _ => format!("Proc {id}"),
            }),
        };
        EnhancementSnapshot {
            data: self.clone(),
            name,
            special,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct ItemRequirement {
    pub item_id: i64,
    pub name: String,
    pub quantity: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct ItemData {
    pub category: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub char_item_id: Option<i64>,
    pub coins: bool,
    pub context: ItemContext,
    pub cost: f64,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub enhancement: Option<Enhancement>,
    pub equipped: bool,
    pub equipment_slot: String,
    pub file: String,
    pub house_item: bool,
    pub item_id: i64,
    pub link: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub max_stack: Option<f64>,
    pub member_only: bool,
    pub meta: String,
    pub name: String,
    pub quantity: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub requirements: Option<Vec<ItemRequirement>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub shop_item_id: Option<i64>,
    pub temporary_item: bool,
    pub wearable: bool,
    pub worn: bool,
}

impl ItemData {
    pub fn snapshot_data(&self) -> ItemSnapshot {
        let class_item = self.category == "Class";
        let class_rank = class_item.then(|| {
            1 + [
                900.0, 3600.0, 10000.0, 22500.0, 44100.0, 78400.0, 129600.0, 202500.0, 302500.0,
            ]
            .into_iter()
            .take_while(|threshold| self.quantity >= *threshold)
            .count() as u8
        });
        let meta = self.meta.trim_matches(js_whitespace);
        let numeric_meta = !meta.is_empty() && meta.bytes().all(|byte| byte.is_ascii_digit());
        let pool = if class_item {
            ItemPool::Class
        } else if self.house_item || self.category == "Guild" {
            ItemPool::House
        } else if matches!(
            self.category.as_str(),
            "Item" | "Note" | "Quest Item" | "Resource"
        ) && !(self.category == "Item" && numeric_meta)
        {
            ItemPool::Misc
        } else {
            ItemPool::Bag
        };
        ItemSnapshot {
            data: Self {
                enhancement: None,
                ..self.clone()
            },
            fields: ItemSnapshotFields {
                armor: self.equipment_slot == "co",
                banked: self.context == ItemContext::Bank,
                cape: self.equipment_slot == "ba",
                class_item,
                class_rank,
                enhancement: self.enhancement.as_ref().map(Enhancement::snapshot_data),
                helm: self.equipment_slot == "he",
                pet: self.equipment_slot == "pe",
                pool,
                weapon: self.equipment_slot == "Weapon",
            },
        }
    }

    pub fn snapshot(&self) -> Value {
        serde_json::to_value(self.snapshot_data()).expect("item fields serialize")
    }
}

fn js_whitespace(value: char) -> bool {
    matches!(value, '\u{0009}'..='\u{000d}' | '\u{0020}' | '\u{00a0}' | '\u{1680}' |
        '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' |
        '\u{205f}' | '\u{3000}' | '\u{feff}')
}

#[derive(Clone, Debug, PartialEq, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct EnhancementSnapshot {
    #[serde(flatten)]
    pub data: Enhancement,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "string | undefined")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "string | undefined")]
    pub special: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, ts_rs::TS)]
#[serde(rename_all = "lowercase")]
pub enum ItemPool {
    Bag,
    Misc,
    Class,
    House,
}

#[derive(Clone, Debug, PartialEq, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct ItemSnapshotFields {
    pub armor: bool,
    pub banked: bool,
    pub cape: bool,
    pub class_item: bool,
    pub class_rank: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub enhancement: Option<EnhancementSnapshot>,
    pub helm: bool,
    pub pet: bool,
    pub pool: ItemPool,
    pub weapon: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, ts_rs::TS)]
#[ts(type = "Omit<ItemData, \"enhancement\"> & ItemSnapshotFields")]
pub struct ItemSnapshot {
    #[serde(flatten)]
    pub data: ItemData,
    #[serde(flatten)]
    pub fields: ItemSnapshotFields,
}
