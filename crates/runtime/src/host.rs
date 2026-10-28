use std::time::{Duration, Instant};

use lucent_host::ScriptLink;
use lucent_host::bridge::{Error, Pending, ScriptEvent};
use lucent_protocol::bridge::{Value, from_json, to_json};
use lucent_protocol::event::ProjectionEvent;
use lucent_protocol::query::{Query, answer};
use serde::Serialize;

use crate::interrupt::Interrupt;

const HOST_TIMEOUT: Duration = Duration::from_secs(5);
const STOP_SLICE: Duration = Duration::from_millis(20);

pub(crate) fn swf(
    link: &ScriptLink,
    interrupt: &Interrupt,
    method: String,
    args: &str,
) -> Result<Option<String>, String> {
    let args: Vec<serde_json::Value> =
        serde_json::from_str(args).map_err(|error| error.to_string())?;
    let value = wait(
        link.call_loader(method, args.into_iter().map(from_json).collect()),
        interrupt,
    )?;
    Ok(to_json(value).map(|value| value.to_string()))
}

pub(crate) fn read(
    link: &ScriptLink,
    interrupt: &Interrupt,
    query: &str,
) -> Result<String, String> {
    let query: Query = serde_json::from_str(query).map_err(|error| error.to_string())?;
    wait(
        link.read_projection(move |projection| answer(projection, query)),
        interrupt,
    )
}

fn wait<T>(pending: Pending<T>, interrupt: &Interrupt) -> Result<T, String> {
    interrupt.block();
    let deadline = Instant::now() + HOST_TIMEOUT;
    let result = loop {
        if interrupt.stop().is_some() {
            break Err("the script was stopped".into());
        }
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            break Err(Error::Timeout.to_string());
        }
        match pending.recv_timeout(left.min(STOP_SLICE)) {
            Err(Error::Timeout) => {}
            result => break result.map_err(|error| error.to_string()),
        }
    };
    interrupt.unblock();
    result
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub(crate) enum Item {
    Loader {
        name: String,
        args: serde_json::Value,
        projected: Vec<ProjectionEvent>,
    },
    Invalidated,
}

impl From<ScriptEvent> for Item {
    fn from(event: ScriptEvent) -> Self {
        match event {
            ScriptEvent::Loader { event, projected } => Item::Loader {
                name: event.name,
                args: to_json(Value::List(event.args)).unwrap_or_default(),
                projected,
            },
            ScriptEvent::Invalidated => Item::Invalidated,
        }
    }
}
