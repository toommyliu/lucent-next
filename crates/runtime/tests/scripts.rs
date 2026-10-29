mod common;
use common::*;
use lucent_protocol::script::ScriptOutcome;
use lucent_runtime::{Kill, RuntimeError};
use std::time::Duration;

#[test]
fn the_daily_package_runs_and_reports_its_maxed_reward() {
    let (_host, mut session, _runtime) = ready_runtime();
    assert_eq!(
        start(
            &mut session,
            package("lucent-dailies", "scripts/farm-elders-blood.js", &[]),
            Default::default()
        )
        .wait(TIMEOUT)
        .unwrap(),
        ScriptOutcome::Completed
    );
    let wrapper = package(
        "lucent-dailies",
        "tests/wrapper.js",
        &[(
            "tests/wrapper.js",
            r#"
            import * as Api from "lucent/api";
            import * as script from "lucent/script";
            import run from "../scripts/farm-elders-blood";
            export default function* () {
                const result = yield* run();
                yield* script.stop(JSON.stringify({
                    ...result,
                    map: yield* Api.map.getName(),
                }));
            }
        "#,
        )],
    );
    assert_eq!(
        start(&mut session, wrapper, Default::default())
            .wait(TIMEOUT)
            .unwrap(),
        ScriptOutcome::StoppedByScript {
            reason: Some(r#"{"status":"maxed","map":"buyhouse"}"#.into()),
        },
    );
}

#[test]
fn inputs_apply_defaults_and_report_missing_required_values() {
    let (_host, mut session, _runtime) = ready_runtime();
    let defaults = loose(&[(
        "inputs.js",
        r#"
        import * as script from "lucent/script";
        export const inputs = {
            id: "test",
            fields: [{ key: "name", label: "Name", type: "string", default: "Tommy" }],
        };
        export default function* () {
            yield* script.stop(yield* script.inputs.get("name"));
        }
    "#,
    )]);
    assert_eq!(
        start(&mut session, defaults, Default::default())
            .wait(TIMEOUT)
            .unwrap(),
        ScriptOutcome::StoppedByScript {
            reason: Some("Tommy".into())
        },
    );
    let required = loose(&[(
        "required.js",
        r#"
        export const inputs = {
            id: "test",
            fields: [{ key: "name", label: "Name", type: "string", required: true }],
        };
        export default function* () {}
    "#,
    )]);
    let outcome = start(&mut session, required, Default::default())
        .wait(TIMEOUT)
        .unwrap();
    match outcome {
        ScriptOutcome::Failed { message, .. } => {
            assert_eq!(message, "required.js is missing required inputs: name.")
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn module_errors_name_the_script_and_location() {
    let (_host, mut session, _runtime) = ready_runtime();
    for (path, source, expected) in [
        (
            "syntax.js",
            "export const x = ;",
            "syntax.js:1:18: SyntaxError: Unexpected token ';'",
        ),
        (
            "missing.js",
            "import './absent';",
            "missing.js:1:8: Cannot find module \"./absent\"",
        ),
        (
            "export.js",
            "import { absent } from 'lucent/api';",
            "export.js:1:10: SyntaxError: The requested module 'lucent/api' does not provide an export named 'absent'",
        ),
        (
            "default.js",
            "export default 1;",
            "default.js must export a generator function as its default export.",
        ),
        (
            "evaluate.js",
            "throw new Error('broken module');",
            "evaluate.js:1:7: broken module",
        ),
        (
            "await.js",
            "await 0;",
            "await.js: Scripts can't use top-level await.",
        ),
    ] {
        match start(&mut session, loose(&[(path, source)]), Default::default())
            .wait(TIMEOUT)
            .unwrap()
        {
            ScriptOutcome::Failed { message, .. } => assert_eq!(message, expected),
            other => panic!("{other:?}"),
        }
    }
    let broken = package(
        "lucent-dailies",
        "tests/broken.js",
        &[("tests/broken.js", "export const x = ;")],
    );
    match start(&mut session, broken, Default::default())
        .wait(TIMEOUT)
        .unwrap()
    {
        ScriptOutcome::Failed { message, .. } => assert_eq!(
            message,
            "@lucent/dailies/tests/broken.js:1:18: SyntaxError: Unexpected token ';'"
        ),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_nested_module_evaluation_error_keeps_the_throwing_module_in_its_details() {
    let (_host, mut session, _runtime) = ready_runtime();
    let script = package(
        "lucent-dailies",
        "tests/entry.js",
        &[
            ("tests/entry.js", "import './middle.js';"),
            ("tests/middle.js", "import './throwing.js';"),
            ("tests/throwing.js", "throw new Error('nested failure');"),
        ],
    );
    match start(&mut session, script, Default::default())
        .wait(TIMEOUT)
        .unwrap()
    {
        ScriptOutcome::Failed { message, details } => {
            assert_eq!(
                message,
                "@lucent/dailies/tests/throwing.js:1:7: nested failure"
            );
            assert!(
                details
                    .unwrap()
                    .contains("at @lucent/dailies/tests/throwing.js:1:7")
            );
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn two_importers_evaluate_their_shared_module_once() {
    let (_host, mut session, _runtime) = ready_runtime();
    let script = loose(&[
        (
            "entry.js",
            r#"
            import "./first.js";
            import "./second.js";
            import * as script from "lucent/script";
            export default function* () {
                yield* script.stop(String(globalThis.sharedModuleEvaluations));
            }
            "#,
        ),
        ("first.js", "import './shared.js';"),
        ("second.js", "import './shared.js';"),
        (
            "shared.js",
            "globalThis.sharedModuleEvaluations = (globalThis.sharedModuleEvaluations ?? 0) + 1;",
        ),
    ]);
    assert_eq!(
        start(&mut session, script, Default::default())
            .wait(TIMEOUT)
            .unwrap(),
        ScriptOutcome::StoppedByScript {
            reason: Some("1".into())
        }
    );
}

#[test]
fn builtin_default_named_and_namespace_exports_share_members() {
    let (_host, mut session, _runtime) = ready_runtime();
    let script = loose(&[(
        "builtins.js",
        r#"
        import { enum as e, null as n } from "lucent/schema";
        import schema from "lucent/schema";
        import Api from "lucent/api";
        import * as Namespace from "lucent/api";
        import * as script from "lucent/script";
        export default function* () {
            yield* script.stop(JSON.stringify([
                e === schema.enum,
                n === schema.null,
                Api.player === Namespace.player,
                Api === Namespace.default,
                yield* Api.map.getName(),
            ]));
        }
    "#,
    )]);
    assert_eq!(
        start(&mut session, script, Default::default())
            .wait(TIMEOUT)
            .unwrap(),
        ScriptOutcome::StoppedByScript {
            reason: Some(r#"[true,true,true,true,"buyhouse"]"#.into())
        },
    );
}

#[test]
fn replacing_a_run_drops_it_before_starting_the_next() {
    let (_host, mut session, _runtime) = ready_runtime();
    let first = yielding(&mut session);
    drop(first);
    completed(&mut session);
    assert_eq!(
        session.attached_link_count().recv_timeout(TIMEOUT).unwrap(),
        1
    );
}

#[test]
fn a_cooperative_stop_survives_an_async_abort_listener_rejection() {
    let (_host, mut session, _runtime) = ready_runtime();
    let first = started_run(
        &mut session,
        r#"
        import * as api from "lucent/api";
        import * as script from "lucent/script";
        export default function* () {
            script.signal.addEventListener("abort", async () => { throw new Error("late"); });
            yield* api.packet.sendToClient("started");
            yield* script.sleep("1 hour");
        }
    "#,
    );
    first.stop_script();
    assert_eq!(
        first.wait(TIMEOUT).unwrap(),
        ScriptOutcome::Stopped { reason: None }
    );
    drop(first);
    completed(&mut session);
}

#[test]
fn a_script_outcome_survives_an_unhandled_rejection_during_serialization() {
    let (_host, mut session, _runtime) = ready_runtime();
    let run = start(
        &mut session,
        loose(&[(
            "serialize.js",
            r#"
            export default function* () {
                Object.prototype.toJSON = function () {
                    if (this.kind === "completed") {
                        delete Object.prototype.toJSON;
                        Promise.reject(new Error("unrelated"));
                    }
                    return this;
                };
            }
            "#,
        )]),
        Default::default(),
    );
    assert_eq!(run.wait(TIMEOUT).unwrap(), ScriptOutcome::Completed);
    drop(run);
    completed(&mut session);
}

#[test]
fn cooperative_stop_settles_a_yielding_script_and_allows_another_run() {
    let (_host, mut session, _runtime) = ready_runtime();
    let run = yielding(&mut session);
    run.stop_script();
    run.stop_script();
    assert_eq!(
        run.wait(TIMEOUT).unwrap(),
        ScriptOutcome::Stopped { reason: None }
    );
    drop(run);
    completed(&mut session);
}

#[test]
fn a_timed_out_wait_leaves_the_script_run_usable() {
    let (_host, mut session, _runtime) = ready_runtime();
    let run = yielding(&mut session);
    assert!(matches!(
        run.wait(Duration::from_millis(1)),
        Err(RuntimeError::Timeout)
    ));
    run.stop_script();
    assert_eq!(
        run.wait(TIMEOUT).unwrap(),
        ScriptOutcome::Stopped { reason: None }
    );
}

#[test]
fn the_watchdog_kills_a_script_and_a_fresh_run_completes() {
    let (_host, mut session, _runtime) = ready_runtime();
    let error = start(
        &mut session,
        loose(&[("loop.js", "export default function* () { while (true) {} }")]),
        Default::default(),
    )
    .wait(TIMEOUT)
    .unwrap_err();
    assert!(matches!(error, RuntimeError::Killed(Kill::Watchdog)));
    completed(&mut session);
}

#[test]
fn an_explicit_kill_ends_a_script_and_a_fresh_run_completes() {
    let (_host, mut session, _runtime) = ready_runtime();
    let run = yielding(&mut session);
    run.kill();
    assert!(matches!(
        run.wait(TIMEOUT),
        Err(RuntimeError::Killed(Kill::Requested))
    ));
    drop(run);
    completed(&mut session);
}

#[test]
fn event_overflow_finishes_script_cleanup_before_reporting_loss() {
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::prelude::*;
    #[derive(Clone)]
    struct Cleanup(Arc<Mutex<bool>>);
    impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for Cleanup {
        fn on_event(
            &self,
            event: &tracing::Event<'_>,
            _: tracing_subscriber::layer::Context<'_, S>,
        ) {
            struct Message(bool);
            impl tracing::field::Visit for Message {
                fn record_debug(
                    &mut self,
                    field: &tracing::field::Field,
                    value: &dyn std::fmt::Debug,
                ) {
                    if field.name() == "message"
                        && format!("{value:?}").contains("overflow cleanup")
                    {
                        self.0 = true;
                    }
                }
            }
            let mut message = Message(false);
            event.record(&mut message);
            if message.0 {
                *self.0.lock().unwrap() = true;
            }
        }
    }
    let cleaned = Arc::new(Mutex::new(false));
    let subscriber = tracing_subscriber::registry().with(Cleanup(cleaned.clone()));
    let dispatch = tracing::Dispatch::new(subscriber);
    let _guard = tracing::dispatcher::set_default(&dispatch);
    let host = lucent_host::testing::headless_host();
    let (mut session, link) = lucent_host::testing::start_probe(&host, ready_movie());
    let setup = lucent_runtime::Runtime::start(link).unwrap();
    ready(&setup);
    let run = start(
        &mut session,
        loose(&[(
            "overflow.js",
            r#"
        import * as api from "lucent/api";
        import * as script from "lucent/script";
        export default function* () {
            script.signal.addEventListener("abort", () => console.log("overflow cleanup"));
            for (let index = 0; index < 1100; index++) {
                yield* api.packet.sendToClient("burst");
            }
            yield* script.sleep("1 hour");
        }
    "#,
        )]),
        Default::default(),
    );
    let outcome = run.wait(TIMEOUT);
    assert!(
        matches!(outcome, Err(RuntimeError::EventsLost)),
        "{outcome:?}"
    );
    assert!(*cleaned.lock().unwrap());
    drop(run);
    completed(&mut session);
}

#[test]
fn dropping_a_script_run_detaches_its_link() {
    let (_host, mut session, _runtime) = ready_runtime();
    let run = yielding(&mut session);
    drop(run);
    assert_eq!(
        session.attached_link_count().recv_timeout(TIMEOUT).unwrap(),
        1
    );
    completed(&mut session);
}

#[test]
fn an_unavailable_host_service_fails_the_script_with_its_name() {
    let (_host, mut session, _runtime) = ready_runtime();
    let script = loose(&[(
        "http.js",
        r#"
        import * as http from "lucent/http";
        export default function* () {
            yield* http.request("https://example.com");
        }
    "#,
    )]);
    match start(&mut session, script, Default::default())
        .wait(TIMEOUT)
        .unwrap()
    {
        ScriptOutcome::Failed { message, .. } => {
            assert_eq!(
                message,
                "http.js:4:25: The http service is not available yet."
            )
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_stopped_run_leaves_no_globals_or_timers() {
    let (_host, mut session, _runtime) = ready_runtime();
    let run = started_run(
        &mut session,
        r#"
        import * as api from "lucent/api";
        import * as script from "lucent/script";
        export default function* () {
            globalThis.leak = 1;
            globalThis.ticks = 0;
            setInterval(() => globalThis.ticks++, 1);
            yield* api.packet.sendToClient("started");
            yield* script.sleep("1 hour");
        }
    "#,
    );
    run.stop_script();
    assert_eq!(
        run.wait(TIMEOUT).unwrap(),
        ScriptOutcome::Stopped { reason: None }
    );
    drop(run);
    let next = start(
        &mut session,
        loose(&[(
            "inspect.js",
            r#"
        import * as script from "lucent/script";
        export default function* () {
            yield* script.sleep(20);
            yield* script.stop(JSON.stringify([typeof globalThis.leak, typeof globalThis.ticks]));
        }
    "#,
        )]),
        Default::default(),
    );
    assert_eq!(
        next.wait(TIMEOUT).unwrap(),
        ScriptOutcome::StoppedByScript {
            reason: Some(r#"["undefined","undefined"]"#.into()),
        }
    );
}

#[test]
fn user_modules_cannot_reach_host_internals() {
    let (_host, mut session, _runtime) = ready_runtime();
    let run = start(
        &mut session,
        loose(&[(
            "internals.js",
            r#"
        import * as api from "lucent/api";
        import * as script from "lucent/script";
        const internals = [typeof __host, typeof __lucent, typeof window.swf];
        export default function* () {
            const player = yield* api.player.get();
            yield* script.stop(JSON.stringify([...internals, player?.username]));
        }
    "#,
        )]),
        Default::default(),
    );
    assert_eq!(
        run.wait(TIMEOUT).unwrap(),
        ScriptOutcome::StoppedByScript {
            reason: Some(r#"["undefined","undefined","undefined","player-a"]"#.into()),
        }
    );
}

#[test]
fn restarting_the_player_stops_a_run_and_a_new_link_runs_again() {
    let (_host, mut session, runtime) = ready_runtime();
    let run = yielding(&mut session);
    session
        .restart(lucent_host::testing::headless_surface())
        .unwrap();
    assert!(matches!(run.wait(TIMEOUT), Err(RuntimeError::Stopped)));
    drop(run);
    drop(runtime);
    let link = session.attach().unwrap();
    session.send(lucent_host::ViewEvent::Resized(
        lucent_host::Viewport::new(100.0, 100.0, 1.0).unwrap(),
    ));
    await_event(&link, "onLoaded");
    let setup = lucent_runtime::Runtime::start(link).unwrap();
    ready(&setup);
    completed(&mut session);
}
