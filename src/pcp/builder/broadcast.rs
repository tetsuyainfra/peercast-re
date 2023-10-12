use crate::{
    pcp::{Atom, GnuId, Id4},
    PKG_SERVANT_VERSION, PKG_SERVANT_VERSION_EX_NUMBER, PKG_SERVANT_VERSION_EX_PREFIX,
    PKG_SERVANT_VERSION_VP,
};

#[derive(Debug)]
pub struct BroadcastGroup(u8);
impl BroadcastGroup {
    const TO_ALL: BroadcastGroup = BroadcastGroup(0xFF);
    const TO_ROOT: BroadcastGroup = BroadcastGroup(0x01);
    const TO_TRACKERS: BroadcastGroup = BroadcastGroup(0x02);
    const TO_RELAYS: BroadcastGroup = BroadcastGroup(0x04);
    fn has(&self, other: &BroadcastGroup) -> bool {
        (self.0 & other.0) != 0
    }
    fn is_all(&self) -> bool {
        self.0 == Self::TO_ALL.0
    }
    fn has_root(&self) -> bool {
        self.has(&Self::TO_ROOT)
    }
    fn has_trackers(&self) -> bool {
        self.has(&Self::TO_TRACKERS)
    }
    fn has_relays(&self) -> bool {
        self.has(&Self::TO_RELAYS)
    }
}

pub struct BroadcastBuilder {
    ttl: u8,
    hops: u8,
    //
    from_session_id: GnuId,
    channel_id: GnuId,
    //
    broadcast_group: BroadcastGroup,
}

impl BroadcastBuilder {
    fn new(
        ttl: u8,
        hops: u8,
        from_session_id: GnuId,
        channel_id: GnuId,
        broadcast_group: BroadcastGroup,
    ) -> Self {
        Self {
            ttl,
            hops,
            from_session_id,
            channel_id,
            broadcast_group,
        }
    }

    fn build(mut self) -> Atom {
        let mut vec = vec![];
        vec.push(Atom::Child((Id4::PCP_BCST_TTL, self.ttl).into()));
        vec.push(Atom::Child((Id4::PCP_BCST_HOPS, self.hops).into()));
        vec.push(Atom::Child(
            (Id4::PCP_BCST_FROM, self.from_session_id).into(),
        ));

        // Versions
        // PCPVersion.SetBcstVersion(bcst);
        vec.push(Atom::Child(
            (Id4::PCP_BCST_VERSION, PKG_SERVANT_VERSION).into(),
        )); // 1218
        vec.push(Atom::Child(
            (Id4::PCP_BCST_VERSION_VP, PKG_SERVANT_VERSION_VP).into(),
        )); // 27
        vec.push(Atom::Child(
            (
                Id4::PCP_BCST_VERSION_EX_PREFIX,
                &PKG_SERVANT_VERSION_EX_PREFIX,
            )
                .into(),
        ));
        vec.push(Atom::Child(
            (
                Id4::PCP_BCST_VERSION_EX_NUMBER,
                *PKG_SERVANT_VERSION_EX_NUMBER,
            )
                .into(),
        ));
        // bcst.SetBcstVersion(ServantVersion);
        // bcst.SetBcstVersionVP(ServantVersionVP);
        // bcst.SetBcstVersionEXPrefix(ServantVersionEXPrefix);
        // bcst.SetBcstVersionEXNumber(ServantVersionEXNumber);

        vec.push(Atom::Child((Id4::PCP_BCST_CHANID, self.channel_id).into()));

        // bcst.SetBcstFrom(channel.PeerCast.SessionID);
        // bcst.SetBcstChannelID(channel.ChannelID);
        // bcst.SetBcstGroup(BroadcastGroup.Root);
        // PostChannelInfo(bcst, channel);
        // PostHostInfo(bcst, channel, playing);
        Atom::Parent((Id4::PCP_BCST, vec).into())
    }
}

/// チャンネル情報をYPに通知する時に利用する
pub fn broadcast_yp_builder(session_id: GnuId, channel_id: GnuId) -> BroadcastBuilder {
    BroadcastBuilder::new(1, 0, session_id, channel_id, BroadcastGroup::TO_ROOT)
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

#[cfg(test)]
mod t {
    use super::BroadcastGroup;

    #[test]
    fn test_groups() {
        let g = BroadcastGroup::TO_ALL;
        assert_eq!(g.is_all(), true);
        assert_eq!(g.has_root(), true);
        assert_eq!(g.has_trackers(), true);
        assert_eq!(g.has_relays(), true);

        let g = BroadcastGroup::TO_ROOT;
        assert_eq!(g.is_all(), false);
        assert_eq!(g.has_root(), true);
        assert_eq!(g.has_trackers(), false);
        assert_eq!(g.has_relays(), false);

        let g = BroadcastGroup::TO_RELAYS;
        assert_eq!(g.is_all(), false);
        assert_eq!(g.has_root(), true);
        assert_eq!(g.has_trackers(), false);
        assert_eq!(g.has_relays(), false);
    }
}
