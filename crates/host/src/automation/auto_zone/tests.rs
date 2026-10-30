use super::*;
use std::{
    cell::{Cell, RefCell},
    sync::{Arc, Mutex},
};
use tracing_subscriber::prelude::*;

const HALF_SECOND: Duration = Duration::from_millis(500);
const REPLY_LIMIT: Duration = Duration::from_secs(5);

#[derive(Default, Clone)]
struct Logs(Arc<Mutex<Vec<String>>>);
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
        self.0.lock().unwrap().push(message.0);
    }
}
impl Logs {
    fn count(&self, text: &str) -> usize {
        self.0
            .lock()
            .unwrap()
            .iter()
            .filter(|message| message.contains(text))
            .count()
    }
}

struct Host {
    config: RefCell<Result<String, String>>,
    snapshot: RefCell<Result<Snapshot, String>>,
    panic: Cell<bool>,
    walks: RefCell<Vec<(i32, i32)>>,
    walk_reply: RefCell<Result<Value, Error>>,
    hold: Cell<bool>,
    held: RefCell<Vec<crate::bridge::Reply<Value>>>,
}
impl Host {
    fn new(map: &str) -> Self {
        Self {
            config: RefCell::new(Ok(format!(r#"{{"enabled":true,"map":"{map}"}}"#))),
            snapshot: RefCell::new(Ok(Snapshot {
                map: map.into(),
                alive: true,
                charge: Charge::None,
            })),
            panic: Cell::new(false),
            walks: RefCell::default(),
            walk_reply: RefCell::new(Ok(Value::Bool(true))),
            hold: Cell::new(false),
            held: RefCell::default(),
        }
    }
}
impl AutoZoneHost for Host {
    fn config(&self) -> Result<String, String> {
        if self.panic.get() {
            panic!("broken native zone boundary");
        }
        self.config.borrow().clone()
    }
    fn snapshot(&self) -> Result<Snapshot, String> {
        if self.panic.get() {
            panic!("broken native zone boundary");
        }
        self.snapshot.borrow().clone()
    }
    fn walk(&self, x: i32, y: i32) -> Pending<Value> {
        self.walks.borrow_mut().push((x, y));
        let (reply, pending) = Pending::channel();
        if self.hold.get() {
            self.held.borrow_mut().push(reply);
        } else {
            reply.send(self.walk_reply.borrow().clone()).unwrap();
        }
        pending
    }
}
fn advance(policy: &mut AutoZone, host: &Host, now: Instant) -> Option<Instant> {
    policy.advance(host, Waker::noop(), now)
}
fn active(map: AutoZoneMap) -> AutoZoneConfig {
    AutoZoneConfig {
        enabled: true,
        map: Some(map),
    }
}

#[test]
fn host_read_failures_retry_once_per_streak_and_use_the_latest_record() {
    let now = Instant::now();
    let logs = Logs::default();
    let dispatch = tracing::Dispatch::new(tracing_subscriber::registry().with(logs.clone()));
    let _guard = tracing::dispatcher::set_default(&dispatch);
    let host = Host::new("moreskulls");
    let mut policy = AutoZone::new(now);
    for fault in [
        Err("temporary zone host failure".into()),
        Ok("{".into()),
        Ok("{}".into()),
        Ok(r#"{"enabled":"yes","map":null}"#.into()),
        Ok(r#"{"enabled":true,"map":"unknown"}"#.into()),
    ] {
        policy.refresh(now);
        *host.config.borrow_mut() = fault;
        for i in 0..2 {
            let at = now + HALF_SECOND * i;
            assert_eq!(advance(&mut policy, &host, at), Some(at + HALF_SECOND));
            assert_eq!(
                advance(&mut policy, &host, at + HALF_SECOND / 2),
                Some(at + HALF_SECOND)
            );
        }
        assert_eq!(logs.count("auto zone tick failed"), 1);
    }
    *host.config.borrow_mut() = Ok(r#"{"enabled":true,"map":"moreskulls"}"#.into());
    advance(&mut policy, &host, now + HALF_SECOND * 2);
    policy.zone("moreskulls".into(), "A".into(), now + HALF_SECOND * 2);
    advance(&mut policy, &host, now + HALF_SECOND * 2);
    assert_eq!(host.walks.borrow().len(), 1);
    assert!((696..=802).contains(&host.walks.borrow()[0].0));
    assert_eq!(logs.count("auto zone recovered"), 1);
    *host.config.borrow_mut() = Err("another failure".into());
    policy.refresh(now + HALF_SECOND * 3);
    advance(&mut policy, &host, now + HALF_SECOND * 3);
    assert_eq!(logs.count("auto zone tick failed"), 2);
}

#[test]
fn panics_and_snapshot_errors_retry_and_recover_with_their_cause() {
    for config_failure in [true, false] {
        let now = Instant::now();
        let logs = Logs::default();
        let dispatch = tracing::Dispatch::new(tracing_subscriber::registry().with(logs.clone()));
        let _guard = tracing::dispatcher::set_default(&dispatch);
        let host = Host::new("ledgermayne");
        let mut policy = AutoZone::new(now);
        if !config_failure {
            advance(&mut policy, &host, now);
        }
        policy.zone("ledgermayne".into(), "A".into(), now);
        host.panic.set(true);
        assert_eq!(advance(&mut policy, &host, now), Some(now + HALF_SECOND));
        assert_eq!(
            advance(&mut policy, &host, now + HALF_SECOND / 2),
            Some(now + HALF_SECOND)
        );
        assert_eq!(
            advance(&mut policy, &host, now + HALF_SECOND),
            Some(now + HALF_SECOND * 2)
        );
        assert_eq!(logs.count("auto zone tick failed"), 1);
        assert_eq!(logs.count("broken native zone boundary"), 1);
        host.panic.set(false);
        if !config_failure {
            *host.snapshot.borrow_mut() = Err("map still unavailable".into());
            advance(&mut policy, &host, now + HALF_SECOND * 2);
            *host.snapshot.borrow_mut() = Ok(Snapshot {
                map: "ledgermayne".into(),
                alive: true,
                charge: Charge::None,
            });
        }
        advance(&mut policy, &host, now + HALF_SECOND * 3);
        assert_eq!(host.walks.borrow().len(), 1);
        assert_eq!(logs.count("auto zone recovered"), 1);
        assert_eq!(advance(&mut policy, &host, now + HALF_SECOND * 10), None);
        assert_eq!(host.walks.borrow().len(), 1);
    }
}

#[test]
fn failed_walks_finish_once_and_leave_the_next_zone_ready() {
    let logs = Logs::default();
    let dispatch = tracing::Dispatch::new(tracing_subscriber::registry().with(logs.clone()));
    let _guard = tracing::dispatcher::set_default(&dispatch);
    for reply in [
        Err(Error::Unavailable("temporary walk failure".into())),
        Ok(Value::Bool(false)),
        Ok(Value::String("bad".into())),
    ] {
        let now = Instant::now();
        let host = Host::new("ledgermayne");
        *host.walk_reply.borrow_mut() = reply;
        let mut policy = AutoZone::new(now);
        policy.zone("ledgermayne".into(), "A".into(), now);
        advance(&mut policy, &host, now);
        assert_eq!(advance(&mut policy, &host, now + HALF_SECOND * 2), None);
        assert_eq!(host.walks.borrow().len(), 1);
        *host.walk_reply.borrow_mut() = Ok(Value::Bool(true));
        policy.zone("ledgermayne".into(), "B".into(), now + HALF_SECOND * 2);
        advance(&mut policy, &host, now + HALF_SECOND * 2);
        assert_eq!(host.walks.borrow().len(), 2);
        assert!((727..=852).contains(&host.walks.borrow()[1].0));
    }
    assert_eq!(logs.count("auto zone tick failed"), 0);
}

#[test]
fn newer_zones_and_config_changes_cancel_delayed_and_pending_walks() {
    let now = Instant::now();
    let host = Host::new("queeniona");
    host.snapshot.borrow_mut().as_mut().unwrap().charge = Charge::Positive;
    let mut policy = AutoZone::new(now);
    advance(&mut policy, &host, now);
    policy.zone("queeniona".into(), "A".into(), now);
    policy.zone("queeniona".into(), "B".into(), now);
    advance(
        &mut policy,
        &host,
        now + HALF_SECOND - Duration::from_nanos(1),
    );
    assert!(host.walks.borrow().is_empty());
    advance(&mut policy, &host, now + HALF_SECOND);
    assert_eq!(host.walks.borrow().len(), 1);
    assert!((111..=272).contains(&host.walks.borrow()[0].0));
    for config in [
        active(AutoZoneMap::Ledgermayne),
        AutoZoneConfig {
            enabled: false,
            map: Some(AutoZoneMap::Queeniona),
        },
    ] {
        policy.configure(active(AutoZoneMap::Queeniona));
        policy.zone("queeniona".into(), "A".into(), now);
        policy.configure(config);
        advance(&mut policy, &host, now + HALF_SECOND);
        assert_eq!(host.walks.borrow().len(), 1);
    }
    for config in [
        active(AutoZoneMap::Moreskulls),
        AutoZoneConfig {
            enabled: false,
            map: Some(AutoZoneMap::Queeniona),
        },
    ] {
        policy.configure(active(AutoZoneMap::Queeniona));
        host.hold.set(true);
        policy.zone("queeniona".into(), "A".into(), now);
        advance(&mut policy, &host, now + HALF_SECOND);
        policy.configure(config);
        assert!(
            host.held
                .borrow_mut()
                .pop()
                .unwrap()
                .send(Ok(Value::Bool(true)))
                .is_err()
        );
        assert_eq!(advance(&mut policy, &host, now + REPLY_LIMIT), None);
    }
}

#[test]
fn queen_iona_resolves_each_charge_and_zone_after_the_delay() {
    for (charge, zone, range) in [
        (Charge::Positive, "A", Some((746..=869, 369..=379))),
        (Charge::Positive, "B", Some((111..=272, 369..=379))),
        (Charge::Negative, "A", Some((111..=272, 369..=379))),
        (Charge::Negative, "B", Some((746..=869, 369..=379))),
        (Charge::None, "A", None),
        (Charge::None, "B", None),
    ] {
        let now = Instant::now();
        let host = Host::new("queeniona");
        let mut policy = AutoZone::new(now);
        advance(&mut policy, &host, now);
        policy.zone("queeniona".into(), zone.into(), now);
        host.snapshot.borrow_mut().as_mut().unwrap().charge = charge;
        advance(&mut policy, &host, now + HALF_SECOND);
        if let Some((x, y)) = range {
            assert_eq!(host.walks.borrow().len(), 1);
            let position = host.walks.borrow()[0];
            assert!(x.contains(&position.0) && y.contains(&position.1));
        } else {
            assert!(host.walks.borrow().is_empty());
        }
    }
}

#[test]
fn dead_players_changed_maps_unknown_zones_and_disabled_configs_do_not_walk() {
    let now = Instant::now();
    let host = Host::new("ledgermayne");
    let mut policy = AutoZone::new(now);
    advance(&mut policy, &host, now);
    for (map, zone, alive) in [
        ("battleon", "A", true),
        ("ledgermayne", "C", true),
        ("ledgermayne", "A", false),
    ] {
        *host.snapshot.borrow_mut() = Ok(Snapshot {
            map: map.into(),
            alive,
            charge: Charge::None,
        });
        policy.zone("ledgermayne".into(), zone.into(), now);
        advance(&mut policy, &host, now);
    }
    policy.configure(AutoZoneConfig::default());
    policy.zone("ledgermayne".into(), "A".into(), now);
    advance(&mut policy, &host, now);
    assert!(host.walks.borrow().is_empty());
}

#[test]
fn overflow_refresh_discards_obsolete_queued_config_and_delayed_work() {
    let now = Instant::now();
    let host = Host::new("queeniona");
    let mut policy = AutoZone::new(now);
    advance(&mut policy, &host, now);
    policy.zone("queeniona".into(), "A".into(), now);
    policy.configure(active(AutoZoneMap::Ledgermayne));
    policy.refresh(now);
    advance(&mut policy, &host, now);
    policy.zone("queeniona".into(), "".into(), now);
    advance(&mut policy, &host, now + HALF_SECOND);
    assert_eq!(&*host.walks.borrow(), &[(490, 320)]);
}

#[test]
fn missing_walk_replies_expire_and_new_zones_replace_held_work() {
    let now = Instant::now();
    let host = Host::new("ledgermayne");
    host.hold.set(true);
    let mut policy = AutoZone::new(now);
    policy.zone("ledgermayne".into(), "A".into(), now);
    assert_eq!(advance(&mut policy, &host, now), Some(now + REPLY_LIMIT));
    assert_eq!(advance(&mut policy, &host, now + REPLY_LIMIT), None);
    policy.zone("ledgermayne".into(), "A".into(), now + REPLY_LIMIT);
    advance(&mut policy, &host, now + REPLY_LIMIT);
    policy.zone("ledgermayne".into(), "B".into(), now + REPLY_LIMIT);
    advance(&mut policy, &host, now + REPLY_LIMIT);
    let positions = host.walks.borrow();
    assert_eq!(positions.len(), 3);
    assert!((727..=852).contains(&positions[2].0));
    assert!(
        host.held
            .borrow_mut()
            .remove(1)
            .send(Ok(Value::Bool(true)))
            .is_err()
    );
}
