use std::time::Duration;

use lucent_host::ScriptLink;
use lucent_host::bridge::ScriptEvent;
use lucent_protocol::bridge::{Value, from_json, to_json};
use lucent_protocol::event::ProjectionEvent;
use lucent_protocol::query::{Query, answer};
use serde::Serialize;

const HOST_TIMEOUT: Duration = Duration::from_secs(5);

pub(crate) fn swf(link: &ScriptLink, method: String, args: &str) -> Result<Option<String>, String> {
    let args: Vec<serde_json::Value> =
        serde_json::from_str(args).map_err(|error| error.to_string())?;
    let value = link
        .call_loader(method, args.into_iter().map(from_json).collect())
        .recv_timeout(HOST_TIMEOUT)
        .map_err(|error| error.to_string())?;
    Ok(to_json(value).map(|value| value.to_string()))
}

pub(crate) fn read(link: &ScriptLink, query: &str) -> Result<String, String> {
    let query: Query = serde_json::from_str(query).map_err(|error| error.to_string())?;
    link.read_projection(move |projection| answer(projection, query))
        .recv_timeout(HOST_TIMEOUT)
        .map_err(|error| error.to_string())
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
