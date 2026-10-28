use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::player::frames::FrameLease;
use crate::player::net::Observer;
use crate::player::{self, Clipboard, Surface, ViewEvent};

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
        let observer = Observer::default();
        Ok(Session {
            player: Some(self.spawn_player(id, &surface, false, observer.clone())?),
            observer,
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
        observer: Observer,
    ) -> Result<player::Handle, HostError> {
        player::spawn(
            format!("player-{session}"),
            surface.share(),
            self.shared.net.handle().clone(),
            self.shared.clipboard.clone(),
            muted,
            observer,
        )
        .map_err(HostError::PlayerThread)
    }
}

pub struct Session {
    player: Option<player::Handle>,
    observer: Observer,
    id: u64,
    surface: Surface,
    muted: bool,
    host: Host,
}

impl Session {
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
        self.player = Some(self.host.spawn_player(
            self.id,
            &self.surface,
            self.muted,
            self.observer.clone(),
        )?);
        Ok(())
    }

    pub fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
        if let Some(player) = &self.player {
            player.set_muted(muted);
        }
    }
}
