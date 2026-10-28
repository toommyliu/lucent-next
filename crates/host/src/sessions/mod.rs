mod link;
pub use link::ScriptLink;

use std::sync::Arc;
use std::time::Duration;

use crate::bridge::{Error as BridgeError, Event, PendingRead};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::player::frames::FrameLease;
use crate::player::{self, Audio, Clipboard, Movie, SessionEvent, Surface, ViewEvent};

#[derive(Clone)]
pub struct Host {
    shared: Arc<Shared>,
}

struct Shared {
    net: tokio::runtime::Runtime,
    clipboard: Clipboard,
    audio: Audio,
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
        Self::with_services(
            Clipboard::system().map_err(HostError::ClipboardThread)?,
            player::audio,
        )
    }

    pub(crate) fn with_services(clipboard: Clipboard, audio: Audio) -> Result<Host, HostError> {
        let net = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .thread_name("lucent-net")
            .enable_all()
            .build()
            .map_err(HostError::NetRuntime)?;
        Ok(Host {
            shared: Arc::new(Shared {
                net,
                clipboard,
                audio,
                next_session: AtomicU64::new(1),
            }),
        })
    }

    pub fn start_session(&self, surface: Surface) -> Result<Session, HostError> {
        self.start_movie(surface, Movie::loader())
    }

    pub(crate) fn start_movie(&self, surface: Surface, movie: Movie) -> Result<Session, HostError> {
        let id = self.shared.next_session.fetch_add(1, Ordering::Relaxed);
        let (player, script_link) = self.spawn_player(id, &surface, false, &movie)?;
        Ok(Session {
            player: Some(player),
            script_link: Some(script_link),
            movie,
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
        movie: &Movie,
    ) -> Result<(player::Handle, ScriptLink), HostError> {
        let player = player::spawn(
            format!("player-{session}"),
            surface.share(),
            self.shared.net.handle().clone(),
            player::Options {
                clipboard: self.shared.clipboard.clone(),
                audio: self.shared.audio,
                movie: movie.clone(),
                muted,
            },
        )
        .map_err(HostError::PlayerThread)?;
        let link = ScriptLink::new(session, player.inbox.clone());
        Ok((player, link))
    }
}

pub struct Session {
    player: Option<player::Handle>,
    script_link: Option<ScriptLink>,
    movie: Movie,
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
            Some(player) => player.inbox.read_projection(read),
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

    pub fn take_script_link(&mut self) -> Option<ScriptLink> {
        self.script_link.take()
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
        self.script_link = None;
        self.player = None;
        self.surface = surface;
        let (player, script_link) =
            self.host
                .spawn_player(self.id, &self.surface, self.muted, &self.movie)?;
        self.player = Some(player);
        self.script_link = Some(script_link);
        Ok(())
    }

    pub fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
        if let Some(player) = &self.player {
            player.set_muted(muted);
        }
    }
}
