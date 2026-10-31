mod common;

use common::TIMEOUT;
use lucent_host::bridge::EventInterests;
use lucent_host::{
    ScriptLink,
    bridge::{ScriptEvent, Value},
    testing,
};
use lucent_protocol::command::Command;
use lucent_runtime::{Kill, RuntimeError, Supervisor};
use serde_json::json;
use std::{
    sync::Arc,
    task::{Wake, Waker},
    thread,
    time::{Duration, Instant},
};

struct Unpark(thread::Thread);
impl Wake for Unpark {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
}
fn drain(link: &ScriptLink) {
    while link.try_event().unwrap().is_some() {}
}
fn patch(link: &ScriptLink, patch: serde_json::Value) {
    link.command(
        serde_json::from_value::<Command>(json!({"kind":"patchAutoZone","patch":patch})).unwrap(),
    )
    .recv_timeout(TIMEOUT)
    .unwrap();
}
fn call_arguments(mut args: Vec<Value>) -> Vec<Value> {
    if let Some(Value::List(values)) = args.get(1) {
        let mut all = vec![args[0].clone()];
        all.extend(values.clone());
        args = all;
    }
    args
}
fn next_call(link: &ScriptLink, method: &str) -> Vec<Value> {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        assert!(Instant::now() < deadline, "did not receive {method}");
        let event = common::await_event(link, "onProbeCall", None);
        if event.args.first() == Some(&Value::String(method.into())) {
            return call_arguments(event.args);
        }
    }
}
fn walks_for(link: &ScriptLink, duration: Duration) -> usize {
    link.set_waker(Waker::from(Arc::new(Unpark(thread::current()))));
    let until = Instant::now() + duration;
    let mut walks = 0;
    loop {
        while let Some(event) = link.try_event().unwrap() {
            if let ScriptEvent::Loader { event, .. } = event
                && event.name == "onProbeCall"
                && event.args.first() == Some(&Value::String("player.walkTo".into()))
            {
                walks += 1;
            }
        }
        let remaining = until.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return walks;
        }
        thread::park_timeout(remaining);
    }
}
fn assert_position(call: &[Value], x: (f64, f64), y: (f64, f64)) {
    assert!(
        matches!(call.get(1), Some(Value::Number(n)) if (x.0..=x.1).contains(n)),
        "{call:?}"
    );
    assert!(
        matches!(call.get(2), Some(Value::Number(n)) if (y.0..=y.1).contains(n)),
        "{call:?}"
    );
}
fn packet(link: &ScriptLink, data: serde_json::Value) {
    link.call_loader(
        "probe.emit",
        vec![
            Value::String("onExtensionResponse".into()),
            Value::String(json!({"type":"json","dataObj":data}).to_string()),
        ],
    )
    .recv_timeout(TIMEOUT)
    .unwrap();
}

fn enter_map(link: &ScriptLink, map: &str) {
    packet(
        link,
        json!({
            "cmd":"moveToArea","areaName":format!("{map}-1"),
            "uoBranch":[{"entID":7,"uoName":"player-a","intHP":100}],
        }),
    );
}

#[test]
fn every_boss_map_moves_for_each_supported_zone() {
    let host = testing::headless_host();
    let (mut session, observer) = testing::start_probe(
        &host,
        common::ready_movie().responding("player.walkTo", [(json!(true), vec![])]),
    );
    common::log_in(&observer);
    let _supervisor = Supervisor::start(&mut session, None).unwrap();
    for (map, ranges) in [
        (
            "ledgermayne",
            [
                ((431., 547.), (234., 239.)),
                ((147., 276.), (353., 357.)),
                ((727., 852.), (353., 356.)),
            ],
        ),
        (
            "moreskulls",
            [
                ((778., 806.), (358., 361.)),
                ((696., 802.), (445., 452.)),
                ((677., 766.), (321., 324.)),
            ],
        ),
        (
            "ultradage",
            [
                ((481., 483.), (296., 300.)),
                ((49., 164.), (406., 412.)),
                ((797., 900.), (400., 402.)),
            ],
        ),
        (
            "darkcarnax",
            [
                ((480., 530.), (419., 432.)),
                ((731., 850.), (431., 432.)),
                ((54., 155.), (431., 432.)),
            ],
        ),
        (
            "astralshrine",
            [
                ((461., 465.), (320., 325.)),
                ((643., 708.), (445., 447.)),
                ((199., 287.), (181., 205.)),
            ],
        ),
        (
            "magnumopus",
            [
                ((466., 470.), (344., 420.)),
                ((682., 813.), (367., 384.)),
                ((170., 285.), (377., 384.)),
            ],
        ),
    ] {
        enter_map(&observer, map);
        patch(&observer, json!({"enabled":true,"map":map}));
        for (zone, (x, y)) in ["", "A", "B"].into_iter().zip(ranges) {
            packet(&observer, json!({"cmd":"event","args":{"zoneSet":zone}}));
            assert_position(&next_call(&observer, "player.walkTo"), x, y);
        }
    }
    enter_map(&observer, "queeniona");
    patch(&observer, json!({"enabled":true,"map":"queeniona"}));
    packet(&observer, json!({"cmd":"event","args":{"zoneSet":""}}));
    assert_position(
        &next_call(&observer, "player.walkTo"),
        (490., 490.),
        (320., 320.),
    );
}

#[test]
fn the_zone_policy_runs_without_a_bot() {
    let host = testing::headless_host();
    let (mut session, observer) = testing::start_probe(
        &host,
        common::ready_movie().responding("player.walkTo", [(json!(true), vec![])]),
    );
    common::log_in(&observer);
    enter_map(&observer, "ledgermayne");
    patch(&observer, json!({"enabled":true,"map":"ledgermayne"}));
    let link = session
        .attach(lucent_host::automation::AutoZonePolicy::INTERESTS)
        .unwrap();
    let mut policy = lucent_host::automation::AutoZonePolicy::new(
        link,
        Waker::from(Arc::new(Unpark(thread::current()))),
    );
    packet(&observer, json!({"cmd":"event","args":{"zoneSet":"A"}}));
    let until = Instant::now() + TIMEOUT;
    loop {
        let next = policy.advance(Instant::now());
        while let Some(event) = observer.try_event().unwrap() {
            if let ScriptEvent::Loader { event, .. } = event
                && event.name == "onProbeCall"
                && event.args.first() == Some(&Value::String("player.walkTo".into()))
            {
                assert_position(&call_arguments(event.args), (147., 276.), (353., 357.));
                return;
            }
        }
        assert!(Instant::now() < until);
        thread::park_timeout(
            next.unwrap_or(until)
                .min(until)
                .saturating_duration_since(Instant::now()),
        );
    }
}
#[test]
fn auto_zone_follows_script_toggles_through_kills() {
    let host = testing::headless_host();
    let (mut session, observer) = testing::start_probe(
        &host,
        common::ready_movie().responding("player.walkTo", [(json!(true), vec![])]),
    );
    common::log_in(&observer);
    enter_map(&observer, "ledgermayne");
    let _supervisor = Supervisor::start(&mut session, None).unwrap();
    let run = common::started_run(
        &mut session,
        r#"
        import * as autozone from "lucent/autozone";
        import * as api from "lucent/api";
        export default function* () {
            yield* autozone.setMap("ledgermayne");
            const state = yield* autozone.setEnabled(true);
            if (!state.enabled || state.map !== "ledgermayne") throw new Error("wrong toggle reply");
            yield* api.packet.sendToClient("started");
            while (true) {}
        }
    "#,
    );
    assert!(matches!(
        run.wait(TIMEOUT),
        Err(RuntimeError::Killed(Kill::Watchdog))
    ));
    common::start(
        &mut session,
        common::loose(&[(
            "read.js",
            r#"
        import * as autozone from "lucent/autozone";
        export default function* () {
            const state = yield* autozone.getState();
            if (!state.enabled || state.map !== "ledgermayne") throw new Error("lost auto zone");
        }
    "#,
        )]),
        Default::default(),
    )
    .wait(TIMEOUT)
    .map(|outcome| {
        assert_eq!(outcome, lucent_protocol::script::ScriptOutcome::Completed);
    })
    .unwrap();
    drain(&observer);
    packet(&observer, json!({"cmd":"event","args":{"zoneSet":"A"}}));
    let call = next_call(&observer, "player.walkTo");
    assert!(matches!(call.get(1), Some(Value::Number(x)) if (147.0..=276.0).contains(x)));
}

#[test]
fn native_auto_zone_reads_saved_config_and_scripts_can_clear_it() {
    let host = testing::headless_host();
    let (mut session, observer) = testing::start_probe(
        &host,
        common::ready_movie().responding("player.walkTo", [(json!(true), vec![])]),
    );
    common::log_in(&observer);
    let run = common::start(
        &mut session,
        common::loose(&[(
            "config.js",
            r#"
        import * as autozone from "lucent/autozone";
        export default function* () {
            const mapped = yield* autozone.setMap("ledgermayne");
            const enabled = yield* autozone.setEnabled(true);
            const state = yield* autozone.getState();
            if (!state.enabled || state.map !== "ledgermayne") throw new Error("stale auto zone read");
            if (mapped === enabled || enabled === state) throw new Error("shared auto zone result");
            state.enabled = false;
            if (!(yield* autozone.isEnabled())) throw new Error("auto zone result changed the host");
        }
    "#,
        )]),
        Default::default(),
    );
    assert_eq!(
        run.wait(TIMEOUT).unwrap(),
        lucent_protocol::script::ScriptOutcome::Completed
    );
    let _supervisor = Supervisor::start(&mut session, None).unwrap();
    enter_map(&observer, "ledgermayne");
    drain(&observer);
    packet(&observer, json!({"cmd":"event","args":{"zoneSet":"B"}}));
    let call = next_call(&observer, "player.walkTo");
    assert!(matches!(call.get(1), Some(Value::Number(x)) if (727.0..=852.0).contains(x)));
    let run = common::start(
        &mut session,
        common::loose(&[(
            "clear.js",
            r#"
        import * as autozone from "lucent/autozone";
        export default function* () {
            const state = yield* autozone.setMap(undefined);
            if (!state.enabled || state.map !== undefined || (yield* autozone.getMap()) !== undefined) throw new Error("map not cleared");
        }
    "#,
        )]),
        Default::default(),
    );
    assert_eq!(
        run.wait(TIMEOUT).unwrap(),
        lucent_protocol::script::ScriptOutcome::Completed
    );
    drain(&observer);
    packet(&observer, json!({"cmd":"event","args":{"zoneSet":"A"}}));
    assert_eq!(
        walks_for(&observer, Duration::from_millis(1200)),
        0,
        "the cleared map still moved the player"
    );
}

#[test]
fn native_auto_zone_moves_once_while_a_script_is_running() {
    let host = testing::headless_host();
    let (mut session, observer) = testing::start_probe(
        &host,
        common::ready_movie().responding("player.walkTo", [(json!(true), vec![])]),
    );
    common::log_in(&observer);
    enter_map(&observer, "ledgermayne");
    observer
        .command(
            serde_json::from_value(json!({
                "kind":"patchAutoZone","patch":{"enabled":true,"map":"ledgermayne"}
            }))
            .unwrap(),
        )
        .recv_timeout(TIMEOUT)
        .unwrap();
    let _supervisor = Supervisor::start(&mut session, None).unwrap();
    let run = common::yielding(&mut session);
    drain(&observer);
    packet(&observer, json!({"cmd":"event","args":{"zoneSet":"A"}}));
    let call = next_call(&observer, "player.walkTo");
    assert!(matches!(call.get(1), Some(Value::Number(x)) if (147.0..=276.0).contains(x)));
    assert_eq!(
        1 + walks_for(&observer, Duration::from_millis(1200)),
        1,
        "auto zone moved in more than one runtime"
    );
    assert!(matches!(
        run.wait(Duration::ZERO),
        Err(RuntimeError::Timeout)
    ));
    run.stop_script();
    run.wait(TIMEOUT).unwrap();
}

#[test]
fn queen_iona_uses_projected_charge_auras_for_both_zones() {
    let host = testing::headless_host();
    let (mut session, observer) = testing::start_probe(
        &host,
        common::ready_movie().responding("player.walkTo", [(json!(true), vec![])]),
    );
    common::log_in(&observer);
    let _supervisor = Supervisor::start(&mut session, None).unwrap();
    for (name, positive) in [
        ("Positive Charge", true),
        ("Positive Charge?", true),
        ("Negative Charge", false),
        ("Negative Charge?", false),
    ] {
        enter_map(&observer, "queeniona");
        patch(&observer, json!({"enabled":true,"map":"queeniona"}));
        for zone in ["A", "B"] {
            packet(&observer, json!({"cmd":"event","args":{"zoneSet":zone}}));
            packet(
                &observer,
                json!({"cmd":"aura+","tInf":"p:7","aura":{"nam":name,"t":"s","dur":10,"stk":1}}),
            );
            let x = if (zone == "A") == positive {
                (746., 869.)
            } else {
                (111., 272.)
            };
            assert_position(&next_call(&observer, "player.walkTo"), x, (369., 379.));
        }
    }
}

#[test]
fn player_restarts_rebind_auto_zone_without_losing_the_config() {
    let host = testing::headless_host();
    let (mut session, observer) = testing::start_probe(
        &host,
        common::ready_movie().responding("player.walkTo", [(json!(true), vec![])]),
    );
    common::log_in(&observer);
    patch(&observer, json!({"enabled":true,"map":"ledgermayne"}));
    let mut supervisor = Supervisor::start(&mut session, None).unwrap();
    supervisor
        .restart_player(&mut session, testing::headless_surface())
        .unwrap();
    let observer = session.attach(EventInterests::ALL).unwrap();
    session.send(lucent_host::ViewEvent::Resized(
        lucent_host::Viewport::new(100., 100., 1.).unwrap(),
    ));
    testing::wait_loaded(&observer);
    common::log_in(&observer);
    enter_map(&observer, "ledgermayne");
    packet(&observer, json!({"cmd":"event","args":{"zoneSet":"B"}}));
    assert_position(
        &next_call(&observer, "player.walkTo"),
        (727., 852.),
        (353., 356.),
    );
}

#[test]
fn native_auto_zone_rereads_the_host_record_after_relevant_event_overflow() {
    let host = testing::headless_host();
    let burst = json!({"type":"json","dataObj":{"cmd":"event","args":{"zoneSet":"A"}}}).to_string();
    let (mut session, observer) = testing::start_probe(
        &host,
        common::ready_movie()
            .responding("player.walkTo", [(json!(true), vec![])])
            .raising_many("probe.overflow", "onExtensionResponse", &burst, 100),
    );
    common::log_in(&observer);
    enter_map(&observer, "ledgermayne");
    patch(&observer, json!({"enabled":true,"map":"ledgermayne"}));
    let mut policy = lucent_host::automation::AutoZonePolicy::new(
        session
            .attach(lucent_host::automation::AutoZonePolicy::INTERESTS)
            .unwrap(),
        Waker::noop().clone(),
    );
    policy.advance(Instant::now());
    for _ in 0..11 {
        observer
            .call_loader("probe.overflow", vec![])
            .recv_timeout(TIMEOUT)
            .unwrap();
    }
    patch(&observer, json!({"map":"moreskulls"}));
    assert_eq!(
        observer.try_event(),
        Err(lucent_host::bridge::Error::Overflow)
    );
    policy.advance(Instant::now());
    enter_map(&observer, "moreskulls");
    packet(&observer, json!({"cmd":"event","args":{"zoneSet":"A"}}));
    policy.advance(Instant::now());
    assert_position(
        &next_call(&observer, "player.walkTo"),
        (696., 802.),
        (445., 452.),
    );
}

#[test]
fn malformed_packets_leave_native_zone_event_delivery_running() {
    let host = testing::headless_host();
    let (mut session, observer) = testing::start_probe(
        &host,
        common::ready_movie().responding("player.walkTo", [(json!(true), vec![])]),
    );
    common::log_in(&observer);
    enter_map(&observer, "ledgermayne");
    patch(&observer, json!({"enabled":true,"map":"ledgermayne"}));
    let _supervisor = Supervisor::start(&mut session, None).unwrap();
    for (zone, x, y) in [
        ("A", (147., 276.), (353., 357.)),
        ("B", (727., 852.), (353., 356.)),
    ] {
        observer
            .call_loader(
                "probe.emit",
                vec![
                    Value::String("onExtensionResponse".into()),
                    Value::String("not json".into()),
                ],
            )
            .recv_timeout(TIMEOUT)
            .unwrap();
        packet(&observer, json!({"cmd":"event","args":{"zoneSet":zone}}));
        assert_position(&next_call(&observer, "player.walkTo"), x, y);
    }
}

#[test]
fn rejected_loader_walks_finish_once_and_the_next_zone_still_moves() {
    let logs = common::logs::Logs::default();
    let _guard = tracing::dispatcher::set_default(&logs.dispatch());
    let host = testing::headless_host();
    let (mut session, observer) = testing::start_probe(
        &host,
        common::ready_movie().responding(
            "player.walkTo",
            [(json!(false), vec![]), (json!(true), vec![])],
        ),
    );
    common::log_in(&observer);
    enter_map(&observer, "ledgermayne");
    patch(&observer, json!({"enabled":true,"map":"ledgermayne"}));
    let _supervisor = Supervisor::start(&mut session, None).unwrap();
    packet(&observer, json!({"cmd":"event","args":{"zoneSet":"A"}}));
    assert_position(
        &next_call(&observer, "player.walkTo"),
        (147., 276.),
        (353., 357.),
    );
    assert_eq!(walks_for(&observer, Duration::from_millis(1200)), 0);
    packet(&observer, json!({"cmd":"event","args":{"zoneSet":"B"}}));
    assert_position(
        &next_call(&observer, "player.walkTo"),
        (727., 852.),
        (353., 356.),
    );
    assert!(logs.messages(tracing::Level::WARN).is_empty());
}

#[test]
fn disabling_auto_zone_cancels_a_delayed_walk_through_the_event_stream() {
    let host = testing::headless_host();
    let (mut session, observer) = testing::start_probe(
        &host,
        common::ready_movie().responding("player.walkTo", [(json!(true), vec![])]),
    );
    common::log_in(&observer);
    enter_map(&observer, "queeniona");
    patch(&observer, json!({"enabled":true,"map":"queeniona"}));
    let mut policy = lucent_host::automation::AutoZonePolicy::new(
        session
            .attach(lucent_host::automation::AutoZonePolicy::INTERESTS)
            .unwrap(),
        Waker::noop().clone(),
    );
    let now = Instant::now();
    assert_eq!(policy.advance(now), None);
    packet(&observer, json!({"cmd":"event","args":{"zoneSet":""}}));
    assert_eq!(policy.advance(now), Some(now + Duration::from_millis(500)));
    patch(&observer, json!({"enabled":false}));
    assert_eq!(policy.advance(now + Duration::from_millis(1)), None);
    assert_eq!(policy.advance(now + Duration::from_secs(1)), None);
    observer
        .call_loader("probe.emit", vec![])
        .recv_timeout(TIMEOUT)
        .unwrap();
    while let Some(event) = observer.try_event().unwrap() {
        assert!(!matches!(event, ScriptEvent::Loader { event, .. }
            if event.name == "onProbeCall" && event.args.first() == Some(&Value::String("player.walkTo".into()))));
    }
    patch(&observer, json!({"enabled":true}));
    packet(&observer, json!({"cmd":"event","args":{"zoneSet":""}}));
    policy.advance(now + Duration::from_secs(1));
    policy.advance(now + Duration::from_millis(1500));
    assert_position(
        &next_call(&observer, "player.walkTo"),
        (490., 490.),
        (320., 320.),
    );
}
