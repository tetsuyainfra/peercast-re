use crate::pcp::builder::{ChannelInfo, TrackInfo};
////////////////////////////////////////////////////////////////////////////////
/// ValidChannelInfo: データが正しい事を保証されたChannelInfo
/// 主にAtomからChannelに情報を伝達する時の中間データとして使われる
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ValidChannelInfo {
    /// 配信名
    pub name: String,
    /// ジャンル
    pub genre: String,
    /// 配信詳細
    pub desc: String,
    /// コメント
    pub comment: String,
    /// コンタクトURL
    pub url: String,
    /// MIME(例: video/x-flv)
    pub stream_type: String,
    /// 拡張子(例: .flv)
    pub stream_ext: String,
    /// ビットレート(kbps単位)
    pub bitrate: i32,
    /// WMV, FLVなどのタイプ(おそらく大文字)
    pub typee: String,
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

////////////////////////////////////////////////////////////////////////////////
/// ValidTrackInfo: データが正しい事を保証されたTrackInfo
/// 主にAtomからChannelに情報を伝達する時の中間データとして使われる
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ValidTrackInfo {
    /// track title
    pub title: String,
    /// track album name
    pub album: String,
    /// track creator
    pub creator: String,
    /// track url
    pub url: String,
    /// track genre
    pub genre: String,
}

impl From<&TrackInfo> for ValidTrackInfo {
    fn from(info: &TrackInfo) -> Self {
        let TrackInfo {
            title,
            creator,
            url,
            album,
            genre,
        } = info;

        let mut vti = Self::default();
        if let Some(title) = title {
            vti.title = String::from_utf8_lossy(title).to_string();
        }
        if let Some(creator) = creator {
            vti.creator = String::from_utf8_lossy(creator).to_string();
        }
        if let Some(url) = url {
            vti.url = String::from_utf8_lossy(url).to_string();
        }
        if let Some(album) = album {
            vti.album = String::from_utf8_lossy(album).to_string();
        }
        if let Some(genre) = genre {
            vti.genre = String::from_utf8_lossy(genre).to_string();
        }

        vti
    }
}
