mod alarm;
mod engine;
mod host;
mod interrupt;

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
use interrupt::{Interrupt, Stop};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Kill {
    #[error("a turn ran for more than 200 ms")]
    Watchdog,
    #[error("it reached the 64 MiB heap limit")]
    HeapLimit,
    #[error("it was stopped")]
    Requested,
}

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
    #[error("the script was killed: {0}")]
    Killed(Kill),
    #[error("{0}")]
    JavaScript(String),
    #[error("couldn't decode the script result: {0}")]
    Decode(#[from] serde_json::Error),
}

pub struct Runtime {
    commands: mpsc::Sender<Command>,
    interrupt: Arc<Interrupt>,
    thread: Option<thread::JoinHandle<()>>,
}

enum Command {
    Wake,
    Stop,
    Kill,
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
        let interrupt = Arc::new(Interrupt::new()?);
        let thread_interrupt = interrupt.clone();
        let (commands, incoming) = mpsc::channel();
        let (ready, boot) = mpsc::sync_channel(1);
        let wake = Arc::new(EventWake {
            commands: commands.clone(),
            queued: AtomicBool::new(false),
        });
        let thread = thread::Builder::new()
            .name(format!("lucent-js-{}", link.id()))
            .spawn(move || {
                let interrupt = thread_interrupt;
                let watchdog_interrupt = interrupt.clone();
                let watchdog = thread::Builder::new()
                    .name(format!("lucent-js-watchdog-{}", link.id()))
                    .spawn(move || watchdog_interrupt.watch());
                let watchdog = match watchdog {
                    Ok(watchdog) => watchdog,
                    Err(error) => {
                        interrupt.shut_down();
                        let _ = ready.send(Err(error.into()));
                        return;
                    }
                };
                let result = (|| {
                    link.set_waker(Waker::from(wake.clone()));
                    let engine = Engine::new(link, interrupt.clone())?;
                    run(engine, &incoming, &wake, &interrupt, &ready)
                })();
                if let Err(error) = result {
                    let _ = ready.send(Err(error));
                }
                interrupt.shut_down();
                let _ = watchdog.join();
            })?;
        let runtime = Self {
            commands,
            interrupt,
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

    pub fn kill(&self) {
        self.interrupt.kill(Kill::Requested);
        let _ = self.commands.send(Command::Kill);
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        self.interrupt.shut_down();
        let _ = self.commands.send(Command::Stop);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn run(
    mut engine: Engine,
    incoming: &mpsc::Receiver<Command>,
    wake: &EventWake,
    interrupt: &Arc<Interrupt>,
    ready: &mpsc::SyncSender<Result<(), RuntimeError>>,
) -> Result<(), RuntimeError> {
    let mut command = Ok(Command::Wake);
    let mut ready = Some(ready);
    loop {
        interrupt.begin_turn();
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
                    return Ok(());
                };
                items
            }
            Ok(Command::Kill) | Err(RecvTimeoutError::Timeout) => Vec::new(),
            Ok(Command::Stop) | Err(RecvTimeoutError::Disconnected) => return Ok(()),
        };
        let mut deadline = engine.turn(&serde_json::to_string(&items).unwrap());
        match interrupt.end_turn() {
            None => {}
            Some(Stop::Kill(kill)) => {
                tracing::warn!(%kill, "script runtime killed");
                engine = Engine::new(engine.kill(kill), interrupt.clone()).map_err(|error| {
                    tracing::error!(%error, "couldn't rebuild the script runtime");
                    error
                })?;
                deadline = None;
            }
            Some(Stop::Shutdown) => return Ok(()),
        }
        if let Some(ready) = ready.take() {
            let _ = ready.send(Ok(()));
        }
        command = match deadline {
            Some(deadline) => {
                incoming.recv_timeout(deadline.saturating_duration_since(Instant::now()))
            }
            None => incoming.recv().map_err(|_| RecvTimeoutError::Disconnected),
        };
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
