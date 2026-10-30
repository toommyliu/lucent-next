use std::time::Duration;

use lucent_host::bridge::{Error, Event, ScriptEvent, Value};
use lucent_host::testing::{self, ProbeMovie};
use lucent_host::{ScriptLink, Session};

const TIMEOUT: Duration = Duration::from_secs(5);

fn emit(link: &ScriptLink, index: usize) {
    assert_eq!(
        link.call_loader(
            "probe.emit",
            vec![Value::String("onDebug".into()), Value::Number(index as f64)],
        )
        .recv_timeout(TIMEOUT),
        Ok(Value::Bool(true))
    );
}

fn event(index: usize) -> ScriptEvent {
    ScriptEvent::Loader {
        event: Event {
            name: "onDebug".into(),
            args: vec![Value::Number(index as f64)],
        },
        projected: vec![],
    }
}

fn count(session: &Session) -> usize {
    session.attached_link_count().recv_timeout(TIMEOUT).unwrap()
}

fn initial_config(link: &ScriptLink) {
    link.call_loader("probe.echo", vec![])
        .recv_timeout(TIMEOUT)
        .unwrap();
    assert_eq!(
        link.try_event(),
        Ok(Some(ScriptEvent::AutoZoneChanged {
            config: Default::default()
        }))
    );
}

#[test]
fn two_links_see_the_same_events_in_order() {
    let host = testing::headless_host();
    let (mut session, first) = testing::start_probe(&host, ProbeMovie::new());
    let second = session.attach().expect("a second link attaches");
    initial_config(&second);
    for index in 0..1_000 {
        emit(&first, index);
    }
    for index in 0..1_000 {
        assert_eq!(first.try_event(), Ok(Some(event(index))));
        assert_eq!(second.try_event(), Ok(Some(event(index))));
    }
    assert_eq!(first.try_event(), Ok(None));
    assert_eq!(second.try_event(), Ok(None));
    assert_eq!(count(&session), 2);
}

#[test]
fn a_dropped_link_is_detached() {
    let host = testing::headless_host();
    let (mut session, link) = testing::start_probe(&host, ProbeMovie::new());
    assert_eq!(count(&session), 1);
    drop(link);
    assert_eq!(count(&session), 0);
    let survivor = session.attach().expect("a new link attaches");
    initial_config(&survivor);
    for index in 0..100 {
        let transient = session.attach().expect("another link attaches");
        assert_eq!(count(&session), 2);
        drop(transient);
        assert_eq!(count(&session), 1);
        emit(&survivor, index);
        assert_eq!(survivor.try_event(), Ok(Some(event(index))));
    }
}

#[test]
fn one_links_overflow_leaves_the_other_intact() {
    let host = testing::headless_host();
    let (mut session, stalled) = testing::start_probe(&host, ProbeMovie::new());
    let draining = session.attach().expect("a second link attaches");
    initial_config(&draining);
    for index in 0..1_100 {
        emit(&draining, index);
        assert_eq!(draining.try_event(), Ok(Some(event(index))));
        assert!(!draining.overflowed());
    }
    assert!(stalled.overflowed());
    assert_eq!(stalled.try_event(), Err(Error::Overflow));
    assert_eq!(stalled.try_event(), Ok(None));
    assert_eq!(draining.try_event(), Ok(None));
    emit(&draining, 1_100);
    assert_eq!(stalled.try_event(), Ok(Some(event(1_100))));
    assert_eq!(draining.try_event(), Ok(Some(event(1_100))));
}

#[test]
fn replies_belong_to_their_calling_link() {
    let host = testing::headless_host();
    let (mut session, first) = testing::start_probe(&host, ProbeMovie::new());
    let second = session.attach().expect("a second link attaches");
    let abandoned = first.call_loader("probe.echo", vec![Value::String("abandoned".into())]);
    drop(first);
    drop(abandoned);
    assert_eq!(
        second
            .call_loader("probe.echo", vec![Value::String("surviving".into())])
            .recv_timeout(TIMEOUT),
        Ok(Value::String("surviving".into()))
    );
    let abandoned = second.command(lucent_protocol::command::Command::Logout {});
    drop(second);
    drop(abandoned);
    let third = session.attach().expect("a new link attaches");
    assert_eq!(
        third
            .command(lucent_protocol::command::Command::RefreshLoggedIn {})
            .recv_timeout(TIMEOUT),
        Err(Error::Unavailable("auth.isLoggedIn".into()))
    );
    assert_eq!(
        third
            .call_loader("probe.echo", vec![Value::Number(42.0)])
            .recv_timeout(TIMEOUT),
        Ok(Value::Number(42.0))
    );
}

#[test]
fn new_links_receive_config_then_events_after_attach() {
    let host = testing::headless_host();
    let (mut session, first) = testing::start_probe(&host, ProbeMovie::new());
    emit(&first, 0);
    let second = session.attach().unwrap();
    initial_config(&second);
    emit(&first, 1);
    assert_eq!(first.try_event(), Ok(Some(event(0))));
    assert_eq!(first.try_event(), Ok(Some(event(1))));
    assert_eq!(second.try_event(), Ok(Some(event(1))));
    assert_eq!(second.try_event(), Ok(None));
    drop(first);
    drop(second);
    assert_eq!(count(&session), 0);
    let third = session.attach().unwrap();
    initial_config(&third);
    emit(&third, 2);
    assert_eq!(third.try_event(), Ok(Some(event(2))));
    assert_eq!(third.try_event(), Ok(None));
}
