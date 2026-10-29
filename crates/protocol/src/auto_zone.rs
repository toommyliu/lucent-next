#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "lowercase")]
pub enum AutoZoneMap {
    Ledgermayne,
    Moreskulls,
    Ultradage,
    Darkcarnax,
    Astralshrine,
    Queeniona,
    Magnumopus,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
pub struct AutoZoneConfig {
    pub enabled: bool,
    pub map: Option<AutoZoneMap>,
}

#[derive(Debug, Default, serde::Deserialize, ts_rs::TS)]
#[ts(optional_fields = nullable)]
pub struct AutoZonePatch {
    pub enabled: Option<bool>,
    #[serde(default, with = "present_map")]
    #[ts(as = "Option<Option<AutoZoneMap>>")]
    pub map: Option<Option<AutoZoneMap>>,
}

mod present_map {
    use super::AutoZoneMap;
    use serde::Deserialize;

    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<Option<AutoZoneMap>>, D::Error> {
        Option::deserialize(deserializer).map(Some)
    }
}
