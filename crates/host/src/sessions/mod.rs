use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

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
        let player = player::spawn(
            format!("player-{id}"),
            surface.share(),
            self.shared.net.handle().clone(),
        )
        .map_err(HostError::PlayerThread)?;
        Ok(Session {
            player,
            _surface: surface,
            _host: self.clone(),
        })
    }
}

pub struct Session {
    player: player::Handle,
    _surface: Surface,
    _host: Host,
}

impl Session {
    pub fn send(&self, event: ViewEvent) {
        self.player.send(event);
    }
}
