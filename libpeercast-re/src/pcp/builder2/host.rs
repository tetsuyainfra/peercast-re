use std::net::{IpAddr, SocketAddr};

use tracing::warn;

use crate::pcp::{
    atom2::{
        decode2::{decode_gnuid, decode_i16, decode_i32, decode_ip, decode_u16, decode_u8, decode_vecu8},
        Atom2Kind, AtomView, Kind, ParentView,
    },
    builder2::ParseError,
    Atom2, GnuId, Id4,
};

#[derive(Debug, Default)]
pub struct HostInfo {
    /// Channel ID
    pub channel_id: Option<GnuId>,
    /// わからん
    pub session_id: Option<GnuId>,

    /// 外部でどうにかする方が良いか？
    pub addresses: Vec<SocketAddr>,

    pub number_listener: Option<i32>,
    pub number_relay: Option<i32>,

    pub uptime: Option<i32>,

    pub version: Option<i32>,
    pub version_vp: Option<i32>,
    pub version_ex_prefix: Option<Vec<u8>>,
    pub version_ex_number: Option<i16>,
    //
    pub flags1: Option<u8>,
    //
    pub uphost_ip: Option<IpAddr>,
    pub uphost_port: Option<i32>, // u16で足りるはずだけどミスって実装されている模様
    pub uphost_hops: Option<i32>, // u8で足りるはずだけどミスって実装されている模様

    pub pos_old: Option<i32>,
    pub pos_new: Option<i32>,
}

impl TryFrom<&ParentView<'_>> for HostInfo {
    type Error = ParseError;

    fn try_from(parent_view: &ParentView<'_>) -> Result<Self, Self::Error> {
        if (parent_view.id() != Id4::PCP_HOST) {
            return Err(ParseError::TargetNotFound);
        }

        let mut h = HostInfo::default();
        let mut ip = None;
        let mut addrs = Vec::new();

        for child in parent_view.children() {
            match child {
                Atom2Kind::Parent(parent_view) => {
                    continue;
                }
                Atom2Kind::Child(cv) => {
                    //
                    match cv.id() {
                        Id4::PCP_HOST_CHANID => h.channel_id = decode_gnuid(&cv).ok(),
                        Id4::PCP_HOST_ID => h.session_id = decode_gnuid(&cv).ok(),
                        Id4::PCP_HOST_IP => ip = decode_ip(&cv).ok(),
                        Id4::PCP_HOST_PORT => {
                            let Some(port) = decode_u16(&cv).ok() else {
                                continue;
                            };
                            if let Some(ip_) = ip.take() {
                                addrs.push(SocketAddr::new(ip_, port));
                            }
                        }
                        Id4::PCP_HOST_NUML => h.number_listener = decode_i32(&cv).ok(),
                        Id4::PCP_HOST_NUMR => h.number_relay = decode_i32(&cv).ok(),
                        Id4::PCP_HOST_UPTIME => h.uptime = decode_i32(&cv).ok(),
                        Id4::PCP_HOST_VERSION => h.version = decode_i32(&cv).ok(),
                        Id4::PCP_HOST_VERSION_VP => h.version_vp = decode_i32(&cv).ok(),
                        Id4::PCP_HOST_VERSION_EX_PREFIX => h.version_ex_prefix = decode_vecu8(&cv).ok(),
                        Id4::PCP_HOST_VERSION_EX_NUMBER => h.version_ex_number = decode_i16(&cv).ok(),
                        Id4::PCP_HOST_FLAGS1 => h.flags1 = decode_u8(&cv).ok(),
                        Id4::PCP_HOST_UPHOST_IP => h.uphost_ip = decode_ip(&cv).ok(),
                        Id4::PCP_HOST_UPHOST_PORT => h.uphost_port = decode_i32(&cv).ok(),
                        Id4::PCP_HOST_UPHOST_HOPS => h.uphost_hops = decode_i32(&cv).ok(),
                        //
                        Id4::PCP_HOST_OLDPOS => h.pos_old = decode_i32(&cv).ok(),
                        Id4::PCP_HOST_NEWPOS => h.pos_new = decode_i32(&cv).ok(),
                        // TODO: Id4::PCP_HOST_OLDPOS =>
                        // TODO: Id4::PCP_HOST_NEWPOS =>
                        _ => {
                            warn!("unkown atom arrived :{:?}", &cv);
                            continue;
                        }
                    }
                }
            }
        }
        h.addresses = addrs;

        Ok(h)
    }
}

impl TryFrom<Atom2> for HostInfo {
    type Error = ParseError;

    fn try_from(atom: Atom2) -> Result<Self, Self::Error> {
        if (atom.id() != Id4::PCP_HOST) {
            return Err(ParseError::TargetNotFound);
        }

        let Atom2Kind::Parent(parent_view) = atom.view() else {
            return Err(ParseError::TargetNotFound);
        };

        HostInfo::try_from(&parent_view)
    }
}

// とりあえず実装しなくてもよさそう
#[derive(Debug, Clone, Copy)]
pub struct HostFlags1(pub u8);

impl HostFlags1 {
    /// 自分がトラッカーである
    pub const IS_TRACKER: HostFlags1 = HostFlags1(0x01);
    /// リレー接続が可能である
    pub const IS_RELAY: HostFlags1 = HostFlags1(0x02);
    /// 視聴接続が可能である
    pub const IS_DIRECT: HostFlags1 = HostFlags1(0x04);
    /// ポートが開いてない
    pub const IS_FIREWALLED: HostFlags1 = HostFlags1(0x08);
    /// データ受信中である
    pub const IS_RECV: HostFlags1 = HostFlags1(0x10);
    /// コントロール接続が可能である(Rootのみ)
    pub const IS_CIN: HostFlags1 = HostFlags1(0x20);

    //
    pub const IS_PRIVATE: HostFlags1 = HostFlags1(0x40);
    pub const NONE: HostFlags1 = HostFlags1(0x00);

    fn new() -> Self {
        Self::NONE.clone()
    }

    fn has(&self, other: &HostFlags1) -> bool {
        (self.0 & other.0) != 0
    }

    pub fn has_recv(&self) -> bool {
        self.has(&Self::IS_RECV)
    }
    pub fn has_relay(&self) -> bool {
        self.has(&Self::IS_RELAY)
    }
    pub fn has_direct(&self) -> bool {
        self.has(&Self::IS_DIRECT)
    }
    pub fn has_cin(&self) -> bool {
        self.has(&Self::IS_CIN)
    }
    pub fn has_tracker(&self) -> bool {
        self.has(&Self::IS_TRACKER)
    }
    pub fn has_firewalled(&self) -> bool {
        self.has(&Self::IS_FIREWALLED)
    }

    fn set(&mut self, other: &HostFlags1, flag: bool) {
        if flag {
            self.0 = self.0 | other.0;
        } else {
            self.0 = self.0 & !other.0;
        }
    }

    pub fn set_recv(mut self, flag: bool) -> Self {
        self.set(&Self::IS_RECV, flag);
        self
    }
    pub fn set_relay(mut self, flag: bool) -> Self {
        self.set(&Self::IS_RELAY, flag);
        self
    }
    pub fn set_direct(mut self, flag: bool) -> Self {
        self.set(&Self::IS_DIRECT, flag);
        self
    }
    pub fn set_cin(mut self, flag: bool) -> Self {
        self.set(&Self::IS_CIN, flag);
        self
    }
    pub fn set_tracker(mut self, flag: bool) -> Self {
        self.set(&Self::IS_TRACKER, flag);
        self
    }
    pub fn set_firewalled(mut self, flag: bool) -> Self {
        self.set(&Self::IS_FIREWALLED, flag);
        self
    }
}
