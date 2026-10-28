use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "lowercase")]
pub enum PacketDirection {
    Client,
    Server,
    Extension,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "lowercase")]
pub enum PacketEncoding {
    String,
    Json,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(tag = "direction", rename_all = "lowercase")]
pub enum Packet {
    Client {
        command: String,
        #[ts(type = "ReadonlyArray<string>")]
        params: Vec<String>,
        raw: String,
        encoding: PacketEncoding,
    },
    Server {
        command: String,
        #[ts(type = "unknown")]
        data: Value,
        raw: String,
        encoding: PacketEncoding,
    },
    Extension {
        command: String,
        #[ts(type = "unknown")]
        data: Value,
        raw: String,
        encoding: PacketEncoding,
    },
}

impl Packet {
    pub fn direction(&self) -> PacketDirection {
        match self {
            Self::Client { .. } => PacketDirection::Client,
            Self::Server { .. } => PacketDirection::Server,
            Self::Extension { .. } => PacketDirection::Extension,
        }
    }

    pub fn command(&self) -> &str {
        match self {
            Self::Client { command, .. }
            | Self::Server { command, .. }
            | Self::Extension { command, .. } => command,
        }
    }

    pub fn raw(&self) -> &str {
        match self {
            Self::Client { raw, .. } | Self::Server { raw, .. } | Self::Extension { raw, .. } => {
                raw
            }
        }
    }

    pub fn encoding(&self) -> PacketEncoding {
        match self {
            Self::Client { encoding, .. }
            | Self::Server { encoding, .. }
            | Self::Extension { encoding, .. } => *encoding,
        }
    }

    pub fn data(&self) -> Option<&Value> {
        match self {
            Self::Client { .. } => None,
            Self::Server { data, .. } | Self::Extension { data, .. } => Some(data),
        }
    }
}
