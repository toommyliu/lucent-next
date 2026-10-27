use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::player::frames::FrameLease;
use crate::player::{self, Surface, ViewEvent};

#[derive(Clone)]
pub struct Host {
    shared: Arc<Shared>,
}

struct Shared {
    net: tokio::runtime::Runtime,
    next_session: AtomicU64,
}

#[derive(Debug, thiserror::Error)]
pub enum HostError {
    #[error("couldn't start the net runtime: {0}")]
    NetRuntime(std::io::Error),
    #[error("couldn't start a player thread: {0}")]
    PlayerThread(std::io::Error),
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
                next_session: AtomicU64::new(1),
            }),
        })
    }

    pub fn start_session(&self, surface: Surface) -> Result<Session, HostError> {
        let id = self.shared.next_session.fetch_add(1, Ordering::Relaxed);
        Ok(Session {
            player: Some(self.spawn_player(id, &surface)?),
            id,
            surface,
            host: self.clone(),
        })
    }

    fn spawn_player(&self, session: u64, surface: &Surface) -> Result<player::Handle, HostError> {
        player::spawn(
            format!("player-{session}"),
            surface.share(),
            self.shared.net.handle().clone(),
        )
        .map_err(HostError::PlayerThread)
    }
}

pub struct Session {
    player: Option<player::Handle>,
    id: u64,
    surface: Surface,
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
        self.player = Some(self.host.spawn_player(self.id, &self.surface)?);
        Ok(())
    }

    pub fn set_muted(&self, muted: bool) {
        if let Some(player) = &self.player {
            player.set_muted(muted);
        }
    }
}
