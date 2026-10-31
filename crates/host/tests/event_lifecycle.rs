use lucent_host::{
    bridge::{Error, ScriptEvent, Value},
    testing,
};
use std::time::Duration;

#[test]
fn stopped_links_drain_queued_player_events() {
    let host = testing::headless_host();
    let (mut session, link) = testing::start_probe(&host, testing::ProbeMovie::new());
    while link.try_event().unwrap().is_some() {}
    link.call_loader(
        "probe.emit",
        vec![
            Value::String("onDebug".into()),
            Value::String("queued".into()),
        ],
    )
    .recv_timeout(Duration::from_secs(5))
    .unwrap();
    session.restart(testing::headless_surface()).unwrap();
    let mut found = false;
    while let Ok(Some(event)) = link.try_event() {
        if let ScriptEvent::Loader { event, .. } = event {
            found |= event.name == "onDebug" && event.args == vec![Value::String("queued".into())];
        }
    }
    assert!(found, "queued player event was discarded");
    assert_eq!(link.try_event(), Err(Error::Stopped));
}
