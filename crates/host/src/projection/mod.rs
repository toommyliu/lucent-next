pub mod auras;
pub mod auth;
mod baselines;
mod combat;
mod entity;
mod item;
mod item_mutations;
mod items;
pub mod monster_drops;
pub mod packet;
pub mod quests;
mod readiness;
pub mod shops;
mod wire;
pub mod world;

pub use item::decode_item;
pub use items::{ItemStore, LiveItem};
pub use readiness::{ReadinessKey, ReadinessReceipts, ReadinessSnapshot};

pub use auth::AuthState;
use lucent_protocol::{Packet, PacketDirection, event::ProjectionEvent, item::ItemContainer};

#[derive(Debug, Default)]
pub struct ProjectionOutput {
    pub events: Vec<ProjectionEvent>,
}

#[derive(Debug)]
pub enum PacketProjection {
    Applied(ProjectionOutput),
    Unhandled,
    NeedHydration(PendingHydration),
}

#[derive(Debug)]
pub struct PendingHydration {
    pub request: world::HydrationRequest,
    continuation: world::WorldContinuation,
    epoch: u64,
}

#[derive(Debug, thiserror::Error)]
#[error("hydration belongs to an earlier connection")]
pub struct StaleHydration;

#[derive(Debug, Default)]
pub struct Projection {
    pub auth: AuthState,
    pub items: ItemStore,
    pub receipts: ReadinessReceipts,
    pub quests: quests::QuestStore,
    pub shops: shops::ShopStore,
    pub world: world::WorldStore,
}

impl Projection {
    pub fn connection(&mut self, status: &str) {
        match status {
            "OnConnectionLost" | "OnConnectionFailed" => self.auth.logged_in = false,
            "OnConnection" => {}
            _ => return,
        }
        self.world.clear_area();
        self.world.clear_self();
        self.quests.clear();
        self.shops.clear();
        self.items.clear();
        self.receipts.reset();
    }

    pub fn readiness(&self, map_name: &str, self_bound: bool) -> ReadinessSnapshot {
        self.receipts.inspect(
            self.items.hydration(ItemContainer::House) > 0,
            self.items.hydration(ItemContainer::Inventory) > 0,
            map_name,
            self_bound,
        )
    }

    pub fn current_readiness(&self) -> ReadinessSnapshot {
        self.readiness(&self.world.map.name, self.world.get_me().is_some())
    }

    pub fn packet(
        &mut self,
        packet: &Packet,
        now_ms: f64,
        bridge_available: bool,
    ) -> PacketProjection {
        if baselines::project_baseline(&mut self.items, &mut self.receipts, packet) {
            return PacketProjection::Applied(ProjectionOutput::default());
        }
        let self_id = self.world.get_me().map(|player| player.data().entity_id);
        if let Some(output) = item_mutations::project_items(&mut self.items, packet, self_id) {
            return PacketProjection::Applied(output);
        }
        if let Some(output) = quests::project_quests(&mut self.quests, packet) {
            return PacketProjection::Applied(output);
        }
        if let Some(output) = shops::project_shop(&mut self.shops, &mut self.items, packet) {
            return PacketProjection::Applied(output);
        }
        if let Some(output) = combat::project_combat(&mut self.world, packet, now_ms) {
            return PacketProjection::Applied(output);
        }
        if packet.direction() == PacketDirection::Extension {
            if let Some(output) = auras::project_standalone_auras(&mut self.world, packet, now_ms) {
                return PacketProjection::Applied(output);
            }
            if packet.command() == "monsterDrops"
                && let Some(response) = monster_drops::decode_monster_drop_response(packet)
            {
                if let Some(id) = response.monster_map_id
                    && !self.world.set_monster_drops(id, response.drops)
                {
                    tracing::warn!(
                        command = "monsterDrops",
                        monster_map_id = id,
                        "drop response references an unknown monster"
                    );
                }
                return PacketProjection::Applied(response.output);
            }
        }
        world::project_world(
            &mut self.world,
            &mut self.items,
            &mut self.receipts,
            &self.auth.username,
            packet,
            now_ms,
            bridge_available,
        )
        .map_or(PacketProjection::Unhandled, |progress| {
            self.world_result(progress)
        })
    }

    pub fn resume_hydration(
        &mut self,
        pending: PendingHydration,
        result: world::HydrationResult,
        now_ms: f64,
    ) -> Result<PacketProjection, StaleHydration> {
        if pending.epoch != self.receipts.epoch() {
            return Err(StaleHydration);
        }
        let progress = world::resume_world(
            &mut self.world,
            &mut self.receipts,
            pending.continuation,
            result,
            now_ms,
        );
        Ok(self.world_result(progress))
    }

    fn world_result(&self, progress: world::WorldProgress) -> PacketProjection {
        match progress {
            world::WorldProgress::Complete(output) => PacketProjection::Applied(output),
            world::WorldProgress::NeedHydration {
                request,
                continuation,
            } => PacketProjection::NeedHydration(PendingHydration {
                request,
                continuation,
                epoch: self.receipts.epoch(),
            }),
        }
    }
}
