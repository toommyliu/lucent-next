mod engine;
mod host;

use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::task::{Wake, Waker};
use std::thread;
use std::time::{Duration, Instant};

use lucent_host::ScriptLink;
use lucent_host::bridge::Error as EventError;
use serde::de::DeserializeOwned;

use engine::Engine;

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("couldn't start the script thread: {0}")]
    Thread(#[from] std::io::Error),
    #[error("the script runtime has stopped")]
    Stopped,
    #[error("timed out waiting for the script runtime")]
    Timeout,
    #[error("script events overflowed and were lost")]
    EventsLost,
    #[error("{0}")]
    JavaScript(String),
    #[error("couldn't decode the script result: {0}")]
    Decode(#[from] serde_json::Error),
}

pub struct Runtime {
    commands: mpsc::Sender<Command>,
    thread: Option<thread::JoinHandle<()>>,
}

enum Command {
    Wake,
    Stop,
    Eval {
        name: String,
        source: String,
        reply: mpsc::Sender<Result<String, RuntimeError>>,
    },
}

struct EventWake {
    commands: mpsc::Sender<Command>,
    queued: AtomicBool,
}

impl Wake for EventWake {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        if !self.queued.swap(true, Ordering::AcqRel) {
            let _ = self.commands.send(Command::Wake);
        }
    }
}

pub struct PendingEval(mpsc::Receiver<Result<String, RuntimeError>>);

impl PendingEval {
    pub fn wait<T: DeserializeOwned>(self, timeout: Duration) -> Result<T, RuntimeError> {
        let json = self.0.recv_timeout(timeout).map_err(|error| match error {
            RecvTimeoutError::Timeout => RuntimeError::Timeout,
            RecvTimeoutError::Disconnected => RuntimeError::Stopped,
        })??;
        Ok(serde_json::from_str(&json)?)
    }
}

impl Runtime {
    pub fn start(link: ScriptLink) -> Result<Self, RuntimeError> {
        let (commands, incoming) = mpsc::channel();
        let (ready, boot) = mpsc::sync_channel(1);
        let wake = Arc::new(EventWake {
            commands: commands.clone(),
            queued: AtomicBool::new(false),
        });
        let thread = thread::Builder::new()
            .name(format!("lucent-js-{}", link.id()))
            .spawn(move || {
                link.set_waker(Waker::from(wake.clone()));
                match Engine::new(link) {
                    Ok(mut engine) => {
                        let Some(items) = take_events(&mut engine) else {
                            return;
                        };
                        engine.turn(&serde_json::to_string(&items).unwrap());
                        let _ = ready.send(Ok(()));
                        run(engine, &incoming, &wake);
                    }
                    Err(error) => {
                        let _ = ready.send(Err(error));
                    }
                }
            })?;
        let runtime = Self {
            commands,
            thread: Some(thread),
        };
        boot.recv().map_err(|_| RuntimeError::Stopped)??;
        Ok(runtime)
    }

    pub fn eval(&self, name: impl Into<String>, source: impl Into<String>) -> PendingEval {
        let (reply, pending) = mpsc::channel();
        let _ = self.commands.send(Command::Eval {
            name: name.into(),
            source: source.into(),
            reply,
        });
        PendingEval(pending)
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        let _ = self.commands.send(Command::Stop);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn run(mut engine: Engine, incoming: &mpsc::Receiver<Command>, wake: &EventWake) {
    loop {
        let command = match engine.wake() {
            Some(deadline) => {
                incoming.recv_timeout(deadline.saturating_duration_since(Instant::now()))
            }
            None => incoming.recv().map_err(|_| RecvTimeoutError::Disconnected),
        };
        let items = match command {
            Ok(Command::Eval {
                name,
                source,
                reply,
            }) => {
                engine.eval(&name, &source, reply);
                Vec::new()
            }
            Ok(Command::Wake) => {
                wake.queued.store(false, Ordering::Release);
                let Some(items) = take_events(&mut engine) else {
                    return;
                };
                items
            }
            Err(RecvTimeoutError::Timeout) => Vec::new(),
            Ok(Command::Stop) | Err(RecvTimeoutError::Disconnected) => return,
        };
        engine.turn(&serde_json::to_string(&items).unwrap());
    }
}

fn take_events(engine: &mut Engine) -> Option<Vec<host::Item>> {
    let mut items = Vec::new();
    loop {
        match engine.try_event() {
            Ok(Some(event)) => items.push(host::Item::from(event)),
            Ok(None) => return Some(items),
            Err(EventError::Overflow) => {
                tracing::error!("script events overflowed and were lost");
                engine.events_lost();
            }
            Err(_) => return None,
        }
    }
}
