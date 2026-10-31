use lucent_protocol::combat_profile::{
    CombatProfileAuraConditionKind, CombatProfileComparison, CombatProfileCondition,
    CombatProfileStatConditionKind, CombatProfileStep, CombatProfileThresholdUnit,
};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Stat {
    pub current: f64,
    pub maximum: f64,
}

impl Stat {
    fn value(self, unit: CombatProfileThresholdUnit) -> f64 {
        match unit {
            CombatProfileThresholdUnit::Value => self.current,
            CombatProfileThresholdUnit::Percent if self.maximum > 0.0 => {
                self.current / self.maximum * 100.0
            }
            CombatProfileThresholdUnit::Percent => 0.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Aura {
    pub name: String,
    pub stack: Option<f64>,
    pub value: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Ally {
    pub entity_id: i64,
    pub username: String,
    pub hp: Stat,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SkillReadiness {
    Ready,
    CooldownUntil(u64),
    #[default]
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TargetKey {
    pub monster_map_id: i64,
    pub generation: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Monster {
    pub key: TargetKey,
    pub hp: f64,
    pub state: LifeState,
    pub rank: Option<usize>,
    pub attack: AttackEligibility,
    pub auras: Vec<Aura>,
}

impl Monster {
    pub fn living(&self) -> bool {
        self.hp > 0.0 && self.state == LifeState::Alive
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LifeState {
    Alive,
    Dead,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttackEligibility {
    Allowed,
    Blocked,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum SelectedTarget {
    #[default]
    None,
    Monster(TargetKey),
    Player {
        alive: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConsumableSlot {
    pub item_id: i64,
    pub ready: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProfileSnapshot {
    pub alive: bool,
    pub hp: Stat,
    pub mp: Stat,
    pub self_auras: Vec<Aura>,
    pub me: Option<Ally>,
    pub allies: Vec<Ally>,
    pub monsters: Vec<Monster>,
    pub selected: SelectedTarget,
    pub skills: [SkillReadiness; 6],
    pub consumable: Option<ConsumableSlot>,
}

pub(crate) fn js_space(character: char) -> bool {
    matches!(character, '\u{0009}'..='\u{000d}' | '\u{0020}' | '\u{00a0}' | '\u{1680}' |
        '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}')
}

pub(crate) fn normalized_text(value: &str) -> String {
    value
        .split(js_space)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn aura_value(auras: &[Aura], name: &str) -> f64 {
    auras
        .iter()
        .find(|aura| {
            aura.name.trim_matches(js_space).to_lowercase()
                == name.trim_matches(js_space).to_lowercase()
        })
        .map(|aura| aura.stack.or(aura.value).unwrap_or(1.0))
        .unwrap_or(0.0)
}

fn compare(actual: f64, op: CombatProfileComparison, expected: u32) -> bool {
    match op {
        CombatProfileComparison::LessThanOrEqual => actual <= f64::from(expected),
        CombatProfileComparison::GreaterThanOrEqual => actual >= f64::from(expected),
    }
}

pub fn matches_step(
    snapshot: &ProfileSnapshot,
    step: &CombatProfileStep,
    target: Option<TargetKey>,
) -> bool {
    step.conditions.iter().all(|condition| match condition {
        CombatProfileCondition::Stat(condition) => {
            let matches =
                |stat: Stat| compare(stat.value(condition.unit), condition.op, condition.value);
            match condition.kind {
                CombatProfileStatConditionKind::SelfHp => matches(snapshot.hp),
                CombatProfileStatConditionKind::SelfMp => matches(snapshot.mp),
                CombatProfileStatConditionKind::AllyHp => {
                    snapshot.me.as_ref().is_some_and(|me| matches(me.hp))
                        || snapshot
                            .allies
                            .iter()
                            .filter(|ally| {
                                snapshot.me.as_ref().is_none_or(|me| {
                                    me.entity_id != ally.entity_id
                                        && me.username.trim_matches(js_space).to_lowercase()
                                            != ally.username.trim_matches(js_space).to_lowercase()
                                })
                            })
                            .any(|ally| matches(ally.hp))
                }
            }
        }
        CombatProfileCondition::Aura(condition) => {
            let actual = match condition.kind {
                CombatProfileAuraConditionKind::SelfAura => {
                    aura_value(&snapshot.self_auras, &condition.aura_name)
                }
                CombatProfileAuraConditionKind::TargetAura => {
                    let selected = match snapshot.selected {
                        SelectedTarget::Monster(key) => Some(key),
                        _ => None,
                    };
                    if let Some(key) = target.or(selected) {
                        snapshot
                            .monsters
                            .iter()
                            .find(|monster| monster.key == key)
                            .map(|monster| aura_value(&monster.auras, &condition.aura_name))
                            .unwrap_or(0.0)
                    } else if matches!(snapshot.selected, SelectedTarget::Player { .. }) {
                        aura_value(&snapshot.self_auras, &condition.aura_name)
                    } else {
                        0.0
                    }
                }
            };
            compare(actual, condition.op, condition.value)
        }
    })
}

pub fn select_target(
    snapshot: &ProfileSnapshot,
    current: Option<TargetKey>,
    fallback_rank: Option<usize>,
) -> Option<&Monster> {
    let rank = |monster: &Monster| monster.rank.or(fallback_rank);
    let eligible = |monster: &&Monster| {
        monster.living() && monster.attack == AttackEligibility::Allowed && rank(monster).is_some()
    };
    let best = snapshot
        .monsters
        .iter()
        .filter(eligible)
        .min_by_key(|monster| rank(monster))?;
    snapshot
        .monsters
        .iter()
        .filter(eligible)
        .find(|monster| Some(monster.key) == current && rank(monster) <= rank(best))
        .or(Some(best))
}
