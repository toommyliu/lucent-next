use super::*;
use crate::bridge::{ScriptEvent, Value};
use crate::{Supervisor, testing};
use serde_json::json;
use std::sync::mpsc;
const TIMEOUT: Duration = Duration::from_secs(10);

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
fn area(link: &ScriptLink, id: i64) {
    emit(
        link,
        json!({"cmd":"moveToArea","areaId":id,"areaName":format!("test-{id}"),
        "uoBranch":[{"entID":7,"uoName":"player-a","intHP":100,"intHPMax":100,"intState":1}],
        "monBranch":[{"MonID":1,"MonMapID":2,"strMonName":"Boss","intHP":50,"intHPMax":50,"intState":1}]}),
    );
}
fn drain(link: &ScriptLink) -> Vec<String> {
    link.call_loader("probe.echo", vec![])
        .recv_timeout(TIMEOUT)
        .unwrap();
    let mut calls = Vec::new();
    while let Some(event) = link.try_event().unwrap() {
        if let ScriptEvent::Loader { event, .. } = event
            && event.name == "onProbeCall"
            && let Some(Value::String(name)) = event.args.first()
        {
            calls.push(name.clone());
        }
    }
    calls
}
fn start(link: &ScriptLink) -> JobId {
    start_profile(link, json!({"steps":[{"skill":1}]}))
}
fn start_profile(link: &ScriptLink, profile: serde_json::Value) -> JobId {
    let deadline = Instant::now() + TIMEOUT;
    let id = loop {
        match link
            .jobs
            .start(&json!({"profile":profile,"targetPriority":["Boss"]}).to_string())
        {
            Ok(id) => break id,
            Err(error) if error.contains("session controller") && Instant::now() < deadline => {
                std::thread::yield_now()
            }
            Err(error) => panic!("{error}"),
        }
    };
    link.jobs.engage(id, true);
    id
}

#[test]
fn stale_incarnations_make_no_loader_calls_before_a_fresh_attack() {
    for change in ["respawn", "area", "connection", "player"] {
        let host = testing::headless_host();
        let (mut session, mut observer) = testing::start_probe(
            &host,
            testing::ProbeMovie::new()
                .combat_facts()
                .constant("flash.getGameObjectS", json!("\"player-a\""))
                .constant("flash.callGameFunction0", json!("true"))
                .constant("world.getAvailableMonsterMapIds", json!([2]))
                .recording("world.getAvailableMonsterMapIds")
                .responding("combat.attackMonster", [(json!(true), vec![])])
                .responding("combat.useSkill", [(json!(true), vec![])]),
        );
        area(&observer, 1);
        let mut supervisor = Supervisor::start(&mut session, None).unwrap();
        let mut owner = session.attach(EventInterests::ALL).unwrap();
        let (entered, barrier) = mpsc::channel();
        let (release, resume) = mpsc::channel();
        let resume = Mutex::new(resume);
        let first = AtomicBool::new(true);
        session.set_profile_dispatch_probe(move |_, _| {
            if first.swap(false, Ordering::AcqRel) {
                entered.send(()).unwrap();
                resume.lock().recv_timeout(TIMEOUT).unwrap();
            }
        });
        let (completed, completion) = mpsc::channel();
        session.set_profile_action_probe(move |stale| {
            completed.send(stale).unwrap();
        });
        let id = start(&owner);
        barrier.recv_timeout(TIMEOUT).unwrap();
        match change {
            "respawn" => {
                emit(
                    &observer,
                    json!({"cmd":"cb","m":{"2":{"intHP":0,"intState":0}}}),
                );
                emit(
                    &observer,
                    json!({"cmd":"cb","m":{"2":{"intHP":50,"intState":1}}}),
                );
            }
            "area" => area(&observer, 2),
            "connection" => {
                observer
                    .call_loader(
                        "probe.emit",
                        vec![
                            Value::String("onConnection".into()),
                            Value::String("OnConnection".into()),
                        ],
                    )
                    .recv_timeout(TIMEOUT)
                    .unwrap();
                area(&observer, 1);
            }
            "player" => {
                supervisor
                    .restart_player(&mut session, testing::headless_surface())
                    .unwrap();
            }
            _ => unreachable!(),
        }
        if change != "player" {
            drain(&observer);
        }
        release.send(()).unwrap();
        if change != "player" {
            assert!(completion.recv_timeout(TIMEOUT).unwrap(), "{change}");
            owner.jobs.cancel(id);
            assert_eq!(drain(&observer), Vec::<String>::new(), "{change}");
        } else {
            owner = session.attach(EventInterests::ALL).unwrap();
            observer = session.attach(EventInterests::ALL).unwrap();
            session.send(crate::ViewEvent::Resized(
                crate::Viewport::new(100.0, 100.0, 1.0).unwrap(),
            ));
            testing::wait_loaded(&observer);
            area(&observer, 1);
        }
        let fresh = start(&owner);
        assert!(!completion.recv_timeout(TIMEOUT).unwrap(), "{change}");
        owner.jobs.cancel(fresh);
        assert!(
            drain(&observer)
                .iter()
                .any(|name| name == "combat.attackMonster"),
            "{change}"
        );
    }
}

#[test]
fn old_deaths_do_not_match_a_new_incarnation() {
    let mut seen = Seen::default();
    let old = TargetKey {
        monster_map_id: 2,
        generation: 0,
    };
    let first = seen.key(1, old);
    let second = seen.key(
        1,
        TargetKey {
            generation: 1,
            ..old
        },
    );
    assert_ne!(first, second);
    assert_eq!(seen.death(1, old), None);
    assert_eq!(
        seen.death(
            1,
            TargetKey {
                generation: 1,
                ..old
            }
        ),
        Some(second)
    );
    let third = seen.key(2, old);
    assert_ne!(second, third);
    assert_eq!(
        seen.death(
            1,
            TargetKey {
                generation: 1,
                ..old
            }
        ),
        None
    );
    let host = testing::headless_host();
    let (mut session, observer) = testing::start_probe(
        &host,
        testing::ProbeMovie::new()
            .constant("flash.getGameObjectS", json!("\"player-a\""))
            .constant("flash.callGameFunction0", json!("true")),
    );
    let deaths = session
        .attach(EventInterests::new(&[EventKind::ProfileDeath]))
        .unwrap();
    area(&observer, 1);
    emit(
        &observer,
        json!({"cmd":"cb","m":{"2":{"intHP":0,"intState":0}}}),
    );
    emit(
        &observer,
        json!({"cmd":"cb","m":{"2":{"intHP":50,"intState":1}}}),
    );
    let Some(ScriptEvent::ProfileDeath { area, key }) = deaths.try_event().unwrap() else {
        panic!("missing stamped death");
    };
    assert_eq!(key.generation, 0);
    let generation = observer
        .read(|state| state.world.get_monster(2).unwrap().generation())
        .unwrap();
    assert_eq!(generation, 1);
    let mut current = Seen::default();
    current.key(area, TargetKey { generation, ..key });
    assert_eq!(current.death(area, key), None);
}

#[test]
fn a_queued_action_rechecks_revocation_on_the_player() {
    let host = testing::headless_host();
    let (session, observer) = testing::start_probe(
        &host,
        testing::ProbeMovie::new()
            .combat_facts()
            .constant("flash.getGameObjectS", json!("\"player-a\""))
            .constant("flash.callGameFunction0", json!("true"))
            .constant("world.getAvailableMonsterMapIds", json!([2]))
            .recording("world.getAvailableMonsterMapIds")
            .responding("combat.attackMonster", [(json!(true), vec![])]),
    );
    area(&observer, 1);
    let (reply, observation) = Pending::channel();
    observer.inbox().post(Msg::ObserveProfile {
        priorities: Arc::from([]),
        clock: JobClock {
            origin: Instant::now(),
        },
        reply,
        probe: None,
    });
    let observation = observation.recv_timeout(TIMEOUT).unwrap();
    let gate = Arc::new(JobGate::new(observation.binding));
    gate.state.lock().engaged = true;
    let key = observation.snapshot.monsters[0].key;
    drain(&observer);
    let (entered, barrier) = mpsc::channel();
    let (release, resume) = mpsc::channel();
    let blocked = session.read_projection(move |_| {
        entered.send(()).unwrap();
        resume.recv_timeout(TIMEOUT).unwrap();
    });
    barrier.recv_timeout(TIMEOUT).unwrap();
    let (reply, action) = Pending::channel();
    observer.inbox().post(Msg::DispatchProfile(
        Dispatch {
            gate: gate.clone(),
            cycle: Some(0),
            binding: observation.binding,
            area: observation.area,
            action: Action::Attack(TargetLease { key, version: 1 }),
            target: Some(key),
            item: None,
            probe: None,
        },
        reply,
    ));
    gate.close();
    release.send(()).unwrap();
    blocked.recv_timeout(TIMEOUT).unwrap();
    assert_eq!(
        action.recv_timeout(TIMEOUT).unwrap(),
        DispatchReply::StaleTarget
    );
    assert!(drain(&observer).is_empty());
}

fn next_skill(link: &ScriptLink) -> String {
    struct Unpark(std::thread::Thread);
    impl std::task::Wake for Unpark {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
    }
    link.set_waker(Waker::from(Arc::new(Unpark(std::thread::current()))));
    let deadline = Instant::now() + TIMEOUT;
    loop {
        assert!(Instant::now() < deadline, "no skill dispatched");
        match link.try_event().unwrap() {
            Some(ScriptEvent::Loader { event, .. })
                if event.name == "onProbeCall"
                    && event.args.first() == Some(&Value::String("combat.useSkill".into())) =>
            {
                let Value::List(args) = &event.args[1] else {
                    panic!("invalid skill args");
                };
                let Value::String(skill) = &args[0] else {
                    panic!("invalid skill slot");
                };
                return skill.clone();
            }
            Some(_) => {}
            None => std::thread::park_timeout(deadline.saturating_duration_since(Instant::now())),
        }
    }
}

#[test]
fn a_parked_unissued_cast_preserves_the_cursor() {
    let host = testing::headless_host();
    let (mut session, observer) = testing::start_probe(
        &host,
        testing::ProbeMovie::new()
            .combat_facts()
            .constant("flash.getGameObjectS", json!("\"player-a\""))
            .constant("flash.callGameFunction0", json!("true"))
            .constant("world.getAvailableMonsterMapIds", json!([2]))
            .responding("combat.attackMonster", [(json!(true), vec![])])
            .responding("combat.useSkill", [(json!(true), vec![])]),
    );
    area(&observer, 1);
    let _supervisor = Supervisor::start(&mut session, None).unwrap();
    let owner = session.attach(EventInterests::ALL).unwrap();
    let (entered, barrier) = mpsc::channel();
    let (release, resume) = mpsc::channel();
    let resume = Mutex::new(resume);
    let first = AtomicBool::new(true);
    session.set_profile_dispatch_probe(move |_, attack| {
        if !attack && first.swap(false, Ordering::AcqRel) {
            entered.send(()).unwrap();
            resume.lock().recv_timeout(TIMEOUT).unwrap();
        }
    });
    let id = start_profile(
        &owner,
        json!({"steps":[{"skill":1},{"skill":2}],"messageTriggers":[{"messageIncludes":"enrage","skill":4}]}),
    );
    barrier.recv_timeout(TIMEOUT).unwrap();
    owner.jobs.engage(id, false);
    emit(
        &observer,
        json!({"cmd":"cb","anims":[{"msg":"enrage","animStr":"cast","cInf":"m:2","tInf":"p:7"}]}),
    );
    release.send(()).unwrap();
    assert_eq!(next_skill(&observer), "4");
    owner.jobs.engage(id, true);
    assert_eq!(next_skill(&observer), "1");
    owner.jobs.cancel(id);
}
