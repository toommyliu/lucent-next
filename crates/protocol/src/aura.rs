use serde::{Deserialize, Serialize};

use crate::event::AuraKind;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(untagged)]
pub enum AuraValue {
    Number(f64),
    Text(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(optional_fields)]
pub struct AuraData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    pub duration: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    pub kind: AuraKind,
    pub name: String,
    pub persistent: bool,
    pub stack: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<AuraValue>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AuraTiming {
    Untimed,
    Seconds(f64),
}

#[derive(Clone, Debug, PartialEq)]
pub enum SeedTimer {
    Untimed,
    Timed {
        remaining_seconds: f64,
        full_seconds: f64,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct AuraSeed {
    pub name: String,
    pub icon: String,
    pub stack: f64,
    pub persistent: bool,
    pub timer: SeedTimer,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AuraApplication {
    pub name: String,
    pub stack: f64,
    pub timing: AuraTiming,
    pub persistent: bool,
    pub restart_duration: bool,
    pub icon: Option<String>,
    pub category: Option<String>,
    pub value: Option<AuraValue>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AuraWithdrawal {
    Remove {
        name: String,
    },
    Decay {
        name: String,
        stack: f64,
        refresh_seconds: Option<f64>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct PassiveAuraInput {
    pub name: String,
    pub category: Option<String>,
    pub icon: Option<String>,
    pub value: Option<AuraValue>,
    pub duration: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PassiveMode {
    Replace,
    Merge,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AuraMutation {
    Seed(Vec<AuraSeed>),
    Apply(Vec<AuraApplication>),
    Withdraw(Vec<AuraWithdrawal>),
    SetStack(Vec<(String, f64)>),
    Passives {
        mode: PassiveMode,
        entries: Vec<PassiveAuraInput>,
    },
    Clear {
        keep_persistent: bool,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum AuraDelta {
    Added { after: AuraData },
    Updated { before: AuraData, after: AuraData },
    Removed { before: AuraData },
}
