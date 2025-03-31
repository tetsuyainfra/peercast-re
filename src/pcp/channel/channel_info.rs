use serde::Serialize;

use crate::pcp::{atom::decode::PcpChannelInfo, Atom, Id4};

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

    pub fn merge_pcp(&mut self, new_val: &PcpChannelInfo) {}
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

// macro_rules! def_accessor {
//     ($name: ident) => {
//         def_accessor!($name, String);
//     };

//     ($name: ident, $type: ty) => {
//         pub fn $name(mut self, $name: $type) -> Self {
//             self.$name = $name;
//             self
//         }

//         paste! {
//             pub fn [<set_ $name>](&mut self, $name: $type) {
//                 self.$name = $name;
//             }
//         }
//     };
// }
