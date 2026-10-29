use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct ServerData {
    pub chat: f64,
    pub count: f64,
    pub language: String,
    pub max: f64,
    pub member_only: bool,
    pub name: String,
    pub online: bool,
}

#[derive(Clone, Debug, Serialize, ts_rs::TS)]
pub struct ServerSnapshot {
    #[serde(flatten)]
    pub data: ServerData,
    pub full: bool,
}

#[derive(Clone, Debug, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct AuthSnapshot {
    pub logged_in: bool,
    pub username: String,
    pub password: String,
    pub servers: Vec<ServerSnapshot>,
}
