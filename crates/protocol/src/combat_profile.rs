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

pub fn normalize_combat_profile(value: &serde_json::Value) -> CombatProfile {
    normalize_profile(value, Some("generic-base")).unwrap_or_else(generic_profile)
}

#[derive(Clone, Debug, PartialEq)]
pub struct UnsupportedCombatProfileVersion(pub f64);

impl std::fmt::Display for UnsupportedCombatProfileVersion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "Unsupported combat profile library version {}",
            self.0
        )
    }
}

impl std::error::Error for UnsupportedCombatProfileVersion {}

pub fn normalize_combat_profile_library(
    value: &serde_json::Value,
) -> Result<CombatProfileLibrary, UnsupportedCombatProfileVersion> {
    use std::collections::{HashMap, HashSet};
    if let Some(version) = value["version"].as_f64()
        && version.fract() == 0.0
        && version > 1.0
    {
        return Err(UnsupportedCombatProfileVersion(version));
    }
    let mut profiles: Vec<_> = array(&value["profiles"])
        .iter()
        .filter_map(|value| normalize_profile(value, None))
        .collect();
    match profiles
        .iter()
        .position(|profile| profile.id == "generic-base")
    {
        None => profiles.insert(0, generic_profile()),
        Some(index) => {
            let generic = profiles.remove(index);
            profiles.insert(0, generic);
        }
    }
    let reserved: HashSet<_> = profiles.iter().map(|profile| profile.id.clone()).collect();
    let mut used = HashSet::new();
    let mut suffixes = HashMap::new();
    for profile in &mut profiles {
        if used.insert(profile.id.clone()) {
            continue;
        }
        let suffix = suffixes.entry(profile.id.clone()).or_insert(2_u64);
        loop {
            let text = format!("-{suffix}");
            let id = format!("{}{text}", truncate_utf16(&profile.id, 80 - text.len()));
            *suffix += 1;
            if !reserved.contains(&id) && used.insert(id.clone()) {
                profile.id = id;
                break;
            }
        }
    }
    Ok(CombatProfileLibrary {
        version: 1,
        profiles,
    })
}

fn array(value: &serde_json::Value) -> &[serde_json::Value] {
    value.as_array().map(Vec::as_slice).unwrap_or_default()
}

fn normalized_int(value: &serde_json::Value, fallback: u32, max: u32) -> u32 {
    value
        .as_f64()
        .map(|value| value.trunc().clamp(0.0, f64::from(max)) as u32)
        .unwrap_or(fallback)
}

fn truncate_utf16(value: &str, limit: usize) -> String {
    let mut length = 0;
    value
        .chars()
        .take_while(|character| {
            length += character.len_utf16();
            length <= limit
        })
        .collect()
}

fn normalized_string(value: &serde_json::Value, limit: usize) -> Option<String> {
    let trimmed = trim_nonempty::<serde_json::Error>(value.as_str()?.to_owned()).ok()?;
    Some(truncate_utf16(&trimmed, limit)).filter(|value| !value.is_empty())
}

fn normalized_enum<T: serde::de::DeserializeOwned>(value: &serde_json::Value) -> Option<T> {
    serde_json::from_value(value.clone()).ok()
}

fn normalized_condition(value: &serde_json::Value) -> Option<CombatProfileCondition> {
    let op = normalized_enum(&value["op"]).unwrap_or(CombatProfileComparison::LessThanOrEqual);
    if let Some(kind) = normalized_enum(&value["type"]) {
        let unit = normalized_enum(&value["unit"]).unwrap_or(CombatProfileThresholdUnit::Percent);
        let max = match unit {
            CombatProfileThresholdUnit::Percent => 100,
            CombatProfileThresholdUnit::Value => 999_999,
        };
        return Some(CombatProfileCondition::Stat(CombatProfileStatCondition {
            kind,
            op,
            value: normalized_int(&value["value"], 0, max),
            unit,
        }));
    }
    Some(CombatProfileCondition::Aura(CombatProfileAuraCondition {
        kind: normalized_enum(&value["type"])?,
        aura_name: normalized_string(&value["auraName"], 80)?,
        op,
        value: normalized_int(&value["value"], 0, 999),
    }))
}

fn normalized_skill(value: &serde_json::Value) -> Option<SkillSlot> {
    value
        .as_f64()
        .and_then(|number| SkillSlot::try_from(number.trunc().clamp(0.0, 5.0)).ok())
}

fn normalized_step(value: &serde_json::Value) -> Option<CombatProfileStep> {
    let wait = normalized_int(&value["waitMs"], 0, 60_000);
    Some(CombatProfileStep {
        skill: normalized_skill(&value["skill"])?,
        conditions: array(&value["conditions"])
            .iter()
            .filter_map(normalized_condition)
            .collect(),
        priority: (value["priority"].as_bool() == Some(true)).then_some(true),
        cooldown_mode: normalized_enum(&value["cooldownMode"]),
        wait_ms: (wait > 0).then_some(wait),
    })
}

fn normalized_trigger(value: &serde_json::Value) -> Option<CombatProfileMessageTrigger> {
    let cooldown = normalized_int(&value["cooldownMs"], 0, 60_000);
    Some(CombatProfileMessageTrigger {
        message_includes: normalized_string(&value["messageIncludes"], 160)?,
        skill: normalized_skill(&value["skill"])?,
        source: normalized_enum(&value["source"]).unwrap_or(CombatProfileMessageTriggerSource::Any),
        cooldown_ms: (cooldown > 0).then_some(cooldown),
    })
}

fn normalize_profile(
    value: &serde_json::Value,
    fallback_id: Option<&str>,
) -> Option<CombatProfile> {
    value.as_object()?;
    let class_names: Vec<_> = array(&value["classNames"])
        .iter()
        .filter_map(|value| normalized_string(value, 80))
        .collect();
    let triggers: Vec<_> = array(&value["messageTriggers"])
        .iter()
        .filter_map(normalized_trigger)
        .collect();
    Some(CombatProfile {
        id: normalized_string(&value["id"], 80).or_else(|| fallback_id.map(str::to_owned))?,
        label: normalized_string(&value["label"], 80).unwrap_or_else(|| "Profile".into()),
        class_names: (!class_names.is_empty()).then_some(class_names),
        consumable: normalized_string(&value["consumable"], 80),
        delay_ms: normalized_int(&value["delayMs"], 150, 60_000),
        cooldown_mode: normalized_enum(&value["cooldownMode"])
            .unwrap_or(CombatProfileCooldownMode::UseIfReady),
        reset_skill_index_on_target_death: (value["resetSkillIndexOnTargetDeath"].as_bool()
            == Some(true))
        .then_some(true),
        steps: array(&value["steps"])
            .iter()
            .filter_map(normalized_step)
            .collect(),
        message_triggers: (!triggers.is_empty()).then_some(triggers),
    })
}

fn generic_profile() -> CombatProfile {
    CombatProfile {
        id: "generic-base".into(),
        label: "Generic".into(),
        class_names: None,
        consumable: None,
        delay_ms: 150,
        cooldown_mode: CombatProfileCooldownMode::UseIfReady,
        reset_skill_index_on_target_death: None,
        message_triggers: None,
        steps: [
            SkillSlot::Skill1,
            SkillSlot::Skill2,
            SkillSlot::Skill3,
            SkillSlot::Skill4,
        ]
        .into_iter()
        .map(|skill| CombatProfileStep {
            skill,
            conditions: Vec::new(),
            priority: None,
            cooldown_mode: None,
            wait_ms: None,
        })
        .collect(),
    }
}
