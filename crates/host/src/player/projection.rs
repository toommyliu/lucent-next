use std::time::{SystemTime, UNIX_EPOCH};

use lucent_protocol::{PacketDirection, hydration::HydrationResult};

use super::{Running, SessionEvent};
use crate::bridge::{self, Error, Event, Value};
use crate::projection::{
    PacketProjection,
    packet::{PacketDecode, parse_packet},
};

impl Running {
    pub(super) fn drain_events(&mut self) {
        loop {
            match self.streams.loader_events.try_recv() {
                Err(Error::Overflow) => self.invalidate_projection(),
                Ok(Some(event)) => {
                    self.project_event(&event);
                    self.streams.loader_notifications.send(event);
                }
                Ok(None) | Err(_) => break,
            }
        }
    }

    fn invalidate_projection(&mut self) {
        self.streams.loader_notifications.mark_overflow();
        self.projection.connection("OnConnectionLost");
        tracing::debug!(
            epoch = self.projection.receipts.epoch(),
            "projection invalidated after event overflow"
        );
        self.streams.notifications.send(SessionEvent::Invalidated);
    }

    fn project_event(&mut self, event: &Event) {
        let Some(Value::String(raw)) = event.args.first() else {
            return;
        };
        let direction = match event.name.as_str() {
            "onConnection" => {
                let previous_epoch = self.projection.receipts.epoch();
                self.projection.connection(raw);
                let epoch = self.projection.receipts.epoch();
                if epoch != previous_epoch {
                    self.streams.notifications.send(SessionEvent::Invalidated);
                }
                tracing::debug!(
                    status = raw,
                    epoch = self.projection.receipts.epoch(),
                    "projection connection changed"
                );
                return;
            }
            "onExtensionResponse" => PacketDirection::Extension,
            "packetFromClient" => PacketDirection::Client,
            "packetFromServer" => PacketDirection::Server,
            _ => return,
        };
        let PacketDecode::Parsed(packet) = parse_packet(direction, raw) else {
            return;
        };
        let mut progress = self.projection.packet(&packet, now_ms(), true);
        loop {
            match progress {
                PacketProjection::Applied(output) => {
                    for event in output.events {
                        self.streams
                            .notifications
                            .send(SessionEvent::Projection(event));
                    }
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
                    let Ok(next) = self.projection.resume_hydration(pending, result, now_ms())
                    else {
                        break;
                    };
                    progress = next;
                }
            }
        }
        tracing::debug!(
            command = packet.command(),
            direction = ?packet.direction(),
            epoch = self.projection.receipts.epoch(),
            missing = ?self.projection.current_readiness().missing,
            map = self.projection.world.map.name,
            cell = self.projection.world.get_me().map(|me| me.data().entity.cell.clone()),
            hp = self.projection.world.get_me().map(|me| me.data().entity.hp),
            players = self.projection.world.players().len(),
            monsters = self.projection.world.monsters().len(),
            inventory = self.projection.items.all(lucent_protocol::item::ItemContainer::Inventory).len(),
            "projection processed packet"
        );
    }
}

fn now_ms() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
        * 1000.0
}

fn to_json(value: Value) -> Option<serde_json::Value> {
    Some(match value {
        Value::Undefined => return None,
        Value::Null => serde_json::Value::Null,
        Value::Bool(value) => value.into(),
        Value::Number(value) => {
            serde_json::Number::from_f64(value).map_or(serde_json::Value::Null, Into::into)
        }
        Value::String(value) => value.into(),
        Value::List(values) => values
            .into_iter()
            .map(|value| to_json(value).unwrap_or(serde_json::Value::Null))
            .collect::<Vec<_>>()
            .into(),
        Value::Object(values) => serde_json::Value::Object(
            values
                .into_iter()
                .filter_map(|(key, value)| to_json(value).map(|value| (key, value)))
                .collect(),
        ),
    })
}

fn from_json(value: serde_json::Value) -> Value {
    match value {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(value) => Value::Bool(value),
        serde_json::Value::Number(value) => Value::Number(value.as_f64().unwrap()),
        serde_json::Value::String(value) => Value::String(value),
        serde_json::Value::Array(values) => {
            Value::List(values.into_iter().map(from_json).collect())
        }
        serde_json::Value::Object(values) => Value::Object(
            values
                .into_iter()
                .map(|(key, value)| (key, from_json(value)))
                .collect(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bridge::{Bridge, Error as BridgeError, Events, event_queue};
    use crate::player::{
        EventStreams, Msg, Read, Surface, Tasks, Viewport, first_viewport, surface,
    };
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

    #[test]
    fn hydration_keeps_useful_fields_when_optional_values_are_undefined() {
        let value = Value::Object(
            [
                ("uoName".into(), Value::String("player-a".into())),
                ("optional".into(), Value::Undefined),
                (
                    "nested".into(),
                    Value::List(vec![
                        Value::Number(7.0),
                        Value::Undefined,
                        Value::Number(f64::NAN),
                    ]),
                ),
            ]
            .into(),
        );
        assert_eq!(
            to_json(value),
            Some(json!({"uoName":"player-a", "nested":[7.0,null,null]}))
        );
    }

    #[test]
    fn undefined_hydration_is_unavailable_while_null_and_numbers_are_values() {
        assert_eq!(to_json(Value::Undefined), None);
        assert_eq!(to_json(Value::Null), Some(json!(null)));
        assert_eq!(to_json(Value::Number(7.0)), Some(json!(7.0)));
    }
    fn running(input: usize, output: usize) -> (Running, Bridge, Events, Events<SessionEvent>) {
        let (bridge, loader_events) = Bridge::with_capacity(input);
        let (loader_notifications, raw) = event_queue(output);
        let (notifications, projected) = event_queue(output);
        let now = Instant::now();
        (
            Running {
                streams: EventStreams {
                    loader_events,
                    loader_notifications,
                    notifications,
                },
                projection: Projection::default(),
                player: PlayerBuilder::new().build(),
                tasks: Tasks::default(),
                viewport: Viewport::new(1024.0, 768.0, 1.0).unwrap(),
                last_tick: now,
                effects: vec![],
                drawn: 0,
                stats_since: now,
                surface: Surface::textures(surface::tests::noop_gpu(), || {}),
            },
            bridge,
            raw,
            projected,
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
        let (mut running, bridge, _raw, _projected) = running(16, 16);
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
        let (mut running, bridge, _raw, _projected) = running(16, 16);
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
        let (mut running, bridge, raw, projected) = running(2, 16);
        running.projection.auth.username = "player-a".into();
        extension(
            &mut running,
            &bridge,
            json!({"cmd":"initInventory", "items":[{"ItemID":1,"iQty":3}]}),
        );
        assert_eq!(read(&mut running, |state| quantity(state, 1)), Some(3.0));
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
        assert_eq!(raw.try_recv(), Err(BridgeError::Overflow));
        assert_eq!(
            projected.try_recv().unwrap(),
            Some(SessionEvent::Invalidated)
        );
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
        let (mut running, bridge, raw, projected) = running(16, 1);
        running.projection.auth.username = "player-a".into();
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
        assert_eq!(raw.try_recv(), Err(BridgeError::Overflow));
        assert_eq!(projected.try_recv(), Err(BridgeError::Overflow));
    }

    #[test]
    fn session_server_debug_packets_do_not_duplicate_extension_mutations() {
        let (mut running, bridge, _raw, _projected) = running(16, 16);
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
        let (mut first, first_bridge, _first_raw, _first_projected) = running(16, 16);
        let (mut second, second_bridge, _second_raw, _second_projected) = running(16, 16);
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
        assert_eq!(first_viewport(&receiver, false), None);
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
        let (mut running, _bridge, _raw, _projected) = running(16, 16);
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
        let (mut running, _bridge, _raw, _projected) = running(16, 16);
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
        let (mut running, bridge, _raw, _projected) = running(16, 16);
        running.projection.auth.username = "player-a".into();
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
        let (mut running, bridge, _raw, projected) = running(16, 16);
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
            queue(&mut running, &bridge, "onConnection", status.into());
            assert_eq!(read(&mut running, |state| quantity(state, 1)), None);
            assert_eq!(read(&mut running, |state| state.receipts.epoch()), epoch);
            assert_eq!(
                projected.try_recv().unwrap(),
                Some(SessionEvent::Invalidated)
            );
        }
        queue(&mut running, &bridge, "onConnection", "unknown".into());
        assert_eq!(read(&mut running, |state| state.receipts.epoch()), 3);
        assert_eq!(projected.try_recv().unwrap(), None);
    }
}
