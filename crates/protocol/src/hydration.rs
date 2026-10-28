use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub enum HydrationMethod {
    #[serde(rename = "player.getUserId")]
    UserId,
    #[serde(rename = "player.getData")]
    PlayerData,
    #[serde(rename = "world.isLoaded")]
    WorldLoaded,
    #[serde(rename = "player.getCell")]
    Cell,
    #[serde(rename = "player.getPad")]
    Pad,
}
impl HydrationMethod {
    pub fn name(self) -> &'static str {
        match self {
            Self::UserId => "player.getUserId",
            Self::PlayerData => "player.getData",
            Self::WorldLoaded => "world.isLoaded",
            Self::Cell => "player.getCell",
            Self::Pad => "player.getPad",
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, ts_rs::TS)]
pub struct HydrationRequest {
    pub method: HydrationMethod,
    #[ts(type = "Array<unknown>")]
    pub args: Vec<Value>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ts_rs::TS)]
#[serde(tag = "status", content = "value", rename_all = "kebab-case")]
pub enum HydrationResult {
    Unavailable,
    Value(#[ts(type = "unknown")] Value),
}
impl HydrationResult {
    pub fn value(&self) -> Option<&Value> {
        match self {
            Self::Unavailable => None,
            Self::Value(value) => Some(value),
        }
    }
}
