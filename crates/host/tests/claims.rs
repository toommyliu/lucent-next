use lucent_host::{
    bridge::{EventInterests, Value},
    testing,
};
use std::time::Duration;

#[test]
fn ambient_claims_share_and_connection_changes_and_player_restart_release_holders() {
    let host = testing::headless_host();
    let (mut session, first) = testing::start_probe(&host, testing::ProbeMovie::new());
    let second = session.attach(EventInterests::ALL).unwrap();
    let ambient = first.acquire_consumable_claim(None).unwrap().unwrap();
    let shared = second.acquire_consumable_claim(None).unwrap().unwrap();
    assert!(ambient.first);
    assert!(!shared.first);
    assert!(
        second
            .acquire_consumable_claim(Some(100))
            .unwrap()
            .is_none()
    );
    first.release_claim(ambient.id);
    assert!(first.acquire_consumable_claim(Some(100)).unwrap().is_none());
    second.release_claim(shared.id);
    let item = first.acquire_consumable_claim(Some(100)).unwrap().unwrap();
    assert!(item.first);
    first.release_claim(ambient.id);
    assert!(
        second
            .acquire_consumable_claim(Some(200))
            .unwrap()
            .is_none()
    );
    first
        .call_loader(
            "probe.emit",
            vec![
                Value::String("onConnection".into()),
                Value::String("OnConnectionLost".into()),
            ],
        )
        .recv_timeout(Duration::from_secs(5))
        .unwrap();
    assert!(first.acquire_consumable_claim(Some(100)).is_err());
    let fresh = session.attach(EventInterests::ALL).unwrap();
    assert!(
        fresh
            .acquire_consumable_claim(Some(200))
            .unwrap()
            .unwrap()
            .first
    );
    session.restart(testing::headless_surface()).unwrap();
    assert!(fresh.acquire_consumable_claim(Some(200)).is_err());
    let replacement = session.attach(EventInterests::ALL).unwrap();
    assert!(
        replacement
            .acquire_consumable_claim(Some(100))
            .unwrap()
            .unwrap()
            .first
    );
}
