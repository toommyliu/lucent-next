use serde::{Deserialize, Deserializer, Serialize, de::Error};
use ts_rs::TS;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(try_from = "f64", into = "u8")]
#[ts(type = "0 | 1 | 2 | 3 | 4 | 5")]
pub enum SkillSlot {
    AutoAttack,
    Skill1,
    Skill2,
    Skill3,
    Skill4,
    Consumable,
}

impl TryFrom<f64> for SkillSlot {
    type Error = &'static str;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        match value {
            0.0 => Ok(Self::AutoAttack),
            1.0 => Ok(Self::Skill1),
            2.0 => Ok(Self::Skill2),
            3.0 => Ok(Self::Skill3),
            4.0 => Ok(Self::Skill4),
            5.0 => Ok(Self::Consumable),
            _ => Err("Skill slot must be an integer between 0 and 5"),
        }
    }
}

impl From<SkillSlot> for u8 {
    fn from(value: SkillSlot) -> Self {
        value as Self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum CombatProfileCooldownMode {
    UseIfReady,
    WaitForCooldown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum CombatProfileThresholdUnit {
    Percent,
    Value,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub enum CombatProfileComparison {
    #[serde(rename = "<=")]
    LessThanOrEqual,
    #[serde(rename = ">=")]
    GreaterThanOrEqual,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum CombatProfileStatConditionKind {
    SelfHp,
    SelfMp,
    AllyHp,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CombatProfileStatCondition {
    #[serde(rename = "type")]
    #[ts(inline)]
    pub kind: CombatProfileStatConditionKind,
    pub op: CombatProfileComparison,
    #[serde(deserialize_with = "bounded_int::<999_999, _>")]
    pub value: u32,
    pub unit: CombatProfileThresholdUnit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum CombatProfileAuraConditionKind {
    SelfAura,
    TargetAura,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CombatProfileAuraCondition {
    #[serde(rename = "type")]
    #[ts(inline)]
    pub kind: CombatProfileAuraConditionKind,
    #[serde(deserialize_with = "trimmed_nonempty_string")]
    pub aura_name: String,
    pub op: CombatProfileComparison,
    #[serde(deserialize_with = "bounded_int::<999, _>")]
    pub value: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(untagged)]
pub enum CombatProfileCondition {
    Stat(CombatProfileStatCondition),
    Aura(CombatProfileAuraCondition),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(optional_fields)]
pub struct CombatProfileStep {
    pub skill: SkillSlot,
    pub conditions: Vec<CombatProfileCondition>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    pub priority: Option<bool>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    pub cooldown_mode: Option<CombatProfileCooldownMode>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "optional_milliseconds"
    )]
    pub wait_ms: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum CombatProfileMessageTriggerSource {
    Any,
    Animation,
    Aura,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(optional_fields)]
pub struct CombatProfileMessageTrigger {
    #[serde(deserialize_with = "trimmed_nonempty_string")]
    pub message_includes: String,
    pub skill: SkillSlot,
    pub source: CombatProfileMessageTriggerSource,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "optional_milliseconds"
    )]
    pub cooldown_ms: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(optional_fields)]
pub struct CombatProfile {
    #[serde(deserialize_with = "trimmed_nonempty_string")]
    pub id: String,
    #[serde(deserialize_with = "trimmed_nonempty_string")]
    pub label: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "class_names"
    )]
    pub class_names: Option<Vec<String>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "consumable"
    )]
    pub consumable: Option<String>,
    #[serde(deserialize_with = "bounded_int::<60_000, _>")]
    pub delay_ms: u32,
    pub cooldown_mode: CombatProfileCooldownMode,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    pub reset_skill_index_on_target_death: Option<bool>,
    pub steps: Vec<CombatProfileStep>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    pub message_triggers: Option<Vec<CombatProfileMessageTrigger>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CombatProfileLibrary {
    #[serde(deserialize_with = "library_version")]
    #[ts(type = "1")]
    pub version: u8,
    pub profiles: Vec<CombatProfile>,
}

fn bounded_int<'de, const MAX: u32, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<u32, D::Error> {
    let value = f64::deserialize(deserializer)?;
    if value.fract() != 0.0 || !(0.0..=f64::from(MAX)).contains(&value) {
        return Err(D::Error::custom(format!(
            "Expected an integer between 0 and {MAX}"
        )));
    }
    Ok(value as u32)
}

fn library_version<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u8, D::Error> {
    match f64::deserialize(deserializer)? {
        1.0 => Ok(1),
        _ => Err(D::Error::custom(
            "Expected combat profile library version 1",
        )),
    }
}

fn present<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(deserializer).map(Some)
}

fn optional_milliseconds<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<u32>, D::Error> {
    bounded_int::<60_000, _>(deserializer).map(Some)
}

fn trim_nonempty<E: Error>(value: String) -> Result<String, E> {
    let trimmed = value.trim_matches(|character| {
        matches!(character,
            '\u{0009}'..='\u{000d}' | '\u{0020}' | '\u{00a0}' | '\u{1680}' |
            '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' |
            '\u{205f}' | '\u{3000}' | '\u{feff}'
        )
    });
    if trimmed.is_empty() {
        return Err(E::custom("Expected a nonempty trimmed string"));
    }
    Ok(trimmed.to_owned())
}

fn trimmed_nonempty_string<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    trim_nonempty(String::deserialize(deserializer)?)
}

fn class_names<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Vec<String>>, D::Error> {
    Vec::<String>::deserialize(deserializer)?
        .into_iter()
        .map(trim_nonempty)
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

fn consumable<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<String>, D::Error> {
    trimmed_nonempty_string(deserializer).map(Some)
}
