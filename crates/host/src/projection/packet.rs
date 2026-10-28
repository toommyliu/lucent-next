use std::sync::LazyLock;

use lucent_protocol::{Packet, PacketDirection, PacketEncoding};
use regex::Regex;
use serde_json::Value;

use super::wire::trim_js;

#[derive(Debug, Clone, PartialEq)]
pub enum PacketDecode {
    Parsed(Packet),
    Unsupported,
    Malformed,
}

pub fn parse_packet(direction: PacketDirection, raw: &str) -> PacketDecode {
    let payload = strip_log_envelope(raw);
    let parsed = match direction {
        PacketDirection::Client => string_packet(payload).map(|(command, params)| Packet::Client {
            command,
            params,
            raw: raw.to_owned(),
            encoding: PacketEncoding::String,
        }),
        PacketDirection::Server => parse_server(payload, raw),
        PacketDirection::Extension => parse_extension(payload, raw),
    };
    match parsed {
        Some(packet) => PacketDecode::Parsed(packet),
        None if is_unsupported(payload) => PacketDecode::Unsupported,
        None => PacketDecode::Malformed,
    }
}

fn strip_log_envelope(raw: &str) -> &str {
    static LENGTH_SUFFIX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r", \(len: [0-9]+\)$").unwrap());
    let trimmed = trim_js(raw);
    let payload = [
        "[Sending - JSON]: ",
        "[Sending - STR]: ",
        "[ RECEIVED ]: ",
        "[Sending]: ",
    ]
    .into_iter()
    .find_map(|prefix| trimmed.strip_prefix(prefix))
    .unwrap_or(trimmed);
    let payload = LENGTH_SUFFIX
        .find(payload)
        .map_or(payload, |matched| &payload[..matched.start()]);
    trim_js(payload)
}

fn string_packet(payload: &str) -> Option<(String, Vec<String>)> {
    if !payload.starts_with("%xt%") {
        return None;
    }
    let params: Vec<_> = payload
        .split('%')
        .filter(|part| !part.is_empty())
        .map(str::to_owned)
        .collect();
    let command_index = if params.get(1).is_some_and(|value| value == "zm") {
        2
    } else {
        1
    };
    Some((params.get(command_index)?.clone(), params))
}

fn parse_server(payload: &str, raw: &str) -> Option<Packet> {
    if let Some((command, params)) = string_packet(payload) {
        return Some(Packet::Server {
            command,
            data: Value::Array(params.into_iter().map(Value::String).collect()),
            raw: raw.to_owned(),
            encoding: PacketEncoding::String,
        });
    }
    let value: Value = serde_json::from_str(payload).ok()?;
    let record = value.as_object()?;
    let data = if record.get("t").is_some_and(Value::is_string)
        && record
            .get("b")
            .and_then(Value::as_object)
            .is_some_and(|body| body.contains_key("o"))
    {
        record.get("b")?.get("o")?.clone()
    } else {
        value
    };
    let command = nonempty_command(&data)?;
    Some(Packet::Server {
        command,
        data,
        raw: raw.to_owned(),
        encoding: PacketEncoding::Json,
    })
}

fn parse_extension(payload: &str, raw: &str) -> Option<Packet> {
    let value: Value = serde_json::from_str(payload).ok()?;
    let record = value.as_object()?;
    let data = record.get("dataObj")?.clone();
    let (command, encoding) = match record.get("type")?.as_str()? {
        "str" => (
            data.as_array()?.first()?.as_str()?.to_owned(),
            PacketEncoding::String,
        ),
        "json" => (nonempty_command(&data)?, PacketEncoding::Json),
        _ => return None,
    };
    Some(Packet::Extension {
        command,
        data,
        raw: raw.to_owned(),
        encoding,
    })
}

fn nonempty_command(data: &Value) -> Option<String> {
    let command = data.as_object()?.get("cmd")?.as_str()?;
    (!command.is_empty()).then(|| command.to_owned())
}

fn is_unsupported(payload: &str) -> bool {
    payload.starts_with('<')
        || serde_json::from_str::<Value>(payload)
            .ok()
            .and_then(|value| {
                value
                    .as_object()
                    .and_then(|record| record.get("type"))
                    .cloned()
            })
            .is_some_and(|value| value == "xml")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn client_packets_keep_the_original_raw_string_and_filtered_params() {
        let raw = " \u{feff}[Sending - STR]: %xt%zm%mv%1%%10%20%, (len: 7) ";
        assert_eq!(
            parse_packet(PacketDirection::Client, raw),
            PacketDecode::Parsed(Packet::Client {
                command: "mv".into(),
                params: vec!["xt", "zm", "mv", "1", "10", "20"]
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
                raw: raw.into(),
                encoding: PacketEncoding::String,
            })
        );
        assert_eq!(
            parse_packet(PacketDirection::Client, r#"{"cmd":"mv"}"#),
            PacketDecode::Malformed
        );
    }

    #[test]
    fn server_packets_decode_wrapped_and_plain_json_and_string_messages() {
        for raw in [
            r#"{"cmd":"ct","hp":3}"#,
            r#"{"t":"xt","b":{"o":{"cmd":"ct","hp":3}}}"#,
        ] {
            assert_eq!(
                parse_packet(PacketDirection::Server, raw),
                PacketDecode::Parsed(Packet::Server {
                    command: "ct".into(),
                    data: json!({"cmd":"ct","hp":3}),
                    raw: raw.into(),
                    encoding: PacketEncoding::Json,
                })
            );
        }
        assert_eq!(
            parse_packet(PacketDirection::Server, "%xt%ct%1%"),
            PacketDecode::Parsed(Packet::Server {
                command: "ct".into(),
                data: json!(["xt", "ct", "1"]),
                raw: "%xt%ct%1%".into(),
                encoding: PacketEncoding::String,
            })
        );
        assert_eq!(
            parse_packet(
                PacketDirection::Server,
                r#"{"t":"xt","b":{"o":[]},"cmd":"ct"}"#
            ),
            PacketDecode::Malformed
        );
    }

    #[test]
    fn json_numbers_preserve_the_reference_rounding() {
        let PacketDecode::Parsed(Packet::Server { data, .. }) = parse_packet(
            PacketDirection::Server,
            r#"{"cmd":"stu","stats":{"critical":0.48629999999999995}}"#,
        ) else {
            panic!("expected server JSON packet");
        };
        assert_eq!(
            data["stats"]["critical"].as_f64().unwrap().to_bits(),
            0.48629999999999995_f64.to_bits()
        );
    }

    #[test]
    fn extension_packets_use_the_extension_envelope_and_accept_empty_string_commands() {
        let raw = r#"{"type":"json","dataObj":{"cmd":"initInventory","items":null}}"#;
        assert_eq!(
            parse_packet(PacketDirection::Extension, raw),
            PacketDecode::Parsed(Packet::Extension {
                command: "initInventory".into(),
                data: json!({"cmd":"initInventory","items":null}),
                raw: raw.into(),
                encoding: PacketEncoding::Json,
            })
        );
        let raw = r#"{"type":"str","dataObj":["",3]}"#;
        assert_eq!(
            parse_packet(PacketDirection::Extension, raw),
            PacketDecode::Parsed(Packet::Extension {
                command: "".into(),
                data: json!(["", 3]),
                raw: raw.into(),
                encoding: PacketEncoding::String,
            })
        );
        for raw in [
            r#"{"cmd":"initInventory","items":[]}"#,
            r#"{"type":"str","dataObj":[]}"#,
            r#"{"type":"json","dataObj":{"cmd":""}}"#,
        ] {
            assert_eq!(
                parse_packet(PacketDirection::Extension, raw),
                PacketDecode::Malformed
            );
        }
    }

    #[test]
    fn unsupported_envelopes_are_distinct_from_malformed_packets() {
        for direction in [
            PacketDirection::Client,
            PacketDirection::Server,
            PacketDirection::Extension,
        ] {
            for raw in [" <msg t='sys'/> ", r#"{"type":"xml","dataObj":"<msg/>"}"#] {
                assert_eq!(parse_packet(direction, raw), PacketDecode::Unsupported);
            }
            for raw in ["garbage", "%xt%", r#"{"cmd":9}"#] {
                assert_eq!(parse_packet(direction, raw), PacketDecode::Malformed);
            }
        }
    }

    #[test]
    fn an_xml_type_is_only_unsupported_if_the_direction_parser_rejects_it() {
        let raw = r#"{"type":"xml","cmd":"ct"}"#;
        assert!(matches!(
            parse_packet(PacketDirection::Server, raw),
            PacketDecode::Parsed(_)
        ));
    }
}
