mod link;
pub(crate) mod operations;
pub use link::ScriptLink;

use lucent_protocol::settings::GameSettings;
use parking_lot::RwLock;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct AttachmentId(pub(crate) u64);

use std::sync::Arc;

use crate::bridge::{EVENT_CAPACITY, PendingRead, script_events};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::player::frames::FrameLease;
use crate::player::{self, Audio, Clipboard, Movie, Surface, ViewEvent};

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
    #[error("couldn't start an operation worker: {0}")]
    OperationThread(std::io::Error),
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
        let settings = Arc::new(RwLock::new(GameSettings::default()));
        let player = self.spawn_player(id, &surface, false, &movie, settings.clone())?;
        let operations = operations::Operations::start(id).map_err(HostError::OperationThread)?;
        Ok(Session {
            next_attachment: 0,
            operations,
            player: Some(player),
            movie,
            id,
            surface,
            muted: false,
            settings,
            host: self.clone(),
        })
    }

    fn spawn_player(
        &self,
        session: u64,
        surface: &Surface,
        muted: bool,
        movie: &Movie,
        settings: Arc<RwLock<GameSettings>>,
    ) -> Result<player::Handle, HostError> {
        player::spawn(
            format!("player-{session}"),
            surface.share(),
            self.shared.net.handle().clone(),
            player::Options {
                clipboard: self.shared.clipboard.clone(),
                audio: self.shared.audio,
                movie: movie.clone(),
                muted,
                settings,
            },
        )
        .map_err(HostError::PlayerThread)
    }
}

pub struct Session {
    player: Option<player::Handle>,
    next_attachment: u64,
    operations: operations::Operations,
    movie: Movie,
    id: u64,
    surface: Surface,
    muted: bool,
    host: Host,
    settings: Arc<RwLock<GameSettings>>,
}

impl Session {
    #[cfg(feature = "testing")]
    pub fn attached_link_count(&self) -> crate::bridge::Pending<usize> {
        let (reply, pending) = crate::bridge::Pending::channel();
        if let Some(player) = &self.player {
            player.inbox.post(player::Msg::LinkCount(reply));
        }
        pending
    }

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

    pub fn attach(&mut self) -> Option<ScriptLink> {
        let player = self.player.as_ref()?;
        let attachment = AttachmentId(self.next_attachment);
        self.next_attachment = self
            .next_attachment
            .checked_add(1)
            .expect("script attachment IDs exhausted");
        let inbox = player.inbox.clone();
        let (producer, events) = script_events(EVENT_CAPACITY);
        inbox.post(player::Msg::Attach(attachment, producer));
        Some(ScriptLink::new(
            self.id,
            attachment,
            events,
            self.settings.clone(),
            operations::Endpoint {
                inbox,
                projection: player.projection.clone(),
                stopped: player.stopped.clone(),
                packets: player.operation_packets.clone(),
            },
            self.operations.sender.clone(),
        ))
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
        let player = self.host.spawn_player(
            self.id,
            &self.surface,
            self.muted,
            &self.movie,
            self.settings.clone(),
        )?;
        self.player = Some(player);
        Ok(())
    }

    pub fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
        if let Some(player) = &self.player {
            player.set_muted(muted);
        }
    }
}
