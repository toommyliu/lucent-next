mod common;

use common::TIMEOUT;
use lucent_host::testing;
use lucent_protocol::script::ScriptOutcome;
use lucent_runtime::Supervisor;

#[test]
fn setters_return_captured_host_records_without_bot_requests() {
    let host = testing::headless_host();
    let (mut session, observer) = testing::start_probe(&host, common::ready_movie());
    common::log_in(&observer);
    let _supervisor = Supervisor::start(&mut session, None).unwrap();
    let run = common::start(
        &mut session,
        common::loose(&[(
            "enable.js",
            r#"
        import * as relogin from "lucent/autorelogin";
        export default function* () {
            yield* relogin.setServer("  Artix  ");
            yield* relogin.setDelay(Infinity);
            const state = yield* relogin.setEnabled(true);
            if (!state.enabled || !state.captured || state.username !== "player-a"
                || state.server !== "Artix" || state.delayMs !== 3000) throw new Error(JSON.stringify(state));
            if (JSON.stringify(yield* relogin.getState()) !== JSON.stringify(state)) throw new Error("records not published");
        }
    "#,
        )]),
        Default::default(),
    );
    assert_eq!(run.wait(TIMEOUT).unwrap(), ScriptOutcome::Completed);
    assert_eq!(session.pending_ask_count(), 0);
}

#[test]
fn credentials_and_status_survive_script_kills_and_player_replacement() {
    use lucent_host::bridge::Value;
    let host = testing::headless_host();
    let (mut session, observer) = testing::start_probe(&host, common::ready_movie());
    common::log_in(&observer);
    let mut supervisor = Supervisor::start(&mut session, None).unwrap();
    let run = common::started_run(
        &mut session,
        r#"
        import * as relogin from "lucent/autorelogin";
        import * as api from "lucent/api";
        export default function* () {
            yield* relogin.setEnabled(true);
            yield* api.packet.sendToClient("started");
            while (true) {}
        }
    "#,
    );
    assert!(matches!(
        run.wait(TIMEOUT),
        Err(lucent_runtime::RuntimeError::Killed(_))
    ));
    let before = session.auto_relogin();
    assert!(before.status.captured);
    supervisor
        .restart_player(&mut session, testing::headless_surface())
        .unwrap();
    assert_eq!(session.auto_relogin(), before);
    session.send(lucent_host::ViewEvent::Resized(
        lucent_host::Viewport::new(100.0, 100.0, 1.0).unwrap(),
    ));
    let replacement = session.attach().unwrap();
    testing::wait_loaded(&replacement);
    common::log_in(&replacement);
    let pending = replacement
        .command(lucent_protocol::command::Command::SetAutoReloginEnabled { enabled: true });
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&pending.recv_timeout(TIMEOUT).unwrap()).unwrap()
            ["status"]["captured"],
        true
    );
    replacement
        .call_loader(
            "probe.emit",
            vec![
                Value::String("onConnection".into()),
                Value::String("OnConnection".into()),
            ],
        )
        .recv_timeout(TIMEOUT)
        .unwrap();
    assert_eq!(session.pending_ask_count(), 0);
}

#[test]
fn killing_a_setter_during_capture_cannot_answer_a_later_run() {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    };
    use std::task::{Wake, Waker};
    struct Notice(mpsc::Sender<()>);
    impl Wake for Notice {
        fn wake(self: Arc<Self>) {
            let _ = self.0.send(());
        }
    }
    let host = testing::headless_host();
    let (mut session, observer) = testing::start_probe(&host, common::ready_movie());
    common::log_in(&observer);
    let link = session.attach().unwrap();
    let (wake, messages) = mpsc::channel();
    let waker = Waker::from(Arc::new(Notice(wake)));
    let mut policy = lucent_host::automation::ReloginPolicy::new(link, waker);
    let (entered, applying) = mpsc::channel();
    let (release, resume) = mpsc::channel();
    let stopping = Arc::new(AtomicBool::new(false));
    let stopped = stopping.clone();
    let worker = std::thread::spawn(move || {
        let mut block = true;
        while !stopped.load(Ordering::Acquire) {
            let deadline = policy.advance(std::time::Instant::now());
            if block {
                entered.send(()).unwrap();
                resume.recv_timeout(TIMEOUT).unwrap();
                block = false;
            }
            let wait = deadline.map_or(TIMEOUT, |at| {
                at.saturating_duration_since(std::time::Instant::now())
            });
            let _ = messages.recv_timeout(wait);
        }
    });
    applying.recv_timeout(TIMEOUT).unwrap();
    let runtime = lucent_runtime::Runtime::start(session.attach().unwrap()).unwrap();
    let (requested, issued) = mpsc::channel();
    let (unblock, resume_player) = mpsc::channel();
    let result = runtime.eval("capture.js",r#"globalThis.capture = __host.command('{"kind":"setAutoReloginEnabled","enabled":true}'); null;"#);
    let until = std::time::Instant::now() + TIMEOUT;
    while session.pending_relogin_count() == 0 {
        assert!(std::time::Instant::now() < until);
        std::thread::yield_now();
    }
    let barrier = session.read_projection(move |_| {
        requested.send(()).unwrap();
        resume_player.recv_timeout(TIMEOUT).unwrap();
    });
    issued.recv_timeout(TIMEOUT).unwrap();
    release.send(()).unwrap();
    runtime.kill();
    assert!(matches!(
        result.wait::<()>(TIMEOUT),
        Err(lucent_runtime::RuntimeError::Killed(_))
    ));
    drop(runtime);
    unblock.send(()).unwrap();
    barrier.recv_timeout(TIMEOUT).unwrap();
    let later = common::start(
        &mut session,
        common::loose(&[(
            "later.js",
            r#"
        import * as relogin from "lucent/autorelogin";
        export default function* () {
            if ((yield* relogin.setDelay(9000)).delayMs !== 9000) throw new Error("answer reached another run");
        }
    "#,
        )]),
        Default::default(),
    );
    assert_eq!(later.wait(TIMEOUT).unwrap(), ScriptOutcome::Completed);
    stopping.store(true, Ordering::Release);
    observer
        .call_loader(
            "probe.emit",
            vec![
                lucent_host::bridge::Value::String("onDebug".into()),
                lucent_host::bridge::Value::String("done".into()),
            ],
        )
        .recv_timeout(TIMEOUT)
        .unwrap();
    worker.join().unwrap();
    assert_eq!(session.pending_ask_count(), 0);
}
