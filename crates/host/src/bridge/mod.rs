use std::sync::mpsc::{self, RecvTimeoutError, TrySendError};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::task::Waker;
use std::time::Duration;

pub use lucent_protocol::bridge::{Event, Value};
use lucent_protocol::event::ProjectionEvent;
use ruffle_core::Player;
use ruffle_core::context::UpdateContext;
use ruffle_core::external::{ExternalInterfaceProvider, Value as ExternalValue};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("the player has stopped")]
    Stopped,
    #[error("the player has not received its first viewport")]
    NotReady,
    #[error("the loader has not registered callback {0}")]
    Unavailable(String),
    #[error("timed out waiting for the player")]
    Timeout,
    #[error("events were discarded because the event queue overflowed")]
    Overflow,
    #[error("the projection read panicked")]
    ReadPanicked,
    #[error("couldn't decode {callback}: {reason}")]
    CommandDecode { callback: String, reason: String },
}

pub struct Pending<T>(mpsc::Receiver<Result<T, Error>>);

pub type PendingCall = Pending<Value>;
pub type PendingRead<T> = Pending<T>;

impl<T> Pending<T> {
    pub(crate) fn channel() -> (mpsc::Sender<Result<T, Error>>, Self) {
        let (sender, receiver) = mpsc::channel();
        (sender, Self(receiver))
    }

    pub(crate) fn stopped() -> Self {
        Self::channel().1
    }

    /// A timeout does not cancel the request. Keep using this reply to receive its result.
    pub fn recv_timeout(&self, timeout: Duration) -> Result<T, Error> {
        self.0.recv_timeout(timeout).map_err(receive_error)?
    }
}

pub(crate) struct Call {
    name: String,
    args: Vec<Value>,
    reply: mpsc::Sender<Result<Value, Error>>,
}

impl Call {
    pub(crate) fn new(name: String, args: Vec<Value>) -> (Call, PendingCall) {
        let (reply, pending) = Pending::channel();
        (Call { name, args, reply }, pending)
    }

    pub(crate) fn reject(self, error: Error) {
        let _ = self.reply.send(Err(error));
    }

    pub(crate) fn dispatch(&mut self, player: &mut Player) -> Result<Value, Error> {
        invoke(player, &self.name, std::mem::take(&mut self.args))
    }

    pub(crate) fn resolve(self, result: Result<Value, Error>) {
        let _ = self.reply.send(result);
    }
}

pub(crate) fn invoke(player: &mut Player, name: &str, args: Vec<Value>) -> Result<Value, Error> {
    player.mutate_with_update_context(|context| {
        match context.external_interface.get_callback(name) {
            Some(callback) => Ok(from_external(callback.call(
                context,
                name,
                args.into_iter().map(into_external),
            ))),
            None => Err(Error::Unavailable(name.to_owned())),
        }
    })
}

#[derive(Clone, Debug, PartialEq)]
pub enum ScriptEvent {
    AutoZoneChanged {
        config: lucent_protocol::auto_zone::AutoZoneConfig,
    },
    Loader {
        event: Event,
        projected: Vec<ProjectionEvent>,
    },
    Invalidated,
    Operation {
        id: u64,
        answer: Result<lucent_protocol::operation::OperationAnswer, Error>,
    },
}

pub(crate) struct ScriptEventSender {
    events: Option<EventSender<ScriptEvent>>,
    waker: Arc<Mutex<Option<Waker>>>,
}

pub(crate) struct ScriptEvents {
    events: Events<ScriptEvent>,
    waker: Arc<Mutex<Option<Waker>>>,
}

pub(crate) fn script_events(capacity: usize) -> (ScriptEventSender, ScriptEvents) {
    let (sender, events) = event_queue(capacity);
    let waker = Arc::new(Mutex::new(None));
    (
        ScriptEventSender {
            events: Some(sender),
            waker: waker.clone(),
        },
        ScriptEvents { events, waker },
    )
}

impl ScriptEventSender {
    pub(crate) fn send(&self, event: ScriptEvent) {
        self.events.as_ref().unwrap().send(event);
        self.wake();
    }

    fn wake(&self) {
        let waker = self.waker.lock().unwrap().clone();
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

impl Drop for ScriptEventSender {
    fn drop(&mut self) {
        self.events.take();
        self.wake();
    }
}

impl ScriptEvents {
    pub(crate) fn try_recv(&self) -> Result<Option<ScriptEvent>, Error> {
        self.events.try_recv()
    }

    pub(crate) fn overflowed(&self) -> bool {
        self.events.overflow.load(Ordering::Acquire)
    }

    pub(crate) fn set_waker(&self, waker: Waker) {
        *self.waker.lock().unwrap() = Some(waker);
    }
}

pub(crate) const EVENT_CAPACITY: usize = 1024;

enum QueueEntry<T> {
    Event(T),
    Overflow,
}

pub(crate) struct EventSender<T> {
    sender: mpsc::SyncSender<QueueEntry<T>>,
    overflow: Arc<AtomicBool>,
}

pub(crate) struct Events<T = Event> {
    receiver: mpsc::Receiver<QueueEntry<T>>,
    overflow: Arc<AtomicBool>,
}

pub(crate) fn event_queue<T>(capacity: usize) -> (EventSender<T>, Events<T>) {
    let (sender, receiver) = mpsc::sync_channel(capacity);
    let overflow = Arc::new(AtomicBool::new(false));
    (
        EventSender {
            sender,
            overflow: overflow.clone(),
        },
        Events { receiver, overflow },
    )
}

impl<T> EventSender<T> {
    pub(crate) fn mark_overflow(&self) {
        self.overflow.store(true, Ordering::Release);
        let _ = self.sender.try_send(QueueEntry::Overflow);
    }
    pub(crate) fn send(&self, event: T) {
        if let Err(TrySendError::Full(_)) = self.sender.try_send(QueueEntry::Event(event)) {
            self.mark_overflow();
        }
    }
}

impl<T> Events<T> {
    fn take_overflow(&self) -> bool {
        if !self.overflow.swap(false, Ordering::AcqRel) {
            return false;
        }
        while self.receiver.try_recv().is_ok() {}
        true
    }

    pub(crate) fn try_recv(&self) -> Result<Option<T>, Error> {
        if self.take_overflow() {
            return Err(Error::Overflow);
        }
        match self.receiver.try_recv() {
            Ok(QueueEntry::Event(event)) => Ok(Some(event)),
            Ok(QueueEntry::Overflow) => Err(Error::Overflow),
            Err(mpsc::TryRecvError::Empty) => Ok(None),
            Err(mpsc::TryRecvError::Disconnected) => Err(Error::Stopped),
        }
    }
}

fn receive_error(error: RecvTimeoutError) -> Error {
    match error {
        RecvTimeoutError::Timeout => Error::Timeout,
        RecvTimeoutError::Disconnected => Error::Stopped,
    }
}

pub(crate) struct Bridge {
    events: EventSender<Event>,
}

impl Bridge {
    pub(crate) fn new() -> (Bridge, Events) {
        Self::with_capacity(EVENT_CAPACITY)
    }

    pub(crate) fn with_capacity(capacity: usize) -> (Bridge, Events) {
        let (events, receiver) = event_queue(capacity);
        (Bridge { events }, receiver)
    }
}

impl ExternalInterfaceProvider for Bridge {
    fn call_method(
        &self,
        _context: &mut UpdateContext<'_>,
        name: &str,
        args: &[ExternalValue],
    ) -> ExternalValue {
        let event = Event {
            name: name.to_owned(),
            args: args.iter().cloned().map(from_external).collect(),
        };
        self.events.send(event);
        ExternalValue::Undefined
    }

    fn on_callback_available(&self, name: &str) {
        tracing::debug!(callback = name, "loader callback registered");
    }

    fn get_id(&self) -> Option<String> {
        None
    }
}

fn from_external(value: ExternalValue) -> Value {
    match value {
        ExternalValue::Undefined => Value::Undefined,
        ExternalValue::Null => Value::Null,
        ExternalValue::Bool(value) => Value::Bool(value),
        ExternalValue::Number(value) => Value::Number(value),
        ExternalValue::String(value) => Value::String(value),
        ExternalValue::Object(value) => Value::Object(
            value
                .into_iter()
                .map(|(key, value)| (key, from_external(value)))
                .collect(),
        ),
        ExternalValue::List(value) => Value::List(value.into_iter().map(from_external).collect()),
    }
}

fn into_external(value: Value) -> ExternalValue {
    match value {
        Value::Undefined => ExternalValue::Undefined,
        Value::Null => ExternalValue::Null,
        Value::Bool(value) => ExternalValue::Bool(value),
        Value::Number(value) => ExternalValue::Number(value),
        Value::String(value) => ExternalValue::String(value),
        Value::Object(value) => ExternalValue::Object(
            value
                .into_iter()
                .map(|(key, value)| (key, into_external(value)))
                .collect(),
        ),
        Value::List(value) => ExternalValue::List(value.into_iter().map(into_external).collect()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Weak;
    use std::task::Wake;

    struct Observe {
        events: Weak<Mutex<ScriptEvents>>,
        seen: mpsc::Sender<Result<Option<ScriptEvent>, Error>>,
    }

    impl Wake for Observe {
        fn wake(self: Arc<Self>) {
            let events = self.events.upgrade().unwrap();
            let events = events.lock().unwrap();
            let event = events.try_recv();
            events.set_waker(Waker::from(self.clone()));
            self.seen.send(event).unwrap();
        }
    }

    #[test]
    fn the_latest_waker_observes_events_and_disconnection_and_can_register_again() {
        let (producer, events) = script_events(1);
        let events = Arc::new(Mutex::new(events));
        let (old_sender, old) = mpsc::channel();
        let (sender, current) = mpsc::channel();
        for seen in [old_sender, sender] {
            events
                .lock()
                .unwrap()
                .set_waker(Waker::from(Arc::new(Observe {
                    events: Arc::downgrade(&events),
                    seen,
                })));
        }
        producer.send(ScriptEvent::Invalidated);
        assert_eq!(
            current.try_recv().unwrap(),
            Ok(Some(ScriptEvent::Invalidated))
        );
        assert_eq!(old.try_recv(), Err(mpsc::TryRecvError::Disconnected));
        drop(producer);
        assert_eq!(current.try_recv().unwrap(), Err(Error::Stopped));
    }
}
