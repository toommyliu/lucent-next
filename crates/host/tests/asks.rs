use lucent_host::{
    ScriptLink,
    bridge::{Error, ScriptEvent},
    testing,
};
use serde_json::json;
use std::time::{Duration, Instant};

fn next(link: &ScriptLink) -> ScriptEvent {
    let deadline = Instant::now() + Duration::from_secs(7);
    loop {
        if let Some(event) = link.try_event().unwrap() {
            if matches!(event, ScriptEvent::AutoZoneChanged { .. }) {
                continue;
            }
            return event;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn an_answer_reaches_only_its_asker() {
    let host = testing::headless_host();
    let (mut session, asker) = testing::start_probe(&host, testing::ProbeMovie::new());
    let other = session.attach().unwrap();
    let bot = session.attach().unwrap();
    bot.claim_bot().unwrap();
    let id = asker.ask(json!({"value":7})).unwrap();
    assert!(
        matches!(next(&bot), ScriptEvent::Ask { request } if request.id == id && request.payload == json!({"value":7}))
    );
    assert!(other.answer(id, Ok(json!(99))).is_err());
    bot.answer(id, Ok(json!(8))).unwrap();
    assert_eq!(
        next(&asker),
        ScriptEvent::Answer {
            id,
            answer: Ok(json!(8))
        }
    );
    assert!(!matches!(
        other.try_event(),
        Ok(Some(ScriptEvent::Answer { .. }))
    ));
    assert_eq!(session.pending_ask_count(), 0);
}

#[test]
fn a_detached_askers_routes_are_dropped() {
    let host = testing::headless_host();
    let (mut session, asker) = testing::start_probe(&host, testing::ProbeMovie::new());
    let bot = session.attach().unwrap();
    bot.claim_bot().unwrap();
    let id = asker.ask(json!(7)).unwrap();
    assert!(matches!(next(&bot), ScriptEvent::Ask { .. }));
    drop(asker);
    assert_eq!(session.pending_ask_count(), 0);
    let replacement = session.attach().unwrap();
    bot.answer(id, Ok(json!(8))).unwrap();
    assert!(!matches!(
        replacement.try_event(),
        Ok(Some(ScriptEvent::Answer { .. }))
    ));
}

#[test]
fn an_ask_times_out_after_its_deadline() {
    let host = testing::headless_host();
    let (mut session, asker) = testing::start_probe(&host, testing::ProbeMovie::new());
    let bot = session.attach().unwrap();
    bot.claim_bot().unwrap();
    let started = Instant::now();
    let id = asker.ask(json!(7)).unwrap();
    std::thread::sleep(Duration::from_millis(4800));
    assert_eq!(asker.try_event(), Ok(None));
    assert_eq!(session.pending_ask_count(), 1);
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(session.pending_ask_count(), 0);
    let ScriptEvent::Answer {
        id: answered,
        answer: Err(error),
    } = next(&asker)
    else {
        panic!("expected timeout");
    };
    assert_eq!(answered, id);
    assert_eq!(error.to_string(), "The bot request timed out");
    assert!(started.elapsed() >= Duration::from_secs(5));
    assert_eq!(session.pending_ask_count(), 0);
    bot.answer(id, Ok(json!(8))).unwrap();
    assert_eq!(asker.try_event(), Ok(None));
}

#[test]
fn no_bot_rejects_without_leaving_a_route() {
    let host = testing::headless_host();
    let (session, asker) = testing::start_probe(&host, testing::ProbeMovie::new());
    assert_eq!(
        asker.ask(json!(7)).unwrap_err().to_string(),
        "No bot is attached"
    );
    assert_eq!(session.pending_ask_count(), 0);
}

#[test]
fn cancellation_keeps_bot_work_and_drops_only_the_callers_route() {
    let host = testing::headless_host();
    let (mut session, asker) = testing::start_probe(&host, testing::ProbeMovie::new());
    let other = session.attach().unwrap();
    let bot = session.attach().unwrap();
    bot.claim_bot().unwrap();
    let id = asker.ask(json!(7)).unwrap();
    other.cancel_ask(id);
    assert_eq!(session.pending_ask_count(), 1);
    asker.cancel_ask(id);
    assert_eq!(session.pending_ask_count(), 0);
    assert!(matches!(next(&bot), ScriptEvent::Ask { request } if request.id == id));
    bot.answer(id, Ok(json!(8))).unwrap();
    assert_eq!(asker.try_event(), Ok(None));
}

#[test]
fn bot_detach_fails_its_asks_and_replacement_cannot_answer_them() {
    let host = testing::headless_host();
    let (mut session, asker) = testing::start_probe(&host, testing::ProbeMovie::new());
    let bot = session.attach().unwrap();
    bot.claim_bot().unwrap();
    let id = asker.ask(json!(7)).unwrap();
    drop(bot);
    let ScriptEvent::Answer {
        answer: Err(error), ..
    } = next(&asker)
    else {
        panic!("expected bot failure");
    };
    assert_eq!(error.to_string(), "The bot has stopped");
    assert_eq!(session.pending_ask_count(), 0);
    let replacement = session.attach().unwrap();
    replacement.claim_bot().unwrap();
    replacement.answer(id, Ok(json!(8))).unwrap();
    assert_eq!(asker.try_event(), Ok(None));
}

#[test]
fn restart_stops_old_routes_and_preserves_only_the_config() {
    use lucent_protocol::auto_relogin::*;
    let host = testing::headless_host();
    let (mut session, asker) = testing::start_probe(&host, testing::ProbeMovie::new());
    let bot = session.attach().unwrap();
    bot.claim_bot().unwrap();
    let records = AutoReloginRecords {
        config: AutoReloginConfig {
            enabled: true,
            delay_ms: 7000.0,
            server: Some("Artix".into()),
        },
        status: AutoReloginStatus {
            captured: true,
            ..Default::default()
        },
    };
    assert!(asker.publish_auto_relogin(records.clone()).is_err());
    bot.publish_auto_relogin(records.clone()).unwrap();
    let id = asker.ask(json!(7)).unwrap();
    session.restart(testing::headless_surface()).unwrap();
    assert_eq!(asker.try_event(), Err(Error::Stopped));
    assert_eq!(session.pending_ask_count(), 0);
    assert_eq!(session.auto_relogin().config, records.config);
    assert!(!session.auto_relogin().status.captured);
    assert_eq!(bot.answer(id, Ok(json!(8))), Err(Error::Stopped));
    assert!(bot.publish_auto_relogin(records).is_err());
}

#[test]
fn asks_remain_available_when_the_bot_player_events_overflow() {
    let host = testing::headless_host();
    let (mut session, asker) = testing::start_probe(&host, testing::ProbeMovie::new());
    let bot = session.attach().unwrap();
    bot.claim_bot().unwrap();
    let id = asker.ask(json!(7)).unwrap();
    for n in 0..1100 {
        asker
            .call_loader(
                "probe.emit",
                vec![
                    lucent_host::bridge::Value::String("onDebug".into()),
                    lucent_host::bridge::Value::Number(n as f64),
                ],
            )
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
    }
    assert!(bot.overflowed());
    assert_eq!(asker.try_event(), Err(Error::Overflow));
    assert!(matches!(next(&bot), ScriptEvent::Ask { request } if request.id == id));
    assert_eq!(bot.try_event(), Err(Error::Overflow));
    bot.answer(id, Ok(json!(8))).unwrap();
    assert_eq!(
        next(&asker),
        ScriptEvent::Answer {
            id,
            answer: Ok(json!(8))
        }
    );
    assert_eq!(session.pending_ask_count(), 0);
}

#[test]
fn a_blocked_player_does_not_block_an_ask() {
    let host = testing::headless_host();
    let (mut session, asker) = testing::start_probe(&host, testing::ProbeMovie::new());
    let bot = session.attach().unwrap();
    bot.claim_bot().unwrap();
    let (entered, wait_entered) = std::sync::mpsc::channel();
    let (release, wait_release) = std::sync::mpsc::channel();
    let blocked = session.read_projection(move |_| {
        entered.send(()).unwrap();
        let _ = wait_release.recv();
    });
    wait_entered.recv_timeout(Duration::from_secs(5)).unwrap();
    let id = asker.ask(json!(7)).unwrap();
    assert!(matches!(next(&bot), ScriptEvent::Ask { request } if request.id == id));
    bot.answer(id, Ok(json!(8))).unwrap();
    assert_eq!(
        next(&asker),
        ScriptEvent::Answer {
            id,
            answer: Ok(json!(8))
        }
    );
    release.send(()).unwrap();
    blocked.recv_timeout(Duration::from_secs(5)).unwrap();
}

#[test]
fn the_router_rejects_a_full_bot_mailbox_without_leaving_a_route() {
    let host = testing::headless_host();
    let (mut session, asker) = testing::start_probe(&host, testing::ProbeMovie::new());
    let bot = session.attach().unwrap();
    bot.claim_bot().unwrap();
    for n in 0..1024 {
        asker.ask(json!(n)).unwrap();
    }
    assert_eq!(
        asker.ask(json!(1024)).unwrap_err().to_string(),
        "The bot request queue is full"
    );
    assert_eq!(session.pending_ask_count(), 1024);
    drop(asker);
    assert_eq!(session.pending_ask_count(), 0);
}

#[test]
fn only_the_current_bot_can_start_an_unexpired_ask() {
    let host = testing::headless_host();
    let (mut session, asker) = testing::start_probe(&host, testing::ProbeMovie::new());
    let bot = session.attach().unwrap();
    bot.claim_bot().unwrap();
    let other = session.attach().unwrap();
    let id = asker.ask(json!(7)).unwrap();
    assert!(bot.is_ask_pending(id));
    assert!(!other.is_ask_pending(id));
    asker.cancel_ask(id);
    assert!(!bot.is_ask_pending(id));
    let id = asker.ask(json!(8)).unwrap();
    std::thread::sleep(Duration::from_millis(5100));
    assert!(!bot.is_ask_pending(id));
    assert_eq!(session.pending_ask_count(), 0);
}

#[test]
fn publishing_records_does_not_wake_script_links() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use std::task::{Wake, Waker};
    struct Wakes(AtomicUsize);
    impl Wake for Wakes {
        fn wake(self: Arc<Self>) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }
    let host = testing::headless_host();
    let (mut session, script) = testing::start_probe(&host, testing::ProbeMovie::new());
    let bot = session.attach().unwrap();
    bot.claim_bot().unwrap();
    while script.try_event().unwrap().is_some() {}
    let wakes = Arc::new(Wakes(AtomicUsize::new(0)));
    script.set_waker(Waker::from(wakes.clone()));
    wakes.0.store(0, Ordering::Relaxed);
    bot.publish_auto_relogin(Default::default()).unwrap();
    assert_eq!(wakes.0.load(Ordering::Relaxed), 0);
    assert_eq!(script.try_event(), Ok(None));
}

#[test]
fn router_errors_describe_bot_authority() {
    let host = testing::headless_host();
    let (mut session, script) = testing::start_probe(&host, testing::ProbeMovie::new());
    let bot = session.attach().unwrap();
    bot.claim_bot().unwrap();
    assert_eq!(
        script.claim_bot().unwrap_err().to_string(),
        "A bot is already attached"
    );
    assert_eq!(
        script.answer(1, Ok(json!(null))).unwrap_err().to_string(),
        "Only the attached bot can answer requests"
    );
    assert_eq!(
        script
            .publish_auto_relogin(Default::default())
            .unwrap_err()
            .to_string(),
        "Only the attached bot can publish auto relogin records"
    );
}

#[test]
fn stopped_links_drain_queued_answers_and_player_events() {
    let host = testing::headless_host();
    let (mut session, script) = testing::start_probe(&host, testing::ProbeMovie::new());
    let bot = session.attach().unwrap();
    bot.claim_bot().unwrap();
    while script.try_event().unwrap().is_some() {}
    let id = script.ask(json!(1)).unwrap();
    bot.answer(id, Ok(json!(2))).unwrap();
    script
        .call_loader(
            "probe.emit",
            vec![
                lucent_host::bridge::Value::String("onDebug".into()),
                lucent_host::bridge::Value::String("queued".into()),
            ],
        )
        .recv_timeout(Duration::from_secs(5))
        .unwrap();
    session.restart(testing::headless_surface()).unwrap();
    assert_eq!(
        script.try_event(),
        Ok(Some(ScriptEvent::Answer {
            id,
            answer: Ok(json!(2))
        }))
    );
    let mut found = false;
    while let Ok(Some(event)) = script.try_event() {
        if let ScriptEvent::Loader { event, .. } = event {
            found |= event.name == "onDebug";
        }
    }
    assert!(found, "queued player event was discarded");
    assert_eq!(script.try_event(), Err(Error::Stopped));
}

#[test]
fn queued_answers_arrive_before_player_event_loss() {
    let host = testing::headless_host();
    let (mut session, asker) = testing::start_probe(&host, testing::ProbeMovie::new());
    let bot = session.attach().unwrap();
    bot.claim_bot().unwrap();
    let ids = [asker.ask(json!(7)).unwrap(), asker.ask(json!(8)).unwrap()];
    for id in ids {
        assert!(matches!(next(&bot), ScriptEvent::Ask { request } if request.id == id));
        bot.answer(id, Ok(json!(id))).unwrap();
    }
    for n in 0..1100 {
        asker
            .call_loader(
                "probe.emit",
                vec![
                    lucent_host::bridge::Value::String("onDebug".into()),
                    lucent_host::bridge::Value::Number(n as f64),
                ],
            )
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
    }
    assert!(asker.overflowed());
    for id in ids {
        assert_eq!(
            asker.try_event(),
            Ok(Some(ScriptEvent::Answer {
                id,
                answer: Ok(json!(id))
            }))
        );
    }
    assert_eq!(asker.try_event(), Err(Error::Overflow));
    assert_eq!(asker.try_event(), Ok(None));
    assert_eq!(session.pending_ask_count(), 0);
}
