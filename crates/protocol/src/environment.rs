use indexmap::IndexMap;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
pub struct EnvironmentAutomation {
    pub boosts: bool,
    pub drops: bool,
    pub quests: bool,
}
impl Default for EnvironmentAutomation {
    fn default() -> Self {
        Self {
            boosts: true,
            drops: true,
            quests: true,
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
pub struct EnvironmentQuestAutoRegister {
    pub requirements: bool,
    pub rewards: bool,
}
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
    ts_rs::TS,
)]
#[serde(rename_all = "kebab-case")]
pub enum EnvironmentItemBucket {
    AcMember,
    AcNonMember,
    NonAcMember,
    NonAcNonMember,
}
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentItemRules {
    pub buckets: Vec<EnvironmentItemBucket>,
    pub reject_else: bool,
}
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentState {
    pub automation: EnvironmentAutomation,
    pub quest_ids: Vec<f64>,
    pub quest_auto_register: EnvironmentQuestAutoRegister,
    #[ts(type = "{ [key: number]: number }")]
    pub quest_rewards: IndexMap<String, f64>,
    pub item_names: Vec<String>,
    pub item_notification_names: Vec<String>,
    pub item_rules: EnvironmentItemRules,
    pub boosts: Vec<String>,
}
#[derive(Clone, Debug, Default, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(optional_fields)]
pub struct EnvironmentDropPolicyPatch {
    pub accept_ac_member_only_drops: Option<bool>,
    pub accept_ac_non_member_drops: Option<bool>,
    pub accept_non_ac_member_only_drops: Option<bool>,
    pub accept_non_ac_non_member_drops: Option<bool>,
    pub reject_unregistered_drops: Option<bool>,
}
#[derive(Debug, serde::Deserialize, ts_rs::TS)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(optional_fields)]
pub enum EnvironmentEdit {
    Clear {},
    AddQuest {
        quest_id: f64,
        reward_item_id: Option<f64>,
    },
    RemoveQuest {
        quest_id: f64,
    },
    SetQuestReward {
        quest_id: f64,
        reward_item_id: f64,
    },
    ClearQuestReward {
        quest_id: f64,
    },
    ClearQuests {},
    SetQuestAutoRegister {
        options: EnvironmentQuestAutoRegister,
    },
    SetAutomationEnabled {
        capability: EnvironmentAutomationCapability,
        enabled: bool,
    },
    AddItem {
        name: String,
    },
    RemoveItem {
        name: String,
    },
    SetItemNotification {
        name: String,
        enabled: bool,
    },
    SetItemRules {
        rules: EnvironmentItemRules,
    },
    UpdateDropPolicy {
        patch: EnvironmentDropPolicyPatch,
    },
    ClearItems {},
    AddBoost {
        name: String,
    },
    RemoveBoost {
        name: String,
    },
    ClearBoosts {},
}
#[derive(Debug, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "lowercase")]
pub enum EnvironmentAutomationCapability {
    Boosts,
    Drops,
    Quests,
}
