mod io;
pub(crate) use io::Snapshot;
use io::{
    Capture, Observation, Observed, Selection, SelectionStatus, ServerList, Wait, loader_json,
    selection,
};

use crate::{
    ScriptLink,
    bridge::{Error, Pending, Value},
    sessions::relogin::{Edit, Request, Shared},
};
use lucent_protocol::{
    auto_relogin::{AutoReloginPhase as Phase, AutoReloginRecords},
    command::Command,
};
use std::{
    sync::Arc,
    task::Waker,
    time::{Duration, Instant},
};

const REPLY_TIMEOUT: Duration = Duration::from_secs(5);
const POLL: Duration = Duration::from_millis(500);

pub struct ReloginPolicy {
    link: ScriptLink,
    policy: Policy,
    waker: Waker,
}
impl ReloginPolicy {
    pub fn new(link: ScriptLink, waker: Waker) -> Self {
        link.set_waker(waker.clone());
        link.records.relogin.set_waker(waker.clone());
        let policy = Policy::for_player(link.records.auto_relogin.read().clone(), Instant::now());
        Self {
            link,
            policy,
            waker,
        }
    }
    pub fn advance(&mut self, now: Instant) -> Option<Instant> {
        let mut draining = false;
        for index in 0..64 {
            match self.link.try_event() {
                Ok(Some(crate::bridge::ScriptEvent::Loader { event, .. }))
                    if event.name == "onConnection" =>
                {
                    if let Some(Value::String(status)) = event.args.first() {
                        self.policy.connection(status, now);
                    }
                }
                Ok(Some(_)) | Err(Error::Overflow) => {}
                Ok(None) => break,
                Err(_) => return None,
            }
            draining = index == 63;
        }
        let shared = self.link.records.relogin.clone();
        let at = self.policy.advance(&self.link, &shared, &self.waker, now);
        *self.link.records.auto_relogin.write() = self.policy.records.clone();
        if draining { Some(now) } else { at }
    }
}

trait ReloginHost {
    fn snapshot(&self) -> Pending<Snapshot>;
    fn command(&self, command: Command) -> Pending<String>;
    fn call(&self, method: &'static str, args: Vec<Value>) -> Pending<Value>;
}
impl ReloginHost for ScriptLink {
    fn snapshot(&self) -> Pending<Snapshot> {
        self.relogin_snapshot()
    }
    fn command(&self, command: Command) -> Pending<String> {
        self.command(command)
    }
    fn call(&self, method: &'static str, args: Vec<Value>) -> Pending<Value> {
        self.call_loader(method, args)
    }
}
struct Setter {
    request: Request,
    work: SetterWork,
}
enum SetterWork {
    Immediate,
    Enable(Box<Enable>),
    Server(Box<ServerList>),
    Retry(Box<Observation>),
}
struct Enable {
    capture: Option<Capture>,
    observation: Observation,
}
impl Setter {
    fn answer(&mut self, answer: Result<String, Error>) {
        if let Some(reply) = self.request.reply.take() {
            let _ = reply.send(answer);
        }
    }
}
#[derive(Default)]
enum Recovery {
    #[default]
    Idle,
    WaitingCredentials(Instant),
    Delay(Instant),
    Attempt {
        resume: bool,
    },
    Kick(Wait<Value>),
    KickDuration(Wait<Value>),
    WaitingKick {
        until: Instant,
        next: Instant,
        check: Option<Wait<Value>>,
    },
    Login(Wait<String>),
    Outcome(Instant),
    Manual(Instant),
    Select {
        reply: Wait<Value>,
        expires: Instant,
    },
    SelectionResult {
        value: Option<Selection>,
        expires: Instant,
    },
    SelectAgain {
        at: Instant,
        expires: Instant,
    },
    Player(Instant),
    PlayerCheck,
    Logout {
        reply: Wait<String>,
        detail: String,
    },
    Backoff(Instant),
    Failed,
}
#[derive(Default)]
struct Policy {
    records: AutoReloginRecords,
    setter: Option<Setter>,
    capture: Option<Capture>,
    capture_at: Option<Instant>,
    observation: Option<Observation>,
    observe_at: Option<Instant>,
    observed: Observed,
    recovery: Recovery,
    attempt: u8,
    watch_until: Option<Instant>,
    fresh_login: bool,
    capture_failing: bool,
}
impl Policy {
    fn for_player(records: AutoReloginRecords, now: Instant) -> Self {
        let attempt = records
            .status
            .attempts_remaining
            .map_or(0, |left| 3 - (left as u8).min(3));
        let recovery = if !records.config.enabled || !records.status.captured {
            Recovery::Idle
        } else {
            match records.status.phase {
                Phase::Armed | Phase::AwaitingSession => Recovery::Idle,
                Phase::Failed => Recovery::Failed,
                Phase::WaitingDelay => Recovery::Delay(now),
                Phase::Backoff { delay_ms } => {
                    Recovery::Backoff(now + Duration::from_secs_f64(delay_ms / 1000.0))
                }
                _ => Recovery::Attempt { resume: false },
            }
        };
        let observe_at = (!matches!(recovery, Recovery::Idle | Recovery::Failed)).then_some(now);
        Self {
            records,
            attempt,
            recovery,
            observe_at,
            ..Self::default()
        }
    }
    fn connection(&mut self, status: &str, now: Instant) {
        if !self.records.config.enabled {
            return;
        }
        match status {
            "OnConnectionLost" | "OnConnectionFailed"
                if matches!(self.recovery, Recovery::Idle) =>
            {
                self.capture_failing = false;
                self.recovery = if self.records.status.captured {
                    Recovery::Delay(now)
                } else {
                    self.capture_at = Some(now);
                    Recovery::WaitingCredentials(now)
                };
                self.records.status.phase = if self.records.status.captured {
                    Phase::WaitingDelay
                } else {
                    Phase::AwaitingSession
                };
                self.records.status.attempts_remaining = None;
                self.records.status.last_error = None;
                self.observed = Observed::default();
                self.observe_at = Some(now);
                self.observation = None;
            }
            "OnConnection" => {
                self.watch_until = Some(now + Duration::from_secs(10));
                self.observe_at = Some(now);
                if !self.records.status.captured {
                    self.capture_at = Some(now);
                }
            }
            _ => {}
        }
    }
    fn armed(&mut self) {
        self.records.status.phase = Phase::Armed;
        self.records.status.attempts_remaining = None;
        self.records.status.last_error = None;
        self.recovery = Recovery::Idle;
        self.observation = None;
        self.observe_at = None;
        self.watch_until = None;
    }
    fn fail(&mut self, detail: &str, retryable: bool, now: Instant) {
        self.records.status.last_error = Some(sentence(detail));
        if retryable && self.attempt < 3 {
            let delay = 5_u64 << self.attempt;
            self.records.status.phase = Phase::Backoff {
                delay_ms: delay as f64 * 1000.0,
            };
            self.recovery = Recovery::Backoff(now + Duration::from_secs(delay));
        } else {
            self.records.status.phase = Phase::Failed;
            self.records.status.attempts_remaining = retryable.then_some(0.0);
            self.recovery = Recovery::Failed;
            self.observation = None;
            self.observe_at = None;
        }
    }
    fn captured(
        &mut self,
        result: Result<(String, String, bool), Error>,
        shared: &Shared,
    ) -> Result<(), Error> {
        let (username, password, ready) = result?;
        if !self.records.status.captured {
            if username.is_empty() || password.is_empty() {
                self.records.status.phase = if ready {
                    Phase::Armed
                } else {
                    Phase::AwaitingSession
                };
            } else {
                self.records.status.captured = true;
                self.records.status.last_error = None;
                self.records.status.attempts_remaining = None;
                self.records.status.username = Some(username.clone());
                *shared.credentials.lock() = Some((username, password));
                if let Recovery::WaitingCredentials(start) = self.recovery {
                    self.recovery = Recovery::Delay(start);
                    self.records.status.phase = Phase::WaitingDelay;
                } else {
                    self.records.status.phase = Phase::Armed;
                    self.recovery = Recovery::Idle;
                }
            }
        }
        Ok(())
    }
    fn recovery_deadline(&self) -> Option<Instant> {
        match &self.recovery {
            Recovery::Idle
            | Recovery::Failed
            | Recovery::WaitingCredentials(_)
            | Recovery::PlayerCheck
            | Recovery::SelectionResult { .. } => None,
            Recovery::Delay(start) => {
                Some(*start + Duration::from_secs_f64(self.records.config.delay_ms / 1000.0))
            }
            Recovery::Attempt { .. } => Some(Instant::now()),
            Recovery::Kick(wait) | Recovery::KickDuration(wait) => Some(wait.deadline),
            Recovery::Login(wait) => Some(wait.deadline),
            Recovery::Outcome(at)
            | Recovery::Manual(at)
            | Recovery::Player(at)
            | Recovery::Backoff(at) => Some(*at),
            Recovery::Select { reply, expires } => Some(reply.deadline.min(*expires)),
            Recovery::SelectAgain { at, .. } => Some(*at),
            Recovery::Logout { reply, .. } => Some(reply.deadline),
            Recovery::WaitingKick { until, next, check } => Some(
                check
                    .as_ref()
                    .map_or((*until).min(*next), |reply| reply.deadline.min(*until)),
            ),
        }
    }
    fn advance(
        &mut self,
        host: &impl ReloginHost,
        shared: &Arc<Shared>,
        waker: &Waker,
        now: Instant,
    ) -> Option<Instant> {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.transition(host, shared, waker, now)
        })) {
            Ok(at) => at,
            Err(_) => {
                tracing::error!("auto relogin transition panicked");
                self.fail("Auto relogin failed unexpectedly.", false, now);
                self.capture = None;
                if self.records.config.enabled && !self.records.status.captured {
                    self.capture_at = Some(now + REPLY_TIMEOUT);
                }
                if let Some(mut setter) = self.setter.take() {
                    setter.answer(Err(Error::ReadPanicked));
                }
                *shared.records.write() = self.records.clone();
                Some(now)
            }
        }
    }
    fn transition(
        &mut self,
        host: &impl ReloginHost,
        shared: &Arc<Shared>,
        waker: &Waker,
        now: Instant,
    ) -> Option<Instant> {
        if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.setters(host, shared, waker, now)
        }))
        .is_err()
        {
            tracing::warn!("auto relogin setter panicked");
            if let Some(mut setter) = self.setter.take() {
                setter.answer(Err(Error::ReadPanicked));
            }
            if self.records.config.enabled && !self.records.status.captured {
                self.capture_at = Some(now + REPLY_TIMEOUT);
            }
        }
        if self.records.config.enabled {
            if !self.records.status.captured && self.capture.is_none() && self.capture_at.is_none()
            {
                self.capture_at = Some(now);
            }
            if self.capture_at.is_some_and(|at| at <= now)
                && self.capture.is_none()
                && self.setter.is_none()
            {
                self.capture = Some(Capture::new(host, waker, now));
                self.capture_at = None;
            }
            if let Some(capture) = &mut self.capture
                && let Some(result) = capture.poll(now)
            {
                self.capture = None;
                match self.captured(result, shared) {
                    Ok(()) => self.capture_failing = false,
                    Err(error) => {
                        if !self.capture_failing {
                            tracing::warn!("auto relogin capture failed: {error}");
                        }
                        self.capture_failing = true;
                    }
                }
                if !self.records.status.captured {
                    self.capture_at = Some(now + REPLY_TIMEOUT);
                }
            }
            if self.observe_at.is_some_and(|at| at <= now) && self.observation.is_none() {
                self.observation = Some(Observation::new(host, waker, now));
                self.observe_at = None;
            }
            if let Some(observation) = &mut self.observation
                && let Some(observed) = observation.poll(now)
            {
                self.observed = observed;
                self.observation = None;
                if self.observed.ready {
                    self.armed();
                } else if !matches!(self.recovery, Recovery::Idle | Recovery::Failed)
                    || self.watch_until.is_some_and(|at| at > now)
                {
                    self.observe_at = Some(now + POLL);
                } else {
                    self.watch_until = None;
                }
            }
            for _ in 0..16 {
                if !self.recover(host, shared, waker, now) {
                    break;
                }
            }
        }
        let setter_at = self.setter.as_ref().map(|s| {
            let deadline = match &s.work {
                SetterWork::Immediate => now,
                SetterWork::Enable(enable) => enable.observation.deadline,
                SetterWork::Retry(observation) => observation.deadline,
                SetterWork::Server(list) => list.deadline,
            };
            if s.request.reply.is_some() {
                deadline.min(s.request.expires)
            } else {
                deadline
            }
        });
        [
            (self.setter.is_none() && shared.has_pending()).then_some(now),
            self.recovery_deadline(),
            self.observe_at,
            self.observation.as_ref().map(|o| o.deadline),
            self.capture_at.filter(|_| self.setter.is_none()),
            self.capture.as_ref().map(|_| now + REPLY_TIMEOUT),
            setter_at,
        ]
        .into_iter()
        .flatten()
        .min()
    }
    fn setters(
        &mut self,
        host: &impl ReloginHost,
        shared: &Arc<Shared>,
        waker: &Waker,
        now: Instant,
    ) {
        for _ in 0..64 {
            if self.setter.is_none() {
                while let Some(mut request) = shared.take() {
                    if !request.alive(now) {
                        if let Some(reply) = request.reply.take() {
                            let _ = reply.send(Err(Error::Timeout));
                        }
                        continue;
                    }
                    let work = match &request.edit {
                        Edit::Enabled(false) => {
                            self.armed();
                            self.records.config.enabled = false;
                            self.records.status.captured = false;
                            self.records.status.username = None;
                            *shared.credentials.lock() = None;
                            self.capture = None;
                            self.capture_at = None;
                            SetterWork::Immediate
                        }
                        Edit::Enabled(true) => {
                            self.records.config.enabled = true;
                            SetterWork::Enable(Box::new(Enable {
                                capture: (!self.records.status.captured)
                                    .then(|| Capture::new(host, waker, now)),
                                observation: Observation::new(host, waker, now),
                            }))
                        }
                        Edit::Delay(delay) => {
                            self.records.config.delay_ms = normalize_delay(*delay);
                            SetterWork::Immediate
                        }
                        Edit::Server(server)
                            if server.as_ref().is_some_and(|s| !s.trim().is_empty()) =>
                        {
                            SetterWork::Server(Box::new(ServerList::new(host, waker, now)))
                        }
                        Edit::Server(server) => {
                            if self.server(server.clone()) {
                                SetterWork::Retry(Box::new(Observation::new(host, waker, now)))
                            } else {
                                SetterWork::Immediate
                            }
                        }
                    };
                    self.setter = Some(Setter { request, work });
                    break;
                }
            }
            let Some(mut setter) = self.setter.take() else {
                break;
            };
            if now >= setter.request.expires {
                setter.answer(Err(Error::Timeout));
            }
            let mut next_work = None;
            let result = match &mut setter.work {
                SetterWork::Immediate => Some(Ok(())),
                SetterWork::Enable(enable) => {
                    let captured = if let Some(capture) = &mut enable.capture {
                        capture
                            .poll(now)
                            .map(|result| self.captured(result, shared))
                    } else {
                        Some(Ok(()))
                    };
                    match captured {
                        Some(Ok(())) => {
                            enable.capture = None;
                            enable.observation.poll(now).map(|observed| {
                                if observed.ready {
                                    self.armed();
                                }
                                self.observed = observed;
                                Ok(())
                            })
                        }
                        other => other,
                    }
                }
                SetterWork::Retry(observation) => observation.poll(now).map(|observed| {
                    if observed.ready {
                        self.armed();
                    } else {
                        self.attempt = 0;
                        self.recovery = Recovery::Attempt { resume: true };
                        self.observe_at = Some(now);
                    }
                    self.observed = observed;
                    Ok(())
                }),
                SetterWork::Server(list) => match list.poll(now) {
                    Some(Ok(names)) => {
                        let Edit::Server(server) = &setter.request.edit else {
                            unreachable!()
                        };
                        let requested = server.as_deref().unwrap().trim();
                        let canonical = names
                            .iter()
                            .find(|s| s.to_lowercase() == requested.to_lowercase())
                            .map_or(requested, |s| s.as_str())
                            .to_owned();
                        if self.server(Some(canonical)) {
                            next_work = Some(SetterWork::Retry(Box::new(Observation::new(
                                host, waker, now,
                            ))));
                            None
                        } else {
                            Some(Ok(()))
                        }
                    }
                    Some(Err(error)) => Some(Err(error)),
                    None => None,
                },
            };
            if let Some(work) = next_work {
                setter.work = work;
            }
            if let Some(result) = result {
                if self.records.config.enabled && !self.records.status.captured {
                    self.capture_at = Some(now + REPLY_TIMEOUT);
                }
                *shared.records.write() = self.records.clone();
                setter.answer(result.map(|()| serde_json::to_string(&self.records).unwrap()));
            } else {
                self.setter = Some(setter);
                break;
            }
        }
    }
    fn server(&mut self, server: Option<String>) -> bool {
        let server = server.map(|s| s.trim().to_owned());
        if self.records.config.server != server {
            self.records.config.server = server;
            self.records.status.last_error = None;
            if self.records.config.enabled
                && self.records.status.captured
                && matches!(self.recovery, Recovery::Failed)
            {
                self.records.status.phase = Phase::WaitingDelay;
                self.records.status.attempts_remaining = None;
                return true;
            }
        }
        false
    }
    fn recover(
        &mut self,
        host: &impl ReloginHost,
        shared: &Shared,
        waker: &Waker,
        now: Instant,
    ) -> bool {
        let state = std::mem::take(&mut self.recovery);
        self.recovery = match state {
            Recovery::Delay(start)
                if now
                    >= start + Duration::from_secs_f64(self.records.config.delay_ms / 1000.0) =>
            {
                self.attempt = 0;
                Recovery::Attempt { resume: false }
            }
            Recovery::Attempt { resume } => {
                self.fresh_login = false;
                self.records.status.attempts_remaining = Some(f64::from(3 - self.attempt));
                self.records.status.last_error = None;
                if resume && self.observed.select {
                    self.records.status.phase = Phase::AwaitingServerSelect;
                    Recovery::Manual(now + Duration::from_millis(250))
                } else {
                    Recovery::Kick(Wait::new(
                        host.call("auth.isTemporarilyKicked", vec![]),
                        waker,
                        now,
                    ))
                }
            }
            Recovery::Kick(reply) => match reply.poll(now) {
                Some(Ok(Value::Bool(true))) => {
                    self.records.status.phase = Phase::WaitingKick;
                    Recovery::KickDuration(Wait::new(
                        host.call(
                            "flash.getGameObject",
                            vec![Value::String("mcLogin.warning.n".into())],
                        ),
                        waker,
                        now,
                    ))
                }
                Some(_) => {
                    return self.login(host, shared, waker, now);
                }
                None => {
                    self.recovery = Recovery::Kick(reply);
                    return false;
                }
            },
            Recovery::KickDuration(reply) => match reply.poll(now) {
                Some(result) => {
                    let json = result.ok().and_then(loader_json);
                    let seconds = json
                        .and_then(|v| v.as_f64())
                        .filter(|n| n.is_finite() && *n > 0.0)
                        .map_or(60.0, |n| n.ceil().min(70.0));
                    Recovery::WaitingKick {
                        until: now + Duration::from_secs(70),
                        next: now + Duration::from_secs_f64((seconds + 1.0).min(70.0)),
                        check: None,
                    }
                }
                None => {
                    self.recovery = Recovery::KickDuration(reply);
                    return false;
                }
            },
            Recovery::WaitingKick {
                until,
                next,
                mut check,
            } => {
                if now >= next && check.is_none() {
                    check = Some(Wait::new(
                        host.call("auth.isTemporarilyKicked", vec![]),
                        waker,
                        now,
                    ));
                }
                if let Some(result) = check.as_ref().and_then(|r| r.poll(now)) {
                    if !matches!(result, Ok(Value::Bool(true))) {
                        return self.login(host, shared, waker, now);
                    }
                    check = None;
                    if now >= until {
                        self.fail("Login could not be submitted", true, now);
                        return true;
                    }
                    self.recovery = Recovery::WaitingKick {
                        until,
                        next: (now + Duration::from_secs(5)).min(until),
                        check,
                    };
                    return false;
                }
                if now >= until {
                    self.fail("Login could not be submitted", true, now);
                    return true;
                }
                self.recovery = Recovery::WaitingKick { until, next, check };
                return false;
            }
            Recovery::Login(reply) => match reply.poll(now) {
                Some(Ok(json)) if json == "true" => {
                    self.records.status.phase = Phase::AwaitingServerSelect;
                    Recovery::Outcome(now + Duration::from_secs(15))
                }
                Some(_) => {
                    self.fail("Login could not be submitted", true, now);
                    return true;
                }
                None => {
                    self.recovery = Recovery::Login(reply);
                    return false;
                }
            },
            Recovery::Outcome(expires) => {
                if self.observed.logged_in {
                    self.records.status.phase = Phase::AwaitingPlayerReady;
                    Recovery::Player(now + Duration::from_secs(10))
                } else if self.observed.select {
                    Recovery::Manual(now + Duration::from_millis(250))
                } else if now >= expires {
                    self.fail("Login did not reach server selection", true, now);
                    return true;
                } else {
                    self.recovery = Recovery::Outcome(expires);
                    return false;
                }
            }
            Recovery::Manual(at) if now >= at => {
                if self.observed.logged_in {
                    self.records.status.phase = Phase::AwaitingPlayerReady;
                    Recovery::Player(now + Duration::from_secs(10))
                } else if let Some(server) = &self.records.config.server {
                    self.records.status.phase = Phase::SelectingServer;
                    Recovery::Select {
                        reply: Wait::new(
                            host.call("auth.connectTo", vec![Value::String(server.clone())]),
                            waker,
                            now,
                        ),
                        expires: now + Duration::from_secs(5),
                    }
                } else {
                    self.armed();
                    return false;
                }
            }
            Recovery::Select { reply, expires } => match reply.poll(now) {
                Some(result) => {
                    self.observation = Some(Observation::new(host, waker, now));
                    self.observe_at = None;
                    Recovery::SelectionResult {
                        value: result.ok().and_then(selection),
                        expires,
                    }
                }
                None => {
                    self.recovery = Recovery::Select { reply, expires };
                    return false;
                }
            },
            Recovery::SelectionResult { value, expires } if self.observation.is_none() => {
                if self.observed.logged_in
                    || value
                        .as_ref()
                        .is_some_and(|s| s.status == SelectionStatus::Selected)
                {
                    self.records.status.phase = Phase::AwaitingPlayerReady;
                    Recovery::Player(now + Duration::from_secs(10))
                } else if value
                    .as_ref()
                    .is_none_or(|s| s.status == SelectionStatus::NotReady)
                    && now < expires
                {
                    Recovery::SelectAgain {
                        at: now + POLL,
                        expires,
                    }
                } else {
                    let detail = value
                        .as_ref()
                        .map_or("Server selection is not ready", |s| s.message.as_str());
                    if value
                        .as_ref()
                        .is_some_and(|s| s.reason.as_deref() == Some("full"))
                    {
                        self.fresh_login = true;
                        Recovery::Logout {
                            reply: Wait::new(host.command(Command::Logout {}), waker, now),
                            detail: detail.to_owned(),
                        }
                    } else {
                        self.fail(
                            detail,
                            value
                                .as_ref()
                                .is_none_or(|s| s.status == SelectionStatus::NotReady),
                            now,
                        );
                        return true;
                    }
                }
            }
            Recovery::SelectAgain { at, expires } if now >= at => {
                if let Some(server) = &self.records.config.server {
                    Recovery::Select {
                        reply: Wait::new(
                            host.call("auth.connectTo", vec![Value::String(server.clone())]),
                            waker,
                            now,
                        ),
                        expires,
                    }
                } else {
                    self.armed();
                    return false;
                }
            }
            Recovery::Player(expires) if now >= expires => {
                self.observation = Some(Observation::new(host, waker, now));
                self.observe_at = None;
                Recovery::PlayerCheck
            }
            Recovery::PlayerCheck if self.observation.is_none() => Recovery::Logout {
                reply: Wait::new(host.command(Command::Logout {}), waker, now),
                detail: "Player did not become ready".into(),
            },
            Recovery::Logout { reply, detail } => match reply.poll(now) {
                Some(result) => {
                    self.fail(
                        if matches!(result, Ok(ref json) if json == "null") {
                            &detail
                        } else {
                            "Logout could not be submitted"
                        },
                        true,
                        now,
                    );
                    return true;
                }
                None => {
                    self.recovery = Recovery::Logout { reply, detail };
                    return false;
                }
            },
            Recovery::Backoff(at) if now >= at => {
                self.attempt += 1;
                Recovery::Attempt {
                    resume: self.observed.select && !self.fresh_login,
                }
            }
            other => {
                self.recovery = other;
                return false;
            }
        };
        true
    }
    fn login(
        &mut self,
        host: &impl ReloginHost,
        shared: &Shared,
        waker: &Waker,
        now: Instant,
    ) -> bool {
        let Some((username, password)) = shared.credentials.lock().clone() else {
            self.fail("Username and password are required", false, now);
            return false;
        };
        self.records.status.phase = Phase::LoggingIn;
        self.recovery = Recovery::Login(Wait::new(
            host.command(Command::Login { username, password }),
            waker,
            now,
        ));
        true
    }
}
fn normalize_delay(delay: f64) -> f64 {
    if delay.is_finite() {
        delay.clamp(0.0, 300_000.0)
    } else {
        3000.0
    }
}
fn sentence(detail: &str) -> String {
    let detail = detail.trim();
    if detail.is_empty() {
        return "Auto relogin failed.".into();
    }
    let mut chars = detail.chars();
    let mut text = chars.next().unwrap().to_uppercase().to_string();
    text.push_str(chars.as_str());
    if !text.ends_with(['.', '!', '?']) {
        text.push('.');
    }
    text
}
#[cfg(test)]
mod tests;
