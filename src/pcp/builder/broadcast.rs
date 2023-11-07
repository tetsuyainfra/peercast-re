use crate::{
    error::AtomParseError,
    pcp::{atom::decode::BroadcastGroup, Atom, GnuId, Id4},
    PKG_SERVANT_VERSION, PKG_SERVANT_VERSION_EX_NUMBER, PKG_SERVANT_VERSION_EX_PREFIX,
    PKG_SERVANT_VERSION_VP,
};

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

    /// チャンネル情報をYPに通知する時に利用する
    pub fn to_yp_builder(session_id: GnuId, channel_id: GnuId) -> BroadcastBuilder {
        BroadcastBuilder::new(1, 0, session_id, channel_id, BroadcastGroup::TO_ROOT)
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
