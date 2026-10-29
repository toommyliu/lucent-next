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
    assert_eq!(other.try_event(), Ok(None));
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
    assert_eq!(replacement.try_event(), Ok(None));
}

#[test]
fn an_ask_times_out_after_its_deadline() {
    let host = testing::headless_host();
    let (mut session, asker) = testing::start_probe(&host, testing::ProbeMovie::new());
    let bot = session.attach().unwrap();
    bot.claim_bot().unwrap();
    let started = Instant::now();
    let id = asker.ask(json!(7)).unwrap();
    assert_eq!(
        next(&asker),
        ScriptEvent::Answer {
            id,
            answer: Err(Error::Timeout)
        }
    );
    assert!(started.elapsed() >= Duration::from_secs(5));
    assert_eq!(session.pending_ask_count(), 0);
    bot.answer(id, Ok(json!(8))).unwrap();
    assert_eq!(asker.try_event(), Ok(None));
}
