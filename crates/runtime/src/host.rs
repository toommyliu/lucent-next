use std::time::Duration;

use lucent_host::ScriptLink;
use lucent_protocol::bridge::{from_json, to_json};
use lucent_protocol::query::{Query, answer};

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
