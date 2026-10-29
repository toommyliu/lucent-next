use lucent_protocol::{
    aura::AuraData,
    auth::AuthSnapshot,
    event::{AuraKind, MapState},
    item::{ItemContainer, ItemSnapshot},
    monster_drop::MonsterDrop,
    query::{Answer, ItemSelector, MonsterSelector, PlayerSelector},
    quest::QuestData,
    readiness::ReadinessSnapshot,
    shop::ShopData,
    world::{MonsterSnapshot, PlayerSnapshot},
};

use super::{Projection, entity::LiveAura};

fn auras(auras: Vec<LiveAura>, kind: Option<AuraKind>) -> Vec<AuraData> {
    auras
        .iter()
        .map(LiveAura::data)
        .filter(|aura| kind.is_none_or(|kind| kind == aura.kind))
        .collect()
}

impl Answer for Projection {
    fn map(&self) -> MapState {
        self.world.map.clone()
    }

    fn me(&self) -> Option<PlayerSnapshot> {
        self.world.get_me().map(|player| player.snapshot_data())
    }

    fn self_entity_id(&self) -> Option<i64> {
        self.world.self_entity_id
    }

    fn player(&self, selector: PlayerSelector) -> Option<PlayerSnapshot> {
        self.world
            .resolve_player(&selector)
            .map(|player| player.snapshot_data())
    }

    fn players(&self) -> Vec<PlayerSnapshot> {
        self.world
            .players()
            .iter()
            .map(|player| player.snapshot_data())
            .collect()
    }

    fn player_auras(&self, entity_id: i64, aura_kind: Option<AuraKind>) -> Vec<AuraData> {
        self.world
            .get_player_by_id(entity_id)
            .map_or_else(Vec::new, |player| auras(player.auras(), aura_kind))
    }

    fn monster(&self, selector: MonsterSelector) -> Option<MonsterSnapshot> {
        self.world
            .resolve_monster(&selector)
            .map(|monster| monster.snapshot_data())
    }

    fn monsters(&self) -> Vec<MonsterSnapshot> {
        self.world
            .monsters()
            .iter()
            .map(|monster| monster.snapshot_data())
            .collect()
    }

    fn monster_auras(&self, monster_map_id: i64, aura_kind: Option<AuraKind>) -> Vec<AuraData> {
        self.world
            .get_monster(monster_map_id)
            .map_or_else(Vec::new, |monster| auras(monster.auras(), aura_kind))
    }

    fn monster_drops(&self, monster_map_id: i64) -> Option<Vec<MonsterDrop>> {
        self.world
            .monster_drops(monster_map_id)
            .map(<[MonsterDrop]>::to_vec)
    }

    fn cells(&self) -> Vec<String> {
        self.world.cells.clone()
    }

    fn cell_pads(&self) -> Vec<String> {
        self.world.cell_pads.clone()
    }

    fn item(&self, container: ItemContainer, selector: ItemSelector) -> Option<ItemSnapshot> {
        self.items
            .resolve(container, &selector)
            .map(|item| item.data().snapshot_data())
    }

    fn items(&self, container: ItemContainer) -> Vec<ItemSnapshot> {
        self.items
            .all(container)
            .iter()
            .map(|item| item.data().snapshot_data())
            .collect()
    }

    fn item_hydration(&self, container: ItemContainer) -> u64 {
        self.items.hydration(container)
    }

    fn readiness(&self) -> ReadinessSnapshot {
        self.current_readiness()
    }

    fn quest(&self, id: f64) -> Option<QuestData> {
        self.quests.get(id).map(|quest| quest.data().clone())
    }

    fn quests(&self) -> Vec<QuestData> {
        self.quests
            .all()
            .iter()
            .map(|quest| quest.data().clone())
            .collect()
    }

    fn accepted_quests(&self) -> Vec<QuestData> {
        self.quests
            .accepted()
            .iter()
            .map(|quest| quest.data().clone())
            .collect()
    }

    fn shop(&self) -> Option<ShopData> {
        self.shops.get().map(|shop| shop.data())
    }

    fn auth(&self) -> AuthSnapshot {
        self.auth.snapshot()
    }
}
