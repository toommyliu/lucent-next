use super::*;
use std::{
    cell::{Cell, RefCell},
    sync::{Arc, Mutex},
};
use tracing_subscriber::prelude::*;

#[derive(Default, Clone)]
struct Logs(Arc<Mutex<Vec<(tracing::Level, String)>>>);
impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for Logs {
    fn on_event(&self, event: &tracing::Event<'_>, _: tracing_subscriber::layer::Context<'_, S>) {
        struct Message(String);
        impl tracing::field::Visit for Message {
            fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
                if field.name() == "message" {
                    self.0 = format!("{value:?}");
                }
            }
        }
        let mut message = Message(String::new());
        event.record(&mut message);
        self.0
            .lock()
            .unwrap()
            .push((*event.metadata().level(), message.0));
    }
}
impl Logs {
    fn count(&self, text: &str) -> usize {
        self.0
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, msg)| msg.contains(text))
            .count()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Boundary {
    Read,
    Login,
    Cached,
    World,
    Player,
    Full,
    Actions,
}
#[derive(Clone)]
enum Fault {
    Error,
    Panic,
    Json(String),
    Value(Value),
    Hold,
}
struct Host {
    ready: Cell<bool>,
    cached: Cell<bool>,
    faults: RefCell<Vec<(Boundary, Fault)>>,
    applied: RefCell<Vec<bool>>,
    held: RefCell<Vec<crate::bridge::Reply<String>>>,
}
impl Default for Host {
    fn default() -> Self {
        Self {
            ready: Cell::new(true),
            cached: Cell::new(true),
            faults: RefCell::default(),
            applied: RefCell::default(),
            held: RefCell::default(),
        }
    }
}
impl Host {
    fn fault(&self, boundary: Boundary) -> Option<Fault> {
        self.faults
            .borrow()
            .iter()
            .find(|(at, _)| *at == boundary)
            .map(|(_, fault)| fault.clone())
    }
}
impl SettingsHost for Host {
    fn ready(&self) -> Pending<bool> {
        let (reply, pending) = Pending::channel();
        reply
            .send(match self.fault(Boundary::Read) {
                Some(Fault::Error) => Err(Error::ReadPanicked),
                Some(Fault::Panic) => panic!("broken native boundary"),
                _ => Ok(self.ready.get()),
            })
            .unwrap();
        pending
    }
    fn cached_logged_in(&self) -> Pending<bool> {
        let (reply, pending) = Pending::channel();
        reply
            .send(match self.fault(Boundary::Cached) {
                Some(Fault::Error) => Err(Error::ReadPanicked),
                _ => Ok(self.cached.get()),
            })
            .unwrap();
        pending
    }
    fn command(&self, command: Command) -> Pending<String> {
        let (boundary, value) = match command {
            Command::RefreshLoggedIn {} => (Boundary::Login, "true"),
            Command::ReapplySettings { actions_only } => {
                self.applied.borrow_mut().push(actions_only);
                (
                    if actions_only {
                        Boundary::Actions
                    } else {
                        Boundary::Full
                    },
                    "null",
                )
            }
            _ => unreachable!(),
        };
        let (reply, pending) = Pending::channel();
        let result = match self.fault(boundary) {
            Some(Fault::Error) => Err(Error::Unavailable("temporary host failure".into())),
            Some(Fault::Json(json)) => Ok(json),
            Some(Fault::Hold) => {
                self.held.borrow_mut().push(reply);
                return pending;
            }
            _ => Ok(value.into()),
        };
        reply.send(result).unwrap();
        pending
    }
    fn call(&self, method: &'static str) -> Pending<Value> {
        let boundary = match method {
            "world.isLoaded" => Boundary::World,
            "player.isLoaded" => Boundary::Player,
            _ => unreachable!(),
        };
        let (reply, pending) = Pending::channel();
        reply
            .send(match self.fault(boundary) {
                Some(Fault::Error) => Err(Error::Unavailable("temporary host failure".into())),
                Some(Fault::Value(value)) => Ok(value),
                _ => Ok(Value::Bool(true)),
            })
            .unwrap();
        pending
    }
}

fn advance(settings: &mut Settings, host: &Host, now: Instant) -> Instant {
    settings.advance(host, Waker::noop(), now)
}

fn retries(boundary: Boundary, fault: Fault) {
    let logs = Logs::default();
    let dispatch = tracing::Dispatch::new(tracing_subscriber::registry().with(logs.clone()));
    let _guard = tracing::dispatcher::set_default(&dispatch);
    let host = Host::default();
    host.faults.borrow_mut().push((boundary, fault));
    if boundary == Boundary::Cached {
        host.faults
            .borrow_mut()
            .push((Boundary::Login, Fault::Error));
    }
    let mut now = Instant::now();
    let mut settings = Settings::new(now);
    for _ in 0..7 {
        now = advance(&mut settings, &host, now);
    }
    assert_eq!(logs.count("settings policy tick failed"), 1);
    assert_eq!(logs.count("settings policy recovered"), 0);
    host.faults.borrow_mut().clear();
    for _ in 0..4 {
        now = advance(&mut settings, &host, now);
    }
    assert_eq!(logs.count("settings policy recovered"), 1);
    assert_eq!(logs.count("settings policy tick failed"), 1);
    assert!(host.applied.borrow().contains(&false));
    assert!(host.applied.borrow().contains(&true));
    host.faults.borrow_mut().push((boundary, Fault::Error));
    if boundary == Boundary::Cached {
        host.faults
            .borrow_mut()
            .push((Boundary::Login, Fault::Error));
    }
    for _ in 0..3 {
        now = advance(&mut settings, &host, now);
    }
    assert_eq!(logs.count("settings policy tick failed"), 2);
}

#[test]
fn failed_settings_ticks_warn_and_retry() {
    for boundary in [
        Boundary::Read,
        Boundary::Cached,
        Boundary::World,
        Boundary::Player,
        Boundary::Full,
        Boundary::Actions,
    ] {
        retries(boundary, Fault::Error);
    }
}
#[test]
fn settings_policy_code_errors_warn_once_and_retry() {
    retries(Boundary::Read, Fault::Panic);
}
#[test]
fn settings_policy_malformed_host_replies_warn_once_and_retry() {
    retries(Boundary::Full, Fault::Json("not json".into()));
    for boundary in [Boundary::World, Boundary::Player] {
        retries(boundary, Fault::Value(Value::String("true".into())));
    }
}
#[test]
fn settings_policy_missing_loader_booleans_warn_once_and_retry() {
    for boundary in [Boundary::World, Boundary::Player] {
        for value in [Value::Undefined, Value::Null] {
            retries(boundary, Fault::Value(value));
        }
    }
}
#[test]
fn settings_policy_invalid_reapply_reply_warns_once_and_retries() {
    for boundary in [Boundary::Full, Boundary::Actions] {
        for value in ["false", "{}"] {
            retries(boundary, Fault::Json(value.into()));
        }
    }
}
#[test]
fn settings_policy_uses_player_readiness() {
    for boundary in [
        Boundary::Read,
        Boundary::Login,
        Boundary::World,
        Boundary::Player,
    ] {
        let host = Host::default();
        match boundary {
            Boundary::Read => host.ready.set(false),
            Boundary::Login => host
                .faults
                .borrow_mut()
                .push((boundary, Fault::Json("false".into()))),
            _ => host
                .faults
                .borrow_mut()
                .push((boundary, Fault::Value(Value::Bool(false)))),
        }
        let mut now = Instant::now();
        let mut settings = Settings::new(now);
        for _ in 0..7 {
            now = advance(&mut settings, &host, now);
        }
        assert!(host.applied.borrow().is_empty());
        assert!(settings.failing.is_empty());
        host.ready.set(true);
        host.faults.borrow_mut().clear();
        for _ in 0..4 {
            now = advance(&mut settings, &host, now);
        }
        assert!(host.applied.borrow().contains(&false));
        assert!(host.applied.borrow().contains(&true));
    }
}
#[test]
fn settings_policy_recovery_waits_for_every_failing_loop() {
    let logs = Logs::default();
    let dispatch = tracing::Dispatch::new(tracing_subscriber::registry().with(logs.clone()));
    let _guard = tracing::dispatcher::set_default(&dispatch);
    let host = Host::default();
    *host.faults.borrow_mut() = vec![
        (Boundary::Full, Fault::Error),
        (Boundary::Actions, Fault::Error),
    ];
    let mut now = Instant::now();
    let mut settings = Settings::new(now);
    for _ in 0..5 {
        now = advance(&mut settings, &host, now);
    }
    assert_eq!(logs.count("tick failed"), 1);
    host.faults
        .borrow_mut()
        .retain(|(boundary, _)| *boundary != Boundary::Actions);
    for _ in 0..4 {
        now = advance(&mut settings, &host, now);
    }
    assert_eq!(logs.count("recovered"), 0);
    assert_eq!(settings.failing, BTreeSet::from([Kind::Full]));
    host.faults.borrow_mut().clear();
    for _ in 0..3 {
        now = advance(&mut settings, &host, now);
    }
    assert_eq!(logs.count("recovered"), 1);
}
#[test]
fn deadlines_follow_completion_and_unanswered_ticks_time_out() {
    let host = Host::default();
    let now = Instant::now();
    let mut settings = Settings::new(now);
    host.faults.borrow_mut().extend([
        (Boundary::Full, Fault::Hold),
        (Boundary::Actions, Fault::Hold),
    ]);
    assert_eq!(advance(&mut settings, &host, now), now + REPLY_TIMEOUT);
    let done = now + Duration::from_secs(3);
    for reply in host.held.borrow_mut().drain(..) {
        reply.send(Ok("null".into())).unwrap();
    }
    assert_eq!(
        advance(&mut settings, &host, done),
        done + Duration::from_millis(500)
    );
    assert_eq!(*host.applied.borrow(), [false, true]);
    assert_eq!(
        advance(&mut settings, &host, done + Duration::from_millis(500)),
        done + Duration::from_secs(1)
    );
    advance(&mut settings, &host, done + Duration::from_secs(6));
    assert!(settings.failing.contains(&Kind::Actions));
}

#[test]
fn independent_settings_policies_keep_separate_failure_streaks() {
    let logs = Logs::default();
    let dispatch = tracing::Dispatch::new(tracing_subscriber::registry().with(logs.clone()));
    let _guard = tracing::dispatcher::set_default(&dispatch);
    let first = Host::default();
    let second = Host::default();
    let now = Instant::now();
    let mut one = Settings::new(now);
    let mut two = Settings::new(now);
    first
        .faults
        .borrow_mut()
        .push((Boundary::World, Fault::Error));
    advance(&mut one, &first, now);
    advance(&mut two, &second, now);
    assert_eq!(logs.count("tick failed"), 1);
    second
        .faults
        .borrow_mut()
        .push((Boundary::World, Fault::Error));
    advance(&mut two, &second, now + Duration::from_secs(1));
    assert_eq!(logs.count("tick failed"), 2);
    first.faults.borrow_mut().clear();
    advance(&mut one, &first, now + Duration::from_secs(1));
    assert_eq!(logs.count("recovered"), 1);
    assert!(!two.failing.is_empty());
}

#[test]
fn readiness_errors_are_reported_when_another_check_is_false() {
    let host = Host::default();
    *host.faults.borrow_mut() = vec![
        (Boundary::Login, Fault::Json("false".into())),
        (Boundary::Player, Fault::Value(Value::Null)),
    ];
    let now = Instant::now();
    let mut settings = Settings::new(now);
    advance(&mut settings, &host, now);
    assert_eq!(
        settings.failing,
        BTreeSet::from([Kind::Full, Kind::Actions])
    );
    assert!(host.applied.borrow().is_empty());
}

#[test]
fn failed_login_refreshes_use_the_cached_login_state() {
    for fault in [
        Fault::Error,
        Fault::Json("not json".into()),
        Fault::Json("{}".into()),
        Fault::Json("\"true\"".into()),
    ] {
        for cached in [false, true] {
            let host = Host::default();
            host.cached.set(cached);
            host.faults
                .borrow_mut()
                .push((Boundary::Login, fault.clone()));
            let now = Instant::now();
            let mut settings = Settings::new(now);
            advance(&mut settings, &host, now);
            if cached {
                assert_eq!(*host.applied.borrow(), [false, true]);
            } else {
                assert!(host.applied.borrow().is_empty());
            }
            assert!(settings.failing.is_empty());
        }
    }
}
