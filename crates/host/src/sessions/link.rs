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
    id: u64,
    attachment: AttachmentId,
    detached: Arc<AtomicBool>,
    inbox: Inbox,
    events: ScriptEvents,
    projection: Arc<RwLock<Projection>>,
    records: super::Records,
    stopped: Arc<AtomicBool>,
    packets: Arc<super::operations::Packets>,
    operations: std::sync::mpsc::SyncSender<Option<super::operations::Request>>,
}

impl ScriptLink {
    pub(crate) fn new(
        id: u64,
        attachment: AttachmentId,
        events: ScriptEvents,
        records: super::Records,
        player: super::operations::Endpoint,
        operations: std::sync::mpsc::SyncSender<Option<super::operations::Request>>,
        asks: Arc<super::asks::Shared>,
    ) -> Self {
        Self {
            asks,
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
    pub fn publish_auto_relogin(
        &self,
        records: lucent_protocol::auto_relogin::AutoReloginRecords,
    ) -> Result<(), bridge::Error> {
        self.asks.publish(self.attachment, || {
            *self.records.auto_relogin.write() = records;
        })
    }

    pub fn operate(
        &self,
        operation: lucent_protocol::operation::Operation,
    ) -> Result<u64, bridge::Error> {
        use std::sync::atomic::AtomicU64;
        static NEXT: AtomicU64 = AtomicU64::new(1);
        if self.stopped.load(Ordering::Acquire) {
            return Err(bridge::Error::Stopped);
        }
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        self.operations
            .try_send(Some(super::operations::Request {
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
            }))
            .map_err(|error| match error {
                std::sync::mpsc::TrySendError::Full(_) => bridge::Error::Overflow,
                std::sync::mpsc::TrySendError::Disconnected(_) => bridge::Error::Stopped,
            })?;
        Ok(id)
    }

    pub fn command(&self, command: lucent_protocol::command::Command) -> Pending<String> {
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
        let bot_link = session.attach().unwrap();
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
        let link = session.attach().unwrap();
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
        let observer = session.attach().unwrap();
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
        let link = session.attach().unwrap();
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
