mod common;

use common::{TIMEOUT, logs::Logs};
use lucent_host::{
    ScriptLink, Session,
    bridge::{EventInterests, ScriptEvent},
    testing,
};
use lucent_protocol::{
    bridge::Value, operation::Operation, query::ItemSelector, script::ScriptOutcome,
};
use lucent_runtime::{Kill, Runtime, RuntimeError};
use serde_json::json;
use std::{
    sync::Arc,
    task::{Wake, Waker},
    time::Instant,
};

fn emit(link: &ScriptLink, data: serde_json::Value) {
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

fn setup(confirm: bool) -> (Session, Runtime, ScriptLink, ScriptLink) {
    let responses = if confirm {
        vec![(
            "onExtensionResponse".into(),
            json!({"type":"json","dataObj":{"cmd":"seia","iRes":1,"o":{"id":400}}}).to_string(),
        )]
    } else {
        vec![]
    };
    session_with(
        common::ready_movie()
            .constant("combat.getConsumableSkillItem", json!({"itemId":100}))
            .responding("inventory.equip", [(json!(true), responses)])
            .constant("world.isActionAvailable", json!(true))
            .constant("player.isMember", json!(true)),
    )
}

fn session_with(movie: testing::ProbeMovie) -> (Session, Runtime, ScriptLink, ScriptLink) {
    let host = testing::headless_host();
    let (mut session, link) = testing::start_probe(&host, movie);
    let runtime = Runtime::start(link).unwrap();
    common::ready(&runtime);
    let control = session.attach(EventInterests::ALL).unwrap();
    emit(
        &control,
        json!({"cmd":"loadInventoryBig","hitems":[{"ItemID":2,"sName":"Home","sType":"House","bEquip":1}],"items":[
            {"ItemID":100,"sName":"Potion","sType":"Item","sLink":"potion","sMeta":"400","iQty":2},
            {"ItemID":101,"sName":"Scroll","sType":"Item","sLink":"scroll","sMeta":"482","iQty":2}
        ]}),
    );
    let observer = session.attach(EventInterests::ALL).unwrap();
    (session, runtime, control, observer)
}

fn next_trace(link: &ScriptLink) -> serde_json::Value {
    struct Unpark(std::thread::Thread);
    impl Wake for Unpark {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
    }
    link.set_waker(Waker::from(Arc::new(Unpark(std::thread::current()))));
    let deadline = Instant::now() + TIMEOUT;
    loop {
        match link.try_event().unwrap() {
            Some(ScriptEvent::Loader { event, .. }) if event.name == "onProbeCall" => {
                if matches!(event.args.first(), Some(Value::String(method)) if matches!(method.as_str(), "inventory.equip" | "inventory.unequipConsumable"))
                {
                    return lucent_protocol::bridge::to_json(Value::List(event.args)).unwrap();
                }
            }
            Some(ScriptEvent::Loader { event, .. }) if event.name == "onExtensionResponse" => {
                let [Value::String(raw)] = event.args.as_slice() else {
                    panic!("{event:?}")
                };
                let packet: serde_json::Value = serde_json::from_str(raw).unwrap();
                if packet["dataObj"]["cmd"] == "seia" {
                    return json!(["seia", packet["dataObj"]["o"]["id"]]);
                }
            }
            Some(_) => {}
            None => {
                let remaining = deadline.saturating_duration_since(Instant::now());
                assert!(!remaining.is_zero(), "missing equip trace");
                std::thread::park_timeout(remaining);
            }
        }
    }
}

fn equip(session: &mut Session, id: u32, marker: &str) -> lucent_runtime::ScriptRun {
    common::start(
        session,
        common::loose(&[(
            "equip.js",
            &format!(
                r#"
        import {{ Effect }} from "effect";
        import * as api from "lucent/api";
        import * as script from "lucent/script";
        export default function* () {{
            const equipped = yield* Effect.raceFirst(api.inventory.equip({id}), Effect.gen(function* () {{
                console.log("{marker}");
                yield* script.sleep("1 hour");
                return false;
            }}));
            yield* script.stop(String(equipped));
        }}
    "#
            ),
        )]),
        Default::default(),
    )
}

#[test]
fn a_consumable_equip_observes_a_synchronous_reply_and_refreshes_the_public_result() {
    let (mut session, _runtime, _control, observer) = setup(true);
    assert_eq!(
        equip(&mut session, 100, "equip submitted")
            .wait(TIMEOUT)
            .unwrap(),
        ScriptOutcome::StoppedByScript {
            reason: Some("true".into())
        }
    );
    assert_eq!(
        [next_trace(&observer), next_trace(&observer)],
        [
            json!(["inventory.equip",[{"itemId":100.0}]]),
            json!(["seia", 400])
        ]
    );
}

#[test]
fn a_queued_equip_timeout_never_reaches_the_loader() {
    let logs = Logs::default();
    let _dispatch = tracing::dispatcher::set_default(&logs.dispatch());
    let (mut session, _runtime, control, observer) = setup(false);
    control
        .operate(Operation::EquipConsumable {
            selector: ItemSelector::Id(100.),
        })
        .unwrap();
    assert_eq!(
        next_trace(&observer),
        json!(["inventory.equip",[{"itemId":100.0}]])
    );
    let run = common::start(
        &mut session,
        common::loose(&[(
            "cancel.js",
            r#"
        import { Effect, Option } from "effect";
        import * as api from "lucent/api";
        import * as script from "lucent/script";
        export default function* () {
            let completions = 0;
            const queued = yield* api.inventory.equip(101).pipe(
                Effect.tap(() => Effect.sync(() => completions++)),
                Effect.timeoutOption("20 millis"));
            console.log("queued equip timed out " + Option.isNone(queued));
            const next = yield* api.inventory.equip(100);
            yield* script.stop(JSON.stringify([Option.isNone(queued),completions,next]));
        }
    "#,
        )]),
        Default::default(),
    );
    logs.wait_for(tracing::Level::INFO, "queued equip timed out true");
    emit(&control, json!({"cmd":"seia","iRes":1,"o":{"id":400}}));
    assert_eq!(
        [next_trace(&observer), next_trace(&observer)],
        [
            json!(["seia", 400]),
            json!(["inventory.equip",[{"itemId":100.0}]])
        ]
    );
    emit(&control, json!({"cmd":"seia","iRes":1,"o":{"id":400}}));
    assert_eq!(
        run.wait(TIMEOUT).unwrap(),
        ScriptOutcome::StoppedByScript {
            reason: Some("[true,0,true]".into())
        }
    );
}

#[test]
fn an_issued_equip_settles_after_its_fiber_times_out() {
    let logs = Logs::default();
    let _dispatch = tracing::dispatcher::set_default(&logs.dispatch());
    let (mut session, _runtime, control, observer) = setup(false);
    let run = common::start(
        &mut session,
        common::loose(&[(
            "cancel.js",
            r#"
        import { Effect, Option } from "effect";
        import * as api from "lucent/api";
        import * as script from "lucent/script";
        export default function* () {
            let completions = 0;
            const first = yield* api.inventory.equip(100).pipe(
                Effect.tap(() => Effect.sync(() => completions++)),
                Effect.timeoutOption("1 second"));
            const next = yield* Effect.raceFirst(api.inventory.equip(100), Effect.gen(function* () {
                console.log("issued equip timed out " + Option.isNone(first));
                yield* script.sleep("1 hour");
                return false;
            }));
            yield* script.stop(JSON.stringify([Option.isNone(first),completions,next]));
        }
    "#,
        )]),
        Default::default(),
    );
    assert_eq!(
        next_trace(&observer),
        json!(["inventory.equip",[{"itemId":100.0}]])
    );
    logs.wait_for(tracing::Level::INFO, "issued equip timed out true");
    emit(&control, json!({"cmd":"seia","iRes":1,"o":{"id":400}}));
    assert_eq!(
        [next_trace(&observer), next_trace(&observer)],
        [
            json!(["seia", 400]),
            json!(["inventory.equip",[{"itemId":100.0}]])
        ]
    );
    emit(&control, json!({"cmd":"seia","iRes":1,"o":{"id":400}}));
    assert_eq!(
        run.wait(TIMEOUT).unwrap(),
        ScriptOutcome::StoppedByScript {
            reason: Some("[true,0,true]".into())
        }
    );
}

#[test]
fn a_killed_runs_equip_settles_before_the_next_runs_equip() {
    let logs = Logs::default();
    let _dispatch = tracing::dispatcher::set_default(&logs.dispatch());
    let (mut session, _runtime, control, observer) = setup(false);
    let first = equip(&mut session, 100, "equip submitted");
    assert_eq!(
        next_trace(&observer),
        json!(["inventory.equip",[{"itemId":100.0}]])
    );
    first.kill();
    assert!(matches!(
        first.wait(TIMEOUT),
        Err(RuntimeError::Killed(Kill::Requested))
    ));
    drop(first);
    let next = equip(&mut session, 100, "successor submitted");
    logs.wait_for(tracing::Level::INFO, "successor submitted");
    emit(&control, json!({"cmd":"seia","iRes":1,"o":{"id":400}}));
    assert_eq!(
        [next_trace(&observer), next_trace(&observer)],
        [
            json!(["seia", 400]),
            json!(["inventory.equip",[{"itemId":100.0}]])
        ]
    );
    emit(&control, json!({"cmd":"seia","iRes":1,"o":{"id":400}}));
    assert_eq!(
        next.wait(TIMEOUT).unwrap(),
        ScriptOutcome::StoppedByScript {
            reason: Some("true".into())
        }
    );
}

fn holder(session: &mut Session, consumable: &str) -> lucent_runtime::ScriptRun {
    common::start(
        session,
        common::loose(&[(
            "fight.js",
            &format!(
                r#"
        import {{ Effect }} from "effect";
        import * as api from "lucent/api";
        import * as script from "lucent/script";
        export default function* () {{
            yield* api.packet.sendToClient("profile");
            yield* Effect.raceFirst(
                api.combat.kill("Frogzard", {{profile:{{steps:[{{skill:5}}],consumable:{consumable}}}}}),
                api.packet.once({{command:"release",direction:"extension",encoding:"json"}}, {{timeout:"1 hour"}}),
            );
            yield* api.packet.sendToClient("released");
            yield* script.sleep("1 hour");
        }}
    "#
            ),
        )]),
        Default::default(),
    )
}

fn await_available(link: &ScriptLink, run: &lucent_runtime::ScriptRun) {
    common::await_event(link, "onDebug", Some(run));
    loop {
        let event = common::await_event(link, "onProbeCall", Some(run));
        if event.args.first() == Some(&Value::String("world.getAvailableMonsterMapIds".into())) {
            return;
        }
    }
}

fn combat_setup(ambient: bool, confirm: bool) -> (Session, Runtime, ScriptLink, ScriptLink) {
    let host = testing::headless_host();
    let (mut session, link) = testing::start_probe(
        &host,
        common::ready_movie()
            .constant("world.isActionAvailable", json!(true))
            .constant("world.getAvailableMonsterMapIds", json!([2]))
            .recording("world.getAvailableMonsterMapIds")
            .constant("combat.attackMonster", json!(true))
            .responding(
                "combat.getConsumableSkillItem",
                if ambient {
                    vec![(json!(null), vec![])]
                } else {
                    vec![(json!(null), vec![]), (json!({"itemId":100}), vec![])]
                },
            )
            .responding(
                "inventory.equip",
                [(
                    json!(true),
                    if confirm {
                        vec![(
                            "onExtensionResponse".into(),
                            json!({"type":"json","dataObj":{"cmd":"seia","iRes":1,"o":{"id":400}}})
                                .to_string(),
                        )]
                    } else {
                        vec![]
                    },
                )],
            ),
    );
    let runtime = Runtime::start(link).unwrap();
    common::ready(&runtime);
    let control = session.attach(EventInterests::ALL).unwrap();
    emit(
        &control,
        json!({"cmd":"loadInventoryBig","hitems":[{"ItemID":2,"sName":"Home","sType":"House","bEquip":1}],"items":[
        {"ItemID":100,"sName":"Potion","sType":"Item","sLink":"potion","sMeta":"400","iQty":2},
        {"ItemID":101,"sName":"Scroll","sType":"Item","sLink":"scroll","sMeta":"482","iQty":2}]}),
    );
    emit(
        &control,
        json!({"cmd":"moveToArea","areaId":1,"areaName":"test-1",
        "uoBranch":[{"entID":7,"uoName":"player-a","intHP":100,"intHPMax":100,"intState":1}],
        "mondef":[{"MonID":1,"strMonName":"Frogzard"}],
        "monBranch":[{"MonID":1,"MonMapID":2,"intHP":50,"intHPMax":50}]}),
    );
    let trace = session.attach(EventInterests::ALL).unwrap();
    (session, runtime, control, trace)
}

#[test]
fn profile_holders_share_one_equip_and_conflicts_fight_without_equipping() {
    let (mut session, _runtime, control, trace) = combat_setup(false, true);
    let first_observer = session.attach(EventInterests::ALL).unwrap();
    let first = holder(&mut session, "\"Potion\"");
    await_available(&first_observer, &first);
    assert!(
        control
            .acquire_consumable_claim(Some(101))
            .unwrap()
            .is_none()
    );
    let shared_observer = session.attach(EventInterests::ALL).unwrap();
    let shared = holder(&mut session, "\"Potion\"");
    await_available(&shared_observer, &shared);
    let conflict_observer = session.attach(EventInterests::ALL).unwrap();
    let conflict = holder(&mut session, "\"Scroll\"");
    await_available(&conflict_observer, &conflict);
    first.kill();
    assert!(matches!(
        first.wait(TIMEOUT),
        Err(RuntimeError::Killed(Kill::Requested))
    ));
    drop(first);
    assert!(
        control
            .acquire_consumable_claim(Some(101))
            .unwrap()
            .is_none()
    );
    shared.kill();
    assert!(matches!(
        shared.wait(TIMEOUT),
        Err(RuntimeError::Killed(Kill::Requested))
    ));
    drop(shared);
    let next = control
        .acquire_consumable_claim(Some(101))
        .unwrap()
        .unwrap();
    assert!(next.first);
    control.release_claim(next.id);
    conflict.kill();
    let _ = conflict.wait(TIMEOUT);
    drop(conflict);
    let mut equips = vec![];
    while let Some(event) = trace.try_event().unwrap() {
        if let ScriptEvent::Loader { event, .. } = event
            && event.name == "onProbeCall"
            && event.args.first() == Some(&Value::String("inventory.equip".into()))
        {
            equips.push(lucent_protocol::bridge::to_json(Value::List(event.args)).unwrap());
        }
    }
    assert_eq!(equips, vec![json!(["inventory.equip",[{"itemId":100.0}]])]);
}

#[test]
fn late_action_replies_cannot_confirm_another_consumable_or_change_its_projection() {
    let packet = |action| {
        (
            "onExtensionResponse".into(),
            json!({"type":"json","dataObj":{
                "cmd":"seia","iRes":1,"o":{"id":action}
            }})
            .to_string(),
        )
    };
    let (mut session, _runtime, _control, observer) = session_with(
        common::ready_movie()
            .constant("world.isActionAvailable", json!(true))
            .responding(
                "combat.getConsumableSkillItem",
                [(json!({"itemId":101}), vec![]), (json!(null), vec![])],
            )
            .responding("inventory.unequipConsumable", [(json!(true), vec![])])
            .responding(
                "inventory.equip",
                [
                    (json!(true), vec![]),
                    (json!(true), vec![packet(400)]),
                    (json!(true), vec![packet(482)]),
                ],
            ),
    );
    let run = common::start(
        &mut session,
        common::loose(&[(
            "late.js",
            r#"
        import * as api from "lucent/api";
        import * as script from "lucent/script";
        export default function* () {
            const first = yield* api.inventory.equip(100);
            const second = yield* api.inventory.equip(101);
            const before = (yield* api.inventory.get(101)).equipped;
            const third = yield* api.inventory.equip(101);
            const after = (yield* api.inventory.get(101)).equipped;
            const unequipped = yield* api.inventory.unequipConsumable(101);
            const afterUnequip = (yield* api.inventory.get(101)).equipped;
            yield* script.stop(JSON.stringify([first,second,before,third,after,unequipped,afterUnequip]));
        }
    "#,
        )]),
        Default::default(),
    );
    assert_eq!(
        run.wait(TIMEOUT).unwrap(),
        ScriptOutcome::StoppedByScript {
            reason: Some("[false,false,false,true,true,true,false]".into())
        }
    );
    let trace: Vec<_> = (0..6).map(|_| next_trace(&observer)).collect();
    assert_eq!(
        trace,
        vec![
            json!(["inventory.equip",[{"itemId":100.0}]]),
            json!(["inventory.equip",[{"itemId":101.0}]]),
            json!(["seia", 400]),
            json!(["inventory.equip",[{"itemId":101.0}]]),
            json!(["seia", 482]),
            json!(["inventory.unequipConsumable",[{"itemId":101.0}]])
        ]
    );
}

#[test]
fn ambient_profile_fibers_release_the_last_share_without_ending_the_run() {
    let (mut session, _runtime, control, trace) = combat_setup(true, true);
    let first_observer = session.attach(EventInterests::ALL).unwrap();
    let first = holder(&mut session, "undefined");
    await_available(&first_observer, &first);
    let shared_observer = session.attach(EventInterests::ALL).unwrap();
    let shared = holder(&mut session, "undefined");
    await_available(&shared_observer, &shared);
    assert!(
        control
            .acquire_consumable_claim(Some(100))
            .unwrap()
            .is_none()
    );
    first.kill();
    assert!(matches!(
        first.wait(TIMEOUT),
        Err(RuntimeError::Killed(Kill::Requested))
    ));
    drop(first);
    assert!(
        control
            .acquire_consumable_claim(Some(100))
            .unwrap()
            .is_none()
    );
    let released = session.attach(EventInterests::ALL).unwrap();
    emit(&control, json!({"cmd":"release"}));
    common::await_event(&released, "onDebug", Some(&shared));
    let next = control
        .acquire_consumable_claim(Some(100))
        .unwrap()
        .unwrap();
    assert!(next.first);
    control.release_claim(next.id);
    assert!(matches!(
        shared.wait(std::time::Duration::ZERO),
        Err(RuntimeError::Timeout)
    ));
    shared.kill();
    let _ = shared.wait(TIMEOUT);
    drop(shared);
    let mut equips = vec![];
    while let Some(event) = trace.try_event().unwrap() {
        if let ScriptEvent::Loader { event, .. } = event
            && event.name == "onProbeCall"
            && event.args.first() == Some(&Value::String("inventory.equip".into()))
        {
            equips.push(event.args);
        }
    }
    assert!(equips.is_empty());
}

#[test]
fn player_restart_and_connection_change_release_live_profile_claims() {
    for restart in [false, true] {
        let (mut session, _runtime, control, observer) = combat_setup(true, true);
        let run = holder(&mut session, "undefined");
        await_available(&observer, &run);
        assert!(
            control
                .acquire_consumable_claim(Some(100))
                .unwrap()
                .is_none()
        );
        if restart {
            session.restart(testing::headless_surface()).unwrap();
        } else {
            control
                .call_loader(
                    "probe.emit",
                    vec![
                        Value::String("onConnection".into()),
                        Value::String("OnConnectionLost".into()),
                    ],
                )
                .recv_timeout(TIMEOUT)
                .unwrap();
        }
        let fresh = session.attach(EventInterests::ALL).unwrap();
        assert!(
            fresh
                .acquire_consumable_claim(Some(100))
                .unwrap()
                .unwrap()
                .first
        );
        run.kill();
        let _ = run.wait(TIMEOUT);
        drop(run);
    }
}

#[test]
fn an_incompatible_profile_starts_while_the_first_preflight_equip_is_pending() {
    let (mut session, _runtime, control, observer) = combat_setup(false, false);
    let trace = session.attach(EventInterests::ALL).unwrap();
    let run = common::start(
        &mut session,
        common::loose(&[(
            "profiles.js",
            r#"
        import { Effect } from "effect";
        import * as api from "lucent/api";
        export default function* () {
            yield* api.packet.sendToClient("profile");
            yield* Effect.raceFirst(
                api.combat.kill("Frogzard",{profile:{steps:[{skill:5}],consumable:"Potion"}}),
                api.combat.kill("Frogzard",{profile:{steps:[{skill:5}],consumable:"Scroll"}}),
            );
        }
    "#,
        )]),
        Default::default(),
    );
    await_available(&observer, &run);
    assert_eq!(
        next_trace(&trace),
        json!(["inventory.equip",[{"itemId":100.0}]])
    );
    assert!(
        control
            .acquire_consumable_claim(Some(101))
            .unwrap()
            .is_none()
    );
    emit(&control, json!({"cmd":"seia","iRes":1,"o":{"id":400}}));
    assert_eq!(next_trace(&trace), json!(["seia", 400]));
    run.kill();
    let _ = run.wait(TIMEOUT);
    drop(run);
    assert!(
        control
            .acquire_consumable_claim(Some(101))
            .unwrap()
            .unwrap()
            .first
    );
}
