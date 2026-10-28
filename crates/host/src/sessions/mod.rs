use std::sync::Arc;
use std::time::Duration;

use crate::bridge::{Call, Error as BridgeError, Event, PendingCall, PendingRead, Value};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::player::frames::FrameLease;
use crate::player::{self, Clipboard, SessionEvent, Surface, ViewEvent};

#[derive(Clone)]
pub struct Host {
    shared: Arc<Shared>,
}

struct Shared {
    net: tokio::runtime::Runtime,
    clipboard: Clipboard,
    next_session: AtomicU64,
}

#[derive(Debug, thiserror::Error)]
pub enum HostError {
    #[error("couldn't start the net runtime: {0}")]
    NetRuntime(std::io::Error),
    #[error("couldn't start a player thread: {0}")]
    PlayerThread(std::io::Error),
    #[error("couldn't start the clipboard thread: {0}")]
    ClipboardThread(std::io::Error),
}

impl Host {
    pub fn new() -> Result<Host, HostError> {
        let net = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .thread_name("lucent-net")
            .enable_all()
            .build()
            .map_err(HostError::NetRuntime)?;
        Ok(Host {
            shared: Arc::new(Shared {
                net,
                clipboard: Clipboard::system().map_err(HostError::ClipboardThread)?,
                next_session: AtomicU64::new(1),
            }),
        })
    }

    pub fn start_session(&self, surface: Surface) -> Result<Session, HostError> {
        let id = self.shared.next_session.fetch_add(1, Ordering::Relaxed);
        Ok(Session {
            player: Some(self.spawn_player(id, &surface, false)?),
            id,
            surface,
            muted: false,
            host: self.clone(),
        })
    }

    fn spawn_player(
        &self,
        session: u64,
        surface: &Surface,
        muted: bool,
    ) -> Result<player::Handle, HostError> {
        player::spawn(
            format!("player-{session}"),
            surface.share(),
            self.shared.net.handle().clone(),
            self.shared.clipboard.clone(),
            muted,
        )
        .map_err(HostError::PlayerThread)
    }
}

pub struct Session {
    player: Option<player::Handle>,
    id: u64,
    surface: Surface,
    muted: bool,
    host: Host,
}

impl Session {
    /// Runs after preceding events and returns owned, serializable data.
    ///
    /// ```
    /// use lucent_host::Session;
    /// use lucent_protocol::item::ItemContainer;
    /// fn snapshot(session: &Session) {
    ///     let _ = session.read_projection(|state| {
    ///         state.items.get(ItemContainer::Inventory, 1).map(|item| item.data().clone())
    ///     });
    /// }
    /// ```
    ///
    /// Live handles cannot cross this boundary.
    ///
    /// ```compile_fail
    /// use lucent_host::Session;
    /// use lucent_protocol::item::ItemContainer;
    /// fn live_handle(session: &Session) {
    ///     let _ = session.read_projection(|state| state.items.get(ItemContainer::Inventory, 1));
    /// }
    /// ```
    pub fn read_projection<R: serde::Serialize + Send + 'static>(
        &self,
        read: impl FnOnce(&crate::projection::Projection) -> R + Send + 'static,
    ) -> PendingRead<R> {
        match &self.player {
            Some(player) => player.read_projection(read),
            None => PendingRead::stopped(),
        }
    }

    pub fn try_event(&self) -> Result<Option<SessionEvent>, BridgeError> {
        self.player
            .as_ref()
            .ok_or(BridgeError::Stopped)?
            .try_event()
    }

    pub fn recv_event(&self, timeout: Duration) -> Result<SessionEvent, BridgeError> {
        self.player
            .as_ref()
            .ok_or(BridgeError::Stopped)?
            .recv_event(timeout)
    }

    /// Raw loader transport for actions and targeted hydration. Ordinary reads use host state.
    pub fn call_loader(&self, name: impl Into<String>, args: Vec<Value>) -> PendingCall {
        match &self.player {
            Some(player) => player.call_loader(name, args),
            None => {
                let (call, pending) = Call::new(name.into(), args);
                call.reject(BridgeError::Stopped);
                pending
            }
        }
    }

    pub fn try_loader_event(&self) -> Result<Option<Event>, BridgeError> {
        self.player
            .as_ref()
            .ok_or(BridgeError::Stopped)?
            .try_loader_event()
    }

    pub fn recv_loader_event(&self, timeout: Duration) -> Result<Event, BridgeError> {
        self.player
            .as_ref()
            .ok_or(BridgeError::Stopped)?
            .recv_loader_event(timeout)
    }

    pub fn send(&self, event: ViewEvent) {
        if let Some(player) = &self.player {
            player.send(event);
        }
    }

    pub fn frame(&self) -> Option<FrameLease> {
        self.surface.lease()
    }

    pub fn restart(&mut self, surface: Surface) -> Result<(), HostError> {
        self.player = None;
        self.surface = surface;
        self.player = Some(self.host.spawn_player(self.id, &self.surface, self.muted)?);
        Ok(())
    }

    pub fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
        if let Some(player) = &self.player {
            player.set_muted(muted);
        }
    }
}
