use paste::paste;

mod broker;
mod channel;
mod channel_stream;
mod manager;
mod node_pool;
mod task;

pub(self) use broker::ChannelBrokerMessage;
pub use broker::{ChannelMessage, ChannelReciever};
pub use channel::{Channel, ChannelType};
pub use manager::ChannelManager;
pub use node_pool::{Node, NodePool};
use serde::Serialize;
pub use task::{BroadcastTaskConfig, RelayTaskConfig, SourceTaskConfig, TaskStatus};

use crate::pcp::{atom, Id4};

use super::Atom;

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
    pub bitrate: u32,
}

/// Channel's track info
#[derive(Debug, Clone, Default, Serialize)]
pub struct TrackInfo {
    pub title: String,
    pub creator: String,
    pub url: String,
    pub album: String,
    // pub genre: String, // PeerCastStation Only?
}

macro_rules! def_accessor {
    ($name: ident) => {
        def_accessor!($name, String);
    };

    ($name: ident, $type: ty) => {
        pub fn $name(mut self, $name: $type) -> Self {
            self.$name = $name;
            self
        }

        paste! {
            pub fn [<set_ $name>](&mut self, $name: $type) {
                self.$name = $name;
            }
        }
    };
}

impl ChannelInfo {
    pub fn new() -> Self {
        Default::default()
    }

    def_accessor!(typ);
    def_accessor!(stream_type);
    def_accessor!(name);
    def_accessor!(genre);
    def_accessor!(desc);
    def_accessor!(comment);
    def_accessor!(url);
    def_accessor!(stream_ext);
    def_accessor!(bitrate, u32);
}

impl From<&Atom> for ChannelInfo {
    fn from(atom: &Atom) -> Self {
        debug_assert_eq!(atom.id(), Id4::PCP_CHAN_INFO);
        let mut i = ChannelInfo::new();
        for a in atom.as_parent().childs() {
            match a.id() {
                Id4::PCP_CHAN_INFO_TYPE => {
                    i.set_typ(_in::to_string(&a.as_child().payload()));
                }
                Id4::PCP_CHAN_INFO_NAME => {
                    i.set_name(_in::to_string(&a.as_child().payload()));
                }
                Id4::PCP_CHAN_INFO_GENRE => {
                    i.set_genre(_in::to_string(&a.as_child().payload()));
                }
                Id4::PCP_CHAN_INFO_DESC => {
                    i.set_desc(_in::to_string(&a.as_child().payload()));
                }
                Id4::PCP_CHAN_INFO_COMMENT => {
                    i.set_comment(_in::to_string(&a.as_child().payload()));
                }
                Id4::PCP_CHAN_INFO_URL => {
                    i.set_url(_in::to_string(&a.as_child().payload()));
                }
                Id4::PCP_CHAN_INFO_STREAMTYPE => {
                    i.set_stream_type(_in::to_string(&a.as_child().payload()));
                }
                Id4::PCP_CHAN_INFO_STREAMEXT => {
                    i.set_stream_ext(_in::to_string(&a.as_child().payload()));
                }
                Id4::PCP_CHAN_INFO_BITRATE => i.set_bitrate(_in::to_u32(&a.as_child().payload())),
                _ => {
                    panic!("Not implemented. and may be no needs...");
                }
            }
        }
        i
    }
}

impl TrackInfo {
    pub fn new() -> Self {
        Default::default()
    }
    def_accessor!(title);
    def_accessor!(album);
    def_accessor!(creator);
    def_accessor!(url);
    // def_accessor!(genre);
}

impl From<&Atom> for TrackInfo {
    fn from(atom: &Atom) -> Self {
        debug_assert_eq!(atom.id(), Id4::PCP_CHAN_TRACK);
        let mut i = TrackInfo::new();
        for a in atom.as_parent().childs() {
            match a.id() {
                Id4::PCP_CHAN_TRACK_TITLE => {
                    i.set_title(_in::to_string(&a.as_child().payload()));
                }
                Id4::PCP_CHAN_TRACK_ALBUM => {
                    i.set_album(_in::to_string(&a.as_child().payload()));
                }
                Id4::PCP_CHAN_TRACK_CREATOR => {
                    i.set_creator(_in::to_string(&a.as_child().payload()));
                }
                Id4::PCP_CHAN_TRACK_URL => {
                    i.set_url(_in::to_string(&a.as_child().payload()));
                }
                // Id4::PCP_CHAN_TRACK_GENRE => {
                //     i.set_genre(_in::to_string(&a.as_child().payload()));
                // }
                _ => {
                    panic!("Not implemented. and may be no needs...");
                }
            }
        }
        i
    }
}

mod _in {
    use bytes::{Buf, Bytes};

    pub use super::super::util::atom::{to_string, to_u32};
}
