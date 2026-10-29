use lucent_host::{
    ScriptLink,
    bridge::{ScriptEvent, Value},
    testing::{self, ProbeMovie},
};
use lucent_protocol::{
    item::ItemContainer,
    operation::{Operation, OperationAnswer},
    query::ItemSelector,
};
use serde_json::json;
use std::time::{Duration, Instant};
const WAIT: Duration = Duration::from_secs(15);

fn emit(link: &ScriptLink, name: &str, payload: String) {
    link.call_loader(
        "probe.emit",
        vec![Value::String(name.into()), Value::String(payload)],
    )
    .recv_timeout(WAIT)
    .unwrap();
}
fn drop_item(link: &ScriptLink, category: &str, house: bool) {
    emit(
        link,
        "onExtensionResponse",
        json!({"type":"json","dataObj":{"cmd":"dropItem","items":{"70":{
            "ItemID":70,"sName":"Dropped Item","sType":category,"bHouse":house,"iQty":2
        }}}})
        .to_string(),
    );
}
fn next(link: &ScriptLink) -> ScriptEvent {
    struct Wake(std::thread::Thread);
    impl std::task::Wake for Wake {
        fn wake(self: std::sync::Arc<Self>) {
            self.0.unpark();
        }
    }
    link.set_waker(std::task::Waker::from(std::sync::Arc::new(Wake(
        std::thread::current(),
    ))));
    let deadline = Instant::now() + WAIT;
    loop {
        match link.try_event().unwrap() {
            Some(event) => return event,
            None => {
                let remaining = deadline.saturating_duration_since(Instant::now());
                assert!(!remaining.is_zero(), "no event within {WAIT:?}");
                std::thread::park_timeout(remaining);
            }
        }
    }
}
fn operation_result(link: &ScriptLink, id: u64) -> Result<bool, lucent_host::bridge::Error> {
    let deadline = Instant::now() + WAIT;
    loop {
        if let Some(ScriptEvent::Operation {
            id: current,
            answer,
        }) = link.try_event()?
        {
            assert_eq!(current, id);
            return answer.map(|OperationAnswer::AcceptDrop(value)| value);
        }
        assert!(Instant::now() < deadline, "accept {id} did not answer");
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn answer(link: &ScriptLink, id: u64) -> bool {
    operation_result(link, id).unwrap()
}
fn accept(link: &ScriptLink) -> u64 {
    link.operate(Operation::AcceptDrop {
        selector: ItemSelector::Name("dropped item".into()),
    })
    .unwrap()
}
fn packet(data: serde_json::Value) -> String {
    json!({"type":"json","dataObj":data}).to_string()
}

#[test]
fn accepts_run_one_at_a_time() {
    let host = testing::headless_host();
    let (_session, link) = testing::start_probe(
        &host,
        ProbeMovie::new()
            .constant("auth.isLoggedIn", json!(true))
            .accepting_drop(
                Duration::from_millis(100),
                packet(json!({"cmd":"getDrop","ItemID":70,"bSuccess":false})),
            ),
    );
    drop_item(&link, "Item", false);
    let first = accept(&link);
    let second = accept(&link);
    let mut order = vec![];
    let mut answers = vec![];
    while answers.len() < 2 {
        match next(&link) {
            ScriptEvent::Loader { event, .. } if event.name == "onProbeAccept" => {
                assert_eq!(event.args, vec![Value::Number(70.0)]);
                order.push("accept");
            }
            ScriptEvent::Loader { event, .. }
                if event.name == "onExtensionResponse"
                    && event.args[0].clone()
                        == Value::String(packet(
                            json!({"cmd":"getDrop","ItemID":70,"bSuccess":false}),
                        )) =>
            {
                order.push("packet")
            }
            ScriptEvent::Operation { id, answer } => {
                assert_eq!(answer.unwrap(), OperationAnswer::AcceptDrop(false));
                answers.push(id);
            }
            _ => {}
        }
    }
    assert_eq!(answers, [first, second]);
    assert_eq!(order, ["accept", "packet", "accept", "packet"]);
}

#[test]
fn a_missing_packet_times_out_and_frees_the_resource() {
    let host = testing::headless_host();
    let (_session, link) = testing::start_probe(
        &host,
        ProbeMovie::new()
            .constant("auth.isLoggedIn", json!(true))
            .constant("drops.accept", json!(true)),
    );
    drop_item(&link, "Item", false);
    let at = Instant::now();
    let first = accept(&link);
    let second = link
        .operate(Operation::AcceptDrop {
            selector: ItemSelector::Id(999.0),
        })
        .unwrap();
    assert!(!answer(&link, first));
    assert!(at.elapsed() >= Duration::from_secs(10));
    assert!(at.elapsed() < Duration::from_secs(13));
    assert!(!answer(&link, second));
}

#[test]
fn accepts_choose_the_container_and_verify_the_projected_quantity() {
    for (category, house_item, flags, container) in [
        ("Item", false, json!({"bBank":"1"}), ItemContainer::Bank),
        ("Class", false, json!({"bBank":1}), ItemContainer::Inventory),
        ("Item", false, json!({"bHouse":true}), ItemContainer::House),
        ("Item", true, json!({}), ItemContainer::House),
        ("Item", false, json!({}), ItemContainer::Inventory),
    ] {
        let mut response = json!({"cmd":"getDrop","ItemID":"70.9","bSuccess":"true","iQty":"2"});
        response
            .as_object_mut()
            .unwrap()
            .extend(flags.as_object().unwrap().clone());
        let host = testing::headless_host();
        let (_session, link) = testing::start_probe(
            &host,
            ProbeMovie::new()
                .constant("auth.isLoggedIn", json!(true))
                .accepting_drop(Duration::ZERO, packet(response)),
        );
        drop_item(&link, category, house_item);
        let id = accept(&link);
        assert!(answer(&link, id), "{category}, {container:?}");
        assert_eq!(
            link.read(|state| state
                .items
                .get(container, 70)
                .map(|item| item.data().quantity))
                .unwrap(),
            Some(2.0)
        );
        assert_eq!(
            link.read(|state| state
                .items
                .get(ItemContainer::Drop, 70)
                .map(|item| item.data().quantity))
                .unwrap(),
            None
        );
    }
}

#[test]
fn a_disconnect_and_an_unavailable_loader_free_the_resource() {
    let host = testing::headless_host();
    let (_session, link) = testing::start_probe(
        &host,
        ProbeMovie::new()
            .constant("auth.isLoggedIn", json!(true))
            .accepting_drop(Duration::ZERO, packet(json!({"cmd":"unrelated"}))),
    );
    drop_item(&link, "Item", false);
    let first = accept(&link);
    while !matches!(next(&link), ScriptEvent::Loader { event, .. } if event.name == "onProbeAccept")
    {
    }
    let pending_until = Instant::now() + Duration::from_millis(100);
    while Instant::now() < pending_until {
        assert!(
            !matches!(
                link.try_event().unwrap(),
                Some(ScriptEvent::Operation { .. })
            ),
            "accept ended before the disconnect"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
    let disconnected = Instant::now();
    emit(&link, "onConnection", "OnConnectionLost".into());
    let second = accept(&link);
    assert!(!answer(&link, first));
    assert!(disconnected.elapsed() < Duration::from_secs(1));
    assert!(!answer(&link, second));
    let (_session, link) = testing::start_probe(
        &host,
        ProbeMovie::new().constant("auth.isLoggedIn", json!(true)),
    );
    drop_item(&link, "Item", false);
    let first = accept(&link);
    let second = accept(&link);
    assert!(!answer(&link, first));
    assert!(!answer(&link, second));
}

#[test]
fn a_player_restart_ends_its_accept_and_the_session_worker_accepts_on_the_new_player() {
    let host = testing::headless_host();
    let (mut session, old) = testing::start_probe(
        &host,
        ProbeMovie::new()
            .constant("auth.isLoggedIn", json!(true))
            .accepting_drop(
                Duration::from_millis(100),
                packet(json!({"cmd":"getDrop","ItemID":70,"bSuccess":true,"iQtyNow":2})),
            ),
    );
    drop_item(&old, "Item", false);
    let old_id = accept(&old);
    while !matches!(next(&old), ScriptEvent::Loader { event, .. } if event.name == "onProbeAccept")
    {
    }
    let stopped = Instant::now();
    session.restart(testing::headless_surface()).unwrap();
    assert_eq!(
        operation_result(&old, old_id),
        Err(lucent_host::bridge::Error::Stopped)
    );
    assert!(stopped.elapsed() < Duration::from_secs(1));
    let current = session.take_script_link().unwrap();
    session.send(lucent_host::ViewEvent::Resized(
        lucent_host::Viewport::new(100.0, 100.0, 1.0).unwrap(),
    ));
    while !matches!(next(&current),ScriptEvent::Loader{event,..} if event.name=="onLoaded") {}
    drop_item(&current, "Item", false);
    let id = accept(&current);
    assert!(answer(&current, id));
}

#[test]
fn malformed_and_unrelated_responses_do_not_end_an_accept() {
    let host = testing::headless_host();
    let (_session, link) = testing::start_probe(
        &host,
        ProbeMovie::new()
            .constant("auth.isLoggedIn", json!(true))
            .accepting_drop(
                Duration::ZERO,
                packet(json!({"cmd":"getDrop","ItemID":70,"bSuccess":"invalid"})),
            ),
    );
    drop_item(&link, "Item", false);
    let id = accept(&link);
    while !matches!(next(&link),ScriptEvent::Loader{event,..} if event.name=="onProbeAccept") {}
    emit(
        &link,
        "onExtensionResponse",
        packet(json!({"cmd":"getDrop","ItemID":99,"bSuccess":true})),
    );
    emit(
        &link,
        "onExtensionResponse",
        packet(json!({"cmd":"getDrop","ItemID":70,"bSuccess":true,"iQtyNow":4})),
    );
    assert!(answer(&link, id));
    assert_eq!(
        link.read(|state| state
            .items
            .get(ItemContainer::Inventory, 70)
            .map(|item| item.data().quantity))
            .unwrap(),
        Some(4.0)
    );
}
