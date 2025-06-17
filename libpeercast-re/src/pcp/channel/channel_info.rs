use serde::Serialize;

use crate::pcp::{atom::decode::PcpChannelInfo, Atom, Id4};

use super::merge_field;

/// Channel's info
/// AtomにするときはNull文字を追加するのを忘れないように
#[derive(Debug, Clone, Default, Serialize)]
pub struct ChannelInfo {
    /// typeは予約語なのでtypにしている
    pub typ: String,
    pub name: String,
    pub genre: String,
    pub desc: String,
    pub comment: String,
    pub url: String,
    pub stream_type: String,
    pub stream_ext: String,
    pub bitrate: i32,
}

impl ChannelInfo {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn merge_pcp(&mut self, val: PcpChannelInfo) {
        merge_field!(self, val, typ);
        merge_field!(self, val, name);
        merge_field!(self, val, genre);
        merge_field!(self, val, desc);
        merge_field!(self, val, comment);
        merge_field!(self, val, url);
        merge_field!(self, val, stream_type);
        merge_field!(self, val, stream_ext);
        merge_field!(self, val, bitrate);
    }
}

impl From<&PcpChannelInfo> for ChannelInfo {
    fn from(pcp_ch_info: &PcpChannelInfo) -> Self {
        let p = pcp_ch_info.clone();
        ChannelInfo {
            typ: p.typ.unwrap_or_default(),
            name: p.name.unwrap_or_default(),
            genre: p.genre.unwrap_or_default(),
            desc: p.desc.unwrap_or_default(),
            comment: p.comment.unwrap_or_default(),
            url: p.url.unwrap_or_default(),
            stream_type: p.stream_type.unwrap_or_default(),
            stream_ext: p.stream_ext.unwrap_or_default(),
            bitrate: p.bitrate.unwrap_or_default(),
        }
    }
}


#[cfg(test)]
mod t {
    use crate::pcp::{decode::PcpChannelInfo, ChannelInfo};

    #[test]
    fn test_merge(){
        let mut ci = ChannelInfo::new();
        let mut info = PcpChannelInfo::default();
        info.bitrate = Some(1024);
        let mut i = info.clone();

        ci.merge_pcp(i);

        assert_eq!(ci.bitrate, info.bitrate.unwrap());
    }

}