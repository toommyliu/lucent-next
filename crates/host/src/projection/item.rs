use lucent_protocol::item::{Enhancement, ItemContainer, ItemContext, ItemData, ItemRequirement};
use serde_json::{Map, Value};

use super::wire::{wire_bool, wire_i64, wire_int, wire_number};

struct Payload<'a>(&'a Map<String, Value>);

impl Payload<'_> {
    fn optional<T>(
        &self,
        key: &str,
        decode: impl FnOnce(&Value) -> Option<T>,
    ) -> Option<Option<T>> {
        match self.0.get(key) {
            None => Some(None),
            Some(value) => decode(value).map(Some),
        }
    }

    fn text(&self, key: &str) -> Option<Option<String>> {
        self.optional(key, |value| value.as_str().map(str::to_owned))
    }
}

fn positive(value: &Value) -> Option<i64> {
    wire_i64(value).filter(|value| *value > 0)
}

fn nonnegative(value: &Value) -> Option<f64> {
    wire_int(value).filter(|value| *value >= 0.0)
}

pub fn decode_item(value: &Value, context: ItemContainer) -> Option<ItemData> {
    decode_with_defaults(value, Some(context.into()), None)
}

pub(crate) fn decode_sparse_item(value: &Value, defaults: Option<&ItemData>) -> Option<ItemData> {
    decode_with_defaults(value, defaults.map(|item| item.context), defaults)
}

fn decode_with_defaults(
    value: &Value,
    context: Option<ItemContext>,
    defaults: Option<&ItemData>,
) -> Option<ItemData> {
    let shop_default = context == Some(ItemContext::Shop);
    let payload = Payload(value.as_object()?);
    let item_id = positive(payload.0.get("ItemID")?)?;
    let char_item_id = payload.optional("CharItemID", positive)?;
    let shop_item_id = payload.optional("ShopItemID", positive)?;
    let dps = payload.optional("EnhDPS", wire_number)?;
    let enhancement_id = payload.optional("EnhID", wire_int)?;
    let i_enh = payload.optional("iEnh", wire_int)?;
    let enhancement_level = payload.optional("EnhLvl", nonnegative)?;
    let i_lvl = payload.optional("iLvl", nonnegative)?;
    let enhancement_pattern = payload.optional("EnhPatternID", nonnegative)?;
    let enhancement_pid = payload.optional("EnhPID", nonnegative)?;
    let pattern_id = payload.optional("PatternID", nonnegative)?;
    let item_proc_id = payload.optional("ItemProcID", nonnegative)?;
    let proc_id = payload.optional("ProcID", nonnegative)?;
    let range = payload.optional("EnhRng", wire_number)?;
    let rarity = payload.optional("EnhRty", wire_number)?;
    let banked = payload.optional("bBank", wire_bool)?.unwrap_or(false);
    let coins = payload
        .optional("bCoins", wire_bool)?
        .unwrap_or_else(|| defaults.is_some_and(|item| item.coins));
    let equipped = payload
        .optional("bEquip", wire_bool)?
        .unwrap_or_else(|| defaults.is_some_and(|item| item.equipped));
    let house = payload.optional("bHouse", wire_bool)?.unwrap_or(false);
    let temporary_item = payload
        .optional("bTemp", wire_bool)?
        .unwrap_or_else(|| defaults.is_some_and(|item| item.temporary_item));
    let member_only = payload
        .optional("bUpg", wire_bool)?
        .unwrap_or_else(|| defaults.is_some_and(|item| item.member_only));
    let wear = payload.optional("bWear", wire_bool)?;
    let cost = payload
        .optional("iCost", wire_number)?
        .unwrap_or_else(|| defaults.map_or(0.0, |item| item.cost));
    let quantity = payload.optional("iQty", wire_int)?;
    let quantity_now = payload.optional("iQtyNow", wire_int)?;
    let max_stack = payload
        .optional("iStk", positive)?
        .map(|value| value as f64);
    let description = payload
        .text("sDesc")?
        .or_else(|| defaults.map(|item| item.description.clone()))
        .unwrap_or_default();
    let equipment_slot = payload.text("sES")?;
    let str_es = payload.text("strES")?;
    let file = payload
        .text("sFile")?
        .or_else(|| defaults.map(|item| item.file.clone()))
        .unwrap_or_default();
    let link = payload
        .text("sLink")?
        .or_else(|| defaults.map(|item| item.link.clone()))
        .unwrap_or_default();
    let meta = payload
        .text("sMeta")?
        .or_else(|| defaults.map(|item| item.meta.clone()))
        .unwrap_or_default();
    let name = payload.text("sName")?.unwrap_or_else(|| {
        defaults.map_or_else(|| format!("Item {item_id}"), |item| item.name.clone())
    });
    let category = payload
        .text("sType")?
        .or_else(|| defaults.map(|item| item.category.clone()))
        .unwrap_or_default();
    let house_item = house
        || defaults.is_some_and(|item| item.house_item)
        || matches!(category.as_str(), "House" | "Floor Item" | "Wall Item");
    let context = if banked && category != "Class" {
        ItemContext::Bank
    } else {
        context.unwrap_or(if temporary_item {
            ItemContext::Temporary
        } else if house_item {
            ItemContext::House
        } else {
            ItemContext::Inventory
        })
    };
    let enhancement = Enhancement {
        dps,
        id: enhancement_id.or(i_enh).filter(|id| *id > 0.0),
        level: enhancement_level
            .or(if shop_default { i_lvl } else { None })
            .filter(|level| *level > 0.0),
        pattern_id: enhancement_pattern
            .or(enhancement_pid)
            .or(pattern_id)
            .filter(|id| *id > 0.0),
        proc_id: item_proc_id.or(proc_id).filter(|id| *id > 0.0),
        range,
        rarity,
    };
    let requirements = match payload.0.get("turnin") {
        None => defaults.and_then(|item| item.requirements.clone()),
        Some(Value::Null) => Some(Vec::new()),
        Some(value) => Some(
            value
                .as_array()?
                .iter()
                .map(|value| {
                    let requirement = Payload(value.as_object()?);
                    Some(ItemRequirement {
                        item_id: positive(requirement.0.get("ItemID")?)?,
                        name: requirement.text("sName")?.unwrap_or_default(),
                        quantity: positive(requirement.0.get("iQty")?)?,
                    })
                })
                .collect::<Option<Vec<_>>>()?,
        ),
    };
    Some(ItemData {
        category,
        char_item_id: char_item_id.or_else(|| defaults.and_then(|item| item.char_item_id)),
        coins,
        context,
        cost,
        description,
        enhancement: (!enhancement.is_empty()).then_some(enhancement),
        equipped,
        equipment_slot: equipment_slot
            .or(str_es)
            .or_else(|| defaults.map(|item| item.equipment_slot.clone()))
            .unwrap_or_default(),
        file,
        house_item,
        item_id,
        link,
        max_stack: max_stack.or_else(|| defaults.and_then(|item| item.max_stack)),
        member_only,
        meta,
        name,
        quantity: quantity_now
            .or(quantity)
            .unwrap_or_else(|| defaults.map_or(1.0, |item| item.quantity))
            .max(0.0),
        requirements,
        shop_item_id: shop_item_id.or_else(|| defaults.and_then(|item| item.shop_item_id)),
        temporary_item,
        wearable: wear.is_some() || defaults.is_some_and(|item| item.wearable),
        worn: wear.unwrap_or_else(|| defaults.is_some_and(|item| item.worn)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn owned_item_normalization_preserves_context_quantity_and_derived_fields() {
        let item = decode_item(
            &json!({
                "ItemID":"0x10", "CharItemID":"12", "sType":"Resource",
                "iQty":-2, "bBank":"true", "bHouse":"1", "bTemp":true,
                "iStk":5, "turnin":null
            }),
            ItemContainer::Inventory,
        )
        .unwrap();
        let snapshot = item.snapshot();
        assert_eq!(snapshot["itemId"], 16);
        assert_eq!(snapshot["context"], "bank");
        assert_eq!(snapshot["quantity"].as_f64(), Some(0.0));
        assert_eq!(snapshot["pool"], "house");
        assert_eq!(snapshot["requirements"], json!([]));
        assert_eq!(snapshot["name"], "Item 16");
        assert!(snapshot.get("enhancement").is_none());
    }

    #[test]
    fn enhancement_precedence_and_wear_presence_match_the_live_model() {
        let item = decode_item(
            &json!({
                "ItemID":1, "sType":"Item", "sMeta":" 123 ", "bWear":false,
                "EnhID":0, "iEnh":42, "EnhLvl":0, "EnhPID":32, "ProcID":14,
                "EnhRng":0, "EnhRty":1.5
            }),
            ItemContainer::Inventory,
        )
        .unwrap();
        let snapshot = item.snapshot();
        assert_eq!(snapshot["wearable"], true);
        assert_eq!(snapshot["worn"], false);
        assert_eq!(snapshot["pool"], "bag");
        assert_eq!(
            snapshot["enhancement"],
            json!({
                "patternId":32.0, "procId":14.0, "range":0.0, "rarity":1.5,
                "name":"Grimskull", "special":"Dauntless"
            })
        );
    }

    #[test]
    fn class_points_and_class_bank_exception_are_preserved() {
        let item = decode_item(
            &json!({
                "ItemID":2, "sType":"Class", "bBank":true, "iQty":3600
            }),
            ItemContainer::Inventory,
        )
        .unwrap();
        assert_eq!(item.snapshot()["classRank"], 3);
        assert_eq!(item.snapshot()["context"], "inventory");
        assert_eq!(item.snapshot()["pool"], "class");
        assert_eq!(
            decode_item(
                &json!({"ItemID":3,"sType":"Item","sMeta":"１２３"}),
                ItemContainer::Inventory
            )
            .unwrap()
            .snapshot()["pool"],
            "misc"
        );
    }

    #[test]
    fn invalid_optional_fields_reject_the_whole_entry_even_when_unused() {
        for extra in [
            json!({"sName":null}),
            json!({"iLvl":-1}),
            json!({"bBank":"FALSE"}),
            json!({"EnhDPS":"Infinity"}),
            json!({"iStk":0}),
            json!({"turnin":[{"ItemID":1,"iQty":0}]}),
            json!({"turnin":[{"ItemID":1,"iQty":2,"sName":null}]}),
            json!({"CharItemID":"9007199254740992"}),
        ] {
            let mut value = json!({"ItemID":1});
            value
                .as_object_mut()
                .unwrap()
                .extend(extra.as_object().unwrap().clone());
            assert!(
                decode_item(&value, ItemContainer::Inventory).is_none(),
                "{value}"
            );
        }
        let item = decode_item(
            &json!({"ItemID":1,"unknown":null}),
            ItemContainer::Inventory,
        )
        .unwrap();
        assert!(item.snapshot().get("requirements").is_none());
    }
}
