use indexmap::IndexMap;
use lucent_protocol::aura::{AuraDelta, AuraMutation, PassiveAuraInput, PassiveMode};
use lucent_protocol::event::{
    AuraKind, EntityKind, LocationChange, MapState, Point, ProjectionEvent,
};
use lucent_protocol::monster_drop::MonsterDrop;
use lucent_protocol::world::{EntityData, EntityPatch, EntityState, MonsterData, PlayerData};
use lucent_protocol::{Packet, PacketDirection, PacketEncoding};
use serde_json::{Map, Value};

use super::auras::{aura_events, seed_auras};
use super::entity::{LiveMonster, LivePlayer};
use super::wire::{trim_js, wire_bool, wire_i64, wire_int, wire_number};
use super::{ItemStore, ProjectionOutput, ReadinessKey, ReadinessReceipts};

fn optional<T>(
    object: &Map<String, Value>,
    key: &str,
    decode: impl FnOnce(&Value) -> Option<T>,
) -> Option<Option<T>> {
    object
        .get(key)
        .map_or(Some(None), |value| decode(value).map(Some))
}
fn text(value: &Value) -> Option<String> {
    value.as_str().map(str::to_owned)
}
fn positive(value: &Value) -> Option<i64> {
    wire_i64(value).filter(|value| *value > 0)
}
fn state(value: &Value) -> Option<EntityState> {
    EntityState::try_from(wire_i64(value)?).ok()
}
fn normalized_name(value: &str) -> String {
    trim_js(value).to_lowercase()
}
fn names_equal(left: &str, right: &str) -> bool {
    !trim_js(left).is_empty()
        && !trim_js(right).is_empty()
        && left.to_lowercase() == right.to_lowercase()
}
fn location_equal(left: &str, right: &str) -> bool {
    left.to_lowercase() == right.to_lowercase()
}

pub fn decode_player(value: &Value) -> Option<(PlayerData, Option<Value>)> {
    let object = value.as_object()?;
    let username = object
        .get("strUsername")
        .and_then(Value::as_str)
        .or_else(|| object.get("uoName").and_then(Value::as_str))?
        .to_owned();
    Some((
        PlayerData {
            entity: EntityData {
                cell: optional(object, "strFrame", text)?.unwrap_or_default(),
                hp: optional(object, "intHP", wire_int)?.unwrap_or(0.0),
                max_hp: optional(object, "intHPMax", wire_int)?.unwrap_or(0.0),
                max_mp: optional(object, "intMPMax", wire_int)?.unwrap_or(0.0),
                mp: optional(object, "intMP", wire_int)?.unwrap_or(0.0),
                state: optional(object, "intState", state)?.unwrap_or_default(),
            },
            afk: optional(object, "afk", wire_bool)?.unwrap_or(false),
            entity_id: positive(object.get("entID")?)?,
            entity_type: optional(object, "entType", text)?.unwrap_or_else(|| "player".into()),
            level: optional(object, "intLevel", wire_int)?.unwrap_or(0.0),
            pad: optional(object, "strPad", text)?.unwrap_or_default(),
            position: Point {
                x: optional(object, "tx", wire_number)?.unwrap_or(0.0),
                y: optional(object, "ty", wire_number)?.unwrap_or(0.0),
            },
            username,
        },
        object.get("au").cloned(),
    ))
}

pub fn decode_monster(value: &Value) -> Option<(MonsterData, Option<Value>, bool)> {
    let object = value.as_object()?;
    let id = positive(object.get("MonMapID")?)?;
    let state = optional(object, "intState", state)?;
    Some((
        MonsterData {
            entity: EntityData {
                cell: optional(object, "strFrame", text)?.unwrap_or_default(),
                hp: optional(object, "intHP", wire_int)?.unwrap_or(0.0),
                max_hp: optional(object, "intHPMax", wire_int)?.unwrap_or(0.0),
                max_mp: optional(object, "intMPMax", wire_int)?.unwrap_or(0.0),
                mp: optional(object, "intMP", wire_int)?.unwrap_or(0.0),
                state: state.unwrap_or_default(),
            },
            aggressive: optional(object, "bRed", wire_bool)?.unwrap_or(false),
            level: optional(object, "iLvl", wire_int)?.unwrap_or(0.0),
            monster_id: optional(object, "MonID", positive)?.unwrap_or(0),
            monster_map_id: id,
            name: optional(object, "strMonName", text)?.unwrap_or_else(|| format!("Monster {id}")),
            race: optional(object, "sRace", text)?.unwrap_or_default(),
        },
        object.get("au").cloned(),
        state.is_some(),
    ))
}

pub fn decode_entity_patch(value: &Value) -> Option<EntityPatch> {
    let object = value.as_object()?;
    Some(EntityPatch {
        afk: optional(object, "afk", wire_bool)?,
        entity_id: optional(object, "entID", positive)?,
        hp: optional(object, "intHP", wire_int)?,
        max_hp: optional(object, "intHPMax", wire_int)?,
        level: optional(object, "intLevel", wire_int)?,
        mp: optional(object, "intMP", wire_int)?,
        max_mp: optional(object, "intMPMax", wire_int)?,
        state: optional(object, "intState", state)?,
        px: optional(object, "px", wire_number)?,
        py: optional(object, "py", wire_number)?,
        cell: optional(object, "strFrame", text)?,
        pad: optional(object, "strPad", text)?,
        tx: optional(object, "tx", wire_number)?,
        ty: optional(object, "ty", wire_number)?,
    })
}

fn patch_entity(entity: &mut EntityData, patch: &EntityPatch) {
    if let Some(value) = patch.hp {
        entity.hp = value;
    }
    if let Some(value) = patch.max_hp {
        entity.max_hp = value;
    }
    if let Some(value) = patch.mp {
        entity.mp = value;
    }
    if let Some(value) = patch.max_mp {
        entity.max_mp = value;
    }
    if let Some(value) = patch.state {
        entity.state = value;
    }
    if let Some(value) = &patch.cell {
        entity.cell.clone_from(value);
    }
}

#[derive(Debug)]
pub struct PlayerWrite {
    pub player: LivePlayer,
    pub became_dead: bool,
}
#[derive(Debug)]
pub struct MonsterWrite {
    pub monster: LiveMonster,
    pub became_dead: bool,
    pub aura_changes: Vec<AuraDelta>,
}

#[derive(Debug, Default)]
pub struct WorldStore {
    pub map: MapState,
    pub cells: Vec<String>,
    pub cell_pads: Vec<String>,
    pub self_name: String,
    pub self_entity_id: Option<i64>,
    players: IndexMap<String, LivePlayer>,
    player_ids: IndexMap<i64, String>,
    monsters: IndexMap<i64, LiveMonster>,
    monster_drops: IndexMap<i64, Vec<MonsterDrop>>,
}

impl WorldStore {
    pub fn players(&self) -> Vec<LivePlayer> {
        self.players.values().cloned().collect()
    }
    pub fn monsters(&self) -> Vec<LiveMonster> {
        self.monsters.values().cloned().collect()
    }
    pub fn get_player(&self, username: &str) -> Option<LivePlayer> {
        self.players.get(&normalized_name(username)).cloned()
    }
    pub fn get_player_by_id(&self, id: i64) -> Option<LivePlayer> {
        self.player_ids
            .get(&id)
            .and_then(|username| self.players.get(username))
            .cloned()
    }
    pub fn get_me(&self) -> Option<LivePlayer> {
        self.players.get(&self.self_name).cloned()
    }
    pub fn get_monster(&self, id: i64) -> Option<LiveMonster> {
        self.monsters.get(&id).cloned()
    }
    pub fn put_player(&mut self, incoming: LivePlayer) -> LivePlayer {
        let data = incoming.data();
        let key = normalized_name(&data.username);
        let current = if let Some(current) = self.players.get(&key) {
            current.replace(data.clone());
            current.clone()
        } else {
            self.players.insert(key.clone(), incoming.clone());
            incoming
        };
        self.player_ids.insert(data.entity_id, key);
        current
    }
    pub fn put_monster(&mut self, incoming: LiveMonster) -> LiveMonster {
        let data = incoming.data();
        if let Some(current) = self.monsters.get(&data.monster_map_id) {
            current.replace(data);
            current.clone()
        } else {
            if let Some(drops) = self.monster_drops.get(&data.monster_map_id) {
                incoming.replace_drops(drops);
            }
            self.monsters.insert(data.monster_map_id, incoming.clone());
            incoming
        }
    }
    pub fn set_self(&mut self, username: &str) {
        self.self_name = normalized_name(username);
        self.self_entity_id = self
            .players
            .get(&self.self_name)
            .map(|player| player.data().entity_id);
    }
    pub fn clear_self(&mut self) {
        self.self_name.clear();
        self.self_entity_id = None;
    }
    pub fn clear_area(&mut self) {
        self.map = MapState::default();
        self.cells.clear();
        self.cell_pads.clear();
        self.players.clear();
        self.player_ids.clear();
        self.monsters.clear();
        self.monster_drops.clear();
    }
    pub fn set_players(&mut self, players: Vec<LivePlayer>) {
        self.players.clear();
        self.player_ids.clear();
        for player in players {
            self.put_player(player);
        }
    }
    pub fn set_monsters(&mut self, monsters: Vec<LiveMonster>) {
        self.monsters.clear();
        for monster in monsters {
            self.put_monster(monster);
        }
    }
    pub fn remove_player(&mut self, username: &str) -> Option<LivePlayer> {
        let current = self.players.shift_remove(&normalized_name(username))?;
        self.player_ids.shift_remove(&current.data().entity_id);
        Some(current)
    }
    pub fn patch_player(&mut self, username: &str, patch: &EntityPatch) -> Option<PlayerWrite> {
        let current = self.get_player(username)?;
        let was_alive = current.data().entity.alive();
        current.update(|data| {
            patch_entity(&mut data.entity, patch);
            if let Some(value) = patch.afk {
                data.afk = value;
            }
            if let Some(value) = patch.entity_id {
                data.entity_id = value;
            }
            if let Some(value) = patch.level {
                data.level = value;
            }
            if let Some(value) = &patch.pad {
                data.pad.clone_from(value);
            }
            if patch.px.is_some() || patch.py.is_some() || patch.tx.is_some() || patch.ty.is_some()
            {
                data.position = Point {
                    x: patch.px.or(patch.tx).unwrap_or(data.position.x),
                    y: patch.py.or(patch.ty).unwrap_or(data.position.y),
                };
            }
        });
        Some(PlayerWrite {
            became_dead: was_alive && !current.data().entity.alive(),
            player: current,
        })
    }
    fn update_monster(
        &mut self,
        id: i64,
        patch: &EntityPatch,
        write_aura_state: bool,
    ) -> Option<MonsterWrite> {
        let current = self.get_monster(id)?;
        let was_alive = current.data().entity.alive();
        let aura_changes = current.update(
            |data| patch_entity(&mut data.entity, patch),
            write_aura_state && patch.state.is_some(),
        );
        Some(MonsterWrite {
            became_dead: was_alive && !current.data().entity.alive(),
            monster: current,
            aura_changes,
        })
    }
    pub fn patch_monster(&mut self, id: i64, patch: &EntityPatch) -> Option<MonsterWrite> {
        self.update_monster(id, patch, false)
    }
    pub fn write_monster(&mut self, id: i64, patch: &EntityPatch) -> Option<MonsterWrite> {
        self.update_monster(id, patch, true)
    }
    pub fn project_auras(
        &mut self,
        kind: EntityKind,
        id: i64,
        mutation: AuraMutation,
        now_ms: f64,
    ) -> Vec<AuraDelta> {
        match kind {
            EntityKind::Player => self
                .get_player_by_id(id)
                .map_or_else(Vec::new, |player| player.project_auras(mutation, now_ms)),
            EntityKind::Monster => self
                .get_monster(id)
                .map_or_else(Vec::new, |monster| monster.project_auras(mutation, now_ms)),
        }
    }
    pub fn set_monster_drops(&mut self, id: i64, drops: Vec<MonsterDrop>) -> bool {
        let Some(monster) = self.monsters.get(&id) else {
            return false;
        };
        monster.replace_drops(&drops);
        self.monster_drops.insert(id, drops);
        true
    }
    pub fn monster_drops(&self, id: i64) -> Option<&[MonsterDrop]> {
        self.monster_drops.get(&id).map(Vec::as_slice)
    }
}

pub use lucent_protocol::hydration::{HydrationMethod, HydrationRequest, HydrationResult};

#[derive(Debug)]
pub enum WorldProgress {
    Complete(ProjectionOutput),
    NeedHydration {
        request: HydrationRequest,
        continuation: WorldContinuation,
    },
}

#[derive(Clone, Debug)]
pub enum Movement {
    Client {
        command: String,
        params: Vec<String>,
    },
    CellTransition,
}

#[derive(Debug)]
pub enum WorldContinuation {
    Area {
        previous_self: Option<LivePlayer>,
        candidates: Vec<LivePlayer>,
        now_ms: f64,
        output: ProjectionOutput,
    },
    Initialize {
        baselines: Vec<Value>,
        command: String,
        auth_username: String,
        output: ProjectionOutput,
    },
    SelfUserId {
        movement: Movement,
        output: ProjectionOutput,
    },
    SelfData {
        user_id: i64,
        movement: Movement,
        output: ProjectionOutput,
    },
    Loaded {
        output: ProjectionOutput,
    },
    Cell {
        current: LivePlayer,
        output: ProjectionOutput,
    },
    Pad {
        current: LivePlayer,
        cell: Option<String>,
        output: ProjectionOutput,
    },
}
impl WorldContinuation {
    pub fn output(&self) -> &ProjectionOutput {
        match self {
            Self::Area { output, .. }
            | Self::Initialize { output, .. }
            | Self::SelfUserId { output, .. }
            | Self::SelfData { output, .. }
            | Self::Loaded { output }
            | Self::Cell { output, .. }
            | Self::Pad { output, .. } => output,
        }
    }
}
fn request(method: HydrationMethod, continuation: WorldContinuation) -> WorldProgress {
    WorldProgress::NeedHydration {
        request: HydrationRequest {
            method,
            args: vec![],
        },
        continuation,
    }
}

fn location_event(player: &LivePlayer, change: LocationChange) -> ProjectionEvent {
    let data = player.data();
    ProjectionEvent::PlayerLocation {
        cell: data.entity.cell,
        entity_id: data.entity_id,
        pad: data.pad,
        position: data.position,
        username: data.username,
        change,
    }
}

fn complete_area(
    world: &mut WorldStore,
    receipts: &mut ReadinessReceipts,
    previous_self: Option<LivePlayer>,
    selected: Option<LivePlayer>,
    now_ms: f64,
    mut output: ProjectionOutput,
) -> WorldProgress {
    if world.map.name.is_empty() {
        receipts.fail(
            ReadinessKey::Map,
            "moveToArea omitted a usable areaName".into(),
        );
    } else {
        receipts.complete(ReadinessKey::Map);
    }
    if let Some(current) = selected {
        if let Some(previous) = previous_self.filter(|previous| !previous.same_instance(&current)) {
            current.project_auras(
                AuraMutation::Passives {
                    mode: PassiveMode::Replace,
                    entries: previous
                        .auras()
                        .iter()
                        .map(|aura| aura.data())
                        .filter(|aura| aura.kind == AuraKind::Passive)
                        .map(|aura| PassiveAuraInput {
                            name: aura.name,
                            category: aura.category,
                            icon: aura.icon,
                            value: aura.value,
                            duration: aura.duration,
                        })
                        .collect(),
                },
                now_ms,
            );
        }
        world.set_self(&current.data().username);
        receipts.complete(ReadinessKey::Player);
    } else {
        receipts.fail(
            ReadinessKey::Player,
            "moveToArea did not establish the local player".into(),
        );
    }
    output.events.push(ProjectionEvent::JoinMap {
        map: world.map.clone(),
    });
    output.events.push(ProjectionEvent::PlayersChanged);
    WorldProgress::Complete(output)
}

fn parse_map(area: Option<&str>) -> (String, f64) {
    let value = trim_js(area.unwrap_or(""));
    if value
        .chars()
        .any(|value| matches!(value, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
    {
        return (value.into(), 0.0);
    }
    if let Some((name, room)) = value
        .rsplit_once('-')
        .filter(|(_, room)| !room.is_empty() && room.bytes().all(|byte| byte.is_ascii_digit()))
    {
        (name.into(), room.parse::<f64>().unwrap_or(f64::INFINITY))
    } else {
        (value.into(), 0.0)
    }
}

fn move_area(
    world: &mut WorldStore,
    receipts: &mut ReadinessReceipts,
    auth_username: &str,
    data: &Value,
    now_ms: f64,
    bridge_available: bool,
) -> WorldProgress {
    let output = ProjectionOutput::default();
    let decoded = data.as_object().and_then(|object| {
        Some((
            optional(object, "areaId", positive)?,
            optional(object, "areaName", text)?,
            optional(object, "monBranch", |value| value.as_array().cloned())?,
            optional(object, "mondef", |value| value.as_array().cloned())?,
            optional(object, "monmap", |value| value.as_array().cloned())?,
            optional(object, "uoBranch", |value| value.as_array().cloned())?,
        ))
    });
    let Some((area_id, area_name, branches, definitions, placements, players)) = decoded else {
        tracing::warn!(command = "moveToArea", "invalid projection input");
        receipts.fail(ReadinessKey::Map, "moveToArea payload is malformed".into());
        return WorldProgress::Complete(output);
    };
    let previous_self = world.get_me();
    let (name, room_number) = parse_map(area_name.as_deref());
    let map = MapState {
        id: area_id.unwrap_or(0),
        name,
        room_number,
    };
    let mut invalid_monsters = 0;
    let mut invalid_players = 0;
    let mut invalid_auras = 0;
    let mut defs = IndexMap::new();
    for value in definitions.unwrap_or_default() {
        let Some(object) = value.as_object() else {
            invalid_monsters += 1;
            continue;
        };
        if let Some(id) = object.get("MonID").and_then(positive) {
            defs.insert(id, object.clone());
        }
    }
    let mut cells = IndexMap::new();
    for value in placements.unwrap_or_default() {
        let Some(object) = value.as_object() else {
            invalid_monsters += 1;
            continue;
        };
        if let (Some(id), Some(cell)) = (
            object.get("MonMapID").and_then(positive),
            object.get("strFrame").and_then(Value::as_str),
        ) {
            cells.insert(id, cell.to_owned());
        }
    }
    let mut monsters = Vec::new();
    for value in branches.unwrap_or_default() {
        let Some(branch) = value.as_object() else {
            invalid_monsters += 1;
            continue;
        };
        let (Some(id), Some(map_id)) = (
            branch.get("MonID").and_then(positive),
            branch.get("MonMapID").and_then(positive),
        ) else {
            invalid_monsters += 1;
            continue;
        };
        let mut merged = defs.get(&id).cloned().unwrap_or_default();
        merged.extend(branch.clone());
        if let Some(cell) = cells.get(&map_id) {
            merged.insert("strFrame".into(), cell.clone().into());
        }
        if let Some((data, au, explicit_state)) = decode_monster(&Value::Object(merged)) {
            let monster = LiveMonster::new(data, explicit_state);
            let seed = seed_auras(&monster, au.as_ref(), now_ms);
            invalid_auras += seed.rejected;
            monsters.push(monster);
        } else {
            invalid_monsters += 1;
        }
    }
    if invalid_monsters > 0 {
        tracing::warn!(
            command = "moveToArea",
            rejected = invalid_monsters,
            collection = "monsters",
            "projection discarded invalid entries"
        );
    }
    let mut incoming = Vec::new();
    for value in players.unwrap_or_default() {
        if let Some((data, au)) = decode_player(&value) {
            let player = LivePlayer::new(data);
            let seed = seed_auras(&player, au.as_ref(), now_ms);
            invalid_auras += seed.rejected;
            incoming.push(player);
        } else {
            invalid_players += 1;
        }
    }
    if invalid_players > 0 {
        tracing::warn!(
            command = "moveToArea",
            rejected = invalid_players,
            collection = "players",
            "projection discarded invalid entries"
        );
    }
    if invalid_auras > 0 {
        tracing::warn!(
            command = "moveToArea",
            rejected = invalid_auras,
            collection = "auras",
            "projection discarded invalid entries"
        );
    }
    if let Some(previous) = &previous_self
        && names_equal(&previous.data().username, auth_username)
        && !incoming
            .iter()
            .any(|player| names_equal(&player.data().username, &previous.data().username))
    {
        incoming.push(previous.clone());
    }
    world.clear_area();
    world.map = map;
    world.set_monsters(monsters);
    world.set_players(incoming.clone());
    let selected = incoming
        .iter()
        .find(|player| names_equal(&player.data().username, auth_username))
        .cloned();
    if selected.is_some() {
        return complete_area(world, receipts, previous_self, selected, now_ms, output);
    }
    if let Some(id) = world.self_entity_id {
        let selected = incoming
            .iter()
            .find(|player| player.data().entity_id == id)
            .cloned();
        return complete_area(world, receipts, previous_self, selected, now_ms, output);
    }
    if bridge_available {
        request(
            HydrationMethod::UserId,
            WorldContinuation::Area {
                previous_self,
                candidates: incoming,
                now_ms,
                output,
            },
        )
    } else {
        complete_area(world, receipts, previous_self, None, now_ms, output)
    }
}

fn decode_initialized(value: &Value) -> Option<PlayerData> {
    let object = value.as_object()?;
    let data = object.get("data")?.as_object()?;
    let uid = optional(object, "uid", positive)?;
    let mut data = data.clone();
    if let Some(uid) = uid {
        data.insert("entID".into(), uid.into());
    }
    decode_player(&Value::Object(data)).map(|(data, _)| data)
}

fn initialize(
    world: &mut WorldStore,
    receipts: &mut ReadinessReceipts,
    baselines: Vec<Value>,
    command: &str,
    auth_username: &str,
    user_id: Option<i64>,
    mut output: ProjectionOutput,
) -> WorldProgress {
    let mut changed = false;
    for value in baselines {
        let Some(fallback) = decode_initialized(&value) else {
            tracing::warn!(command = command, "invalid projection input");
            continue;
        };
        let current = world
            .get_player_by_id(fallback.entity_id)
            .or_else(|| world.get_player(&fallback.username))
            .unwrap_or_else(|| {
                changed = true;
                world.put_player(LivePlayer::new(fallback))
            });
        let data = current.data();
        if user_id == Some(data.entity_id) || names_equal(&data.username, auth_username) {
            world.set_self(&data.username);
        }
    }
    if world.get_me().is_some() {
        receipts.complete(ReadinessKey::Player);
    } else {
        receipts.fail(
            ReadinessKey::Player,
            format!("{command} did not establish the local player"),
        );
    }
    if changed {
        output.events.push(ProjectionEvent::PlayersChanged);
    }
    WorldProgress::Complete(output)
}

fn move_current(
    world: &mut WorldStore,
    current: Option<LivePlayer>,
    movement: Movement,
    mut output: ProjectionOutput,
) -> WorldProgress {
    let Some(current) = current else {
        tracing::warn!(
            command = "movement",
            "projection input requires a local player"
        );
        return WorldProgress::Complete(output);
    };
    match movement {
        Movement::CellTransition => request(
            HydrationMethod::Cell,
            WorldContinuation::Cell { current, output },
        ),
        Movement::Client { command, params } => {
            let mut patch = EntityPatch::default();
            let change = if command == "moveToCell" {
                let Some(cell) = params.get(4) else {
                    return WorldProgress::Complete(output);
                };
                patch.cell = Some(cell.clone());
                patch.pad = params.get(5).cloned();
                LocationChange::Cell
            } else {
                let (Some(x), Some(y)) = (
                    params
                        .get(4)
                        .and_then(|value| wire_int(&Value::String(value.clone()))),
                    params
                        .get(5)
                        .and_then(|value| wire_int(&Value::String(value.clone()))),
                ) else {
                    return WorldProgress::Complete(output);
                };
                patch.tx = Some(x);
                patch.ty = Some(y);
                LocationChange::Walk {
                    destination: Point { x, y },
                }
            };
            if let Some(result) = world.patch_player(&current.data().username, &patch) {
                output.events.push(location_event(&result.player, change));
            }
            WorldProgress::Complete(output)
        }
    }
}

fn self_data(
    world: &mut WorldStore,
    movement: Movement,
    user_id: Option<i64>,
    output: ProjectionOutput,
) -> WorldProgress {
    let Some(id) = user_id else {
        return move_current(world, None, movement, output);
    };
    if let Some(existing) = world.get_player_by_id(id) {
        world.set_self(&existing.data().username);
        return move_current(world, Some(existing), movement, output);
    }
    request(
        HydrationMethod::PlayerData,
        WorldContinuation::SelfData {
            user_id: id,
            movement,
            output,
        },
    )
}

fn get_self(
    world: &mut WorldStore,
    movement: Movement,
    bridge_available: bool,
    output: ProjectionOutput,
) -> WorldProgress {
    if let Some(current) = world.get_me() {
        return move_current(world, Some(current), movement, output);
    }
    if !bridge_available {
        return move_current(world, None, movement, output);
    }
    if let Some(id) = world.self_entity_id {
        return self_data(world, movement, Some(id), output);
    }
    request(
        HydrationMethod::UserId,
        WorldContinuation::SelfUserId { movement, output },
    )
}

pub fn resume_world(
    world: &mut WorldStore,
    receipts: &mut ReadinessReceipts,
    continuation: WorldContinuation,
    result: HydrationResult,
    _now_ms: f64,
) -> WorldProgress {
    match continuation {
        WorldContinuation::Area {
            previous_self,
            candidates,
            now_ms,
            output,
        } => {
            let selected = result.value().and_then(positive).and_then(|id| {
                candidates
                    .into_iter()
                    .find(|player| player.data().entity_id == id)
            });
            complete_area(world, receipts, previous_self, selected, now_ms, output)
        }
        WorldContinuation::Initialize {
            baselines,
            command,
            auth_username,
            output,
        } => initialize(
            world,
            receipts,
            baselines,
            &command,
            &auth_username,
            result.value().and_then(positive),
            output,
        ),
        WorldContinuation::SelfUserId { movement, output } => {
            self_data(world, movement, result.value().and_then(positive), output)
        }
        WorldContinuation::SelfData {
            user_id,
            movement,
            output,
        } => {
            let current = result.value().and_then(Value::as_object).and_then(|data| {
                let mut value = data.clone();
                value.insert("entID".into(), user_id.into());
                if let Some((data, _)) = decode_player(&Value::Object(value)) {
                    let player = world.put_player(LivePlayer::new(data));
                    world.set_self(&player.data().username);
                    Some(player)
                } else {
                    tracing::warn!(command = "player.getData", "invalid projection input");
                    None
                }
            });
            move_current(world, current, movement, output)
        }
        WorldContinuation::Loaded { output } => {
            if result.value().and_then(Value::as_bool) != Some(true) {
                WorldProgress::Complete(output)
            } else {
                get_self(world, Movement::CellTransition, true, output)
            }
        }
        WorldContinuation::Cell { current, output } => request(
            HydrationMethod::Pad,
            WorldContinuation::Pad {
                current,
                cell: result.value().and_then(Value::as_str).map(str::to_owned),
                output,
            },
        ),
        WorldContinuation::Pad {
            current,
            cell,
            mut output,
        } => {
            if let (Some(cell), Some(pad)) = (
                cell.filter(|cell| !cell.is_empty()),
                result
                    .value()
                    .and_then(Value::as_str)
                    .filter(|pad| !pad.is_empty()),
            ) && let Some(result) = world.patch_player(
                &current.data().username,
                &EntityPatch {
                    cell: Some(cell),
                    pad: Some(pad.into()),
                    ..EntityPatch::default()
                },
            ) {
                output
                    .events
                    .push(location_event(&result.player, LocationChange::Cell));
            }
            WorldProgress::Complete(output)
        }
    }
}

fn parse_csv(value: &str) -> Value {
    let mut fields = Map::new();
    for token in value.split(',') {
        if let Some((key, value)) = token.split_once(':').filter(|(key, _)| !key.is_empty()) {
            fields.insert(key.into(), value.into());
        }
    }
    Value::Object(fields)
}

fn player_update(packet: &Packet) -> Option<(String, EntityPatch)> {
    let data = packet.data()?;
    if packet.encoding() == PacketEncoding::String {
        let values = data.as_array()?;
        let username = values.get(2)?.as_str()?.to_owned();
        let patch = parse_csv(values.get(3)?.as_str()?);
        Some((username, decode_entity_patch(&patch)?))
    } else {
        Some((
            data.get("unm")?.as_str()?.into(),
            decode_entity_patch(data.get("o")?)?,
        ))
    }
}

fn monster_update(packet: &Packet) -> Option<(i64, EntityPatch)> {
    let data = packet.data()?;
    if packet.encoding() == PacketEncoding::String {
        let values = data.as_array()?;
        Some((
            positive(values.get(2)?)?,
            decode_entity_patch(&parse_csv(values.get(3)?.as_str()?))?,
        ))
    } else {
        Some((
            positive(data.get("id")?)?,
            decode_entity_patch(data.get("o")?)?,
        ))
    }
}

pub fn project_world(
    world: &mut WorldStore,
    items: &mut ItemStore,
    receipts: &mut ReadinessReceipts,
    auth_username: &str,
    packet: &Packet,
    now_ms: f64,
    bridge_available: bool,
) -> Option<WorldProgress> {
    if packet.direction() == PacketDirection::Client {
        return if let Packet::Client {
            command, params, ..
        } = packet
        {
            if matches!(command.as_str(), "moveToCell" | "mv") {
                Some(get_self(
                    world,
                    Movement::Client {
                        command: command.clone(),
                        params: params.clone(),
                    },
                    bridge_available,
                    ProjectionOutput::default(),
                ))
            } else {
                None
            }
        } else {
            None
        };
    }
    if packet.direction() != PacketDirection::Extension {
        return None;
    }
    let data = packet.data()?;
    let mut output = ProjectionOutput::default();
    match packet.command() {
        "moveToArea" => {
            return Some(move_area(
                world,
                receipts,
                auth_username,
                data,
                now_ms,
                bridge_available,
            ));
        }
        command @ ("initUserData" | "initUserDatas") => {
            let baselines = if command == "initUserData" {
                Some(vec![data.clone()])
            } else {
                data.get("a").and_then(Value::as_array).cloned()
            };
            let Some(baselines) = baselines else {
                tracing::warn!(command = command, "invalid projection input");
                receipts.fail(
                    ReadinessKey::Player,
                    format!("{command} payload is malformed"),
                );
                return Some(WorldProgress::Complete(output));
            };
            return Some(if let Some(id) = world.self_entity_id {
                initialize(
                    world,
                    receipts,
                    baselines,
                    command,
                    auth_username,
                    Some(id),
                    output,
                )
            } else if bridge_available {
                request(
                    HydrationMethod::UserId,
                    WorldContinuation::Initialize {
                        baselines,
                        command: command.into(),
                        auth_username: auth_username.into(),
                        output,
                    },
                )
            } else {
                initialize(
                    world,
                    receipts,
                    baselines,
                    command,
                    auth_username,
                    None,
                    output,
                )
            });
        }
        "exitArea" => {
            if let Some(username) = data
                .as_array()
                .and_then(|data| data.get(3))
                .and_then(Value::as_str)
                && world.remove_player(username).is_some()
            {
                output.events.push(ProjectionEvent::PlayersChanged);
            }
        }
        "uotls" => {
            let Some((username, patch)) = player_update(packet) else {
                tracing::warn!(command = "uotls", "invalid projection input");
                if data
                    .as_object()
                    .and_then(|data| data.get("unm"))
                    .and_then(Value::as_str)
                    .is_some_and(|username| names_equal(username, auth_username))
                {
                    receipts.fail(
                        ReadinessKey::Player,
                        "initial uotls player payload is malformed".into(),
                    );
                }
                return Some(WorldProgress::Complete(output));
            };
            let mut changed = false;
            let current = if let Some(current) = world.get_player(&username) {
                current
            } else {
                let baseline = if packet.encoding() == PacketEncoding::Json {
                    data.get("o").and_then(Value::as_object).and_then(|data| {
                        let mut data = data.clone();
                        data.insert("uoName".into(), username.clone().into());
                        decode_player(&Value::Object(data))
                    })
                } else {
                    None
                };
                let Some((baseline, _)) = baseline else {
                    tracing::warn!(
                        command = "uotls",
                        entity_kind = "player",
                        "projection input references an unknown entity"
                    );
                    if names_equal(&username, auth_username) {
                        receipts.fail(
                            ReadinessKey::Player,
                            "initial uotls omitted the local player baseline".into(),
                        );
                    }
                    return Some(WorldProgress::Complete(output));
                };
                changed = true;
                world.put_player(LivePlayer::new(baseline))
            };
            let before = current.data();
            let location_changed = patch
                .cell
                .as_ref()
                .is_some_and(|cell| !location_equal(&before.entity.cell, cell))
                || patch
                    .pad
                    .as_ref()
                    .is_some_and(|pad| !location_equal(&before.pad, pad));
            if let Some(result) = world.patch_player(&before.username, &patch) {
                let current = result.player.data();
                if names_equal(&current.username, auth_username) {
                    world.set_self(&current.username);
                    receipts.complete(ReadinessKey::Player);
                } else if world.get_me().is_some() {
                    receipts.complete(ReadinessKey::Player);
                }
                if changed {
                    output.events.push(ProjectionEvent::PlayersChanged);
                }
                if patch.cell.is_some()
                    || patch.pad.is_some()
                    || patch.px.is_some()
                    || patch.py.is_some()
                    || patch.tx.is_some()
                    || patch.ty.is_some()
                {
                    let destination = patch
                        .tx
                        .zip(patch.ty)
                        .filter(|(x, y)| *x != 0.0 || *y != 0.0)
                        .map(|(x, y)| Point { x, y });
                    let reported = patch.px.is_some()
                        || patch.py.is_some()
                        || patch.tx.is_some_and(|x| x != 0.0)
                        || patch.ty.is_some_and(|y| y != 0.0);
                    let change = destination.map_or_else(
                        || {
                            if location_changed || !reported {
                                LocationChange::Cell
                            } else {
                                LocationChange::Position
                            }
                        },
                        |destination| LocationChange::Walk { destination },
                    );
                    output.events.push(location_event(&result.player, change));
                }
                if result.became_dead {
                    output.events.push(ProjectionEvent::PlayerDeath {
                        entity_id: current.entity_id,
                        username: current.username.clone(),
                    });
                }
                if let Some(afk) = patch.afk {
                    output.events.push(ProjectionEvent::PlayerAfk {
                        afk,
                        entity_id: current.entity_id,
                        username: current.username,
                    });
                }
            }
        }
        "mtls" => {
            let Some((id, patch)) = monster_update(packet) else {
                tracing::warn!(command = "mtls", "invalid projection input");
                return Some(WorldProgress::Complete(output));
            };
            if let Some(result) = world.write_monster(id, &patch) {
                if result.became_dead {
                    output
                        .events
                        .push(ProjectionEvent::MonsterDeath { monster_map_id: id });
                }
                output.events.extend(aura_events(
                    EntityKind::Monster,
                    id,
                    &result.aura_changes,
                    None,
                    None,
                ));
            } else {
                tracing::warn!(
                    command = "mtls",
                    entity_kind = "monster",
                    entity_id = id,
                    "projection input references an unknown entity"
                );
            }
        }
        "mtcid" => {
            return Some(if bridge_available {
                request(
                    HydrationMethod::WorldLoaded,
                    WorldContinuation::Loaded { output },
                )
            } else {
                WorldProgress::Complete(output)
            });
        }
        "clearAuras" => {
            if let Some(current) = world.get_me() {
                let id = current.data().entity_id;
                let changes = world.project_auras(
                    EntityKind::Player,
                    id,
                    AuraMutation::Clear {
                        keep_persistent: true,
                    },
                    now_ms,
                );
                output
                    .events
                    .extend(aura_events(EntityKind::Player, id, &changes, None, None));
            }
        }
        "event" => {
            if let Some(args) = data.get("args").and_then(Value::as_object)
                && let Some(zone) = optional(args, "zoneSet", text)
            {
                output.events.push(ProjectionEvent::Zone {
                    map: world.map.name.clone(),
                    zone: zone.unwrap_or_default(),
                });
            }
        }
        "respawnMon" => {
            let ids = data
                .as_array()
                .and_then(|data| data.get(2))
                .and_then(Value::as_str)
                .unwrap_or("");
            for id in ids
                .split(',')
                .filter_map(|value| positive(&Value::String(value.into())))
            {
                if let Some(monster) = world.get_monster(id) {
                    let current = monster.data();
                    world.patch_monster(
                        id,
                        &EntityPatch {
                            hp: Some(current.entity.max_hp),
                            mp: Some(current.entity.max_mp),
                            state: Some(EntityState::Idle),
                            ..EntityPatch::default()
                        },
                    );
                    output
                        .events
                        .push(ProjectionEvent::MonsterRespawn { monster_map_id: id });
                } else {
                    tracing::warn!(
                        command = "respawnMon",
                        entity_kind = "monster",
                        entity_id = id,
                        "projection input references an unknown entity"
                    );
                }
            }
        }
        "addGoldExp" => {
            if let Some(points) = data.get("iCP").and_then(wire_int)
                && let Some(class) = items
                    .all(lucent_protocol::item::ItemContainer::Inventory)
                    .into_iter()
                    .find(|item| {
                        let data = item.data();
                        data.category == "Class" && data.equipped
                    })
            {
                class.update(|data| data.quantity = (data.quantity + points).max(0.0));
            }
            if let (Some(id), Some("m")) = (
                data.get("id").and_then(positive),
                data.get("typ").and_then(Value::as_str),
            ) && let Some(result) = world.patch_monster(
                id,
                &EntityPatch {
                    hp: Some(0.0),
                    mp: Some(0.0),
                    state: Some(EntityState::Dead),
                    ..EntityPatch::default()
                },
            ) && result.became_dead
            {
                output
                    .events
                    .push(ProjectionEvent::MonsterDeath { monster_map_id: id });
            }
        }
        _ => return None,
    }
    Some(WorldProgress::Complete(output))
}

#[cfg(test)]
mod tests {
    use super::*;
    use lucent_protocol::PacketEncoding;
    use serde_json::json;

    fn packet(command: &str, data: Value) -> Packet {
        Packet::Extension {
            command: command.into(),
            data,
            raw: String::new(),
            encoding: PacketEncoding::Json,
        }
    }
    fn complete(progress: WorldProgress) -> ProjectionOutput {
        match progress {
            WorldProgress::Complete(output) => output,
            WorldProgress::NeedHydration { request, .. } => {
                panic!("Unexpected hydration {}", request.method.name())
            }
        }
    }

    #[test]
    fn area_baseline_commits_before_self_hydration_and_orders_notifications() {
        let mut world = WorldStore::default();
        let mut items = ItemStore::default();
        let mut receipts = ReadinessReceipts::default();
        let input = packet(
            "moveToArea",
            json!({"areaId":2,"areaName":"testroom-17","mondef":[{"MonID":4,"strMonName":"Target"}],"monmap":[{"MonMapID":7,"strFrame":"Boss"}],"monBranch":[{"MonID":4,"MonMapID":7,"intHP":100,"intHPMax":100}],"uoBranch":[{"entID":3,"uoName":"player-a","intHP":50}]}),
        );
        let progress = project_world(
            &mut world,
            &mut items,
            &mut receipts,
            "",
            &input,
            1000.0,
            true,
        )
        .unwrap();
        let WorldProgress::NeedHydration {
            request,
            continuation,
        } = progress
        else {
            panic!("Expected self-ID hydration");
        };
        assert_eq!(request.method, HydrationMethod::UserId);
        assert_eq!(world.map.name, "testroom");
        assert_eq!(world.map.room_number, 17.0);
        assert_eq!(world.get_monster(7).unwrap().data().entity.cell, "Boss");
        assert!(
            !receipts
                .inspect(false, false, &world.map.name, world.get_me().is_some())
                .state
                .map
        );
        let output = complete(resume_world(
            &mut world,
            &mut receipts,
            continuation,
            HydrationResult::Value(json!(3)),
            9000.0,
        ));
        assert_eq!(
            output.events,
            vec![
                ProjectionEvent::JoinMap {
                    map: world.map.clone()
                },
                ProjectionEvent::PlayersChanged
            ]
        );
        assert_eq!(world.get_me().unwrap().data().username, "player-a");
        assert!(
            receipts
                .inspect(false, false, &world.map.name, true)
                .state
                .map
        );
    }

    #[test]
    fn authoritative_world_leaf_beats_init_metadata_and_client_walk_preserves_identity() {
        let mut world = WorldStore::default();
        let mut items = ItemStore::default();
        let mut receipts = ReadinessReceipts::default();
        let input = packet(
            "moveToArea",
            json!({"areaName":"testroom-1","uoBranch":[{"entID":3,"uoName":"player-a","intHP":50,"intHPMax":100,"strFrame":"Enter"}]}),
        );
        complete(
            project_world(
                &mut world,
                &mut items,
                &mut receipts,
                "player-a",
                &input,
                0.0,
                false,
            )
            .unwrap(),
        );
        let held = world.get_me().unwrap();
        let input = packet(
            "initUserData",
            json!({"uid":3,"data":{"uoName":"player-a","intHP":999}}),
        );
        complete(
            project_world(
                &mut world,
                &mut items,
                &mut receipts,
                "player-a",
                &input,
                0.0,
                false,
            )
            .unwrap(),
        );
        assert_eq!(world.get_me().unwrap().data().entity.hp, 50.0);
        let input = Packet::Client {
            command: "mv".into(),
            params: vec!["xt", "zm", "mv", "1", "11", "22"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
            raw: String::new(),
            encoding: PacketEncoding::String,
        };
        let output = complete(
            project_world(
                &mut world,
                &mut items,
                &mut receipts,
                "player-a",
                &input,
                0.0,
                false,
            )
            .unwrap(),
        );
        assert!(held.same_instance(&world.get_me().unwrap()));
        assert_eq!(held.data().position, Point { x: 11.0, y: 22.0 });
        assert!(
            matches!(&output.events[0], ProjectionEvent::PlayerLocation { change: LocationChange::Walk { destination }, .. } if *destination == Point { x:11.0, y:22.0 })
        );
        world.clear_area();
        world.clear_self();
        world.put_player(LivePlayer::new(PlayerData {
            entity_id: 3,
            username: "player-a".into(),
            ..PlayerData::default()
        }));
        assert!(!held.same_instance(&world.get_player("player-a").unwrap()));
        assert_eq!(held.data().entity.hp, 50.0);
    }

    #[test]
    fn uotls_event_order_distinguishes_walk_death_and_afk() {
        let mut world = WorldStore::default();
        let mut items = ItemStore::default();
        let mut receipts = ReadinessReceipts::default();
        world.put_player(LivePlayer::new(PlayerData {
            entity: EntityData {
                hp: 10.0,
                state: EntityState::Idle,
                ..EntityData::default()
            },
            entity_id: 3,
            username: "player-a".into(),
            ..PlayerData::default()
        }));
        let input = packet(
            "uotls",
            json!({"unm":"player-a","o":{"tx":5,"ty":6,"intHP":0,"afk":true}}),
        );
        let output = complete(
            project_world(
                &mut world,
                &mut items,
                &mut receipts,
                "player-a",
                &input,
                0.0,
                false,
            )
            .unwrap(),
        );
        assert!(matches!(
            output.events[0],
            ProjectionEvent::PlayerLocation {
                change: LocationChange::Walk { .. },
                ..
            }
        ));
        assert!(matches!(
            output.events[1],
            ProjectionEvent::PlayerDeath { entity_id: 3, .. }
        ));
        assert!(matches!(
            output.events[2],
            ProjectionEvent::PlayerAfk { afk: true, .. }
        ));
    }
}
