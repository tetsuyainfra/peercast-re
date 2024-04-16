use std::net::{IpAddr, SocketAddr};

use axum::extract::Host;
use bytes::Bytes;
use tracing::warn;

use crate::{
    error::AtomParseError,
    pcp::{
        decode::{
            decode_bytes, decode_gnuid, decode_i16, decode_i32, decode_ip, decode_u16, decode_u32,
            decode_u8,
        },
        Atom, GnuId, Id4,
    },
};

// とりあえず実装しなくてもよさそう
#[derive(Debug, Clone, Copy)]
pub struct HostFlags1(pub u8);

impl HostFlags1 {
    pub const IS_TRACKER: HostFlags1 = HostFlags1(0x01);
    pub const IS_RELAY: HostFlags1 = HostFlags1(0x02);
    pub const IS_DIRECT: HostFlags1 = HostFlags1(0x04);
    pub const IS_FIREWALLED: HostFlags1 = HostFlags1(0x08);
    pub const IS_RECV: HostFlags1 = HostFlags1(0x10);
    pub const IS_CIN: HostFlags1 = HostFlags1(0x20);
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

#[derive(Debug, Clone, Default)]
pub struct PcpHost {
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
    pub version_ex_prefix: Option<Bytes>,
    pub version_ex_number: Option<i16>,
    //
    pub flags1: Option<u8>,
    //
    pub uphost_ip: Option<IpAddr>,
    pub uphost_port: Option<i32>, // u16で足りるはずだけどミスって実装されている模様
    pub uphost_hops: Option<i32>, // u8で足りるはずだけどミスって実装されている模様
}

impl PcpHost {
    pub fn parse(atom: &Atom) -> Result<PcpHost, AtomParseError> {
        if !(atom.id() == Id4::PCP_HOST && atom.is_parent()) {
            return Err(AtomParseError::NotFoundValue);
        }
        let mut p = PcpHost::default();
        let mut ip = None;
        let mut addrs = vec![];
        for a in atom.as_parent().childs() {
            if a.is_child() {
                let a = a.as_child();
                match a.id() {
                    Id4::PCP_HOST_CHANID => p.channel_id = Some(decode_gnuid(a)?),
                    Id4::PCP_HOST_ID => p.session_id = Some(decode_gnuid(a)?),
                    Id4::PCP_HOST_IP => ip = Some(decode_ip(a)?),
                    Id4::PCP_HOST_PORT => {
                        if ip.is_some() {
                            let ip = ip.take().unwrap();
                            let port = decode_u16(a)?;
                            addrs.push((ip, port).into())
                        }
                    }
                    Id4::PCP_HOST_NUML => p.number_listener = Some(decode_i32(a)?),
                    Id4::PCP_HOST_NUMR => p.number_relay = Some(decode_i32(a)?),
                    Id4::PCP_HOST_UPTIME => p.uptime = Some(decode_i32(a)?),
                    Id4::PCP_HOST_VERSION => p.version = Some(decode_i32(a)?),
                    Id4::PCP_HOST_VERSION_VP => p.version_vp = Some(decode_i32(a)?),
                    Id4::PCP_HOST_VERSION_EX_PREFIX => p.version_ex_prefix = Some(decode_bytes(a)?),
                    Id4::PCP_HOST_VERSION_EX_NUMBER => p.version_ex_number = Some(decode_i16(a)?),
                    Id4::PCP_HOST_FLAGS1 => p.flags1 = Some(decode_u8(a)?),
                    Id4::PCP_HOST_UPHOST_IP => p.uphost_ip = Some(decode_ip(a)?),
                    Id4::PCP_HOST_UPHOST_PORT => p.uphost_port = Some(decode_i32(a)?),
                    Id4::PCP_HOST_UPHOST_HOPS => p.uphost_hops = Some(decode_i32(a)?),
                    _ => {
                        warn!("unkown atom arrived :{:?}", &a)
                    }
                }
            }
        }
        p.addresses = addrs;

        Ok(p)
    }
}

#[cfg(test)]
mod t {
    use super::HostFlags1;

    #[test]
    fn test_flag1() {
        let f = HostFlags1::new();
        assert_eq!(f.has_recv(), false);
        assert_eq!(f.has_relay(), false);
        assert_eq!(f.has_direct(), false);
        assert_eq!(f.has_cin(), false);
        assert_eq!(f.has_tracker(), false);
        assert_eq!(f.has_firewalled(), false);
        //
        let f = f.set_firewalled(true);
        assert_eq!(f.has_recv(), false);
        assert_eq!(f.has_relay(), false);
        assert_eq!(f.has_direct(), false);
        assert_eq!(f.has_cin(), false);
        assert_eq!(f.has_tracker(), false);
        assert_eq!(f.has_firewalled(), true);
        //
        let f = f.set_firewalled(false);
        assert_eq!(f.has_firewalled(), false);
        // call twice
        let f = f.set_firewalled(false);
        assert_eq!(f.has_firewalled(), false);
    }
}
