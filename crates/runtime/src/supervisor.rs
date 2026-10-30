use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    task::{Wake, Waker},
    thread,
    time::Duration,
};

use lucent_host::{HostError, ScriptLink, Session, Surface, bridge};
use lucent_protocol::script::{Script, ScriptOutcome};
use tracing::{Level, error, info, warn};

use crate::{Bot, Kill, RuntimeError, ScriptRun};

pub struct ScriptRequest {
    script: Script,
    inbox: mpsc::Sender<Msg>,
    messages: mpsc::Receiver<Msg>,
}

#[derive(Clone)]
pub struct ScriptControls(mpsc::Sender<Msg>);

impl ScriptRequest {
    pub fn new(script: Script) -> (Self, ScriptControls) {
        let (inbox, messages) = mpsc::channel();
        let controls = ScriptControls(inbox.clone());
        (
            Self {
                script,
                inbox,
                messages,
            },
            controls,
        )
    }
}

impl ScriptControls {
    pub fn stop(&self) {
        let _ = self.0.send(Msg::Stop);
    }
    pub fn kill(&self) {
        let _ = self.0.send(Msg::Kill);
    }
    pub fn status(&self) {
        let _ = self.0.send(Msg::Status);
    }
}

pub struct Supervisor {
    inbox: mpsc::Sender<Msg>,
    has_script: bool,
    thread: Option<thread::JoinHandle<()>>,
}

enum Msg {
    Player(Box<Player>),
    Wake,
    Stop,
    Kill,
    Status,
    Shutdown,
}
struct Player {
    bot: ScriptLink,
    watch: Option<ScriptLink>,
}

impl Supervisor {
    pub fn start(
        session: &mut Session,
        script: Option<ScriptRequest>,
    ) -> Result<Self, RuntimeError> {
        let (inbox, messages, script) = match script {
            Some(ScriptRequest {
                script,
                inbox,
                messages,
            }) => (inbox, messages, Some(script)),
            None => {
                let (inbox, messages) = mpsc::channel();
                (inbox, messages, None)
            }
        };
        let has_script = script.is_some();
        let first = attach(session, has_script).ok_or(RuntimeError::Stopped)?;
        let supervision = Supervision {
            session: first.bot.id(),
            notify: Arc::new(Notify {
                inbox: inbox.clone(),
                queued: AtomicBool::new(false),
            }),
            script: match script {
                Some(script) => ScriptState::Waiting {
                    script,
                    watch: None,
                },
                None => ScriptState::Idle,
            },
            bot: None,
            bots_started: 0,
        };
        let dispatch = tracing::dispatcher::get_default(Clone::clone);
        let thread = thread::Builder::new()
            .name(format!("lucent-supervisor-{}", first.bot.id()))
            .spawn(move || {
                let _dispatch = tracing::dispatcher::set_default(&dispatch);
                supervision.run(first, messages);
            })?;
        Ok(Self {
            inbox,
            has_script,
            thread: Some(thread),
        })
    }

    pub fn restart_player(
        &mut self,
        session: &mut Session,
        surface: Surface,
    ) -> Result<(), HostError> {
        session.restart(surface)?;
        if let Some(player) = attach(session, self.has_script) {
            let _ = self.inbox.send(Msg::Player(Box::new(player)));
        }
        Ok(())
    }
}

impl Drop for Supervisor {
    fn drop(&mut self) {
        let _ = self.inbox.send(Msg::Shutdown);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn attach(session: &mut Session, has_script: bool) -> Option<Player> {
    let bot = session.attach()?;
    let watch = if has_script {
        Some(session.attach()?)
    } else {
        None
    };
    Some(Player { bot, watch })
}

struct Notify {
    inbox: mpsc::Sender<Msg>,
    queued: AtomicBool,
}
impl Wake for Notify {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        if !self.queued.swap(true, Ordering::AcqRel) {
            let _ = self.inbox.send(Msg::Wake);
        }
    }
}

struct Supervision {
    session: u64,
    notify: Arc<Notify>,
    script: ScriptState,
    bot: Option<CurrentBot>,
    bots_started: u32,
}
struct CurrentBot {
    bot: Bot,
    number: u32,
}
enum ScriptState {
    Idle,
    Waiting {
        script: Script,
        watch: Option<ScriptLink>,
    },
    Running {
        name: String,
        run: ScriptRun,
    },
}

impl Supervision {
    fn run(mut self, first: Player, messages: mpsc::Receiver<Msg>) {
        self.player(first);
        self.settle();
        while let Ok(message) = messages.recv() {
            match message {
                Msg::Player(player) => self.player(*player),
                Msg::Wake => self.notify.queued.store(false, Ordering::Release),
                Msg::Stop => self.control(false),
                Msg::Kill => self.control(true),
                Msg::Status => {
                    self.settle();
                    self.status();
                }
                Msg::Shutdown => return,
            }
            self.settle();
        }
    }

    fn player(&mut self, player: Player) {
        let mut ended_run = false;
        if let ScriptState::Running { .. } = &self.script {
            let ScriptState::Running { name, run } =
                std::mem::replace(&mut self.script, ScriptState::Idle)
            else {
                unreachable!()
            };
            let result = match run.wait(Duration::ZERO) {
                Err(RuntimeError::Timeout) => Err(RuntimeError::Stopped),
                result => result,
            };
            drop(run);
            self.ended(&name, result);
            ended_run = true;
        }
        if let Some(CurrentBot { bot, number }) = self.bot.take() {
            let result = bot.wait(Duration::ZERO);
            self.bot_ended(number, result);
            drop(bot);
        }
        self.bots_started += 1;
        let number = self.bots_started;
        match Bot::spawn(player.bot, Waker::from(self.notify.clone())) {
            Ok(bot) => {
                info!("session {}: bot {number} started", self.session);
                self.bot = Some(CurrentBot { bot, number });
            }
            Err(error) => error!("session {}: bot couldn't start: {error}", self.session),
        }
        if let ScriptState::Waiting { script, watch } = &mut self.script {
            if let Some(link) = &player.watch {
                link.set_waker(Waker::from(self.notify.clone()));
            }
            *watch = player.watch;
            info!(
                "session {}: {} is waiting for login",
                self.session,
                script.entry()
            );
        }
        if ended_run {
            self.bot_line();
        }
    }

    fn settle_bot(&mut self) {
        if let Some(CurrentBot { bot, number }) = &self.bot {
            let result = bot.wait(Duration::ZERO);
            if !matches!(result, Err(RuntimeError::Timeout)) {
                self.bot_ended(*number, result);
                self.bot = None;
            }
        }
    }

    fn settle(&mut self) {
        self.settle_bot();
        if let ScriptState::Running { name, run } = &self.script {
            let result = run.wait(Duration::ZERO);
            if !matches!(result, Err(RuntimeError::Timeout)) {
                self.ended(name, result);
                self.script = ScriptState::Idle;
                self.bot_line();
            }
        }
        if let ScriptState::Waiting {
            watch: Some(watch), ..
        } = &self.script
        {
            match logged_in(watch) {
                Ok(true) => self.start_run(),
                Ok(false) => {}
                Err(_) => {
                    if let ScriptState::Waiting { watch, .. } = &mut self.script {
                        *watch = None;
                    }
                }
            }
        }
    }

    fn start_run(&mut self) {
        let ScriptState::Waiting {
            script,
            watch: Some(watch),
        } = std::mem::replace(&mut self.script, ScriptState::Idle)
        else {
            unreachable!()
        };
        let name = script.entry().to_string();
        match ScriptRun::spawn(
            watch,
            script,
            Default::default(),
            Waker::from(self.notify.clone()),
        ) {
            Ok(run) => {
                info!("session {}: {name} started", self.session);
                self.script = ScriptState::Running { name, run };
            }
            Err(error) => {
                self.ended(&name, Err(error));
                self.bot_line();
            }
        }
    }

    fn control(&mut self, kill: bool) {
        match &self.script {
            ScriptState::Running { name, run } => {
                if kill {
                    run.kill();
                    info!("session {}: killing {name}", self.session);
                } else {
                    run.stop_script();
                    info!("session {}: stopping {name}", self.session);
                }
            }
            ScriptState::Waiting { script, .. } => {
                info!(
                    "session {}: {} was cancelled before it started",
                    self.session,
                    script.entry()
                );
                self.script = ScriptState::Idle;
                self.bot_line();
            }
            ScriptState::Idle => info!("session {}: no script is running", self.session),
        }
    }

    fn status(&mut self) {
        self.bot_line();
        match &self.script {
            ScriptState::Idle => info!("session {}: no script is running", self.session),
            ScriptState::Waiting { script, .. } => info!(
                "session {}: {} is waiting for login",
                self.session,
                script.entry()
            ),
            ScriptState::Running { name, .. } => {
                info!("session {}: {name} is running", self.session)
            }
        }
    }

    fn bot_line(&mut self) {
        self.settle_bot();
        match &self.bot {
            Some(CurrentBot { bot, number }) => info!(
                turns = bot.turns(),
                "session {}: bot {number} is running", self.session
            ),
            None => info!("session {}: no bot is running", self.session),
        }
    }

    fn bot_ended(&self, number: u32, result: Result<(), RuntimeError>) {
        match result {
            Ok(()) | Err(RuntimeError::Stopped | RuntimeError::Timeout) => info!(
                "session {}: bot {number} stopped with its player",
                self.session
            ),
            Err(error) => error!("session {}: bot {number} ended: {error}", self.session),
        }
    }

    fn ended(&self, name: &str, result: Result<ScriptOutcome, RuntimeError>) {
        let (level, line) = ended(name, result);
        if level == Level::WARN {
            warn!("session {}: {line}", self.session);
        } else {
            info!("session {}: {line}", self.session);
        }
    }
}

fn logged_in(link: &ScriptLink) -> Result<bool, bridge::Error> {
    loop {
        match link.try_event() {
            Ok(Some(_)) | Err(bridge::Error::Overflow) => {}
            Ok(None) => break,
            Err(error) => return Err(error),
        }
    }
    link.read(|projection| projection.current_readiness().missing.is_empty())
}

fn ended(name: &str, result: Result<ScriptOutcome, RuntimeError>) -> (Level, String) {
    let (level, end) = match result {
        Ok(ScriptOutcome::Completed) => (Level::INFO, "completed".into()),
        Ok(ScriptOutcome::Failed { message, details }) => (
            Level::WARN,
            match details {
                Some(details) => format!("failed: {message}\n{details}"),
                None => format!("failed: {message}"),
            },
        ),
        Ok(ScriptOutcome::StoppedByScript { reason }) => {
            (Level::INFO, with_reason("stopped itself", reason))
        }
        Ok(ScriptOutcome::Exited {
            close_client: false,
        }) => (Level::INFO, "exited".into()),
        Ok(ScriptOutcome::Exited { close_client: true }) => (
            Level::INFO,
            "exited and asked to close the game client".into(),
        ),
        Ok(ScriptOutcome::Stopped { reason }) => (Level::INFO, with_reason("was stopped", reason)),
        Err(RuntimeError::Killed(Kill::Requested)) => (Level::WARN, "was killed".into()),
        Err(RuntimeError::Killed(kill)) => (Level::WARN, format!("was killed: {kill}")),
        Err(RuntimeError::Stopped) => (Level::WARN, "ended because its player stopped".into()),
        Err(error) => (Level::WARN, format!("ended: {error}")),
    };
    (level, format!("{name} {end}"))
}
fn with_reason(end: &str, reason: Option<String>) -> String {
    match reason {
        Some(reason) => format!("{end}: {reason}"),
        None => end.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn outcome_lines_name_the_script_and_the_end() {
        for (result, level, line) in [
            (Ok(ScriptOutcome::Completed), Level::INFO, "x.js completed"),
            (
                Ok(ScriptOutcome::Failed {
                    message: "boom".into(),
                    details: None,
                }),
                Level::WARN,
                "x.js failed: boom",
            ),
            (
                Ok(ScriptOutcome::Failed {
                    message: "boom".into(),
                    details: Some("stack".into()),
                }),
                Level::WARN,
                "x.js failed: boom\nstack",
            ),
            (
                Ok(ScriptOutcome::StoppedByScript { reason: None }),
                Level::INFO,
                "x.js stopped itself",
            ),
            (
                Ok(ScriptOutcome::StoppedByScript {
                    reason: Some("done".into()),
                }),
                Level::INFO,
                "x.js stopped itself: done",
            ),
            (
                Ok(ScriptOutcome::Exited {
                    close_client: false,
                }),
                Level::INFO,
                "x.js exited",
            ),
            (
                Ok(ScriptOutcome::Exited { close_client: true }),
                Level::INFO,
                "x.js exited and asked to close the game client",
            ),
            (
                Ok(ScriptOutcome::Stopped { reason: None }),
                Level::INFO,
                "x.js was stopped",
            ),
            (
                Ok(ScriptOutcome::Stopped {
                    reason: Some("enough".into()),
                }),
                Level::INFO,
                "x.js was stopped: enough",
            ),
            (
                Err(RuntimeError::Killed(Kill::Requested)),
                Level::WARN,
                "x.js was killed",
            ),
            (
                Err(RuntimeError::Killed(Kill::Watchdog)),
                Level::WARN,
                "x.js was killed: a turn used more than 200 ms of CPU time",
            ),
            (
                Err(RuntimeError::Killed(Kill::HeapLimit)),
                Level::WARN,
                "x.js was killed: it reached the 64 MiB heap limit",
            ),
            (
                Err(RuntimeError::Stopped),
                Level::WARN,
                "x.js ended because its player stopped",
            ),
            (
                Err(RuntimeError::EventsLost),
                Level::WARN,
                "x.js ended: script events overflowed and were lost",
            ),
            (
                Err(RuntimeError::JavaScript("boom".into())),
                Level::WARN,
                "x.js ended: boom",
            ),
        ] {
            assert_eq!(ended("x.js", result), (level, line.into()));
        }
    }
}
