use crate::query::ItemSelector;

#[derive(Debug, serde::Deserialize, ts_rs::TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Operation {
    AcceptDrop { selector: ItemSelector },
    EquipConsumable { selector: ItemSelector },
    PrepareConsumableClaim { claim: String },
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, ts_rs::TS)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum OperationAnswer {
    AcceptDrop(bool),
    EquipConsumable(bool),
}
