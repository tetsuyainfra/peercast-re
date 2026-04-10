use std::net::SocketAddr;

use peercast_gnuid::GnuId;
use serde::Serialize;

use crate::model::{ValidChannelInfo, ValidTrackInfo};

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
    pub contact_url: String,
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
    pub fn from_info(id: GnuId, ch: ValidChannelInfo, track: ValidTrackInfo) -> Self {
        Self {
            id,
            tracker_addr: None,

            name: ch.name,
            genre: ch.genre.clone(),
            display_genre: None,
            desc: ch.desc,
            comment: ch.comment,
            contact_url: ch.url,
            stream_type: ch.stream_type,
            stream_ext: ch.stream_ext,
            bitrate: ch.bitrate,
            typee: ch.typee,
            track: TrackMeta::from_info(track),
            //
            number_of_listener: 0,
            number_of_relay: 0,
            created_at: chrono::Utc::now(),
            //
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
