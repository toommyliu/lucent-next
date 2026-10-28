use std::task::Waker;

use crate::bridge::{self, PendingCall, PendingRead, ScriptEvent, ScriptEvents, Value};
use crate::player::Inbox;
use crate::projection::Projection;

pub struct ScriptLink {
    id: u64,
    inbox: Inbox,
    events: ScriptEvents,
}

impl ScriptLink {
    pub(crate) fn new(id: u64, inbox: Inbox, events: ScriptEvents) -> Self {
        Self { id, inbox, events }
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

    pub fn read_projection<R: serde::Serialize + Send + 'static>(
        &self,
        read: impl FnOnce(&Projection) -> R + Send + 'static,
    ) -> PendingRead<R> {
        self.inbox.read_projection(read)
    }
}
