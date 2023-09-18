use core::panic;
use std::{
    default,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
};

use bytes::Buf;
use tracing::warn;
use tracing_subscriber::field::debug;

use crate::pcp::{session::Session, Atom, GnuId, Id4};

pub struct HostBuilder {}

impl HostBuilder {
    pub fn new() -> Self {
        HostBuilder {}
    }

    pub fn build() -> Atom {
        let addr = Ipv4Addr::new(127, 0, 0, 1);
        let ip_u32: u32 = addr.into();
        todo!()
    }
}

#[derive(Debug, Clone)]
pub struct HostInfo {
    pub channel_id: Option<GnuId>,
    pub session_id: GnuId,

    pub local_address: Option<SocketAddr>,
    pub global_address: Option<SocketAddr>,

    pub relay_count: i32,
    pub listener_count: i32,

    pub uptime: i32,
    pub version: i32,
    pub version_vp: i32,
    pub version_extra: Option<VersionExtra>,

    // firewall, tracker, relay_full, direct_full, receiving, control_full
    pub flag1: u8,

    pub old_pos: u32,
    pub new_pos: u32,

    pub uphost: Option<(SocketAddr, Option<u32>)>, //(address, Hops)
}

#[derive(Debug, Clone)]
pub struct VersionExtra {
    pub prefix: [u8; 2],
    pub number: u16,
}

impl HostInfo {
    pub fn new(channel_id: Option<GnuId>, session_id: GnuId) -> Self {
        Self {
            channel_id,
            session_id,
            global_address: None,
            local_address: None, // インターフェイスのIPアドレスらしい？
            relay_count: 0,
            listener_count: 0,
            uptime: 0,
            version: 0,
            version_vp: 0,
            version_extra: None,
            flag1: 0,
            old_pos: 0,
            new_pos: 0,
            uphost: None,
        }
    }
}

impl HostInfo {
    pub fn parse(atom: &Atom) -> HostInfo {
        if atom.id() != Id4::PCP_HOST {
            panic!("not PCP_HOST")
        } else if atom.is_child() {
            panic!("not Parent")
        }

        let mut info = HostInfo::new(None, 0.into());
        let mut ips = vec![];
        let mut ports = vec![];
        let (mut extra_prefix, mut extra_number) = (None, None);
        let (mut up_ip, mut up_port, mut up_hops) = (None, None, None);
        for atom in atom.as_parent().childs() {
            match atom.id() {
                Id4::PCP_HOST_CHANID => info.channel_id = Some(get_gnuid(atom)),
                Id4::PCP_HOST_ID => info.session_id = get_gnuid(atom),
                Id4::PCP_HOST_IP => ips.push(get_ip(atom)),
                Id4::PCP_HOST_PORT => ports.push(get_u16(atom)),
                Id4::PCP_HOST_NUML => info.listener_count = get_i32(atom),
                Id4::PCP_HOST_NUMR => info.relay_count = get_i32(atom),
                Id4::PCP_HOST_UPTIME => info.uptime = get_i32(atom),
                Id4::PCP_HOST_VERSION => info.version = get_i32(atom),
                Id4::PCP_HOST_VERSION_VP => info.version_vp = get_i32(atom),
                Id4::PCP_HOST_VERSION_EX_PREFIX => extra_prefix = Some(get_2bytes(atom)),
                Id4::PCP_HOST_VERSION_EX_NUMBER => extra_number = Some(get_u16(atom)),
                Id4::PCP_HOST_FLAGS1 => info.flag1 = get_u8(atom),
                Id4::PCP_HOST_OLDPOS => info.old_pos = get_u32(atom),
                Id4::PCP_HOST_NEWPOS => info.new_pos = get_u32(atom),
                Id4::PCP_HOST_UPHOST_IP => up_ip = Some(get_ip(atom)),
                Id4::PCP_HOST_UPHOST_PORT => up_port = Some(get_u32(atom) as u16),
                Id4::PCP_HOST_UPHOST_HOPS => up_hops = Some(get_u32(atom)),
                _ => {
                    warn!("unknown atom is arrived");
                }
            }
        }
        match (ips.len(), ports.len()) {
            (2, 2) => {
                // pop() means get last element.
                info.local_address =
                    Some(SocketAddr::new(ips.pop().unwrap(), ports.pop().unwrap()));
                info.global_address =
                    Some(SocketAddr::new(ips.pop().unwrap(), ports.pop().unwrap()));
            }
            _ => {}
        }

        match (extra_prefix, extra_number) {
            (Some(prefix), Some(number)) => {
                info.version_extra = Some(VersionExtra { prefix, number })
            }
            (None, None) => {}
            _ => {
                warn!(
                    ?extra_prefix,
                    ?extra_number,
                    "PCP_HOST_VERSION_EX_* value have. but, something occur.",
                );
            }
        }

        match (up_ip, up_port, up_hops) {
            (Some(ip), Some(port), hops) => {
                info.uphost = Some((SocketAddr::from((ip, port)), hops))
            }
            (None, None, None) => {}
            _ => {
                warn!(
                    ?up_ip,
                    ?up_port,
                    ?up_hops,
                    "PCP_HOST_UPHOST_* value have. but, something occur.",
                );
            }
        };

        info
    }
}

fn get_gnuid(atom: &Atom) -> GnuId {
    debug_assert_eq!(atom.len(), 16);
    GnuId::from(atom.as_child().payload().get_u128())
}

fn get_ip(atom: &Atom) -> IpAddr {
    use std::net::{Ipv4Addr, Ipv6Addr};
    let c = atom.as_child();
    match c.len() {
        4 => Ipv4Addr::from(c.payload().get_u32_le()).into(),
        16 => Ipv6Addr::from(c.payload().get_u128()).into(),
        _ => panic!("not ipv4 or ipv6"),
    }
}

fn get_u8(atom: &Atom) -> u8 {
    debug_assert_eq!(atom.len(), 1);
    atom.as_child().payload().get_u8()
}

fn get_u16(atom: &Atom) -> u16 {
    debug_assert_eq!(atom.len(), 2);
    atom.as_child().payload().get_u16_le()
}
fn get_u32(atom: &Atom) -> u32 {
    debug_assert_eq!(atom.len(), 4);
    atom.as_child().payload().get_u32_le()
}

fn get_i32(atom: &Atom) -> i32 {
    debug_assert_eq!(atom.len(), 4);
    atom.as_child().payload().get_i32_le()
}

fn get_2bytes(atom: &Atom) -> [u8; 2] {
    debug_assert_eq!(atom.len(), 2);
    let mut b = atom.as_child().payload();
    //FIXME: 良い書き方が分らん
    [b.get_u8(), b.get_u8()]
}
