use std::net::SocketAddr;

use chrono::{DateTime, Utc};
use libpeercast_re::{GnuId, model::ChannelMeta};
use serde::{Deserialize, Serialize};

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

impl From<&IndexInfo> for ChannelMeta {
    fn from(value: &IndexInfo) -> Self {
        let mut j = ChannelMeta::Empty();
        let IndexInfo {
            id,
            name,
            tracker_addr,
            contact_url,
            genre,
            desc,
            comment,
            typee,
            stream_type,
            stream_ext,
            bitrate,
            number_of_listener,
            number_of_relay,
            created_at,
        } = value;
        j.id = id.clone();
        j.typee = typee.clone();
        j.name = name.clone();
        j.tracker_addr = tracker_addr.clone();
        j.url = contact_url.clone();
        j.genre = genre.clone();
        j.display_genre = Some(genre.clone());
        j.desc = desc.clone();
        j.comment = comment.clone();
        j.stream_ext = stream_ext.clone();
        j.stream_type = stream_type.clone();
        j.bitrate = *bitrate;
        j.number_of_listener = *number_of_listener;
        j.number_of_relay = *number_of_relay;
        j.created_at = created_at.unwrap_or_else(|| chrono::Utc::now());
        j
    }
}
