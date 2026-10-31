use super::*;
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
struct Scenario {
    name: String,
    request: Request,
    initial: Value,
    commands: Vec<Expected>,
    outcome: bool,
    finished_at: u64,
    updates: Vec<Update>,
}
#[derive(Deserialize)]
struct Expected {
    method: String,
    at: u64,
    args: Vec<String>,
    patch: Value,
    reply: bool,
}
#[derive(Deserialize)]
struct Update {
    at: u64,
    patch: Value,
}
fn patch(snapshot: &mut Value, patch: &Value) {
    for (key, value) in patch.as_object().unwrap() {
        snapshot[key] = value.clone();
    }
}
#[test]
fn shared_travel_transcripts() {
    let scenarios: Vec<Scenario> = serde_json::from_str(include_str!(
        "../../../tests/fixtures/travel/transcripts.json"
    ))
    .unwrap();
    for scenario in scenarios {
        let mut job = TravelJob::new(scenario.request).unwrap();
        let mut snapshot = scenario.initial;
        let mut now = 0;
        let mut event = Event::Tick;
        let mut commands = scenario.commands.iter();
        let mut updates = scenario.updates.iter().peekable();
        let mut completed = false;
        for _ in 0..5000 {
            while updates.peek().is_some_and(|update| update.at <= now) {
                patch(&mut snapshot, &updates.next().unwrap().patch);
            }
            let effects = job.advance(
                &serde_json::from_value(snapshot.clone()).unwrap(),
                event,
                now,
            );
            if let Some(outcome) = effects.outcome {
                assert_eq!(
                    outcome,
                    Outcome::Returned(scenario.outcome),
                    "{}",
                    scenario.name
                );
                assert!(
                    commands.next().is_none(),
                    "{}: unissued commands",
                    scenario.name
                );
                assert_eq!(
                    now, scenario.finished_at,
                    "{}: completion clock",
                    scenario.name
                );
                completed = true;
                break;
            }
            if let Some(command) = effects.command {
                let expected = commands
                    .next()
                    .unwrap_or_else(|| panic!("{}: unexpected {command:?}", scenario.name));
                assert_eq!(
                    command.action.loader_call(),
                    (expected.method.as_str(), expected.args.clone()),
                    "{} at {now}",
                    scenario.name
                );
                assert_eq!(now, expected.at, "{}: command clock", scenario.name);
                patch(&mut snapshot, &expected.patch);
                event = Event::Completed {
                    id: command.id,
                    reply: if matches!(command.action, Action::CellReady(_)) {
                        Reply::CellReady(expected.reply)
                    } else if expected.reply {
                        Reply::Accepted
                    } else {
                        Reply::Unavailable
                    },
                };
            } else {
                now = effects
                    .next_deadline
                    .unwrap_or_else(|| panic!("{}: stalled", scenario.name));
                event = Event::Tick;
            }
        }
        assert!(completed, "{}: unbounded", scenario.name);
    }
}

#[test]
fn cancellation_ignores_late_replies() {
    let mut job = TravelJob::new(Request::Jump {
        cell: "Safe".into(),
        pad: None,
    })
    .unwrap();
    let scenarios: Vec<Scenario> = serde_json::from_str(include_str!(
        "../../../tests/fixtures/travel/transcripts.json"
    ))
    .unwrap();
    let snapshot: Snapshot = serde_json::from_value(scenarios[0].initial.clone()).unwrap();
    let command = job.advance(&snapshot, Event::Tick, 0).command.unwrap();
    assert!(
        job.advance(
            &snapshot,
            Event::Completed {
                id: command.id + 1,
                reply: Reply::Accepted
            },
            0
        )
        .command
        .is_none()
    );
    assert_eq!(
        job.advance(&snapshot, Event::Cancel, 0).outcome,
        Some(Outcome::Cancelled)
    );
    let late = job.advance(
        &snapshot,
        Event::Completed {
            id: command.id,
            reply: Reply::Accepted,
        },
        1,
    );
    assert!(late.command.is_none());
    assert!(late.outcome.is_none());
}

#[test]
fn rejects_unbounded_input_and_requires_a_valid_random_draw() {
    assert_eq!(
        TravelJob::new(Request::Jump {
            cell: "x".repeat(MAX_TEXT + 1),
            pad: None
        })
        .unwrap_err(),
        InputError::TooLarge
    );
    assert_eq!(
        TravelJob::new(Request::Join {
            target: "doom-private".into(),
            cell: None,
            pad: None,
            random_room: None
        })
        .unwrap_err(),
        InputError::RandomRoomRequired
    );
}
