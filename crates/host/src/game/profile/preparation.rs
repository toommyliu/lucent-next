use lucent_protocol::combat_profile::{CombatProfile, SkillSlot};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConsumableItem {
    pub item_id: i64,
    pub name: String,
    pub category: String,
    pub link: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConsumableClaim {
    First,
    Shared,
    Conflict,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConsumablePlan {
    Unused,
    ClaimCurrent(Option<i64>),
    Equip { item_id: i64 },
    Ready { item_id: i64 },
    Unavailable(String),
}

pub fn plan_consumable(
    profile: &CombatProfile,
    item: Option<&ConsumableItem>,
    current: Option<i64>,
    claim: ConsumableClaim,
    idle: bool,
) -> ConsumablePlan {
    let uses = profile.consumable.is_some()
        || profile
            .steps
            .iter()
            .any(|step| step.skill == SkillSlot::Consumable)
        || profile
            .message_triggers
            .iter()
            .flatten()
            .any(|trigger| trigger.skill == SkillSlot::Consumable);
    if !uses {
        return ConsumablePlan::Unused;
    }
    let Some(name) = &profile.consumable else {
        return ConsumablePlan::ClaimCurrent(current);
    };
    let unavailable = |message: String| {
        ConsumablePlan::Unavailable(format!(
            "{message} Skill 5 will use whichever consumable is equipped."
        ))
    };
    let Some(item) = item else {
        return unavailable(format!("{name} was not found."));
    };
    let link = item.link.trim_matches(super::js_space).to_lowercase();
    if item.category != "Item" || matches!(link.as_str(), "" | "none" | "elixir" | "tonic") {
        return unavailable(format!("{} cannot be equipped.", item.name));
    }
    match claim {
        ConsumableClaim::Conflict => unavailable(format!(
            "Could not equip {} because another combat profile controls preflight equipment.",
            item.name
        )),
        ConsumableClaim::Shared => ConsumablePlan::Ready {
            item_id: item.item_id,
        },
        ConsumableClaim::First if current == Some(item.item_id) => ConsumablePlan::Ready {
            item_id: item.item_id,
        },
        ConsumableClaim::First if !idle => unavailable(format!(
            "Could not equip {} while you were in combat.",
            item.name
        )),
        ConsumableClaim::First => ConsumablePlan::Equip {
            item_id: item.item_id,
        },
    }
}

pub fn class_warning(
    profile: &CombatProfile,
    equipped_item: Option<&str>,
    class_name: &str,
) -> Option<String> {
    let classes = profile.class_names.as_deref().unwrap_or_default();
    if classes.is_empty()
        || classes.iter().any(|expected| {
            expected.to_lowercase() == equipped_item.unwrap_or_default().to_lowercase()
                || expected.to_lowercase() == class_name.to_lowercase()
        })
    {
        return None;
    }
    let equipped = equipped_item
        .filter(|name| !name.is_empty())
        .unwrap_or(class_name);
    if equipped.is_empty() {
        return None;
    }
    let list = match classes {
        [only] => only.clone(),
        [first, second] => format!("{first} or {second}"),
        _ => format!(
            "{}, or {}",
            classes[..classes.len() - 1].join(", "),
            classes.last()?
        ),
    };
    Some(format!(
        "This profile is for {list}, but {equipped} is equipped."
    ))
}
