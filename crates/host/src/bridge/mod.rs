use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::Duration;

pub use lucent_protocol::bridge::{Event, Value};
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
}

pub struct PendingCall(mpsc::Receiver<Result<Value, Error>>);

impl PendingCall {
    /// A timeout leaves the call queued. Wait on this reply again instead of retrying the action.
    pub fn recv_timeout(&self, timeout: Duration) -> Result<Value, Error> {
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
        let (reply, result) = mpsc::channel();
        (Call { name, args, reply }, PendingCall(result))
    }

    pub(crate) fn reject(self, error: Error) {
        let _ = self.reply.send(Err(error));
    }

    pub(crate) fn dispatch(self, player: &mut Player) {
        let result = player.mutate_with_update_context(|context| {
            match context.external_interface.get_callback(&self.name) {
                Some(callback) => Ok(from_external(callback.call(
                    context,
                    &self.name,
                    self.args.into_iter().map(into_external),
                ))),
                None => Err(Error::Unavailable(self.name)),
            }
        });
        let _ = self.reply.send(result);
    }
}

pub(crate) struct Events(mpsc::Receiver<Event>);

impl Events {
    pub(crate) fn try_recv(&self) -> Result<Option<Event>, Error> {
        match self.0.try_recv() {
            Ok(event) => Ok(Some(event)),
            Err(mpsc::TryRecvError::Empty) => Ok(None),
            Err(mpsc::TryRecvError::Disconnected) => Err(Error::Stopped),
        }
    }

    pub(crate) fn recv_timeout(&self, timeout: Duration) -> Result<Event, Error> {
        self.0.recv_timeout(timeout).map_err(receive_error)
    }
}

fn receive_error(error: RecvTimeoutError) -> Error {
    match error {
        RecvTimeoutError::Timeout => Error::Timeout,
        RecvTimeoutError::Disconnected => Error::Stopped,
    }
}

pub(crate) struct Bridge {
    events: mpsc::Sender<Event>,
}

impl Bridge {
    pub(crate) fn new() -> (Bridge, Events) {
        let (events, receiver) = mpsc::channel();
        (Bridge { events }, Events(receiver))
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
        if self.events.send(event).is_err() {
            tracing::debug!(event = name, "loader event receiver closed");
        }
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
