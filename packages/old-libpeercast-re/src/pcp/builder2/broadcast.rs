use nom::Err;
use tracing::warn;

use crate::pcp::{
    atom2::{
        decode2::{decode_gnuid, decode_i16, decode_i32, decode_u8, decode_vecu8},
        Atom2Kind, AtomView,
    },
    builder2::{ChanInfo, HostInfo, InfoParseError},
    Atom, Atom2, GnuId, Id4,
};

pub struct BroadcastBuilder2 {
    ttl: u8,
    hops: u8,
    //
    from_session_id: GnuId,
    channel_id: GnuId,
    //
    broadcast_group: BroadcastGroup,
}

impl BroadcastBuilder2 {
    fn new(ttl: u8, hops: u8, from_session_id: GnuId, channel_id: GnuId, broadcast_group: BroadcastGroup) -> Self {
        Self {
            ttl,
            hops,
            from_session_id,
            channel_id,
            broadcast_group,
        }
    }

    // fn build(mut self) -> AtomMut {
    //     let mut vec = vec![];
    //     vec.push(Atom::Child((Id4::PCP_BCST_TTL, self.ttl).into()));
    //     vec.push(Atom::Child((Id4::PCP_BCST_HOPS, self.hops).into()));
    //     vec.push(Atom::Child((Id4::PCP_BCST_FROM, self.from_session_id).into()));

    //     // Versions
    //     // PCPVersion.SetBcstVersion(bcst);
    //     vec.push(Atom::Child((Id4::PCP_BCST_VERSION, PKG_SERVANT_VERSION).into())); // 1218
    //     vec.push(Atom::Child((Id4::PCP_BCST_VERSION_VP, PKG_SERVANT_VERSION_VP).into())); // 27
    //     vec.push(Atom::Child((Id4::PCP_BCST_VERSION_EX_PREFIX, &PKG_SERVANT_VERSION_EX_PREFIX).into()));
    //     vec.push(Atom::Child((Id4::PCP_BCST_VERSION_EX_NUMBER, *PKG_SERVANT_VERSION_EX_NUMBER).into()));
    //     // bcst.SetBcstVersion(ServantVersion);
    //     // bcst.SetBcstVersionVP(ServantVersionVP);
    //     // bcst.SetBcstVersionEXPrefix(ServantVersionEXPrefix);
    //     // bcst.SetBcstVersionEXNumber(ServantVersionEXNumber);

    //     vec.push(Atom::Child((Id4::PCP_BCST_CHANID, self.channel_id).into()));

    //     // bcst.SetBcstFrom(channel.PeerCast.SessionID);
    //     // bcst.SetBcstChannelID(channel.ChannelID);
    //     // bcst.SetBcstGroup(BroadcastGroup.Root);
    //     // PostChannelInfo(bcst, channel);
    //     // PostHostInfo(bcst, channel, playing);
    //     Atom::Parent((Id4::PCP_BCST, vec).into())
    // }

    // /// チャンネル情報をYPに通知する時に利用する
    // pub fn to_yp_builder(session_id: GnuId, channel_id: GnuId) -> BroadcastBuilder {
    //     BroadcastBuilder::new(1, 0, session_id, channel_id, BroadcastGroup::TO_ROOT)
    // }
}

#[derive(Debug, Default)]
pub struct BroadcastInfo {
    pub ttl: Option<u8>,
    pub hops: Option<u8>,
    pub from_session_id: Option<GnuId>,
    //
    pub version: Option<i32>,
    pub version_vp: Option<i32>,
    pub version_ex_prefix: Option<Vec<u8>>,
    pub version_ex_number: Option<i16>,
    //
    pub channel_id: Option<GnuId>,
    pub broadcast_group: Option<BroadcastGroup>,

    pub chan: Option<ChanInfo>,
    pub host: Option<HostInfo>,
}

impl TryFrom<&Atom2> for BroadcastInfo {
    type Error = super::InfoParseError;

    fn try_from(a: &Atom2) -> Result<Self, Self::Error> {
        if (a.id() != Id4::PCP_BCST) {
            return Err(InfoParseError::TargetNotFound);
        }

        let Atom2Kind::Parent(pv) = a.view() else {
            return Err(InfoParseError::TargetNotFound);
        };

        let mut b = BroadcastInfo::default();
        for a in pv.children() {
            match a {
                Atom2Kind::Child(child_view) => match child_view.id() {
                    Id4::PCP_BCST_TTL => b.ttl = decode_u8(&child_view).ok(),
                    Id4::PCP_BCST_HOPS => b.hops = decode_u8(&child_view).ok(),
                    Id4::PCP_BCST_FROM => b.from_session_id = decode_gnuid(&child_view).ok(),
                    Id4::PCP_BCST_VERSION => b.version = decode_i32(&child_view).ok(),
                    Id4::PCP_BCST_VERSION_VP => b.version_vp = decode_i32(&child_view).ok(),
                    Id4::PCP_BCST_VERSION_EX_NUMBER => b.version_ex_number = decode_i16(&child_view).ok(),
                    Id4::PCP_BCST_VERSION_EX_PREFIX => b.version_ex_prefix = decode_vecu8(&child_view).ok(),
                    Id4::PCP_BCST_CHANID => b.channel_id = decode_gnuid(&child_view).ok(),
                    Id4::PCP_BCST_GROUP => b.broadcast_group = decode_u8(&child_view).ok().map(Into::into),
                    _ => {
                        warn!("unknown atom arrived :{:?}", &child_view)
                    }
                },
                Atom2Kind::Parent(parent_view) => match parent_view.id() {
                    Id4::PCP_CHAN => b.chan = ChanInfo::try_from(&parent_view).ok(),
                    Id4::PCP_HOST => b.host = HostInfo::try_from(&parent_view).ok(),
                    _ => {
                        warn!("unknown atom arrived :{:?}", &parent_view)
                    }
                },
            }
        }

        Ok(b)
    }
}

#[derive(Debug)]
pub struct BroadcastGroup(u8);
impl BroadcastGroup {
    pub const TO_ALL: BroadcastGroup = BroadcastGroup(0xFF);
    pub const TO_ROOT: BroadcastGroup = BroadcastGroup(0x01);
    pub const TO_TRACKERS: BroadcastGroup = BroadcastGroup(0x02);
    pub const TO_RELAYS: BroadcastGroup = BroadcastGroup(0x04);

    fn has(&self, other: &BroadcastGroup) -> bool {
        (self.0 & other.0) != 0
    }
    pub fn is_all(&self) -> bool {
        self.0 == Self::TO_ALL.0
    }
    pub fn has_root(&self) -> bool {
        self.has(&Self::TO_ROOT)
    }
    pub fn has_trackers(&self) -> bool {
        self.has(&Self::TO_TRACKERS)
    }
    pub fn has_relays(&self) -> bool {
        self.has(&Self::TO_RELAYS)
    }
}

impl From<u8> for BroadcastGroup {
    fn from(value: u8) -> Self {
        Self(value)
    }
}

// // トラッカーである自分からYPへの通知。
// void Channel::broadcastTrackerUpdate(const GnuID &svID, bool force /* = false */)
// https://github.com/plonk/peercast-yt/blob/787be6405cc2d82a5d26c0023aaa5d1973c13802/core/common/channel.cpp#L962C14-L962C14

// 恐らくTrackerからReleyへの通知
// if (isBroadcasting())
// https://github.com/plonk/peercast-yt/blob/787be6405cc2d82a5d26c0023aaa5d1973c13802/core/common/channel.cpp#L1052

// Tracker？
// https://github.com/plonk/peercast-yt/blob/787be6405cc2d82a5d26c0023aaa5d1973c13802/core/common/cstream.cpp#L72
// atom.writeChar(PCP_BCST_GROUP, PCP_BCST_GROUP_TRACKERS);

// RootからTrackerへ設定を伝えてる・・・？
// https://github.com/plonk/peercast-yt/blob/787be6405cc2d82a5d26c0023aaa5d1973c13802/core/common/servmgr.cpp#L1916
// void ServMgr::broadcastRootSettings(bool getUpdate)
// atom.writeChar(PCP_BCST_GROUP, PCP_BCST_GROUP_TRACKERS);
