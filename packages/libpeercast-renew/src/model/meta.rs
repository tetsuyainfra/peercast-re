use std::net::SocketAddr;

use peercast_gnuid::GnuId;
use serde::Serialize;

use crate::{
    model::{ValidChannelInfo, ValidTrackInfo},
    pcp::builder::TrackInfo,
    repository::traits::Channel,
};

////////////////////////////////////////////////////////////////////////////////
/// ChannelMeta: データが正しい事を保証されたChannelInfo
/// 主にAtomからChannelに情報を伝達する時の中間データとして使われる
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct ChannelMeta {
    // pub track: TrackMeta,
    /// チャンネルID
    pub id: GnuId,

    /// 配信者アドレス(もしくは初期接続先アドレス)
    pub tracker_addr: Option<SocketAddr>,

    /// チャンネル名
    pub name: String,
    /// 連絡先URL
    pub url: String,
    /// 生のジャンル(namespace, listener_hideableなどの指定を含む)
    pub genre: String,
    /// ユーザーに表示されるジャンル(変化がなければNone)
    pub display_genre: Option<String>,
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

    /// WMV, FLVなどのタイプ
    #[serde(rename = "type")]
    pub typee: String,

    /// トラック情報
    pub track: TrackMeta,

    /// リスナー数
    pub number_of_listener: i32,
    /// リレー数
    pub number_of_relay: i32,
    /// 作成日時
    pub created_at: chrono::DateTime<chrono::Utc>, // FIX: 外部のCDNなどとの兼ね合いで配信時間が00:00意外になる可能性あり

    /// ネームスペース
    pub namespace: Option<String>,
}

impl ChannelMeta {
    pub fn Empty() -> Self {
        Self {
            id: GnuId::zero(), // <-- 0は特別なID
            tracker_addr: None,
            name: String::new(),
            genre: String::new(),
            display_genre: None,
            desc: String::new(),
            comment: String::new(),
            url: String::new(),
            stream_type: String::new(),
            stream_ext: String::new(),
            bitrate: 0,
            typee: String::new(),
            track: TrackMeta::default(),
            number_of_listener: 0,
            number_of_relay: 0,
            created_at: chrono::Utc::now(),
            namespace: None,
        }
    }

    pub fn with_valid(id: GnuId, valid_info: ValidChannelInfo, valid_track: ValidTrackInfo) -> Self {
        Self {
            id,
            tracker_addr: None,
            name: valid_info.name,
            genre: valid_info.genre,
            display_genre: None,
            desc: valid_info.desc,
            comment: valid_info.comment,
            url: valid_info.url,
            stream_type: valid_info.stream_type,
            stream_ext: valid_info.stream_ext,
            bitrate: valid_info.bitrate,
            typee: valid_info.typee,
            track: TrackMeta::from_info(valid_track),
            number_of_listener: 0,
            number_of_relay: 0,
            created_at: chrono::Utc::now(),
            namespace: None,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct TrackMeta {
    /// トラック名
    pub title: String,
    /// アーティスト名
    pub creator: String,
    /// アルバム名
    pub album: String,
    /// ジャンル
    pub genre: String,
    /// コメント
    pub url: String,
}

impl TrackMeta {
    pub fn from_info(track: ValidTrackInfo) -> Self {
        Self {
            title: track.title,
            creator: track.creator,
            album: track.album,
            genre: track.genre,
            url: track.url,
        }
    }
}
