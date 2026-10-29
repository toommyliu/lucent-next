#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AutoReloginConfig {
    pub enabled: bool,
    pub delay_ms: f64,
    pub server: Option<String>,
}
impl Default for AutoReloginConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            delay_ms: 3000.0,
            server: None,
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(
    tag = "tag",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum AutoReloginPhase {
    #[default]
    Armed,
    AwaitingSession,
    AwaitingPlayerReady,
    AwaitingServerSelect,
    Backoff {
        delay_ms: f64,
    },
    Failed,
    LoggingIn,
    SelectingServer,
    WaitingDelay,
    WaitingKick,
}
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AutoReloginStatus {
    pub phase: AutoReloginPhase,
    pub attempts_remaining: Option<f64>,
    pub last_error: Option<String>,
    pub captured: bool,
}
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct AutoReloginRecords {
    pub config: AutoReloginConfig,
    pub status: AutoReloginStatus,
}
