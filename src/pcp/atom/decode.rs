//! encode Atom to Native Literal

mod macros;
mod pcp_broadcast;
mod pcp_channel;
mod pcp_channel_info;
mod pcp_host;
mod pcp_track_info;

use std::net::{IpAddr, Ipv4Addr};

use bytes::{Buf, Bytes};

use crate::{error::AtomParseError, pcp::GnuId};

use super::ChildAtom;

pub use pcp_broadcast::{BroadcastGroup, PcpBroadcast};
pub use pcp_channel::PcpChannel;
pub use pcp_channel_info::PcpChannelInfo;
pub use pcp_host::PcpHost;
pub use pcp_track_info::PcpTrackInfo;

#[inline]
pub fn len_check(atom: &ChildAtom, length: u32) -> Result<(), AtomParseError> {
    if atom.len() != length {
        return Err(AtomParseError::ValueError);
    }
    Ok(())
}

/// Payload(i8) to i8
pub fn decode_i8(atom: &ChildAtom) -> Result<i8, AtomParseError> {
    len_check(atom, 1)?;
    let v = atom.payload().get_i8();
    Ok(v)
}

/// Payload(u8) to u8
pub fn decode_u8(atom: &ChildAtom) -> Result<u8, AtomParseError> {
    len_check(atom, 1)?;
    let v = atom.payload().get_u8();
    Ok(v)
}

/// Payload(i16LE) to i16
pub fn decode_i16(atom: &ChildAtom) -> Result<i16, AtomParseError> {
    len_check(atom, 2)?;
    let v = atom.payload().get_i16_le(); // LE
    Ok(v)
}

/// Payload(u16LE) to u16
pub fn decode_u16(atom: &ChildAtom) -> Result<u16, AtomParseError> {
    len_check(atom, 2)?;
    let v = atom.payload().get_u16_le(); // LE
    Ok(v)
}

/// Payload(i32LE) to i32
pub fn decode_i32(atom: &ChildAtom) -> Result<i32, AtomParseError> {
    len_check(atom, 4)?;
    let v = atom.payload().get_i32_le(); // LE
    Ok(v)
}

/// Payload(u32LE) to u32
pub fn decode_u32(atom: &ChildAtom) -> Result<u32, AtomParseError> {
    len_check(atom, 4)?;
    let v = atom.payload().get_u32_le(); // LE
    Ok(v)
}

/// Payload(GnuID[u8; 16]) to GnuId
pub fn decode_gnuid(atom: &ChildAtom) -> Result<GnuId, AtomParseError> {
    len_check(atom, 16)?;
    let v = atom.payload().get_u128(); // BE
    Ok(GnuId::from(v))
}

/// Payload(utf-8[u8; X]) to GnuId
pub fn decode_string(atom: &ChildAtom) -> Result<String, AtomParseError> {
    let b = atom.payload();

    let mut s = String::from_utf8_lossy(&b).to_string();
    // 末尾が\0以外の時だけ元に戻せばよい
    match s.pop() {
        None => {}
        Some('\0') => {}
        Some(other) => s.push(other),
    }
    Ok(s)
}

// Payload([u8; N]) to Bytes
pub fn decode_bytes(atom: &ChildAtom) -> Result<Bytes, AtomParseError> {
    let b = atom.payload();
    Ok(b)
}

/// Payload(IpV4 or IpV6) to GnuId
pub fn decode_ip(atom: &ChildAtom) -> Result<IpAddr, AtomParseError> {
    if len_check(atom, 4).is_ok() {
        // IPv4
        let ip_u32 = atom.payload().get_u32_le();
        let ip = Ipv4Addr::from(ip_u32);
        Ok(ip.into())
    } else if len_check(atom, 6).is_ok() {
        // IPv6
        todo!()
    } else {
        Err(AtomParseError::ValueError)
    }
}
