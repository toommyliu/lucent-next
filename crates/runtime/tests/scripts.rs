mod common;
use common::*;
use lucent_protocol::script::ScriptOutcome;
use lucent_runtime::{Kill, RuntimeError};
use std::time::Duration;

#[test]
fn the_daily_package_runs_and_reports_its_maxed_reward() {
    let (_host, _session, runtime) = ready_runtime();
    assert_eq!(
        runtime
            .run_script(
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
        runtime
            .run_script(wrapper, Default::default())
            .wait(TIMEOUT)
            .unwrap(),
        ScriptOutcome::StoppedByScript {
            reason: Some(r#"{"status":"maxed","map":"buyhouse"}"#.into()),
        },
    );
}

#[test]
fn inputs_apply_defaults_and_report_missing_required_values() {
    let (_host, _session, runtime) = ready_runtime();
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
        runtime
            .run_script(defaults, Default::default())
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
    let outcome = runtime
        .run_script(required, Default::default())
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
    let (_host, _session, runtime) = ready_runtime();
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
        match runtime
            .run_script(loose(&[(path, source)]), Default::default())
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
    match runtime
        .run_script(broken, Default::default())
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
    let (_host, _session, runtime) = ready_runtime();
    let script = package(
        "lucent-dailies",
        "tests/entry.js",
        &[
            ("tests/entry.js", "import './middle.js';"),
            ("tests/middle.js", "import './throwing.js';"),
            ("tests/throwing.js", "throw new Error('nested failure');"),
        ],
    );
    match runtime
        .run_script(script, Default::default())
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
    let (_host, _session, runtime) = ready_runtime();
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
        runtime
            .run_script(script, Default::default())
            .wait(TIMEOUT)
            .unwrap(),
        ScriptOutcome::StoppedByScript {
            reason: Some("1".into())
        }
    );
}

#[test]
fn builtin_default_named_and_namespace_exports_share_members() {
    let (_host, _session, runtime) = ready_runtime();
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
        runtime
            .run_script(script, Default::default())
            .wait(TIMEOUT)
            .unwrap(),
        ScriptOutcome::StoppedByScript {
            reason: Some(r#"[true,true,true,true,"buyhouse"]"#.into())
        },
    );
}

#[test]
fn replacing_a_script_stops_the_first_run_with_a_replacement_reason() {
    let (_host, _session, runtime) = ready_runtime();
    let first = yielding(&runtime);
    completed(&runtime);
    assert_eq!(
        first.wait(TIMEOUT).unwrap(),
        ScriptOutcome::Stopped {
            reason: Some("Replaced by another script".into())
        }
    );
}

#[test]
fn a_replaced_run_reports_its_stop_despite_an_async_abort_listener_rejection() {
    let (_host, _session, runtime) = ready_runtime();
    let started = runtime.eval(
        "listen.js",
        "new Promise(resolve => { globalThis.started = resolve; })",
    );
    let first = runtime.run_script(
        loose(&[(
            "abort.js",
            r#"
            import * as script from "lucent/script";
            export default function* () {
                script.signal.addEventListener("abort", async () => { throw new Error("late"); });
                globalThis.started("running");
                yield* script.sleep("1 hour");
            }
            "#,
        )]),
        Default::default(),
    );
    assert_eq!(started.wait::<String>(TIMEOUT).unwrap(), "running");
    let second = runtime.run_script(
        loose(&[("replacement.js", "export default function* () {}")]),
        Default::default(),
    );
    assert_eq!(
        first.wait(TIMEOUT).unwrap(),
        ScriptOutcome::Stopped {
            reason: Some("Replaced by another script".into())
        }
    );
    assert_eq!(second.wait(TIMEOUT).unwrap(), ScriptOutcome::Completed);
}

#[test]
fn a_script_outcome_survives_an_unhandled_rejection_during_serialization() {
    let (_host, _session, runtime) = ready_runtime();
    let run = runtime.run_script(
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
    completed(&runtime);
}

#[test]
fn cooperative_stop_settles_a_yielding_script_and_allows_another_run() {
    let (_host, _session, runtime) = ready_runtime();
    let run = yielding(&runtime);
    runtime.stop_script();
    runtime.stop_script();
    assert_eq!(
        run.wait(TIMEOUT).unwrap(),
        ScriptOutcome::Stopped { reason: None }
    );
    completed(&runtime);
}

#[test]
fn a_timed_out_wait_leaves_the_script_run_usable() {
    let (_host, _session, runtime) = ready_runtime();
    let run = yielding(&runtime);
    assert!(matches!(
        run.wait(Duration::from_millis(1)),
        Err(RuntimeError::Timeout)
    ));
    runtime.stop_script();
    assert_eq!(
        run.wait(TIMEOUT).unwrap(),
        ScriptOutcome::Stopped { reason: None }
    );
}

#[test]
fn the_watchdog_kills_a_script_and_rebuilds_for_the_next_run() {
    let (_host, _session, runtime) = ready_runtime();
    let error = runtime
        .run_script(
            loose(&[("loop.js", "export default function* () { while (true) {} }")]),
            Default::default(),
        )
        .wait(TIMEOUT)
        .unwrap_err();
    assert!(matches!(error, RuntimeError::Killed(Kill::Watchdog)));
    completed(&runtime);
}

#[test]
fn an_explicit_kill_ends_a_script_and_rebuilds_for_the_next_run() {
    let (_host, _session, runtime) = ready_runtime();
    let run = yielding(&runtime);
    runtime.kill();
    assert!(matches!(
        run.wait(TIMEOUT),
        Err(RuntimeError::Killed(Kill::Requested))
    ));
    completed(&runtime);
}

#[test]
fn event_overflow_finishes_script_cleanup_before_reporting_loss() {
    let (_host, _session, runtime) = ready_runtime();
    let script = loose(&[(
        "overflow.js",
        r#"
        import * as script from "lucent/script";
        export default function* () {
            script.signal.addEventListener("abort", () => { globalThis.cleaned = true; });
            for (let i = 0; i < 1100; i++) {
                window.swf["probe.emit"]("onDebug", String(i));
            }
            yield* script.sleep("1 hour");
        }
    "#,
    )]);
    let run = runtime.run_script(script, Default::default());
    assert!(matches!(run.wait(TIMEOUT), Err(RuntimeError::EventsLost)));
    assert!(
        runtime
            .eval("cleaned.js", "globalThis.cleaned")
            .wait::<bool>(TIMEOUT)
            .unwrap()
    );
    completed(&runtime);
}

#[test]
fn dropping_the_runtime_settles_a_pending_script() {
    let (_host, _session, runtime) = ready_runtime();
    let run = yielding(&runtime);
    drop(runtime);
    assert!(matches!(run.wait(TIMEOUT), Err(RuntimeError::Stopped)));
}

#[test]
fn an_unavailable_host_service_fails_the_script_with_its_name() {
    let (_host, _session, runtime) = ready_runtime();
    let script = loose(&[(
        "http.js",
        r#"
        import * as http from "lucent/http";
        export default function* () {
            yield* http.request("https://example.com");
        }
    "#,
    )]);
    match runtime
        .run_script(script, Default::default())
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
    let (_host, _session, runtime) = ready_runtime();
    let started = runtime.eval(
        "started.js",
        "new Promise(resolve => { globalThis.started = resolve; })",
    );
    let run = runtime.run_script(
        loose(&[(
            "leak.js",
            r#"
        import * as script from "lucent/script";
        export default function* () {
            globalThis.leak = 1;
            globalThis.ticks = 0;
            setInterval(() => globalThis.ticks++, 1);
            globalThis.started(true);
            yield* script.sleep("1 hour");
        }
    "#,
        )]),
        Default::default(),
    );
    assert!(started.wait::<bool>(TIMEOUT).unwrap());
    runtime.stop_script();
    assert_eq!(
        run.wait(TIMEOUT).unwrap(),
        ScriptOutcome::Stopped { reason: None }
    );
    drop(run);
    let next = runtime.run_script(
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
    let (_host, _session, runtime) = ready_runtime();
    let run = runtime.run_script(
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
