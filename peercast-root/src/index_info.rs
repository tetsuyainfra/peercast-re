use std::{net::SocketAddr, path::PathBuf};

use chrono::{DateTime, Utc};
use libpeercast_re::pcp::GnuId;
use serde::{Deserialize, Serialize};

use crate::TomlConfigError;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct FooterToml {
    #[serde(default)]
    pub infomations: Vec<IndexInfo>,
}

impl FooterToml {
    pub fn from_path(path: &PathBuf) -> Result<Self, TomlConfigError> {
        let s = std::fs::read_to_string(path)?;
        let t = toml::from_str(&s)?;
        Ok(t)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IndexInfo {
    #[serde(default = "GnuId::zero", skip_serializing_if = "GnuId::is_none")]
    pub id: GnuId,

    pub name: String,

    // 必要ないのでスキップ
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tracker_addr: Option<SocketAddr>,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub contact_url: String,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub genre: String,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub desc: String,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub comment: String,

    #[serde(default, skip_serializing_if = "String::is_empty", rename = "type")]
    pub typee: String,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub stream_type: String,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub stream_ext: String,

    #[serde(default, skip_serializing_if = "is_default")]
    pub bitrate: i32,

    #[serde(default, skip_serializing_if = "is_default")]
    pub number_of_listener: i32,

    #[serde(default, skip_serializing_if = "is_default")]
    pub number_of_relay: i32,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
}

fn is_default<T: Default + PartialEq>(t: &T) -> bool {
    *t == T::default()
}
