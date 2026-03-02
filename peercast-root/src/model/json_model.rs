use std::{net::SocketAddr, str::FromStr};

use chrono::{DateTime, TimeZone, Utc};
use libpeercast_re::{
    model::{ValidChannelInfo, ValidTrackInfo},
    pcp::GnuId,
    repository::Channel,
};
use serde::Serialize;

use crate::channel::RootChannel2;
use crate::model::IndexInfo;

//-------------------------------------------------------------------------------
/// 公開APIで使用するChannelInfo
//-------------------------------------------------------------------------------
#[derive(Debug, Clone, Serialize)]
pub struct JsonChannelInfo {
    /// チャンネルID
    pub id: GnuId,
    /// チャンネル名
    pub name: String,
    /// 配信者アドレス(もしくは初期接続先アドレス)
    pub tracker_addr: Option<SocketAddr>,
    /// 連絡先URL
    pub contact_url: String,
    /// ジャンル
    pub genre: String,
    /// 生のジャンル文字列(namespace, listener_hideableなどの指定を含む)
    pub raw_genre: String,
    /// 説明文
    pub desc: String,
    /// コメント
    pub comment: String,
    /// MIME(例: video/x-flv)
    pub stream_type: String,
    /// 拡張子(例: .flv)
    pub stream_ext: String,
    /// ビットレート(kbps単位)
    pub bitrate: i32,
    // filetype: String,
    // status: String,
    /// リスナー数
    pub number_of_listener: i32,
    /// リレー数
    pub number_of_relay: i32,
    /// 作成日時
    pub created_at: DateTime<Utc>, // FIX: 外部のCDNなどとの兼ね合いで配信時間が00:00意外になる可能性あり
    /// トラック情報
    pub track: JsonTrackInfo,

    /// WMV, FLVなどのタイプ
    #[serde(rename = "type")]
    pub typee: String,
}

impl JsonChannelInfo {
    pub fn to_line_of_index_txt(&self) -> String {
        self::line_of_index_txt_from_json(self)
    }

    #[allow(non_snake_case)]
    pub fn Empty() -> Self {
        // println!("DATETIME              {}", Utc.timestamp_opt(0, 0).unwrap());
        Self {
            id: GnuId::NONE,
            name: "".into(),
            tracker_addr: None,
            contact_url: "".into(),
            genre: "".into(),
            raw_genre: "".into(),
            desc: "".into(),
            comment: "".into(),
            typee: "".into(),
            stream_type: "".into(),
            stream_ext: "".into(),
            bitrate: 0,
            number_of_listener: 0,
            number_of_relay: 0,
            created_at: Utc.timestamp_opt(0, 0).unwrap(),
            track: JsonTrackInfo {
                title: "".into(),
                creator: "".into(),
                url: "".into(),
                album: "".into(),
                genre: "".into(),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct JsonTrackInfo {
    pub title: String,
    pub creator: String,
    pub url: String,
    pub album: String,
    pub genre: String,
}

impl From<ValidTrackInfo> for JsonTrackInfo {
    fn from(t: ValidTrackInfo) -> Self {
        JsonTrackInfo {
            title: t.title,
            creator: t.creator,
            url: t.url,
            album: t.album,
            genre: t.genre,
        }
    }
}

impl From<&RootChannel2> for JsonChannelInfo {
    fn from(ch: &RootChannel2) -> Self {
        let ValidChannelInfo {
            typee,
            name,
            genre,
            desc,
            comment,
            url,
            stream_type,
            stream_ext,
            bitrate,
        } = ch.channel_info().unwrap_or_default();

        JsonChannelInfo {
            id: ch.cid(),
            name,
            tracker_addr: ch.tracker_address(),
            contact_url: url,
            genre: genre.clone(),
            raw_genre: genre,
            desc,
            comment,
            typee,
            stream_type,
            stream_ext,
            bitrate,
            number_of_listener: ch.number_of_listener(),
            number_of_relay: ch.number_of_relay(),
            created_at: ch.created_at(),
            track: ch.track_info().unwrap_or_default().into(),
        }
    }
}

impl From<&IndexInfo> for JsonChannelInfo {
    fn from(value: &IndexInfo) -> Self {
        let mut j = JsonChannelInfo::Empty();
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
        j.contact_url = contact_url.clone();
        j.genre = genre.clone();
        j.raw_genre = genre.clone();
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

fn line_of_index_txt_from_json(info: &JsonChannelInfo) -> String {
    create_index_line(
        &info.name,
        &info.id,
        &info.tracker_addr,
        &info.contact_url,
        &info.genre,
        &info.desc,
        &info.comment,
        info.number_of_listener,
        info.number_of_relay,
        info.bitrate,
        &info.typee,
        &info.stream_type,
        &info.stream_ext,
        &info.created_at,
    )
}
// 順番に合わせる
// https://github.com/plonk/peercast-yt/blob/b60f176317406e79a5468ba80da8be1d83bb6126/core/common/public.cpp#L102
fn create_index_line(
    name: &String,
    id: &GnuId,
    tracker_addr: &Option<SocketAddr>,
    contact_url: &String,
    genre: &String,
    desc: &String,
    comment: &String,
    number_of_listener: i32,
    number_of_relay: i32,
    bitrate: i32,
    typee: &String,
    _stream_type: &String,
    _stream_ext: &String,
    created_at: &DateTime<Utc>,
) -> String {
    use html_escape::{encode_quoted_attribute, encode_safe};
    let diff_time = Utc::now() - created_at;
    let hour = diff_time.num_hours();
    let min = diff_time.num_minutes() % 60;

    let addr = tracker_addr.as_ref().map(|a| a.to_string()).unwrap_or_default();

    format!(
        "{name}<>{id}<>{addr}<>{contact_url}<>{genre}<>{desc}<>{number_of_listener}<>{number_of_relay}<>{bitrate}<>{typee}<><><><><>{name_escaped}<>{time_hour}:{time_min:02}<>click<>{comment}<>0",
        name = encode_safe(&name.clone()),
        id = id,
        addr = addr,
        contact_url = encode_quoted_attribute(&contact_url),
        genre = encode_safe(&genre),
        desc = encode_safe(&desc),
        number_of_listener = number_of_listener,
        number_of_relay = number_of_relay,
        bitrate = bitrate,
        typee = encode_safe(&typee),
        name_escaped = encode_safe(&name),
        time_hour = hour,
        time_min = min,
        comment = comment
    )
}
