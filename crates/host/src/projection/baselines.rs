use lucent_protocol::{Packet, PacketDirection, item::ItemContainer};
use serde_json::Value;

use super::{ItemStore, ReadinessKey, ReadinessReceipts, item::decode_item, wire::wire_bool};

pub fn project_baseline(
    items: &mut ItemStore,
    receipts: &mut ReadinessReceipts,
    packet: &Packet,
) -> bool {
    let command = packet.command();
    if packet.direction() != PacketDirection::Extension
        || !matches!(
            command,
            "loadInventoryBig" | "initInventory" | "loadHouseInventory"
        )
    {
        return false;
    }
    let data = packet.data().unwrap();
    let keys: &[ReadinessKey] = match command {
        "loadInventoryBig" => &[ReadinessKey::Inventory, ReadinessKey::HouseInventory],
        "initInventory" => &[ReadinessKey::Inventory],
        _ => &[ReadinessKey::HouseInventory],
    };
    let valid = data.as_object().is_some_and(|object| {
        object
            .get("bitSuccess")
            .is_none_or(|value| wire_bool(value).is_some())
            && ["items", "hitems"].iter().all(|key| {
                object
                    .get(*key)
                    .is_none_or(|value| value.is_null() || value.is_array())
            })
    });
    if !valid {
        for key in keys {
            receipts.fail(*key, format!("{command} has a malformed container payload"));
        }
        tracing::warn!(command, "invalid inventory baseline");
        return true;
    }
    if data.get("bitSuccess").and_then(wire_bool) == Some(false) {
        for key in keys {
            receipts.fail(
                *key,
                format!("{command} reported an unsuccessful baseline load"),
            );
        }
        return true;
    }
    for key in keys {
        let (field, container, operation) = match (command, key) {
            ("loadInventoryBig", ReadinessKey::HouseInventory) => (
                "hitems",
                ItemContainer::House,
                "items:loadInventoryBig:house-entries".into(),
            ),
            (_, ReadinessKey::HouseInventory) => (
                "items",
                ItemContainer::House,
                format!("items:{command}:entries"),
            ),
            _ => (
                "items",
                ItemContainer::Inventory,
                format!("items:{command}:entries"),
            ),
        };
        let Some(values) = data.get(field) else {
            receipts.fail(*key, format!("{command} omitted {field}"));
            continue;
        };
        let values: &[Value] = values.as_array().map_or(&[], Vec::as_slice);
        let accepted: Vec<_> = values
            .iter()
            .filter_map(|value| decode_item(value, container))
            .collect();
        let rejected = values.len() - accepted.len();
        items.replace(container, accepted);
        if rejected == 0 {
            receipts.complete(*key);
        } else {
            receipts.fail(
                *key,
                format!("{operation} contained {rejected} malformed item entries"),
            );
            tracing::warn!(
                command,
                ?container,
                rejected,
                "inventory baseline contains invalid entries"
            );
        }
    }
    true
}
