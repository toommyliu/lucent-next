use std::sync::Arc;
use std::time::{Duration, Instant};

use ruffle_core::backend::audio::NullAudioBackend;
use swf::avm1::types::{Action, DefineFunction, Push, Value};
use swf::{Compression, Header, Rectangle, SwfStr, Tag, Twips};

use crate::player::{Clipboard, Movie, surface::noop_gpu};
use crate::{Host, Session, Surface, ViewEvent, Viewport};

pub fn headless_host() -> Host {
    Host::with_services(
        Clipboard::memory().expect("couldn't start the in-memory clipboard"),
        || Box::new(NullAudioBackend::new()),
    )
    .expect("couldn't start the headless host")
}

pub struct ProbeMovie(Movie);

impl Default for ProbeMovie {
    fn default() -> Self {
        Self::new()
    }
}

impl ProbeMovie {
    pub fn new() -> Self {
        Self(Movie(Arc::from(fixture_swf())))
    }
}

pub fn start_probe(host: &Host, movie: ProbeMovie) -> Session {
    let surface = Surface::textures(noop_gpu(), || {});
    let session = host
        .start_movie(surface, movie.0)
        .expect("couldn't start the probe player");
    session.send(ViewEvent::Resized(
        Viewport::new(100.0, 100.0, 1.0).unwrap(),
    ));
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        assert!(
            !remaining.is_zero(),
            "probe did not emit onLoaded within 10 seconds"
        );
        match session.recv_loader_event(remaining) {
            Ok(event) if event.name == "onLoaded" => return session,
            Ok(_) => {}
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

fn fixture_swf() -> Vec<u8> {
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
    let user_id_body = actions(&[push(vec![Value::Int(7)]), Action::Return]);
    let mut main = Vec::new();
    for (name, params, body) in [
        ("probe.echo", vec![string("x")], echo_body.as_slice()),
        (
            "probe.emit",
            vec![string("name"), string("payload")],
            emit_body.as_slice(),
        ),
        ("player.getUserId", vec![], user_id_body.as_slice()),
    ] {
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
    fn restart_reloads_the_movie_and_ends_the_previous_link() {
        let host = headless_host();
        let mut session = start_probe(&host, ProbeMovie::new());
        let old = session.take_script_link().unwrap();
        assert!(session.take_script_link().is_none());
        session
            .restart(Surface::textures(noop_gpu(), || {}))
            .unwrap();
        assert_eq!(
            old.call_loader("probe.echo", vec![])
                .recv_timeout(Duration::from_secs(1)),
            Err(Error::Stopped)
        );
        assert_eq!(
            old.read_projection(|_| 42)
                .recv_timeout(Duration::from_secs(1)),
            Err(Error::Stopped)
        );
        let current = session.take_script_link().unwrap();
        assert_eq!(current.id(), old.id());
        session.send(ViewEvent::Resized(
            Viewport::new(100.0, 100.0, 1.0).unwrap(),
        ));
        assert_eq!(
            session
                .recv_loader_event(Duration::from_secs(10))
                .unwrap()
                .name,
            "onLoaded"
        );
        assert_eq!(
            current
                .call_loader("probe.echo", vec![Value::Number(42.0)])
                .recv_timeout(Duration::from_secs(5)),
            Ok(Value::Number(42.0))
        );
    }
}
