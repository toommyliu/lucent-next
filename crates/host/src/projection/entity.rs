use std::sync::{Arc, RwLock};

use indexmap::IndexMap;
use lucent_protocol::aura::{
    AuraApplication, AuraData, AuraDelta, AuraMutation, AuraSeed, AuraTiming, AuraWithdrawal,
    PassiveMode, SeedTimer,
};
use lucent_protocol::event::AuraKind;
use lucent_protocol::monster_drop::MonsterDrop;
use lucent_protocol::world::{
    EntityState, MonsterData, MonsterSnapshot, PlayerData, PlayerSnapshot,
};
use serde_json::Value;

#[derive(Clone, Debug)]
pub struct LiveAura(Arc<RwLock<AuraData>>);

impl LiveAura {
    pub fn data(&self) -> AuraData {
        self.0.read().unwrap().clone()
    }
    pub fn same_instance(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Debug)]
pub struct AuraState {
    values: IndexMap<String, LiveAura>,
    accepts_active_sync: bool,
}

impl Default for AuraState {
    fn default() -> Self {
        Self::new()
    }
}

impl AuraState {
    pub fn new() -> Self {
        Self {
            values: IndexMap::new(),
            accepts_active_sync: true,
        }
    }
    pub fn all(&self) -> Vec<LiveAura> {
        self.values.values().cloned().collect()
    }
    pub fn get(&self, name: &str, kind: Option<AuraKind>) -> Option<LiveAura> {
        let name = super::wire::trim_js(name).to_lowercase();
        self.values
            .values()
            .find(|aura| {
                let aura = aura.0.read().unwrap();
                super::wire::trim_js(&aura.name).to_lowercase() == name
                    && kind.is_none_or(|kind| kind == aura.kind)
            })
            .cloned()
    }
    fn commit(
        &mut self,
        kind: AuraKind,
        name: &str,
        edit: impl FnOnce(Option<&AuraData>) -> Option<AuraData>,
        changes: &mut Vec<AuraDelta>,
    ) {
        let key = format!(
            "{}:{name}",
            if kind == AuraKind::Active {
                "active"
            } else {
                "passive"
            }
        );
        let current = self.values.get(&key).cloned();
        let before = current.as_ref().map(LiveAura::data);
        let after = edit(before.as_ref());
        match (current, before, after) {
            (None, None, Some(after)) => {
                self.values
                    .insert(key, LiveAura(Arc::new(RwLock::new(after.clone()))));
                changes.push(AuraDelta::Added { after });
            }
            (Some(_), Some(before), None) => {
                self.values.shift_remove(&key);
                changes.push(AuraDelta::Removed { before });
            }
            (Some(current), Some(before), Some(after)) if before != after => {
                *current.0.write().unwrap() = after.clone();
                changes.push(AuraDelta::Updated { before, after });
            }
            _ => {}
        }
    }
    pub fn project(&mut self, mutation: AuraMutation, now_ms: f64) -> Vec<AuraDelta> {
        if !self.accepts_active_sync
            && matches!(
                mutation,
                AuraMutation::Apply(_) | AuraMutation::Withdraw(_) | AuraMutation::SetStack(_)
            )
        {
            return vec![];
        }
        let mut changes = Vec::new();
        match mutation {
            AuraMutation::Seed(entries) => {
                let entries: IndexMap<_, _> = entries
                    .into_iter()
                    .map(|entry| (entry.name.clone(), entry))
                    .collect();
                let removed: Vec<_> = self
                    .values
                    .values()
                    .map(LiveAura::data)
                    .filter(|aura| {
                        aura.kind == AuraKind::Active && !entries.contains_key(&aura.name)
                    })
                    .collect();
                for aura in removed {
                    self.commit(AuraKind::Active, &aura.name, |_| None, &mut changes);
                }
                for entry in entries.values() {
                    self.commit(
                        AuraKind::Active,
                        &entry.name,
                        |before| Some(seed(before, entry, now_ms)),
                        &mut changes,
                    );
                }
            }
            AuraMutation::Apply(entries) => {
                for entry in entries {
                    self.commit(
                        AuraKind::Active,
                        &entry.name,
                        |before| Some(apply(before, &entry, now_ms)),
                        &mut changes,
                    );
                }
            }
            AuraMutation::Withdraw(entries) => {
                for entry in entries {
                    match entry {
                        AuraWithdrawal::Remove { name } => {
                            self.commit(AuraKind::Active, &name, |_| None, &mut changes)
                        }
                        AuraWithdrawal::Decay {
                            name,
                            stack,
                            refresh_seconds,
                        } => self.commit(
                            AuraKind::Active,
                            &name,
                            |before| {
                                before.map(|before| {
                                    let mut after = before.clone();
                                    after.stack = stack;
                                    if let Some(seconds) =
                                        refresh_seconds.filter(|_| before.expires_at.is_some())
                                    {
                                        after.duration = before.duration.max(seconds);
                                        after.expires_at = Some(now_ms + seconds * 1000.0);
                                    }
                                    after
                                })
                            },
                            &mut changes,
                        ),
                    }
                }
            }
            AuraMutation::SetStack(entries) => {
                for (name, stack) in entries {
                    self.commit(
                        AuraKind::Active,
                        &name,
                        |before| {
                            before.map(|before| {
                                let mut after = before.clone();
                                after.stack = stack.max(1.0);
                                after
                            })
                        },
                        &mut changes,
                    );
                }
            }
            AuraMutation::Passives { mode, entries } => {
                let entries: IndexMap<_, _> = entries
                    .into_iter()
                    .map(|entry| (entry.name.clone(), entry))
                    .collect();
                if mode == PassiveMode::Replace {
                    let removed: Vec<_> = self
                        .values
                        .values()
                        .map(LiveAura::data)
                        .filter(|aura| {
                            aura.kind == AuraKind::Passive && !entries.contains_key(&aura.name)
                        })
                        .collect();
                    for aura in removed {
                        self.commit(AuraKind::Passive, &aura.name, |_| None, &mut changes);
                    }
                }
                for entry in entries.values() {
                    self.commit(
                        AuraKind::Passive,
                        &entry.name,
                        |_| {
                            Some(AuraData {
                                category: entry.category.clone(),
                                duration: entry.duration,
                                expires_at: None,
                                icon: entry.icon.clone(),
                                kind: AuraKind::Passive,
                                name: entry.name.clone(),
                                persistent: false,
                                stack: 1.0,
                                value: entry.value.clone(),
                            })
                        },
                        &mut changes,
                    );
                }
            }
            AuraMutation::Clear { keep_persistent } => {
                let removed: Vec<_> = self
                    .values
                    .values()
                    .map(LiveAura::data)
                    .filter(|aura| {
                        aura.kind == AuraKind::Active && (!keep_persistent || !aura.persistent)
                    })
                    .collect();
                for aura in removed {
                    self.commit(AuraKind::Active, &aura.name, |_| None, &mut changes);
                }
            }
        }
        changes
    }
    pub fn write_monster_state(&mut self, state: EntityState) -> Vec<AuraDelta> {
        self.accepts_active_sync = state != EntityState::Dead;
        if state == EntityState::InCombat {
            vec![]
        } else {
            self.project(
                AuraMutation::Clear {
                    keep_persistent: false,
                },
                0.0,
            )
        }
    }
}

fn seed(before: Option<&AuraData>, entry: &AuraSeed, now_ms: f64) -> AuraData {
    let (duration, expires_at) = match entry.timer {
        SeedTimer::Untimed => (0.0, None),
        SeedTimer::Timed {
            remaining_seconds,
            full_seconds,
        } => (
            full_seconds.max(remaining_seconds),
            Some(now_ms + remaining_seconds * 1000.0),
        ),
    };
    AuraData {
        category: before.and_then(|before| before.category.clone()),
        duration,
        expires_at,
        icon: Some(entry.icon.clone()),
        kind: AuraKind::Active,
        name: entry.name.clone(),
        persistent: entry.persistent,
        stack: entry.stack.max(1.0),
        value: before.and_then(|before| before.value.clone()),
    }
}
fn apply(before: Option<&AuraData>, entry: &AuraApplication, now_ms: f64) -> AuraData {
    let (duration, expires_at) = match entry.timing {
        AuraTiming::Untimed => (0.0, None),
        AuraTiming::Seconds(duration) => (
            if entry.restart_duration {
                duration
            } else {
                before.map_or(0.0, |before| before.duration).max(duration)
            },
            Some(now_ms + duration * 1000.0),
        ),
    };
    AuraData {
        category: entry
            .category
            .clone()
            .or_else(|| before.and_then(|before| before.category.clone())),
        duration,
        expires_at,
        icon: entry
            .icon
            .clone()
            .or_else(|| before.and_then(|before| before.icon.clone())),
        kind: AuraKind::Active,
        name: entry.name.clone(),
        persistent: entry.persistent,
        stack: entry.stack,
        value: entry
            .value
            .clone()
            .or_else(|| before.and_then(|before| before.value.clone())),
    }
}

#[derive(Debug)]
pub struct PlayerRecord {
    pub data: PlayerData,
    pub auras: AuraState,
}
#[derive(Clone, Debug)]
pub struct LivePlayer(pub(crate) Arc<RwLock<PlayerRecord>>);
impl LivePlayer {
    pub fn new(data: PlayerData) -> Self {
        Self(Arc::new(RwLock::new(PlayerRecord {
            data,
            auras: AuraState::new(),
        })))
    }
    pub fn data(&self) -> PlayerData {
        self.0.read().unwrap().data.clone()
    }
    pub fn same_instance(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
    pub fn snapshot(&self) -> Value {
        serde_json::to_value(self.snapshot_data()).unwrap()
    }
    pub fn snapshot_data(&self) -> PlayerSnapshot {
        let record = self.0.read().unwrap();
        PlayerSnapshot {
            data: record.data.clone(),
            fields: record
                .data
                .entity
                .snapshot_fields(record.auras.all().iter().map(LiveAura::data).collect()),
        }
    }
    pub fn auras(&self) -> Vec<LiveAura> {
        self.0.read().unwrap().auras.all()
    }
    pub fn aura(&self, name: &str, kind: Option<AuraKind>) -> Option<LiveAura> {
        self.0.read().unwrap().auras.get(name, kind)
    }
    pub fn project_auras(&self, mutation: AuraMutation, now_ms: f64) -> Vec<AuraDelta> {
        self.0.write().unwrap().auras.project(mutation, now_ms)
    }
    pub(crate) fn update(&self, update: impl FnOnce(&mut PlayerData)) {
        update(&mut self.0.write().unwrap().data);
    }
    pub(crate) fn replace(&self, data: PlayerData) {
        self.0.write().unwrap().data = data;
    }
}

#[derive(Debug)]
pub struct MonsterRecord {
    pub data: MonsterData,
    pub generation: u64,
    pub auras: AuraState,
    pub drops: IndexMap<i64, MonsterDrop>,
}
#[derive(Clone, Debug)]
pub struct LiveMonster(pub(crate) Arc<RwLock<MonsterRecord>>);
impl LiveMonster {
    pub fn new(data: MonsterData, explicit_state: bool) -> Self {
        let mut auras = AuraState::new();
        if explicit_state {
            auras.write_monster_state(data.entity.state);
        }
        Self(Arc::new(RwLock::new(MonsterRecord {
            data,
            generation: 0,
            auras,
            drops: IndexMap::new(),
        })))
    }
    pub fn data(&self) -> MonsterData {
        self.0.read().unwrap().data.clone()
    }
    pub fn generation(&self) -> u64 {
        self.0.read().unwrap().generation
    }
    pub fn same_instance(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
    pub fn snapshot(&self) -> Value {
        serde_json::to_value(self.snapshot_data()).unwrap()
    }
    pub fn snapshot_data(&self) -> MonsterSnapshot {
        let record = self.0.read().unwrap();
        MonsterSnapshot {
            data: record.data.clone(),
            fields: record
                .data
                .entity
                .snapshot_fields(record.auras.all().iter().map(LiveAura::data).collect()),
            drops: record.drops.values().cloned().collect(),
        }
    }
    pub fn auras(&self) -> Vec<LiveAura> {
        self.0.read().unwrap().auras.all()
    }
    pub fn aura(&self, name: &str, kind: Option<AuraKind>) -> Option<LiveAura> {
        self.0.read().unwrap().auras.get(name, kind)
    }
    pub fn project_auras(&self, mutation: AuraMutation, now_ms: f64) -> Vec<AuraDelta> {
        self.0.write().unwrap().auras.project(mutation, now_ms)
    }
    pub(crate) fn update(
        &self,
        update: impl FnOnce(&mut MonsterData),
        write_aura_state: bool,
    ) -> Vec<AuraDelta> {
        let mut record = self.0.write().unwrap();
        let was_alive = record.data.entity.alive();
        update(&mut record.data);
        if !was_alive && record.data.entity.alive() {
            record.generation = record
                .generation
                .checked_add(1)
                .expect("monster generation exhausted");
        }
        let state = record.data.entity.state;
        if write_aura_state {
            record.auras.write_monster_state(state)
        } else {
            vec![]
        }
    }
    pub(crate) fn replace(&self, data: MonsterData) {
        self.update(|current| *current = data, false);
    }
    pub fn replace_drops(&self, drops: &[MonsterDrop]) {
        let mut record = self.0.write().unwrap();
        record.drops.clear();
        for drop in drops {
            record.drops.insert(drop.item.item_id, drop.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lucent_protocol::aura::{AuraValue, PassiveAuraInput};
    fn application(name: &str, duration: f64) -> AuraApplication {
        AuraApplication {
            name: name.into(),
            stack: 1.0,
            timing: AuraTiming::Seconds(duration),
            persistent: false,
            restart_duration: false,
            icon: None,
            category: None,
            value: None,
        }
    }
    #[test]
    fn active_auras_keep_identity_and_server_authoritative_lifetime() {
        let mut state = AuraState::new();
        state.project(
            AuraMutation::Apply(vec![application("Shield", 5.0)]),
            1000.0,
        );
        let held = state.get("shield", None).unwrap();
        state.project(
            AuraMutation::Apply(vec![application("Shield", 2.0)]),
            5000.0,
        );
        assert!(held.same_instance(&state.get("Shield", None).unwrap()));
        assert_eq!(held.data().duration, 5.0);
        assert_eq!(held.data().expires_at, Some(7000.0));
        assert_eq!(state.all().len(), 1);
        state.project(
            AuraMutation::Withdraw(vec![AuraWithdrawal::Remove {
                name: "Shield".into(),
            }]),
            10000.0,
        );
        state.project(
            AuraMutation::Apply(vec![application("Shield", 3.0)]),
            10000.0,
        );
        assert!(!held.same_instance(&state.get("Shield", None).unwrap()));
        assert_eq!(held.data().expires_at, Some(7000.0));
    }
    #[test]
    fn monster_server_state_gates_active_updates_but_seed_and_passive_updates_survive() {
        let mut state = AuraState::new();
        state.project(
            AuraMutation::Apply(vec![application("Counter Attack", 5.0)]),
            0.0,
        );
        assert_eq!(state.write_monster_state(EntityState::Dead).len(), 1);
        assert!(
            state
                .project(AuraMutation::Apply(vec![application("Blocked", 5.0)]), 1.0)
                .is_empty()
        );
        state.project(
            AuraMutation::Passives {
                mode: PassiveMode::Merge,
                entries: vec![PassiveAuraInput {
                    name: "Passive".into(),
                    category: None,
                    icon: None,
                    value: Some(AuraValue::Text("v".into())),
                    duration: 0.0,
                }],
            },
            1.0,
        );
        assert_eq!(state.all().len(), 1);
        state.write_monster_state(EntityState::Idle);
        state.project(AuraMutation::Apply(vec![application("Accepted", 1.0)]), 1.0);
        assert_eq!(state.all().len(), 2);
    }

    fn focus_application() -> AuraApplication {
        AuraApplication {
            name: "Focus".into(),
            stack: 5.0,
            timing: AuraTiming::Seconds(8.0),
            persistent: true,
            restart_duration: true,
            icon: Some("iwd1,ied1".into()),
            category: Some("buff".into()),
            value: Some(AuraValue::Text("Ravenous".into())),
        }
    }

    fn focus_state(timed: bool) -> AuraState {
        let mut state = AuraState::new();
        let entry = if timed {
            focus_application()
        } else {
            AuraApplication {
                stack: 5.0,
                timing: AuraTiming::Untimed,
                ..application("Focus", 0.0)
            }
        };
        state.project(AuraMutation::Apply(vec![entry]), 1_000_000.0);
        state
    }

    fn assert_focus_timer(state: &AuraState, duration: f64, expires_at: Option<f64>, stack: f64) {
        let aura = state.get("Focus", None).unwrap().data();
        assert_eq!(aura.name, "Focus");
        assert_eq!(aura.kind, AuraKind::Active);
        assert_eq!(aura.duration, duration);
        assert_eq!(aura.expires_at, expires_at);
        assert_eq!(aura.stack, stack);
        let snapshot = serde_json::to_value(aura).unwrap();
        if expires_at.is_none() {
            assert!(snapshot.get("expiresAt").is_none());
        }
    }

    #[test]
    fn restarting_an_aura_replaces_its_full_duration() {
        let mut state = focus_state(true);
        state.project(
            AuraMutation::Apply(vec![AuraApplication {
                timing: AuraTiming::Seconds(3.0),
                ..focus_application()
            }]),
            1_002_000.0,
        );
        assert_focus_timer(&state, 3.0, Some(1_005_000.0), 5.0);
    }

    #[test]
    fn untimed_application_erases_expiry() {
        let mut state = focus_state(true);
        state.project(
            AuraMutation::Apply(vec![AuraApplication {
                timing: AuraTiming::Untimed,
                stack: 0.0,
                ..focus_application()
            }]),
            1_002_000.0,
        );
        assert_focus_timer(&state, 0.0, None, 0.0);
    }

    #[test]
    fn decay_retains_full_duration() {
        let mut state = focus_state(true);
        state.project(
            AuraMutation::Withdraw(vec![AuraWithdrawal::Decay {
                name: "Focus".into(),
                stack: 2.0,
                refresh_seconds: Some(2.0),
            }]),
            1_003_000.0,
        );
        assert_focus_timer(&state, 8.0, Some(1_005_000.0), 2.0);
    }

    #[test]
    fn decay_extends_full_duration() {
        let mut state = focus_state(true);
        state.project(
            AuraMutation::Withdraw(vec![AuraWithdrawal::Decay {
                name: "Focus".into(),
                stack: 2.0,
                refresh_seconds: Some(12.0),
            }]),
            1_003_000.0,
        );
        assert_focus_timer(&state, 12.0, Some(1_015_000.0), 2.0);
    }

    #[test]
    fn decay_does_not_time_an_untimed_aura() {
        let mut state = focus_state(false);
        state.project(
            AuraMutation::Withdraw(vec![AuraWithdrawal::Decay {
                name: "Focus".into(),
                stack: 2.0,
                refresh_seconds: Some(12.0),
            }]),
            1_003_000.0,
        );
        assert_focus_timer(&state, 0.0, None, 2.0);
    }

    #[test]
    fn missing_decay_time_preserves_deadline() {
        let mut state = focus_state(true);
        state.project(
            AuraMutation::Withdraw(vec![AuraWithdrawal::Decay {
                name: "Focus".into(),
                stack: 2.0,
                refresh_seconds: None,
            }]),
            1_003_000.0,
        );
        assert_focus_timer(&state, 8.0, Some(1_008_000.0), 2.0);
    }

    #[test]
    fn negative_decay_refreshes_deadline() {
        let mut state = focus_state(true);
        state.project(
            AuraMutation::Withdraw(vec![AuraWithdrawal::Decay {
                name: "Focus".into(),
                stack: 2.0,
                refresh_seconds: Some(-2.0),
            }]),
            1_003_000.0,
        );
        assert_focus_timer(&state, 8.0, Some(1_001_000.0), 2.0);
    }

    #[test]
    fn zero_decay_retains_timed_state() {
        let mut state = focus_state(true);
        state.project(
            AuraMutation::Withdraw(vec![AuraWithdrawal::Decay {
                name: "Focus".into(),
                stack: 2.0,
                refresh_seconds: Some(0.0),
            }]),
            1_003_000.0,
        );
        assert_focus_timer(&state, 8.0, Some(1_003_000.0), 2.0);
    }

    #[test]
    fn stack_assignment_preserves_timer() {
        let mut state = focus_state(true);
        state.project(
            AuraMutation::SetStack(vec![("Focus".into(), 1.0)]),
            1_003_000.0,
        );
        assert_focus_timer(&state, 8.0, Some(1_008_000.0), 1.0);
    }

    #[test]
    fn seed_and_assignment_clamp_zero_stacks_but_application_does_not() {
        let mut state = focus_state(true);
        state.project(
            AuraMutation::SetStack(vec![("Focus".into(), 0.0)]),
            1_000_000.0,
        );
        assert_eq!(state.get("Focus", None).unwrap().data().stack, 1.0);
        let mut seeded = AuraState::new();
        seeded.project(
            AuraMutation::Seed(vec![AuraSeed {
                name: "Focus".into(),
                icon: "ice".into(),
                stack: 0.0,
                persistent: false,
                timer: SeedTimer::Untimed,
            }]),
            1_000_000.0,
        );
        assert_eq!(seeded.get("Focus", None).unwrap().data().stack, 1.0);
        let mut applied = AuraState::new();
        applied.project(
            AuraMutation::Apply(vec![AuraApplication {
                stack: 0.0,
                ..focus_application()
            }]),
            1_000_000.0,
        );
        assert_eq!(applied.get("Focus", None).unwrap().data().stack, 0.0);
    }

    #[test]
    fn seed_and_assignment_clamp_negative_stacks_but_application_does_not() {
        let mut state = focus_state(true);
        state.project(
            AuraMutation::SetStack(vec![("Focus".into(), -2.0)]),
            1_000_000.0,
        );
        assert_eq!(state.get("Focus", None).unwrap().data().stack, 1.0);
        let mut seeded = AuraState::new();
        seeded.project(
            AuraMutation::Seed(vec![AuraSeed {
                name: "Focus".into(),
                icon: "ice".into(),
                stack: -2.0,
                persistent: false,
                timer: SeedTimer::Untimed,
            }]),
            1_000_000.0,
        );
        assert_eq!(seeded.get("Focus", None).unwrap().data().stack, 1.0);
        let mut applied = AuraState::new();
        applied.project(
            AuraMutation::Apply(vec![AuraApplication {
                stack: -2.0,
                ..focus_application()
            }]),
            1_000_000.0,
        );
        assert_eq!(applied.get("Focus", None).unwrap().data().stack, -2.0);
    }

    #[test]
    fn application_retains_omitted_metadata_and_replaces_persistence() {
        let mut state = focus_state(true);
        state.project(
            AuraMutation::Apply(vec![AuraApplication {
                timing: AuraTiming::Untimed,
                ..application("Focus", 0.0)
            }]),
            1_002_000.0,
        );
        assert_eq!(
            serde_json::to_value(state.get("Focus", None).unwrap().data()).unwrap(),
            serde_json::json!({
                "name": "Focus", "kind": "active", "stack": 1.0, "duration": 0.0,
                "persistent": false, "icon": "iwd1,ied1", "category": "buff", "value": "Ravenous"
            })
        );
    }

    #[test]
    fn clear_preserves_persistent_active_auras_and_passive_membership() {
        let mut state = AuraState::new();
        state.project(
            AuraMutation::Apply(vec![
                AuraApplication {
                    stack: 5.0,
                    icon: Some("iwd1,ied1".into()),
                    ..application("Focus", 8.0)
                },
                AuraApplication {
                    stack: 5.0,
                    icon: Some("iwd1,ied1".into()),
                    persistent: true,
                    ..application("Potent Battle Elixir", 8.0)
                },
            ]),
            1_000_000.0,
        );
        state.project(
            AuraMutation::Passives {
                mode: PassiveMode::Merge,
                entries: vec![
                    PassiveAuraInput {
                        name: "Focus".into(),
                        category: None,
                        icon: None,
                        value: None,
                        duration: 0.0,
                    },
                    PassiveAuraInput {
                        name: "Brand of Chaos".into(),
                        category: None,
                        icon: None,
                        value: None,
                        duration: 0.0,
                    },
                ],
            },
            1_000_000.0,
        );
        let changes = state.project(
            AuraMutation::Clear {
                keep_persistent: true,
            },
            1_001_000.0,
        );
        assert!(
            matches!(changes.as_slice(), [AuraDelta::Removed { before }] if before.name == "Focus" && before.kind == AuraKind::Active)
        );
        assert_eq!(
            state
                .all()
                .iter()
                .map(|aura| aura.data().name)
                .collect::<Vec<_>>(),
            vec!["Potent Battle Elixir", "Focus", "Brand of Chaos"]
        );
        assert!(
            state
                .project(
                    AuraMutation::Clear {
                        keep_persistent: true
                    },
                    1_001_000.0
                )
                .is_empty()
        );
        let held = state.get("Focus", None).unwrap();
        state.project(
            AuraMutation::Passives {
                mode: PassiveMode::Replace,
                entries: vec![
                    PassiveAuraInput {
                        name: "Focus".into(),
                        category: None,
                        icon: None,
                        value: None,
                        duration: 2.0,
                    },
                    PassiveAuraInput {
                        name: "New Class".into(),
                        category: None,
                        icon: None,
                        value: None,
                        duration: 0.0,
                    },
                ],
            },
            1_002_000.0,
        );
        assert!(held.same_instance(&state.get("Focus", None).unwrap()));
        assert_eq!(
            state
                .all()
                .iter()
                .map(|aura| aura.data().name)
                .collect::<Vec<_>>(),
            vec!["Potent Battle Elixir", "Focus", "New Class"]
        );
        assert_eq!(state.get("Focus", None).unwrap().data().duration, 2.0);
    }

    #[test]
    fn exact_name_removal_keeps_other_spellings_and_cross_kind_lookup_order() {
        let mut state = AuraState::new();
        state.project(
            AuraMutation::Passives {
                mode: PassiveMode::Merge,
                entries: vec![PassiveAuraInput {
                    name: "Focus".into(),
                    category: None,
                    icon: None,
                    value: None,
                    duration: 0.0,
                }],
            },
            1_000_000.0,
        );
        state.project(
            AuraMutation::Apply(vec![
                AuraApplication {
                    stack: 5.0,
                    icon: Some("iwd1,ied1".into()),
                    ..application("Focus", 8.0)
                },
                AuraApplication {
                    stack: 5.0,
                    icon: Some("iwd1,ied1".into()),
                    ..application("focus", 8.0)
                },
                AuraApplication {
                    stack: 5.0,
                    icon: Some("iwd1,ied1".into()),
                    ..application(" Focus ", 8.0)
                },
            ]),
            1_000_000.0,
        );
        let names = |state: &AuraState| {
            state
                .all()
                .iter()
                .map(|aura| {
                    let aura = aura.data();
                    (aura.kind, aura.name)
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(
            names(&state),
            vec![
                (AuraKind::Passive, "Focus".into()),
                (AuraKind::Active, "Focus".into()),
                (AuraKind::Active, "focus".into()),
                (AuraKind::Active, " Focus ".into())
            ]
        );
        let auras = state.all();
        assert!(state.get(" FOCUS ", None).unwrap().same_instance(&auras[0]));
        assert!(
            state
                .get(" FOCUS ", Some(AuraKind::Active))
                .unwrap()
                .same_instance(&auras[1])
        );
        assert!(state.get(" FoCuS ", None).is_some());
        state.project(
            AuraMutation::Withdraw(vec![AuraWithdrawal::Remove {
                name: "focus".into(),
            }]),
            1_000_000.0,
        );
        assert_eq!(
            names(&state),
            vec![
                (AuraKind::Passive, "Focus".into()),
                (AuraKind::Active, "Focus".into()),
                (AuraKind::Active, " Focus ".into())
            ]
        );
    }

    #[test]
    fn detached_aura_deltas_fold_into_the_complete_next_snapshot() {
        let focus = focus_application();
        let passive = |name: &str, duration| PassiveAuraInput {
            name: name.into(),
            duration,
            category: None,
            icon: None,
            value: None,
        };
        let mutations = vec![
            AuraMutation::Apply(vec![
                focus.clone(),
                AuraApplication {
                    name: "focus".into(),
                    stack: 4.0,
                    ..focus.clone()
                },
            ]),
            AuraMutation::Passives {
                mode: PassiveMode::Merge,
                entries: vec![passive("Focus", 0.0), passive("Brand of Chaos", 0.0)],
            },
            AuraMutation::Apply(vec![
                AuraApplication {
                    stack: 2.0,
                    ..focus.clone()
                },
                AuraApplication {
                    stack: 3.0,
                    ..focus.clone()
                },
            ]),
            AuraMutation::Withdraw(vec![AuraWithdrawal::Decay {
                name: "Focus".into(),
                stack: 2.0,
                refresh_seconds: Some(2.0),
            }]),
            AuraMutation::SetStack(vec![("focus".into(), 1.0), ("Missing".into(), 1.0)]),
            AuraMutation::Seed(vec![
                AuraSeed {
                    name: "Focus".into(),
                    icon: "ice".into(),
                    stack: 5.0,
                    persistent: false,
                    timer: SeedTimer::Untimed,
                },
                AuraSeed {
                    name: "Focus".into(),
                    icon: "ice".into(),
                    stack: 4.0,
                    persistent: true,
                    timer: SeedTimer::Untimed,
                },
            ]),
            AuraMutation::Passives {
                mode: PassiveMode::Replace,
                entries: vec![passive("Focus", 0.0), passive("Focus", 2.0)],
            },
            AuraMutation::Clear {
                keep_persistent: true,
            },
            AuraMutation::Withdraw(vec![AuraWithdrawal::Remove {
                name: "Focus".into(),
            }]),
            AuraMutation::Passives {
                mode: PassiveMode::Replace,
                entries: vec![],
            },
            AuraMutation::Apply(vec![focus.clone()]),
        ];
        let mut state = AuraState::new();
        let mut saved = Vec::new();
        for (index, mutation) in mutations.into_iter().enumerate() {
            let key = |aura: &AuraData| format!("{:?}:{}", aura.kind, aura.name);
            let mut folded: IndexMap<_, _> = state
                .all()
                .iter()
                .map(|aura| {
                    let data = aura.data();
                    (key(&data), data)
                })
                .collect();
            let changes = state.project(mutation, 1_000_000.0 + index as f64 * 1000.0);
            let snapshot = format!("{changes:?}");
            for change in &changes {
                match change {
                    AuraDelta::Removed { before } => {
                        folded.shift_remove(&key(before));
                    }
                    AuraDelta::Added { after } | AuraDelta::Updated { after, .. } => {
                        folded.insert(key(after), after.clone());
                    }
                }
            }
            assert_eq!(
                folded.into_values().collect::<Vec<_>>(),
                state.all().iter().map(LiveAura::data).collect::<Vec<_>>()
            );
            saved.push((changes, snapshot));
        }
        assert_eq!(
            state
                .get("Focus", Some(AuraKind::Active))
                .unwrap()
                .data()
                .stack,
            5.0
        );
        for (changes, snapshot) in saved {
            assert_eq!(format!("{changes:?}"), snapshot);
        }
        assert!(
            state
                .project(AuraMutation::Apply(vec![focus]), 1_010_000.0)
                .is_empty()
        );
    }
}
