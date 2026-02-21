use chrono_tz::Mexico::General;

use crate::pcp::{builder2::ChannelInfo, channel2::channel_info};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ValidChannelInfo {
    pub typee: String,
    pub name: String,
    pub genre: String,
    pub desc: String,
    pub comment: String,
    pub url: String,
    /// MIME(例: video/x-flv)
    pub stream_type: String,
    /// 拡張子(例: .flv)
    pub stream_ext: String,
    pub bitrate: i32,
}

impl From<&ChannelInfo> for ValidChannelInfo {
    fn from(c: &ChannelInfo) -> Self {
        let ChannelInfo {
            typee,
            name,
            genre,
            desc,
            comment,
            url,
            stream_type,
            stream_ext,
            bitrate,
        } = c;

        let mut vci = Self::default();
        if let Some(typee) = typee {
            vci.typee = String::from_utf8_lossy(typee).to_string();
        }
        if let Some(name) = name {
            vci.name = String::from_utf8_lossy(name).to_string();
        }
        if let Some(genre) = genre {
            vci.genre = String::from_utf8_lossy(genre).to_string();
        }

        if let Some(desc) = desc {
            vci.desc = String::from_utf8_lossy(desc).to_string();
        }
        if let Some(comment) = comment {
            vci.comment = String::from_utf8_lossy(comment).to_string();
        }
        if let Some(url) = url {
            vci.url = String::from_utf8_lossy(url).to_string();
        }
        if let Some(stream_type) = stream_type {
            vci.stream_type = String::from_utf8_lossy(stream_type).to_string();
        }
        if let Some(stream_ext) = stream_ext {
            vci.stream_ext = String::from_utf8_lossy(stream_ext).to_string();
        }
        if let Some(bitrate) = bitrate {
            vci.bitrate = *bitrate;
        }

        vci
    }
}
