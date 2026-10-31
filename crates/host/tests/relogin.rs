use lucent_host::{automation::ReloginPolicy, bridge::Pending, testing};
use lucent_protocol::command::Command;
use serde_json::{Value, json};
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    task::{Wake, Waker},
    time::{Duration, Instant},
};

struct Notice(mpsc::Sender<()>);
impl Wake for Notice {
    fn wake(self: Arc<Self>) {
        let _ = self.0.send(());
    }
}
fn receive(
    policy: &mut ReloginPolicy,
    wake: &mpsc::Receiver<()>,
    pending: Pending<String>,
) -> Value {
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        let deadline = policy.advance(Instant::now());
        if let Some(result) = pending.try_recv() {
            return serde_json::from_str(&result.unwrap()).unwrap();
        }
        assert!(Instant::now() < until);
        let wait = deadline
            .unwrap_or(until)
            .min(until)
            .saturating_duration_since(Instant::now());
        let _ = wake.recv_timeout(wait);
    }
}
#[test]
fn native_setters_work_without_a_bot_and_retain_records_across_player_restart() {
    let host = testing::headless_host();
    let (mut session, writer) = testing::start_probe(
        &host,
        testing::ProbeMovie::new().constant("auth.login", json!(null)),
    );
    writer
        .command(Command::Login {
            username: "Hero".into(),
            password: "pw".into(),
        })
        .recv_timeout(Duration::from_secs(5))
        .unwrap();
    let (send, wake) = mpsc::channel();
    let mut policy = ReloginPolicy::new(
        session.attach(ReloginPolicy::INTERESTS).unwrap(),
        Waker::from(Arc::new(Notice(send))),
    );
    let enabled = receive(
        &mut policy,
        &wake,
        writer.command(Command::SetAutoReloginEnabled { enabled: true }),
    );
    assert_eq!(enabled["status"]["captured"], true);
    assert_eq!(enabled["status"]["username"], "Hero");
    assert_eq!(
        serde_json::to_value(session.auto_relogin()).unwrap(),
        enabled
    );
    session.restart(testing::headless_surface()).unwrap();
    assert_eq!(
        serde_json::to_value(session.auto_relogin()).unwrap(),
        enabled
    );
    assert_eq!(policy.advance(Instant::now()), None);
}
#[test]
fn publishing_native_records_does_not_wake_unrelated_script_links() {
    struct Counter(AtomicUsize);
    impl Wake for Counter {
        fn wake(self: Arc<Self>) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }
    let host = testing::headless_host();
    let (mut session, writer) = testing::start_probe(&host, testing::ProbeMovie::new());
    let observer = session
        .attach(lucent_host::bridge::EventInterests::ALL)
        .unwrap();
    let mut policy = ReloginPolicy::new(
        session.attach(ReloginPolicy::INTERESTS).unwrap(),
        Waker::noop().clone(),
    );
    session
        .read_projection(|_| ())
        .recv_timeout(Duration::from_secs(5))
        .unwrap();
    while observer.try_event().unwrap().is_some() {}
    let counter = Arc::new(Counter(AtomicUsize::new(0)));
    observer.set_waker(Waker::from(counter.clone()));
    let pending = writer.command(Command::SetAutoReloginDelay { delay_ms: 7000.0 });
    policy.advance(Instant::now());
    assert_eq!(
        serde_json::from_str::<Value>(&pending.recv_timeout(Duration::ZERO).unwrap()).unwrap()["config"]
            ["delayMs"],
        7000.0
    );
    assert_eq!(counter.0.load(Ordering::Relaxed), 0);
    assert_eq!(observer.try_event(), Ok(None));
}

#[test]
fn loader_connection_events_drive_native_relogin_without_a_bot() {
    use lucent_host::bridge::{ScriptEvent, Value as BridgeValue};
    use lucent_protocol::auto_relogin::AutoReloginPhase;
    let host = testing::headless_host();
    let (mut session, writer) = testing::start_probe(
        &host,
        testing::ProbeMovie::new()
            .constant("auth.login", json!(null))
            .recording("auth.login")
            .constant("auth.isLoggedIn", json!(false))
            .constant("world.isLoaded", json!(false))
            .constant("player.isLoaded", json!(false))
            .constant("flash.isNull", json!(false)),
    );
    writer
        .command(Command::Login {
            username: "Hero".into(),
            password: "pw".into(),
        })
        .recv_timeout(Duration::from_secs(5))
        .unwrap();
    let (send, wake) = mpsc::channel();
    let waker = Waker::from(Arc::new(Notice(send)));
    writer.set_waker(waker.clone());
    let mut policy = ReloginPolicy::new(session.attach(ReloginPolicy::INTERESTS).unwrap(), waker);
    receive(
        &mut policy,
        &wake,
        writer.command(Command::SetAutoReloginDelay { delay_ms: 0.0 }),
    );
    assert_eq!(
        receive(
            &mut policy,
            &wake,
            writer.command(Command::SetAutoReloginEnabled { enabled: true })
        )["status"]["captured"],
        true
    );
    writer
        .call_loader(
            "probe.emit",
            vec![
                BridgeValue::String("onConnection".into()),
                BridgeValue::String("OnConnectionLost".into()),
            ],
        )
        .recv_timeout(Duration::from_secs(5))
        .unwrap();
    let until = Instant::now() + Duration::from_secs(5);
    let mut logins = 0;
    loop {
        let deadline = policy.advance(Instant::now());
        while let Some(event) = writer.try_event().unwrap() {
            if matches!(event,ScriptEvent::Loader{event,..} if event.name=="onProbeCall" && event.args.first()==Some(&BridgeValue::String("auth.login".into())))
            {
                logins += 1;
            }
        }
        if logins == 2 && session.auto_relogin().status.phase == AutoReloginPhase::Armed {
            break;
        }
        assert!(
            Instant::now() < until,
            "relogin did not reach server selection"
        );
        let _ = wake.recv_timeout(
            deadline
                .unwrap_or(until)
                .min(until)
                .saturating_duration_since(Instant::now()),
        );
    }
    assert_eq!(
        session.auto_relogin().status.username.as_deref(),
        Some("Hero")
    );
}

#[test]
fn dropped_connection_events_recover_from_the_current_host_state() {
    use lucent_host::bridge::Value as BridgeValue;
    let host = testing::headless_host();
    let (mut session, writer) = testing::start_probe(
        &host,
        testing::ProbeMovie::new()
            .constant("auth.login", json!(null))
            .constant("auth.isLoggedIn", json!(false))
            .constant("world.isLoaded", json!(false))
            .constant("player.isLoaded", json!(false))
            .constant("flash.isNull", json!(true)),
    );
    writer
        .command(Command::Login {
            username: "Hero".into(),
            password: "pw".into(),
        })
        .recv_timeout(Duration::from_secs(5))
        .unwrap();
    let (send, wake) = mpsc::channel();
    let mut policy = ReloginPolicy::new(
        session.attach(ReloginPolicy::INTERESTS).unwrap(),
        Waker::from(Arc::new(Notice(send))),
    );
    receive(
        &mut policy,
        &wake,
        writer.command(Command::SetAutoReloginDelay { delay_ms: 0.0 }),
    );
    receive(
        &mut policy,
        &wake,
        writer.command(Command::SetAutoReloginEnabled { enabled: true }),
    );
    for _ in 0..1025 {
        writer
            .call_loader(
                "probe.emit",
                vec![
                    BridgeValue::String("onConnection".into()),
                    BridgeValue::String("OnConnectionLost".into()),
                ],
            )
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
    }
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        let deadline = policy.advance(Instant::now());
        if session.auto_relogin().status.attempts_remaining == Some(3.0) {
            break;
        }
        assert!(
            Instant::now() < until,
            "lost connection did not resume recovery after event overflow"
        );
        let _ = wake.recv_timeout(
            deadline
                .unwrap_or(until)
                .min(until)
                .saturating_duration_since(Instant::now()),
        );
    }
}
