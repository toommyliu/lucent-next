use lucent_protocol::{query::Query, settings::GameSettings};
use parking_lot::RwLock;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::task::Waker;
use std::time::{Duration, Instant};

use crate::bridge::{self, Pending, PendingCall, ScriptEvent, ScriptEvents, Value};
use crate::player::Inbox;
use crate::projection::{Projection, answer};

pub struct ScriptLink {
    id: u64,
    inbox: Inbox,
    events: ScriptEvents,
    projection: Arc<RwLock<Projection>>,
    settings: Arc<RwLock<GameSettings>>,
    stopped: Arc<AtomicBool>,
}

impl ScriptLink {
    pub(crate) fn new(
        id: u64,
        inbox: Inbox,
        events: ScriptEvents,
        projection: Arc<RwLock<Projection>>,
        settings: Arc<RwLock<GameSettings>>,
        stopped: Arc<AtomicBool>,
    ) -> Self {
        Self {
            id,
            inbox,
            events,
            projection,
            settings,
            stopped,
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn try_event(&self) -> Result<Option<ScriptEvent>, bridge::Error> {
        self.events.try_recv()
    }

    pub fn overflowed(&self) -> bool {
        self.events.overflowed()
    }

    pub fn set_waker(&self, waker: Waker) {
        self.events.set_waker(waker);
    }

    pub fn call_loader(&self, name: impl Into<String>, args: Vec<Value>) -> PendingCall {
        self.inbox.call_loader(name, args)
    }

    pub fn command(&self, command: lucent_protocol::command::Command) -> Pending<String> {
        let (reply, pending) = Pending::channel();
        self.inbox.post(crate::player::Msg::Command(command, reply));
        pending
    }

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
        let start = Instant::now();
        let settings = self
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
            answer(projection, &settings, query.take().unwrap())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing;
    use std::{sync::mpsc, thread};

    #[test]
    fn settings_reads_do_not_wait_for_the_projection() {
        let host = testing::headless_host();
        let mut session = host.start_session(testing::headless_surface()).unwrap();
        let link = session.take_script_link().unwrap();
        let _projection = link.projection.write();
        let mut query = Some(Query::Settings {});
        let answer = link.query_for(Duration::ZERO, &mut query).unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&answer).unwrap()["walkSpeed"],
            8.0
        );
    }

    #[test]
    fn a_contended_read_times_out_without_calling_its_reader_and_stops_with_its_player() {
        let host = testing::headless_host();
        let mut session = host.start_session(testing::headless_surface()).unwrap();
        let link = session.take_script_link().unwrap();
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
