use std::time::{SystemTime, UNIX_EPOCH};

use lucent_protocol::{
    PacketDirection,
    bridge::{from_json, to_json},
    event::ProjectionEvent,
    hydration::HydrationResult,
};

use super::Running;
use crate::bridge::{self, Error, Event, ScriptEvent, Value};
use crate::projection::{
    PacketProjection, Projection,
    packet::{PacketDecode, parse_packet},
};

impl Running {
    pub(super) fn drain_events(&mut self) {
        loop {
            let received = match self.loader_events.try_recv() {
                Ok(Some(event)) => Some(event),
                Err(Error::Overflow) => None,
                _ => break,
            };
            let mut projection = self.projection.write();
            let epoch = projection.receipts.epoch();
            let event = match received {
                None => {
                    self.invalidate_projection(&mut projection);
                    None
                }
                Some(event) => {
                    let projected = self.project_event(&mut projection, &event);
                    Some(ScriptEvent::Loader { event, projected })
                }
            };
            let invalidated = projection.receipts.epoch() != epoch;
            let ready = projection.current_readiness().missing.is_empty();
            drop(projection);
            if let Some(event) = event {
                self.publish(event);
            }
            if ready && !self.settings_applied {
                self.settings_applied = true;
                self.reapply_settings(false);
            }
            if invalidated {
                self.publish(ScriptEvent::Invalidated);
            }
        }
    }

    fn publish(&self, event: ScriptEvent) {
        if let Some(stream) = &self.script_events {
            stream.send(event);
        }
    }

    fn invalidate_projection(&self, projection: &mut Projection) {
        projection.connection("OnConnectionLost");
        tracing::debug!(
            epoch = projection.receipts.epoch(),
            "projection invalidated after event overflow"
        );
    }

    fn project_event(&self, projection: &mut Projection, event: &Event) -> Vec<ProjectionEvent> {
        let Some(Value::String(raw)) = event.args.first() else {
            return vec![];
        };
        let direction = match event.name.as_str() {
            "onConnection" => {
                projection.connection(raw);
                tracing::debug!(
                    status = raw,
                    epoch = projection.receipts.epoch(),
                    "projection connection changed"
                );
                return vec![];
            }
            "onExtensionResponse" => PacketDirection::Extension,
            "packetFromClient" => PacketDirection::Client,
            "packetFromServer" => PacketDirection::Server,
            _ => return vec![],
        };
        let PacketDecode::Parsed(packet) = parse_packet(direction, raw) else {
            return vec![];
        };
        let mut progress = projection.packet(&packet, now_ms(), true);
        let mut projected = vec![];
        loop {
            match progress {
                PacketProjection::Applied(output) => {
                    projected = output.events;
                    break;
                }
                PacketProjection::Unhandled => break,
                PacketProjection::NeedHydration(pending) => {
                    tracing::debug!(
                        method = pending.request.method.name(),
                        "projection hydration requested"
                    );
                    let result = bridge::invoke(
                        &mut self.player.lock().unwrap(),
                        pending.request.method.name(),
                        pending
                            .request
                            .args
                            .iter()
                            .cloned()
                            .map(from_json)
                            .collect(),
                    )
                    .ok()
                    .and_then(to_json)
                    .map_or(HydrationResult::Unavailable, HydrationResult::Value);
                    let Ok(next) = projection.resume_hydration(pending, result, now_ms()) else {
                        break;
                    };
                    progress = next;
                }
            }
        }
        tracing::debug!(
            command = packet.command(),
            direction = ?packet.direction(),
            epoch = projection.receipts.epoch(),
            missing = ?projection.current_readiness().missing,
            map = projection.world.map.name,
            cell = projection.world.get_me().map(|me| me.data().entity.cell.clone()),
            hp = projection.world.get_me().map(|me| me.data().entity.hp),
            players = projection.world.players().len(),
            monsters = projection.world.monsters().len(),
            inventory = projection.items.all(lucent_protocol::item::ItemContainer::Inventory).len(),
            "projection processed packet"
        );
        projected
    }
}

fn now_ms() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
        * 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bridge::{Bridge, Error as BridgeError, ScriptEvents, script_events};
    use crate::player::{Msg, Read, Surface, Tasks, Viewport, first_viewport, surface};
    use crate::projection::Projection;
    use lucent_protocol::{item::ItemContainer, readiness::ReadinessKey};
    use ruffle_core::{
        PlayerBuilder,
        external::{ExternalInterfaceProvider, Value as ExternalValue},
    };
    use serde_json::json;
    use std::{
        sync::mpsc,
        time::{Duration, Instant},
    };

    fn running(input: usize, output: usize) -> (Running, Bridge, ScriptEvents) {
        let (bridge, loader_events) = Bridge::with_capacity(input);
        let (producer, events) = script_events(output);
        let now = Instant::now();
        (
            Running {
                settings: Default::default(),
                settings_applied: false,
                loader_events,
                script_events: Some(producer),
                projection: std::sync::Arc::new(parking_lot::RwLock::new(Projection::default())),
                player: PlayerBuilder::new().build(),
                tasks: Tasks::default(),
                viewport: Viewport::new(1024.0, 768.0, 1.0).unwrap(),
                last_tick: now,
                effects: vec![],
                drawn: 0,
                stats_since: now,
                surface: Surface::textures(surface::noop_gpu(), || {}),
            },
            bridge,
            events,
        )
    }

    fn queue(running: &mut Running, bridge: &Bridge, name: &str, raw: String) {
        running
            .player
            .lock()
            .unwrap()
            .mutate_with_update_context(|context| {
                bridge.call_method(context, name, &[ExternalValue::String(raw)]);
            });
    }

    fn extension(running: &mut Running, bridge: &Bridge, data: serde_json::Value) {
        queue(
            running,
            bridge,
            "onExtensionResponse",
            json!({"type":"json", "dataObj":data}).to_string(),
        );
    }

    fn area(running: &mut Running, bridge: &Bridge) {
        extension(
            running,
            bridge,
            json!({
                "cmd":"moveToArea", "areaName":"testroom-1",
                "uoBranch":[{"entID":7,"uoName":"player-a","intHP":100}]
            }),
        );
    }

    fn read<R: serde::Serialize + Send + 'static>(
        running: &mut Running,
        f: impl FnOnce(&Projection) -> R + Send + 'static,
    ) -> R {
        let (request, reply) = Read::new(f);
        let _ = running.handle(Msg::Read(request));
        reply.recv_timeout(Duration::ZERO).unwrap()
    }

    fn quantity(state: &Projection, id: i64) -> Option<f64> {
        state
            .items
            .get(ItemContainer::Inventory, id)
            .map(|item| item.data().quantity)
    }

    #[test]
    fn session_read_observes_preceding_bridge_baseline_and_delta_without_polling_events() {
        let (mut running, bridge, _events) = running(16, 16);
        extension(
            &mut running,
            &bridge,
            json!({"cmd":"initInventory", "items":[{"ItemID":1,"iQty":3}]}),
        );
        extension(
            &mut running,
            &bridge,
            json!({"cmd":"addItems", "items":{"1":{"ItemID":1,"iQty":2}}}),
        );
        assert_eq!(read(&mut running, |state| quantity(state, 1)), Some(5.0));
        assert!(read(&mut running, |state| state
            .current_readiness()
            .state
            .inventory));
    }

    #[test]
    fn session_unavailable_hydration_keeps_processing_following_events() {
        let (mut running, bridge, _events) = running(16, 16);
        area(&mut running, &bridge);
        extension(
            &mut running,
            &bridge,
            json!({"cmd":"initInventory", "items":[]}),
        );
        assert_eq!(
            read(&mut running, |state| (
                state.world.get_me().is_some(),
                state.current_readiness().missing
            )),
            (
                false,
                vec![ReadinessKey::HouseInventory, ReadinessKey::Player]
            )
        );
    }

    #[test]
    fn session_input_overflow_invalidates_before_reads_and_fresh_baselines_recover() {
        let (mut running, bridge, events) = running(2, 16);
        running.projection.write().auth.username = "player-a".into();
        extension(
            &mut running,
            &bridge,
            json!({"cmd":"initInventory", "items":[{"ItemID":1,"iQty":3}]}),
        );
        assert_eq!(read(&mut running, |state| quantity(state, 1)), Some(3.0));
        assert!(
            matches!(events.try_recv().unwrap(), Some(ScriptEvent::Loader { event, .. }) if event.name == "onExtensionResponse")
        );
        area(&mut running, &bridge);
        extension(
            &mut running,
            &bridge,
            json!({"cmd":"initInventory", "items":[]}),
        );
        queue(&mut running, &bridge, "onDebug", "overflow".into());
        assert_eq!(
            read(&mut running, |state| (
                state.receipts.epoch(),
                quantity(state, 1),
                state.current_readiness().missing
            )),
            (1, None, ReadinessKey::ALL.to_vec())
        );
        assert_eq!(events.try_recv().unwrap(), Some(ScriptEvent::Invalidated));
        extension(
            &mut running,
            &bridge,
            json!({"cmd":"loadInventoryBig", "items":[{"ItemID":2,"iQty":9}], "hitems":[]}),
        );
        area(&mut running, &bridge);
        assert_eq!(
            read(&mut running, |state| (
                quantity(state, 2),
                state.current_readiness().missing
            )),
            (Some(9.0), vec![])
        );
    }

    #[test]
    fn session_observer_overflow_reports_loss_without_invalidating_projection() {
        let (mut running, bridge, events) = running(16, 1);
        running.projection.write().auth.username = "player-a".into();
        area(&mut running, &bridge);
        extension(
            &mut running,
            &bridge,
            json!({"cmd":"initInventory", "items":[{"ItemID":1,"iQty":8}]}),
        );
        assert_eq!(
            read(&mut running, |state| (
                state.receipts.epoch(),
                quantity(state, 1)
            )),
            (0, Some(8.0))
        );
        assert_eq!(events.try_recv(), Err(BridgeError::Overflow));
    }

    #[test]
    fn session_server_debug_packets_do_not_duplicate_extension_mutations() {
        let (mut running, bridge, _events) = running(16, 16);
        extension(
            &mut running,
            &bridge,
            json!({"cmd":"initInventory", "items":[{"ItemID":1,"iQty":3}]}),
        );
        queue(
            &mut running,
            &bridge,
            "packetFromServer",
            json!({"cmd":"addItems", "items":{"1":{"ItemID":1,"iQty":2}}}).to_string(),
        );
        extension(
            &mut running,
            &bridge,
            json!({"cmd":"addItems", "items":{"1":{"ItemID":1,"iQty":2}}}),
        );
        assert_eq!(read(&mut running, |state| quantity(state, 1)), Some(5.0));
    }

    #[test]
    fn session_projection_is_isolated_between_players() {
        let (mut first, first_bridge, _first_events) = running(16, 16);
        let (mut second, second_bridge, _second_events) = running(16, 16);
        extension(
            &mut first,
            &first_bridge,
            json!({"cmd":"initInventory", "items":[{"ItemID":1,"iQty":3}]}),
        );
        extension(
            &mut second,
            &second_bridge,
            json!({"cmd":"initInventory", "items":[{"ItemID":1,"iQty":9}]}),
        );
        assert_eq!(read(&mut first, |state| quantity(state, 1)), Some(3.0));
        assert_eq!(read(&mut second, |state| quantity(state, 1)), Some(9.0));
    }

    #[test]
    fn session_reads_before_viewport_are_not_ready_and_dropped_inboxes_stop_reads() {
        let (sender, receiver) = mpsc::channel();
        let (request, reply) = Read::new(|_| 42);
        sender.send(Msg::Read(request)).unwrap();
        sender.send(Msg::Stop).unwrap();
        assert!(first_viewport(&receiver, false, &Default::default()).is_none());
        assert_eq!(
            reply.recv_timeout(Duration::ZERO),
            Err(BridgeError::NotReady)
        );
        let (request, reply) = Read::new(|_| 42);
        sender.send(Msg::Read(request)).unwrap();
        drop(receiver);
        assert_eq!(
            reply.recv_timeout(Duration::ZERO),
            Err(BridgeError::Stopped)
        );
    }

    #[test]
    fn session_read_timeout_preserves_the_queued_reply() {
        let (mut running, _bridge, _events) = running(16, 16);
        let (request, reply) = Read::new(|_| 42);
        assert_eq!(
            reply.recv_timeout(Duration::ZERO),
            Err(BridgeError::Timeout)
        );
        let _ = running.handle(Msg::Read(request));
        assert_eq!(reply.recv_timeout(Duration::ZERO), Ok(42));
    }

    #[test]
    fn panicking_read_returns_an_error_and_the_player_keeps_serving_reads() {
        let (mut running, _bridge, _events) = running(16, 16);
        let (request, reply) = Read::new::<()>(|_| panic!("read failed"));
        let _ = running.handle(Msg::Read(request));
        assert_eq!(
            reply.recv_timeout(Duration::ZERO),
            Err(BridgeError::ReadPanicked)
        );
        assert_eq!(read(&mut running, |_| 42), 42);
    }

    #[test]
    fn session_connection_loss_invalidates_preceding_baselines_and_old_world() {
        let (mut running, bridge, _events) = running(16, 16);
        running.projection.write().auth.username = "player-a".into();
        area(&mut running, &bridge);
        extension(
            &mut running,
            &bridge,
            json!({"cmd":"initInventory", "items":[]}),
        );
        assert!(read(&mut running, |state| state
            .current_readiness()
            .state
            .inventory
            && state.world.get_me().is_some()));
        queue(
            &mut running,
            &bridge,
            "onConnection",
            "OnConnectionLost".into(),
        );
        assert_eq!(
            read(&mut running, |state| (
                state.receipts.epoch(),
                state.world.get_me().is_some(),
                state.current_readiness().missing
            )),
            (1, false, ReadinessKey::ALL.to_vec())
        );
    }

    #[test]
    fn session_connection_resets_notify_projected_event_consumers() {
        let (mut running, bridge, events) = running(16, 16);
        for (status, epoch) in [
            ("OnConnection", 1),
            ("OnConnectionLost", 2),
            ("OnConnectionFailed", 3),
        ] {
            extension(
                &mut running,
                &bridge,
                json!({"cmd":"initInventory", "items":[{"ItemID":1,"iQty":3}]}),
            );
            assert_eq!(read(&mut running, |state| quantity(state, 1)), Some(3.0));
            assert!(
                matches!(events.try_recv().unwrap(), Some(ScriptEvent::Loader { event, .. }) if event.name == "onExtensionResponse")
            );
            queue(&mut running, &bridge, "onConnection", status.into());
            assert_eq!(read(&mut running, |state| quantity(state, 1)), None);
            assert_eq!(read(&mut running, |state| state.receipts.epoch()), epoch);
            assert_eq!(
                events.try_recv().unwrap(),
                Some(ScriptEvent::Loader {
                    event: Event {
                        name: "onConnection".into(),
                        args: vec![Value::String(status.into())]
                    },
                    projected: vec![],
                })
            );
            assert_eq!(events.try_recv().unwrap(), Some(ScriptEvent::Invalidated));
        }
        queue(&mut running, &bridge, "onConnection", "unknown".into());
        assert_eq!(read(&mut running, |state| state.receipts.epoch()), 3);
        assert_eq!(
            events.try_recv().unwrap(),
            Some(ScriptEvent::Loader {
                event: Event {
                    name: "onConnection".into(),
                    args: vec![Value::String("unknown".into())]
                },
                projected: vec![],
            })
        );
        assert_eq!(events.try_recv().unwrap(), None);
    }
}
