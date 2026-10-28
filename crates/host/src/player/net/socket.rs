use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::*;

pub(super) const MAX_PACKET_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ConnectionId(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Direction {
    ToServer,
    ToClient,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CloseReason {
    RemoteClosed,
    PlayerClosed,
    DeliveryStopped,
    ReadFailed,
    WriteFailed,
    ConnectFailed,
    TimedOut,
    PacketTooLarge(Direction),
    Cancelled,
}

pub(crate) enum Event<'a> {
    Opened {
        id: ConnectionId,
        host: &'a str,
        port: u16,
    },
    Packet {
        id: ConnectionId,
        direction: Direction,
        sequence: u64,
        payload: &'a [u8],
    },
    Closed {
        id: ConnectionId,
        reason: CloseReason,
        tails: [usize; 2],
    },
}

type Callback = dyn Fn(Event<'_>) + Send + Sync;

#[derive(Clone)]
pub(crate) struct Observer(Arc<Wire>);

struct Wire {
    next: AtomicU64,
    callback: Box<Callback>,
}

impl Default for Observer {
    fn default() -> Self {
        Self::new(|event| match event {
            Event::Opened { id, host, port } => {
                tracing::debug!(connection = id.0, host, port, "socket opened");
            }
            Event::Packet {
                id,
                direction,
                sequence,
                payload,
            } => {
                tracing::trace!(
                    connection = id.0,
                    ?direction,
                    sequence,
                    bytes = payload.len(),
                    "socket packet"
                );
            }
            Event::Closed { id, reason, tails } => {
                tracing::debug!(
                    connection = id.0,
                    ?reason,
                    to_server_tail = tails[0],
                    to_client_tail = tails[1],
                    "socket closed"
                );
            }
        })
    }
}

impl Observer {
    pub(crate) fn new(callback: impl Fn(Event<'_>) + Send + Sync + 'static) -> Self {
        Self(Arc::new(Wire {
            next: AtomicU64::new(1),
            callback: Box::new(callback),
        }))
    }

    pub(super) fn emit(&self, event: Event<'_>) {
        (self.0.callback)(event);
    }
}

#[derive(Default)]
pub(super) struct Framer {
    tail: Vec<u8>,
    tail_len: usize,
    sequence: u64,
}

impl Framer {
    pub(super) fn tail_len(&self) -> usize {
        self.tail_len
    }

    pub(super) fn observe(
        &mut self,
        mut bytes: &[u8],
        mut packet: impl FnMut(u64, &[u8]),
    ) -> Result<(), ()> {
        while !bytes.is_empty() {
            let end = bytes.iter().position(|byte| *byte == 0);
            let part = &bytes[..end.unwrap_or(bytes.len())];
            self.tail_len += part.len();
            if self.tail_len > MAX_PACKET_BYTES {
                return Err(());
            }
            if let Some(end) = end {
                self.sequence += 1;
                if self.tail.is_empty() {
                    packet(self.sequence, part);
                } else {
                    self.tail.extend_from_slice(part);
                    packet(self.sequence, &self.tail);
                    self.tail.clear();
                }
                self.tail_len = 0;
                bytes = &bytes[end + 1..];
            } else {
                self.tail.extend_from_slice(part);
                break;
            }
        }
        Ok(())
    }
}

struct Connection {
    observer: Observer,
    id: ConnectionId,
    to_server: Framer,
    to_client: Framer,
    reason: CloseReason,
}

impl Connection {
    fn open(observer: Observer, host: &str, port: u16) -> Self {
        let id = ConnectionId(observer.0.next.fetch_add(1, Ordering::Relaxed));
        observer.emit(Event::Opened { id, host, port });
        Self {
            observer,
            id,
            to_server: Framer::default(),
            to_client: Framer::default(),
            reason: CloseReason::Cancelled,
        }
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        self.observer.emit(Event::Closed {
            id: self.id,
            reason: self.reason,
            tails: [self.to_server.tail_len(), self.to_client.tail_len()],
        });
    }
}

impl Net {
    pub(crate) fn connect(
        &self,
        host: String,
        port: u16,
        timeout: Duration,
        outgoing: async_channel::Receiver<Vec<u8>>,
        mut deliver: impl FnMut(SocketEvent) -> bool + Send + 'static,
    ) {
        let Some(host) = GameHost::parse(&host) else {
            tracing::warn!(%host, port, "socket host rejected");
            deliver(SocketEvent::Failed);
            return;
        };
        let observer = self.observer.clone();
        #[cfg(test)]
        let address = self
            .routes
            .iter()
            .find(|(name, _)| *name == host.0)
            .map(|(_, addr)| *addr);
        self.track(self.rt.spawn(async move {
            let mut connection = Connection::open(observer, &host.0, port);
            let connecting = async {
                #[cfg(test)]
                if let Some(address) = address {
                    return tokio::net::TcpStream::connect(address).await;
                }
                tokio::net::TcpStream::connect((host.0.as_str(), port)).await
            };
            let stream = match tokio::time::timeout(timeout, connecting).await {
                Ok(Ok(stream)) => stream,
                Ok(Err(error)) => {
                    tracing::warn!(host = %host.0, port, %error, "socket connection failed");
                    connection.reason = CloseReason::ConnectFailed;
                    deliver(SocketEvent::Failed);
                    return;
                }
                Err(_) => {
                    connection.reason = CloseReason::TimedOut;
                    deliver(SocketEvent::TimedOut);
                    return;
                }
            };
            if !deliver(SocketEvent::Connected) {
                connection.reason = CloseReason::DeliveryStopped;
                return;
            }
            let (mut read, mut write) = stream.into_split();
            let Connection {
                observer,
                id,
                to_server,
                to_client,
                ..
            } = &mut connection;
            let reading = async {
                let mut bytes = [0; 4096];
                loop {
                    let count = match read.read(&mut bytes).await {
                        Ok(0) => return CloseReason::RemoteClosed,
                        Ok(count) => count,
                        Err(_) => return CloseReason::ReadFailed,
                    };
                    if to_client
                        .observe(&bytes[..count], |sequence, payload| {
                            observer.emit(Event::Packet {
                                id: *id,
                                direction: Direction::ToClient,
                                sequence,
                                payload,
                            })
                        })
                        .is_err()
                    {
                        return CloseReason::PacketTooLarge(Direction::ToClient);
                    }
                    if !deliver(SocketEvent::Data(bytes[..count].to_vec())) {
                        return CloseReason::DeliveryStopped;
                    }
                }
            };
            let writing = async {
                while let Ok(bytes) = outgoing.recv().await {
                    if to_server
                        .observe(&bytes, |sequence, payload| {
                            observer.emit(Event::Packet {
                                id: *id,
                                direction: Direction::ToServer,
                                sequence,
                                payload,
                            })
                        })
                        .is_err()
                    {
                        return CloseReason::PacketTooLarge(Direction::ToServer);
                    }
                    if write.write_all(&bytes).await.is_err() {
                        return CloseReason::WriteFailed;
                    }
                }
                CloseReason::PlayerClosed
            };
            connection.reason =
                tokio::select! { reason = reading => reason, reason = writing => reason };
            drop(connection);
            deliver(SocketEvent::Closed);
        }));
    }
}
