use std::sync::Arc;
use std::task::{Wake, Waker};
use std::thread;
use std::time::{Duration, Instant};

use ruffle_core::backend::audio::NullAudioBackend;
use swf::avm1::types::{Action, DefineFunction, Push, Value};
use swf::{Compression, Header, Rectangle, SwfStr, Tag, Twips};

use crate::bridge::ScriptEvent;
use crate::player::{Clipboard, Movie, surface::noop_gpu};
use crate::{Host, ScriptLink, Session, Surface, ViewEvent, Viewport};

pub fn headless_host() -> Host {
    Host::with_services(
        Clipboard::memory().expect("couldn't start the in-memory clipboard"),
        || Box::new(NullAudioBackend::new()),
    )
    .expect("couldn't start the headless host")
}

#[derive(Default)]
pub struct ProbeMovie {
    constants: Vec<(String, serde_json::Value)>,
    raises: Vec<(String, String, String)>,
    records: Vec<String>,
    guarded_records: Vec<String>,
}

impl ProbeMovie {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn recording(mut self, name: impl Into<String>) -> Self {
        self.records.push(name.into());
        self
    }

    pub fn recording_when_ready(mut self, name: impl Into<String>) -> Self {
        let name = name.into();
        self.guarded_records.push(name.clone());
        self.records.push(name);
        self
    }

    pub fn constant(mut self, name: impl Into<String>, value: serde_json::Value) -> Self {
        self.constants.push((name.into(), value));
        self
    }

    pub fn raising(
        mut self,
        name: impl Into<String>,
        event: impl Into<String>,
        payload: impl Into<String>,
    ) -> Self {
        self.raises
            .push((name.into(), event.into(), payload.into()));
        self
    }
}

pub fn headless_surface() -> Surface {
    Surface::textures(noop_gpu(), || {})
}

pub fn start_probe(host: &Host, movie: ProbeMovie) -> (Session, ScriptLink) {
    let (session, link) = start_probe_without_viewport(host, movie);
    session.send(ViewEvent::Resized(
        Viewport::new(100.0, 100.0, 1.0).unwrap(),
    ));
    wait_loaded(&link);
    (session, link)
}

pub fn start_probe_without_viewport(host: &Host, movie: ProbeMovie) -> (Session, ScriptLink) {
    let mut session = host
        .start_movie(headless_surface(), Movie(Arc::from(fixture_swf(&movie))))
        .expect("couldn't start the probe player");
    let link = session.take_script_link().unwrap();
    (session, link)
}

struct Unpark(thread::Thread);

impl Wake for Unpark {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
}

fn wait_loaded(link: &ScriptLink) {
    link.set_waker(Waker::from(Arc::new(Unpark(thread::current()))));
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        assert!(
            !remaining.is_zero(),
            "probe did not emit onLoaded within 10 seconds"
        );
        match link.try_event() {
            Ok(Some(ScriptEvent::Loader { event, .. })) if event.name == "onLoaded" => return,
            Ok(Some(_)) => {}
            Ok(None) => thread::park_timeout(remaining),
            Err(error) => {
                panic!("probe failed while waiting up to 10 seconds for onLoaded: {error}")
            }
        }
    }
}

fn string(value: &str) -> &SwfStr {
    SwfStr::from_utf8_str(value)
}

fn actions(list: &[Action<'_>]) -> Vec<u8> {
    let mut output = Vec::new();
    let mut writer = swf::avm1::write::Writer::new(&mut output, 10);
    for action in list {
        writer.write_action(action).unwrap();
    }
    output
}

fn push(values: Vec<Value<'_>>) -> Action<'_> {
    Action::Push(Push { values })
}

fn external_interface() -> Vec<Action<'static>> {
    vec![
        push(vec![Value::Str(string("flash"))]),
        Action::GetVariable,
        push(vec![Value::Str(string("external"))]),
        Action::GetMember,
        push(vec![Value::Str(string("ExternalInterface"))]),
        Action::GetMember,
    ]
}

fn push_json<'a>(output: &mut Vec<Action<'a>>, value: &'a serde_json::Value) {
    use serde_json::Value as Json;
    match value {
        Json::Null => output.push(push(vec![Value::Null])),
        Json::Bool(value) => output.push(push(vec![Value::Bool(*value)])),
        Json::Number(value) => output.push(push(vec![match value
            .as_i64()
            .and_then(|value| i32::try_from(value).ok())
        {
            Some(value) => Value::Int(value),
            None => Value::Double(value.as_f64().unwrap()),
        }])),
        Json::String(value) => output.push(push(vec![Value::Str(string(value))])),
        Json::Array(values) => {
            for value in values.iter().rev() {
                push_json(output, value);
            }
            output.push(push(vec![Value::Int(values.len() as i32)]));
            output.push(Action::InitArray);
        }
        Json::Object(fields) => {
            for (key, value) in fields {
                output.push(push(vec![Value::Str(string(key))]));
                push_json(output, value);
            }
            output.push(push(vec![Value::Int(fields.len() as i32)]));
            output.push(Action::InitObject);
        }
    }
}

fn fixture_swf(movie: &ProbeMovie) -> Vec<u8> {
    let constants = &movie.constants;
    let echo_body = actions(&[
        push(vec![Value::Str(string("x"))]),
        Action::GetVariable,
        Action::Return,
    ]);
    let mut emit = vec![
        push(vec![Value::Str(string("payload"))]),
        Action::GetVariable,
        push(vec![Value::Str(string("name"))]),
        Action::GetVariable,
        push(vec![Value::Int(2)]),
    ];
    emit.extend(external_interface());
    emit.extend([
        push(vec![Value::Str(string("call"))]),
        Action::CallMethod,
        Action::Pop,
        push(vec![Value::Bool(true)]),
        Action::Return,
    ]);
    let emit_body = actions(&emit);
    let ready_body = actions(&[
        push(vec![Value::Str(string("_root.probeReady")), Value::Bool(true)]),
        Action::SetVariable,
        push(vec![Value::Bool(true)]),
        Action::Return,
    ]);
    let user_id_body = actions(&[push(vec![Value::Int(7)]), Action::Return]);
    let constant_bodies: Vec<(&str, Vec<u8>)> = constants
        .iter()
        .map(|(name, value)| {
            let mut body = Vec::new();
            push_json(&mut body, value);
            body.push(Action::Return);
            (name.as_str(), actions(&body))
        })
        .collect();
    let raise_bodies: Vec<(&str, Vec<u8>)> = movie
        .raises
        .iter()
        .map(|(name, event, payload)| {
            let mut body = vec![
                push(vec![Value::Str(string(payload))]),
                push(vec![Value::Str(string(event))]),
                push(vec![Value::Int(2)]),
            ];
            body.extend(external_interface());
            body.extend([
                push(vec![Value::Str(string("call"))]),
                Action::CallMethod,
                Action::Pop,
            ]);
            (name.as_str(), actions(&body))
        })
        .collect();
    let record_bodies: Vec<(&str, Vec<u8>)> = movie
        .records
        .iter()
        .map(|name| {
            let mut body = vec![
                push(vec![Value::Str(string("value"))]),
                Action::GetVariable,
                push(vec![
                    Value::Str(string(name)),
                    Value::Str(string("onProbeCall")),
                    Value::Int(3),
                ]),
            ];
            body.extend(external_interface());
            body.extend([
                push(vec![Value::Str(string("call"))]),
                Action::CallMethod,
                Action::Pop,
            ]);
            if movie.guarded_records.contains(name) {
                let mut rejected = vec![push(vec![
                    Value::Str(string(name)),
                    Value::Str(string("onProbeNotReady")),
                    Value::Int(2),
                ])];
                rejected.extend(external_interface());
                rejected.extend([
                    push(vec![Value::Str(string("call"))]),
                    Action::CallMethod,
                    Action::Pop,
                    push(vec![Value::Str(string("settings called before readiness"))]),
                    Action::Throw,
                ]);
                let offset = actions(&rejected).len() as i16;
                let mut guarded = vec![
                    push(vec![Value::Str(string("_root.probeReady"))]),
                    Action::GetVariable,
                    Action::If(swf::avm1::types::If { offset }),
                ];
                guarded.extend(rejected);
                guarded.extend(body);
                body = guarded;
            }
            (name.as_str(), actions(&body))
        })
        .collect();
    let mut callbacks = vec![
        ("probe.ready", vec![], ready_body.as_slice()),
        ("probe.echo", vec![string("x")], echo_body.as_slice()),
        (
            "probe.emit",
            vec![string("name"), string("payload")],
            emit_body.as_slice(),
        ),
    ];
    if !constants.iter().any(|(name, _)| name == "player.getUserId") {
        callbacks.push(("player.getUserId", vec![], user_id_body.as_slice()));
    }
    callbacks.extend(
        constant_bodies
            .iter()
            .chain(&raise_bodies)
            .map(|(name, body)| (*name, vec![], body.as_slice())),
    );
    callbacks.extend(
        record_bodies
            .iter()
            .map(|(name, body)| (*name, vec![string("value")], body.as_slice())),
    );
    let mut main = Vec::new();
    for (name, params, body) in callbacks {
        main.push(Action::DefineFunction(DefineFunction {
            name: string(""),
            params,
            actions: body,
        }));
        main.push(push(vec![
            Value::Null,
            Value::Str(string(name)),
            Value::Int(3),
        ]));
        main.extend(external_interface());
        main.extend([
            push(vec![Value::Str(string("addCallback"))]),
            Action::CallMethod,
            Action::Pop,
        ]);
    }
    main.push(push(vec![Value::Str(string("onLoaded")), Value::Int(1)]));
    main.extend(external_interface());
    main.extend([
        push(vec![Value::Str(string("call"))]),
        Action::CallMethod,
        Action::Pop,
        Action::Stop,
    ]);
    let main_bytes = actions(&main);
    let header = Header {
        compression: Compression::None,
        version: 10,
        stage_size: Rectangle {
            x_min: Twips::ZERO,
            x_max: Twips::from_pixels(100.0),
            y_min: Twips::ZERO,
            y_max: Twips::from_pixels(100.0),
        },
        frame_rate: swf::Fixed8::from_f32(24.0),
        num_frames: 1,
    };
    let mut output = Vec::new();
    swf::write_swf(
        &header,
        &[Tag::DoAction(&main_bytes), Tag::ShowFrame],
        &mut output,
    )
    .unwrap();
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bridge::{Error, Value};

    #[test]
    fn a_constant_answers_its_loader_call_with_nested_values() {
        let host = headless_host();
        let (_session, link) = start_probe(
            &host,
            ProbeMovie::new()
                .constant(
                    "bank.getItems",
                    serde_json::json!([
                        {"ItemID": 1, "sName": "Sword", "iQty": 2.5, "bTemp": false},
                        {"ItemID": -3, "sName": "", "tags": [null, "a"]}
                    ]),
                )
                .constant("player.getUserId", serde_json::json!(9)),
        );
        assert_eq!(
            link.call_loader("bank.getItems", vec![])
                .recv_timeout(Duration::from_secs(5))
                .map(lucent_protocol::bridge::to_json),
            Ok(Some(serde_json::json!([
                {"ItemID": 1.0, "bTemp": false, "iQty": 2.5, "sName": "Sword"},
                {"ItemID": -3.0, "sName": "", "tags": [null, "a"]}
            ])))
        );
        assert_eq!(
            link.call_loader("player.getUserId", vec![])
                .recv_timeout(Duration::from_secs(5)),
            Ok(Value::Number(9.0))
        );
    }

    #[test]
    fn restart_reloads_the_movie_and_ends_the_previous_link() {
        let host = headless_host();
        let (mut session, old) = start_probe(&host, ProbeMovie::new());
        assert!(session.take_script_link().is_none());
        session.restart(headless_surface()).unwrap();
        assert_eq!(old.try_event(), Err(Error::Stopped));
        assert_eq!(
            old.call_loader("probe.echo", vec![])
                .recv_timeout(Duration::from_secs(1)),
            Err(Error::Stopped)
        );
        assert_eq!(old.read(|_| 42), Err(Error::Stopped));
        let current = session.take_script_link().unwrap();
        assert_eq!(current.id(), old.id());
        session.send(ViewEvent::Resized(
            Viewport::new(100.0, 100.0, 1.0).unwrap(),
        ));
        wait_loaded(&current);
        assert_eq!(
            current
                .call_loader("probe.echo", vec![Value::Number(42.0)])
                .recv_timeout(Duration::from_secs(5)),
            Ok(Value::Number(42.0))
        );
    }
}
