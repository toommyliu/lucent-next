mod engine;
mod host;

use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::Duration;

use lucent_host::ScriptLink;
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
    #[error("{0}")]
    JavaScript(String),
    #[error("couldn't decode the script result: {0}")]
    Decode(#[from] serde_json::Error),
}

pub struct Runtime {
    commands: Option<mpsc::Sender<Command>>,
    thread: Option<thread::JoinHandle<()>>,
}

enum Command {
    Eval {
        name: String,
        source: String,
        reply: mpsc::Sender<Result<String, RuntimeError>>,
    },
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
        let thread = thread::Builder::new()
            .name(format!("lucent-js-{}", link.id()))
            .spawn(move || {
                let engine = Engine::new(link).and_then(|mut engine| {
                    engine.eval("runtime.js", include_str!("../../../assets/runtime.js"))?;
                    Ok(engine)
                });
                let mut engine = match engine {
                    Ok(engine) => engine,
                    Err(error) => {
                        let _ = ready.send(Err(error));
                        return;
                    }
                };
                let _ = ready.send(Ok(()));
                for command in incoming {
                    match command {
                        Command::Eval {
                            name,
                            source,
                            reply,
                        } => {
                            let _ = reply.send(engine.eval(&name, &source));
                        }
                    }
                }
            })?;
        let runtime = Self {
            commands: Some(commands),
            thread: Some(thread),
        };
        boot.recv().map_err(|_| RuntimeError::Stopped)??;
        Ok(runtime)
    }

    pub fn eval(&self, name: impl Into<String>, source: impl Into<String>) -> PendingEval {
        let (reply, pending) = mpsc::channel();
        if let Some(commands) = &self.commands {
            let _ = commands.send(Command::Eval {
                name: name.into(),
                source: source.into(),
                reply,
            });
        }
        PendingEval(pending)
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        self.commands.take();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
