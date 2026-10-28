use serde::{Serialize, Serializer, ser::SerializeStruct};
use ts_rs::TS;

use crate::item::ItemData;

#[derive(Clone, Debug, PartialEq, TS)]
#[ts(rename_all = "camelCase")]
pub struct ShopData {
    pub house: bool,
    pub id: i64,
    #[ts(as = "Vec<crate::item::ItemSnapshot>")]
    pub items: Vec<ItemData>,
    pub limited: bool,
    pub merge: bool,
    pub name: String,
}

impl Serialize for ShopData {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut fields = serializer.serialize_struct("ShopData", 6)?;
        fields.serialize_field("house", &self.house)?;
        fields.serialize_field("id", &self.id)?;
        fields.serialize_field(
            "items",
            &self
                .items
                .iter()
                .map(ItemData::snapshot)
                .collect::<Vec<_>>(),
        )?;
        fields.serialize_field("limited", &self.limited)?;
        fields.serialize_field("merge", &self.merge)?;
        fields.serialize_field("name", &self.name)?;
        fields.end()
    }
}
