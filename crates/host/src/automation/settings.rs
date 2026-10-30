use std::{
    collections::BTreeSet,
    task::Waker,
    time::{Duration, Instant},
};

use lucent_protocol::command::Command;

use crate::{
    ScriptLink,
    bridge::{Error, Pending, Value},
};

const REPLY_TIMEOUT: Duration = Duration::from_secs(5);

pub struct SettingsPolicy {
    inbox: crate::player::Inbox,
    status: crate::PlayerStatus,
    settings: Settings,
    waker: Waker,
}

impl SettingsPolicy {
    pub fn new(link: ScriptLink, waker: Waker) -> Self {
        Self {
            inbox: link.inbox(),
            status: link.player_status(),
            settings: Settings::new(Instant::now()),
            waker,
        }
    }

    pub fn advance(&mut self, now: Instant) -> Option<Instant> {
        if self.status.is_stopped() {
            return None;
        }
        Some(self.settings.advance(&self.inbox, &self.waker, now))
    }
}

trait SettingsHost {
    fn ready(&self) -> Pending<bool>;
    fn cached_logged_in(&self) -> Pending<bool>;
    fn command(&self, command: Command) -> Pending<String>;
    fn call(&self, method: &'static str) -> Pending<Value>;
}

impl SettingsHost for crate::player::Inbox {
    fn ready(&self) -> Pending<bool> {
        self.read_projection(|projection| projection.current_readiness().missing.is_empty())
    }
    fn cached_logged_in(&self) -> Pending<bool> {
        self.read_projection(|projection| projection.auth.logged_in)
    }
    fn command(&self, command: Command) -> Pending<String> {
        let (reply, pending) = Pending::channel();
        self.post(crate::player::Msg::Command(command, reply));
        pending
    }
    fn call(&self, method: &'static str) -> Pending<Value> {
        self.call_loader(method, vec![])
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    Full,
    Actions,
}

impl Kind {
    fn interval(self) -> Duration {
        match self {
            Self::Full => Duration::from_secs(1),
            Self::Actions => Duration::from_millis(500),
        }
    }
}

struct Settings {
    ticks: [Tick; 2],
    failing: BTreeSet<Kind>,
}

impl Settings {
    fn new(now: Instant) -> Self {
        Self {
            ticks: [Tick::new(Kind::Full, now), Tick::new(Kind::Actions, now)],
            failing: BTreeSet::new(),
        }
    }

    fn advance(&mut self, host: &impl SettingsHost, waker: &Waker, now: Instant) -> Instant {
        for tick in &mut self.ticks {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                tick.advance(host, waker, now)
            }))
            .unwrap_or_else(|panic| {
                let message = panic
                    .downcast_ref::<&str>()
                    .map(|text| (*text).to_owned())
                    .or_else(|| panic.downcast_ref::<String>().cloned())
                    .unwrap_or_default();
                Some(Err(format!("settings transition panicked: {message}")))
            });
            if let Some(result) = result {
                tick.state = State::Scheduled(now + tick.kind.interval());
                match result {
                    Ok(()) => {
                        if self.failing.remove(&tick.kind) && self.failing.is_empty() {
                            tracing::info!("settings policy recovered");
                        }
                    }
                    Err(error) => {
                        if self.failing.is_empty() {
                            tracing::warn!("settings policy tick failed: {error}");
                        }
                        self.failing.insert(tick.kind);
                    }
                }
            }
        }
        self.ticks
            .iter()
            .map(|tick| tick.state.deadline())
            .min()
            .unwrap()
    }
}

struct Tick {
    kind: Kind,
    state: State,
}

enum State {
    Scheduled(Instant),
    HostReady(Pending<bool>, Instant),
    Checking(Readiness, Instant),
    Applying(Pending<String>, Instant),
}

impl State {
    fn deadline(&self) -> Instant {
        match self {
            Self::Scheduled(at) | Self::HostReady(_, at) | Self::Applying(_, at) => *at,
            Self::Checking(readiness, at) => readiness.deadline(*at),
        }
    }
}

fn pending<T>(reply: Pending<T>, waker: &Waker) -> Pending<T> {
    reply.set_waker(waker.clone());
    reply
}

fn receive<T>(reply: &Pending<T>, deadline: Instant, now: Instant) -> Option<Result<T, String>> {
    reply
        .try_recv()
        .map(|result| result.map_err(|error| error.to_string()))
        .or_else(|| (now >= deadline).then(|| Err(Error::Timeout.to_string())))
}

enum Reply<T> {
    Pending(Pending<T>),
    Received(T),
}

impl<T> Reply<T> {
    fn poll(&mut self, deadline: Instant, now: Instant) -> Result<(), String> {
        if let Self::Pending(reply) = self
            && let Some(result) = receive(reply, deadline, now)
        {
            *self = Self::Received(result?);
        }
        Ok(())
    }
}

enum Login {
    Refreshing(Pending<String>, Instant),
    Cached(Pending<bool>, Instant),
    Received(bool),
}

impl Login {
    fn poll(
        &mut self,
        host: &impl SettingsHost,
        waker: &Waker,
        now: Instant,
    ) -> Result<(), String> {
        for _ in 0..2 {
            match self {
                Self::Refreshing(reply, deadline) => {
                    let Some(result) = receive(reply, *deadline, now) else {
                        return Ok(());
                    };
                    *self = match result.and_then(|json| {
                        serde_json::from_str::<bool>(&json).map_err(|error| error.to_string())
                    }) {
                        Ok(value) => Self::Received(value),
                        Err(_) => Self::Cached(
                            pending(host.cached_logged_in(), waker),
                            now + REPLY_TIMEOUT,
                        ),
                    };
                }
                Self::Cached(reply, deadline) => {
                    let Some(result) = receive(reply, *deadline, now) else {
                        return Ok(());
                    };
                    *self = Self::Received(result?);
                }
                Self::Received(_) => break,
            }
        }
        Ok(())
    }
}

struct Readiness {
    logged_in: Login,
    world: Reply<Value>,
    player: Reply<Value>,
}

impl Readiness {
    fn deadline(&self, loader_deadline: Instant) -> Instant {
        let login = match self.logged_in {
            Login::Refreshing(_, at) | Login::Cached(_, at) => Some(at),
            Login::Received(_) => None,
        };
        let loader = (matches!(self.world, Reply::Pending(_))
            || matches!(self.player, Reply::Pending(_)))
        .then_some(loader_deadline);
        login
            .into_iter()
            .chain(loader)
            .min()
            .unwrap_or(loader_deadline)
    }

    fn poll(
        &mut self,
        host: &impl SettingsHost,
        waker: &Waker,
        deadline: Instant,
        now: Instant,
    ) -> Result<Option<bool>, String> {
        self.logged_in.poll(host, waker, now)?;
        self.world.poll(deadline, now)?;
        self.player.poll(deadline, now)?;
        match (&self.logged_in, &self.world, &self.player) {
            (Login::Received(login), Reply::Received(world), Reply::Received(player)) => {
                let world = boolean(world)?;
                let player = boolean(player)?;
                Ok(Some(*login && world && player))
            }
            _ => Ok(None),
        }
    }
}

fn boolean(value: &Value) -> Result<bool, String> {
    match value {
        Value::Bool(value) => Ok(*value),
        _ => Err("Expected boolean loader reply".into()),
    }
}

enum Step {
    Pending,
    Advanced,
    Complete,
}

impl Tick {
    fn new(kind: Kind, now: Instant) -> Self {
        Self {
            kind,
            state: State::Scheduled(now),
        }
    }

    fn advance(
        &mut self,
        host: &impl SettingsHost,
        waker: &Waker,
        now: Instant,
    ) -> Option<Result<(), String>> {
        let mut step = || -> Result<Step, String> {
            match &mut self.state {
                State::Scheduled(at) => {
                    if now < *at {
                        return Ok(Step::Pending);
                    }
                    self.state =
                        State::HostReady(pending(host.ready(), waker), now + REPLY_TIMEOUT);
                }
                State::HostReady(reply, deadline) => {
                    let Some(reply) = receive(reply, *deadline, now) else {
                        return Ok(Step::Pending);
                    };
                    if !reply? {
                        return Ok(Step::Complete);
                    }
                    self.state = State::Checking(
                        Readiness {
                            logged_in: Login::Refreshing(
                                pending(host.command(Command::RefreshLoggedIn {}), waker),
                                now + REPLY_TIMEOUT,
                            ),
                            world: Reply::Pending(pending(host.call("world.isLoaded"), waker)),
                            player: Reply::Pending(pending(host.call("player.isLoaded"), waker)),
                        },
                        now + REPLY_TIMEOUT,
                    );
                }
                State::Checking(readiness, deadline) => {
                    let Some(ready) = readiness.poll(host, waker, *deadline, now)? else {
                        return Ok(Step::Pending);
                    };
                    if !ready {
                        return Ok(Step::Complete);
                    }
                    self.state = State::Applying(
                        pending(
                            host.command(Command::ReapplySettings {
                                actions_only: self.kind == Kind::Actions,
                            }),
                            waker,
                        ),
                        now + REPLY_TIMEOUT,
                    );
                }
                State::Applying(reply, deadline) => {
                    let Some(reply) = receive(reply, *deadline, now) else {
                        return Ok(Step::Pending);
                    };
                    if serde_json::from_str::<serde_json::Value>(&reply?)
                        .map_err(|error| error.to_string())?
                        != serde_json::Value::Null
                    {
                        return Err("Invalid reapplySettings reply".into());
                    }
                    return Ok(Step::Complete);
                }
            }
            Ok(Step::Advanced)
        };
        for _ in 0..4 {
            match step() {
                Ok(Step::Complete) => return Some(Ok(())),
                Err(error) => return Some(Err(error)),
                Ok(Step::Pending) => return None,
                Ok(Step::Advanced) => {}
            }
        }
        None
    }
}

#[cfg(test)]
mod tests;
