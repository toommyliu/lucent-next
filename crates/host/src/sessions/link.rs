use lucent_protocol::query::Query;
use parking_lot::RwLock;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::task::Waker;
use std::time::{Duration, Instant};

use super::AttachmentId;
use crate::bridge::{self, Pending, PendingCall, ScriptEvent, ScriptEvents, Value};
use crate::player::{Inbox, Msg};
use crate::projection::{Projection, answer};

#[derive(Clone)]
pub struct PlayerStatus(Arc<AtomicBool>);

impl PlayerStatus {
    pub fn is_stopped(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

pub struct ScriptLink {
    asks: Arc<super::asks::Shared>,
    claim_binding: super::claims::Binding,
    id: u64,
    attachment: AttachmentId,
    detached: Arc<AtomicBool>,
    inbox: Inbox,
    events: ScriptEvents,
    projection: Arc<RwLock<Projection>>,
    pub(crate) records: super::Records,
    stopped: Arc<AtomicBool>,
    packets: Arc<super::operations::Packets>,
    operations: Arc<super::operations::Shared>,
}

impl ScriptLink {
    pub(crate) fn new(
        id: u64,
        attachment: AttachmentId,
        events: ScriptEvents,
        records: super::Records,
        player: super::operations::Endpoint,
        operations: Arc<super::operations::Shared>,
        asks: Arc<super::asks::Shared>,
    ) -> Self {
        Self {
            asks,
            claim_binding: player.packets.binding(),
            id,
            attachment,
            detached: Arc::default(),
            inbox: player.inbox,
            events,
            projection: player.projection,
            records,
            stopped: player.stopped,
            packets: player.packets,
            operations,
        }
    }

    pub(crate) fn inbox(&self) -> Inbox {
        self.inbox.clone()
    }

    pub fn player_status(&self) -> PlayerStatus {
        PlayerStatus(self.stopped.clone())
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn try_event(&self) -> Result<Option<ScriptEvent>, bridge::Error> {
        match self.asks.try_event(self.attachment) {
            Ok(Some(event)) => Ok(Some(event)),
            Ok(None) => self.events.try_recv(),
            Err(bridge::Error::Stopped) if self.stopped.load(Ordering::Acquire) => {
                self.events.try_recv()
            }
            Err(error) => Err(error),
        }
    }

    pub fn overflowed(&self) -> bool {
        self.events.overflowed()
    }

    pub fn set_waker(&self, waker: Waker) {
        self.events.set_waker(waker.clone());
        self.asks.set_waker(self.attachment, waker);
    }

    pub(crate) fn relogin_snapshot(&self) -> Pending<crate::automation::relogin::Snapshot> {
        self.inbox
            .read_projection(|projection| crate::automation::relogin::Snapshot {
                username: projection.auth.username.clone(),
                password: projection.auth.password.clone(),
                connected: projection.connected,
                logged_in: projection.auth.logged_in,
                ready: projection.current_readiness().missing.is_empty(),
                servers: projection
                    .auth
                    .servers()
                    .iter()
                    .map(|s| s.snapshot().data.name)
                    .collect(),
            })
    }

    pub fn cached_logged_in(&self) -> Pending<bool> {
        self.inbox
            .read_projection(|projection| projection.auth.logged_in)
    }

    pub fn readiness(&self) -> Pending<bool> {
        self.inbox
            .read_projection(|projection| projection.current_readiness().missing.is_empty())
    }

    pub fn call_loader(&self, name: impl Into<String>, args: Vec<Value>) -> PendingCall {
        self.inbox.call_loader(name, args)
    }

    pub fn claim_bot(&self) -> Result<(), bridge::Error> {
        self.asks.claim_bot(self.attachment)
    }

    pub fn ask(&self, payload: serde_json::Value) -> Result<u64, bridge::Error> {
        if self.stopped.load(Ordering::Acquire) {
            return Err(bridge::Error::Stopped);
        }
        self.asks.ask(self.attachment, payload)
    }
    pub fn is_ask_pending(&self, id: u64) -> bool {
        self.asks.is_pending(self.attachment, id)
    }
    pub fn cancel_ask(&self, id: u64) {
        self.asks.cancel(self.attachment, id);
    }
    pub fn answer(
        &self,
        id: u64,
        answer: Result<serde_json::Value, bridge::Error>,
    ) -> Result<(), bridge::Error> {
        self.asks.answer(self.attachment, id, answer)
    }
    pub fn operate(
        &self,
        operation: lucent_protocol::operation::Operation,
    ) -> Result<u64, bridge::Error> {
        let id = self.prepare_operation()?;
        if let Err(error) = self.submit_operation(id, operation) {
            self.cancel_operation(id);
            return Err(error);
        }
        Ok(id)
    }

    pub fn prepare_operation(&self) -> Result<u64, bridge::Error> {
        if self.stopped.load(Ordering::Acquire) {
            return Err(bridge::Error::Stopped);
        }
        self.operations.prepare(self.attachment)
    }

    pub fn submit_operation(
        &self,
        id: u64,
        operation: lucent_protocol::operation::Operation,
    ) -> Result<(), bridge::Error> {
        self.operations.submit(super::operations::Request {
            id,
            attachment: self.attachment,
            detached: self.detached.clone(),
            operation,
            player: super::operations::Endpoint {
                inbox: self.inbox.clone(),
                projection: self.projection.clone(),
                stopped: self.stopped.clone(),
                packets: self.packets.clone(),
            },
        })
    }

    pub fn acquire_consumable_claim(
        &self,
        item: Option<i64>,
    ) -> Result<Option<super::ClaimGrant>, bridge::Error> {
        self.packets.acquire_claim(
            super::claims::Owner::Script(self.attachment),
            self.claim_binding,
            item,
        )
    }

    pub fn release_claim(&self, id: u64) {
        self.packets
            .release_claim(super::claims::Owner::Script(self.attachment), id);
    }

    pub fn cancel_operation(&self, id: u64) {
        self.operations.cancel(self.attachment, id);
    }

    #[cfg(feature = "testing")]
    pub fn set_operation_probe(&self, probe: impl Fn(u64, bool) + Send + Sync + 'static) {
        self.operations.set_probe(probe);
    }

    #[cfg(feature = "testing")]
    pub fn operation_canceller(&self) -> impl Fn(u64) + Send + Sync + 'static + use<> {
        let operations = self.operations.clone();
        let owner = self.attachment;
        move |id| operations.cancel(owner, id)
    }

    pub fn command(&self, command: lucent_protocol::command::Command) -> Pending<String> {
        use super::relogin::Edit;
        use lucent_protocol::command::Command;
        let submit = |edit| {
            if self.stopped.load(Ordering::Acquire) {
                return Pending::stopped();
            }
            self.records
                .relogin
                .submit(edit, self.detached.clone(), Instant::now())
        };
        let command = match command {
            Command::SetAutoReloginEnabled { enabled } => return submit(Edit::Enabled(enabled)),
            Command::SetAutoReloginDelay { delay_ms } => return submit(Edit::Delay(delay_ms)),
            Command::SetAutoReloginServer { server } => return submit(Edit::Server(server)),
            other => other,
        };
        let (reply, pending) = Pending::channel();
        self.inbox.post(crate::player::Msg::Command(command, reply));
        pending
    }

    #[cfg(feature = "testing")]
    pub fn query(&self, query: Query) -> Result<String, bridge::Error> {
        let mut query = Some(query);
        loop {
            match self.query_for(Duration::from_millis(20), &mut query) {
                Err(bridge::Error::Timeout) => {}
                result => return result,
            }
        }
    }

    pub fn query_for(
        &self,
        timeout: Duration,
        query: &mut Option<Query>,
    ) -> Result<String, bridge::Error> {
        if self.stopped.load(Ordering::Acquire) {
            return Err(bridge::Error::Stopped);
        }
        if matches!(query, Some(Query::AutoRelogin {})) {
            let records = self
                .records
                .auto_relogin
                .try_read_for(timeout)
                .ok_or(bridge::Error::Timeout)?;
            query.take();
            return Ok(serde_json::to_string(&*records).unwrap());
        }
        if matches!(query, Some(Query::AutoZone {})) {
            let config = self
                .records
                .auto_zone
                .try_read_for(timeout)
                .ok_or(bridge::Error::Timeout)?;
            if self.stopped.load(Ordering::Acquire) {
                return Err(bridge::Error::Stopped);
            }
            query.take();
            return Ok(serde_json::to_string(&*config).unwrap());
        }
        let start = Instant::now();
        let settings = self
            .records
            .settings
            .try_read_for(timeout)
            .ok_or(bridge::Error::Timeout)?;
        if self.stopped.load(Ordering::Acquire) {
            return Err(bridge::Error::Stopped);
        }
        if matches!(query, Some(Query::Settings {})) {
            query.take();
            return Ok(serde_json::to_string(&*settings).unwrap());
        }
        self.read_for(timeout.saturating_sub(start.elapsed()), |projection| {
            answer(
                projection,
                &settings,
                &self.records.auto_zone,
                &self.records.auto_relogin,
                query.take().unwrap(),
            )
        })
    }

    pub fn read<R>(&self, read: impl FnOnce(&Projection) -> R) -> Result<R, bridge::Error> {
        let mut read = Some(read);
        loop {
            match self.read_for(Duration::from_millis(20), |projection| {
                read.take().unwrap()(projection)
            }) {
                Err(bridge::Error::Timeout) => {}
                result => return result,
            }
        }
    }

    pub fn read_for<R>(
        &self,
        timeout: Duration,
        read: impl FnOnce(&Projection) -> R,
    ) -> Result<R, bridge::Error> {
        if self.stopped.load(Ordering::Acquire) {
            return Err(bridge::Error::Stopped);
        }
        let projection = self
            .projection
            .try_read_for(timeout)
            .ok_or(bridge::Error::Timeout)?;
        if self.stopped.load(Ordering::Acquire) {
            return Err(bridge::Error::Stopped);
        }
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| read(&projection)))
            .map_err(|_| bridge::Error::ReadPanicked)
    }
}

impl Drop for ScriptLink {
    fn drop(&mut self) {
        self.detached.store(true, Ordering::Release);
        self.packets
            .release_owner(super::claims::Owner::Script(self.attachment));
        self.operations.detach(self.attachment);
        self.asks.detach(self.attachment);
        self.inbox.post(Msg::Detach(self.attachment));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing;
    use std::{sync::mpsc, thread};

    #[test]
    fn auto_zone_invalidation_rereads_the_host_record_without_a_config_event() {
        let host = testing::headless_host();
        let (mut session, writer) = testing::start_probe(&host, testing::ProbeMovie::new());
        let mut link = session
            .attach(crate::automation::AutoZonePolicy::INTERESTS)
            .unwrap();
        let (events, receiver) =
            crate::bridge::script_events(8, crate::bridge::EventInterests::ALL);
        link.events = receiver;
        let mut policy = crate::automation::AutoZonePolicy::new(link, Waker::noop().clone());
        let now = Instant::now();
        assert_eq!(policy.advance(now), None);
        writer
            .command(
                serde_json::from_value(serde_json::json!({
                    "kind":"patchAutoZone", "patch":{"enabled":true,"map":"queeniona"}
                }))
                .unwrap(),
            )
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
        events.send(ScriptEvent::Invalidated);
        assert_eq!(policy.advance(now), None);
        events.send(ScriptEvent::Loader {
            event: crate::bridge::Event {
                name: "onExtensionResponse".into(),
                args: vec![],
            },
            projected: vec![lucent_protocol::event::ProjectionEvent::Zone {
                map: "queeniona".into(),
                zone: "".into(),
            }],
        });
        assert_eq!(policy.advance(now), Some(now + Duration::from_millis(500)));
    }

    #[test]
    fn projection_and_config_contention_retry_auto_zone_without_a_failure() {
        let logs = crate::automation::test_logs::Logs::default();
        let _guard = tracing::dispatcher::set_default(&logs.dispatch());
        for config_lock in [false, true] {
            let host = testing::headless_host();
            let (mut session, writer) = testing::start_probe(
                &host,
                testing::ProbeMovie::new()
                    .responding("player.walkTo", [(serde_json::json!(true), vec![])]),
            );
            writer
                .command(
                    serde_json::from_value(serde_json::json!({
                        "kind":"patchAutoZone", "patch":{"enabled":true,"map":"queeniona"}
                    }))
                    .unwrap(),
                )
                .recv_timeout(Duration::from_secs(5))
                .unwrap();
            let mut link = session
                .attach(crate::automation::AutoZonePolicy::INTERESTS)
                .unwrap();
            let projection = link.projection.clone();
            {
                let mut state = projection.write();
                state.packet(&lucent_protocol::Packet::Extension {
                    command: "moveToArea".into(),
                    data: serde_json::json!({"cmd":"moveToArea", "areaName":"queeniona-1", "uoBranch":[{"entID":7,"uoName":"player-a","intHP":100}]}),
                    raw: String::new(), encoding: lucent_protocol::PacketEncoding::Json,
                }, 0., false);
                state.world.set_self("player-a");
                assert!(state.world.get_me().unwrap().data().entity.alive());
            }
            let config = link.records.auto_zone.clone();
            let (events, receiver) =
                crate::bridge::script_events(8, crate::bridge::EventInterests::ALL);
            link.events = receiver;
            let mut policy = crate::automation::AutoZonePolicy::new(link, Waker::noop().clone());
            let now = Instant::now();
            if !config_lock {
                policy.advance(now);
                events.send(ScriptEvent::Loader {
                    event: crate::bridge::Event {
                        name: "onExtensionResponse".into(),
                        args: vec![],
                    },
                    projected: vec![lucent_protocol::event::ProjectionEvent::Zone {
                        map: "queeniona".into(),
                        zone: "".into(),
                    }],
                });
                policy.advance(now);
            }
            let projection_guard = (!config_lock).then(|| projection.write());
            let config_guard = config_lock.then(|| config.write());
            let at = now + Duration::from_millis(500);
            let before = Instant::now();
            let next = policy.advance(at).expect("contention must retry");
            assert!(
                before.elapsed() < Duration::from_millis(100),
                "the policy blocked on a writer"
            );
            assert!(
                next > at && next <= at + Duration::from_millis(20),
                "contention used the failure backoff: {next:?}"
            );
            drop(projection_guard);
            drop(config_guard);
            if config_lock {
                assert_eq!(policy.advance(next), None);
                events.send(ScriptEvent::Loader {
                    event: crate::bridge::Event {
                        name: "onExtensionResponse".into(),
                        args: vec![],
                    },
                    projected: vec![lucent_protocol::event::ProjectionEvent::Zone {
                        map: "queeniona".into(),
                        zone: "".into(),
                    }],
                });
                policy.advance(next);
                policy.advance(next + Duration::from_millis(500));
            } else {
                policy.advance(next);
            }
            writer
                .call_loader("probe.echo", vec![])
                .recv_timeout(Duration::from_secs(5))
                .unwrap();
            let mut walks = vec![];
            while let Some(event) = writer.try_event().unwrap() {
                if let ScriptEvent::Loader { event, .. } = event
                    && event.name == "onProbeCall"
                    && event.args.first() == Some(&Value::String("player.walkTo".into()))
                {
                    walks.push(event.args);
                }
            }
            assert_eq!(
                walks,
                vec![vec![
                    Value::String("player.walkTo".into()),
                    Value::List(vec![Value::Number(490.), Value::Number(320.)])
                ]]
            );
        }
        assert_eq!(logs.count("auto zone tick failed"), 0);
        assert_eq!(logs.count("auto zone recovered"), 0);
    }

    #[test]
    fn auto_zone_boot_reads_receive_patches_queued_before_attach() {
        use lucent_protocol::auto_zone::{AutoZoneConfig, AutoZoneMap};
        let host = testing::headless_host();
        let (mut session, writer) = testing::start_probe(&host, testing::ProbeMovie::new());
        let (blocked, entered) = mpsc::channel();
        let (release, resume) = mpsc::channel();
        let barrier = session.read_projection(move |_| {
            blocked.send(()).unwrap();
            resume.recv_timeout(Duration::from_secs(5)).unwrap();
        });
        entered.recv_timeout(Duration::from_secs(5)).unwrap();
        let patch = writer.command(
            serde_json::from_value(serde_json::json!({
                "kind":"patchAutoZone", "patch":{"enabled":true,"map":"ledgermayne"}
            }))
            .unwrap(),
        );
        let bot_link = session.attach(crate::bridge::EventInterests::ALL).unwrap();
        let mut query = Some(Query::AutoZone {});
        let boot = bot_link
            .query_for(Duration::from_secs(1), &mut query)
            .unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&boot).unwrap(),
            serde_json::json!({"enabled":false,"map":null})
        );
        release.send(()).unwrap();
        barrier.recv_timeout(Duration::from_secs(5)).unwrap();
        patch.recv_timeout(Duration::from_secs(5)).unwrap();
        bot_link
            .command(
                serde_json::from_value(serde_json::json!({"kind":"patchAutoZone","patch":{}}))
                    .unwrap(),
            )
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
        assert_eq!(
            bot_link.try_event().unwrap(),
            Some(ScriptEvent::AutoZoneChanged {
                config: AutoZoneConfig {
                    enabled: true,
                    map: Some(AutoZoneMap::Ledgermayne)
                }
            })
        );
    }

    #[test]
    fn settings_reads_do_not_wait_for_the_projection() {
        let host = testing::headless_host();
        let mut session = host.start_session(testing::headless_surface()).unwrap();
        let link = session.attach(crate::bridge::EventInterests::ALL).unwrap();
        let _projection = link.projection.write();
        let mut query = Some(Query::Settings {});
        let answer = link.query_for(Duration::ZERO, &mut query).unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&answer).unwrap()["walkSpeed"],
            8.0
        );
    }

    #[test]
    fn an_orphaned_accept_runs_to_its_end() {
        let host = testing::headless_host();
        let (mut session, link) = testing::start_probe(&host, testing::ProbeMovie::new()
            .constant("auth.isLoggedIn", serde_json::json!(true))
            .accepting_drop(Duration::from_millis(100), serde_json::json!({
                "type":"json","dataObj":{"cmd":"getDrop","ItemID":70,"bSuccess":true,"iQtyNow":3}
            }).to_string()));
        link.call_loader(
            "probe.emit",
            vec![
                Value::String("onExtensionResponse".into()),
                Value::String(
                    serde_json::json!({"type":"json","dataObj":{"cmd":"dropItem","items":{"70":{
                        "ItemID":70,"sName":"Dropped Item","iQty":2
                    }}}})
                    .to_string(),
                ),
            ],
        )
        .recv_timeout(Duration::from_secs(1))
        .unwrap();
        link.operate(lucent_protocol::operation::Operation::AcceptDrop {
            selector: lucent_protocol::query::ItemSelector::Id(70.0),
        })
        .unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(1);
        loop {
            if matches!(link.try_event().unwrap(), Some(ScriptEvent::Loader { event, .. }) if event.name == "onProbeAccept")
            {
                break;
            }
            assert!(std::time::Instant::now() < deadline);
            thread::sleep(Duration::from_millis(1));
        }
        let observer = session.attach(crate::bridge::EventInterests::ALL).unwrap();
        drop(link);
        let id = observer
            .operate(lucent_protocol::operation::Operation::AcceptDrop {
                selector: lucent_protocol::query::ItemSelector::Id(999.0),
            })
            .unwrap();
        loop {
            if let Some(ScriptEvent::Operation {
                id: answered,
                answer,
            }) = observer.try_event().unwrap()
            {
                assert_eq!(answered, id);
                assert_eq!(
                    answer,
                    Ok(lucent_protocol::operation::OperationAnswer::AcceptDrop(
                        false
                    ))
                );
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "worker did not finish the orphaned accept and answer the next request"
            );
            thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(
            observer
                .read(|projection| projection
                    .items
                    .get(lucent_protocol::item::ItemContainer::Inventory, 70)
                    .map(|item| item.data().quantity))
                .unwrap(),
            Some(3.0)
        );
        assert_eq!(observer.try_event(), Ok(None));
    }

    #[test]
    fn a_contended_read_times_out_without_calling_its_reader_and_stops_with_its_player() {
        let host = testing::headless_host();
        let mut session = host.start_session(testing::headless_surface()).unwrap();
        let link = session.attach(crate::bridge::EventInterests::ALL).unwrap();
        let projection = link.projection.clone();
        let _guard = projection.write();
        let (timed_out, result) = mpsc::channel();
        let reader = thread::spawn(move || {
            let mut called = false;
            let result = link.read_for(Duration::from_millis(20), |_| called = true);
            timed_out.send((result, called)).unwrap();
            link.read(|_| 42)
        });
        assert_eq!(
            result.recv_timeout(Duration::from_secs(5)).unwrap(),
            (Err(bridge::Error::Timeout), false)
        );
        drop(session);
        assert_eq!(reader.join().unwrap(), Err(bridge::Error::Stopped));
    }
}
