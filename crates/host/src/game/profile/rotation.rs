use super::{ProfileSnapshot, SkillReadiness, TargetKey, matches_step, normalized_text};
use lucent_protocol::combat_profile::{
    CombatProfile, CombatProfileCooldownMode, CombatProfileMessageTrigger,
    CombatProfileMessageTriggerSource, SkillSlot,
};

#[derive(Clone, Debug, Default)]
pub struct Rotation {
    index: usize,
    reset_version: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedCast {
    pub skill: SkillSlot,
    pub readiness: CastReadiness,
    pub wait_ms: u32,
    next_index: Option<usize>,
    reset_version: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CastReadiness {
    IfReady,
    Wait,
}

pub fn skill_available(snapshot: &ProfileSnapshot, skill: SkillSlot, item: Option<i64>) -> bool {
    skill != SkillSlot::Consumable
        || item.is_none_or(|item| {
            snapshot
                .consumable
                .is_some_and(|slot| slot.item_id == item && slot.ready)
        })
}

impl Rotation {
    pub fn reset(&mut self) {
        self.index = 0;
        self.reset_version = self.reset_version.wrapping_add(1);
    }

    pub fn prepare(
        &self,
        profile: &CombatProfile,
        snapshot: &ProfileSnapshot,
        target: Option<TargetKey>,
        item: Option<i64>,
    ) -> Option<PreparedCast> {
        let prepare = |index: usize, priority: bool| {
            let step = &profile.steps[index];
            let mode = step.cooldown_mode.unwrap_or(profile.cooldown_mode);
            if !matches_step(snapshot, step, target)
                || !skill_available(snapshot, step.skill, item)
                || (mode == CombatProfileCooldownMode::UseIfReady
                    && snapshot.skills[usize::from(u8::from(step.skill))] != SkillReadiness::Ready)
            {
                return None;
            }
            Some(PreparedCast {
                skill: step.skill,
                readiness: match mode {
                    CombatProfileCooldownMode::UseIfReady => CastReadiness::IfReady,
                    CombatProfileCooldownMode::WaitForCooldown => CastReadiness::Wait,
                },
                wait_ms: step.wait_ms.unwrap_or(0),
                next_index: (!priority).then(|| (index + 1) % profile.steps.len()),
                reset_version: self.reset_version,
            })
        };
        profile
            .steps
            .iter()
            .enumerate()
            .filter(|(_, step)| step.priority == Some(true))
            .find_map(|(index, _)| prepare(index, true))
            .or_else(|| {
                (0..profile.steps.len())
                    .find_map(|offset| prepare((self.index + offset) % profile.steps.len(), false))
            })
    }

    pub fn complete(&mut self, cast: &PreparedCast, _succeeded: bool) {
        if self.reset_version == cast.reset_version
            && let Some(index) = cast.next_index
        {
            self.index = index;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageSource {
    Animation,
    Aura,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    pub text: String,
    pub source: MessageSource,
    pub monster_map_id: Option<i64>,
}

pub fn matches_message(trigger: &CombatProfileMessageTrigger, message: &Message) -> bool {
    let source_matches = match trigger.source {
        CombatProfileMessageTriggerSource::Any => true,
        CombatProfileMessageTriggerSource::Animation => message.source == MessageSource::Animation,
        CombatProfileMessageTriggerSource::Aura => message.source == MessageSource::Aura,
    };
    let expected = normalized_text(&trigger.message_includes);
    source_matches && !expected.is_empty() && normalized_text(&message.text).contains(&expected)
}
