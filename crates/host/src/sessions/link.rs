use crate::bridge::{PendingCall, PendingRead, Value};
use crate::player::Inbox;
use crate::projection::Projection;

pub struct ScriptLink {
    id: u64,
    inbox: Inbox,
}

impl ScriptLink {
    pub(crate) fn new(id: u64, inbox: Inbox) -> Self {
        Self { id, inbox }
    }

    pub fn id(&self) -> u64 {
        self.id
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
