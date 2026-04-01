use std::net::{IpAddr, Ipv4Addr};

use bytes::Buf;

use crate::{
    error::Atom2ParseError,
    pcp::{
        atom2::{AtomView, ChildView},
        GnuId,
    },
};

#[inline]
pub fn len_check(atom: &ChildView<'_>, length: u32) -> Result<(), Atom2ParseError> {
    if atom.length() != length {
        return Err(Atom2ParseError::InvalidFormat);
    }
    Ok(())
}

/// Payload(i8) to i8
pub fn decode_i8(atom: &ChildView<'_>) -> Result<i8, Atom2ParseError> {
    len_check(atom, 1)?;
    let v = atom.payload().get_i8();
    Ok(v)
}

/// Payload(u8) to u8
pub fn decode_u8(atom: &ChildView<'_>) -> Result<u8, Atom2ParseError> {
    len_check(atom, 1)?;
    let v = atom.payload().get_u8();
    Ok(v)
}

/// Payload(i16LE) to i16
pub fn decode_i16(atom: &ChildView<'_>) -> Result<i16, Atom2ParseError> {
    len_check(atom, 2)?;
    let v = atom.payload().get_i16_le(); // LE
    Ok(v)
}

/// Payload(u16LE) to u16
pub fn decode_u16(atom: &ChildView<'_>) -> Result<u16, Atom2ParseError> {
    len_check(atom, 2)?;
    let v = atom.payload().get_u16_le(); // LE
    Ok(v)
}

/// Payload(i32LE) to i32
pub fn decode_i32(atom: &ChildView<'_>) -> Result<i32, Atom2ParseError> {
    len_check(atom, 4)?;
    let v = atom.payload().get_i32_le(); // LE
    Ok(v)
}

/// Payload(u32LE) to u32
pub fn decode_u32(atom: &ChildView<'_>) -> Result<u32, Atom2ParseError> {
    len_check(atom, 4)?;
    let v = atom.payload().get_u32_le(); // LE
    Ok(v)
}

/// Payload(GnuID[u8; 16]) to GnuId
pub fn decode_gnuid(atom: &ChildView<'_>) -> Result<GnuId, Atom2ParseError> {
    len_check(atom, 16)?;
    let v = atom.payload().get_u128(); // BE
    Ok(GnuId::from(v))
}

/// Payload(utf-8[u8; X]) to GnuId
pub fn decode_vecu8(atom: &ChildView<'_>) -> Result<Vec<u8>, Atom2ParseError> {
    let b = atom.payload();
    Ok(b.to_vec())
}

// /// Payload(IpV4 or IpV6) to GnuId
pub fn decode_ip(atom: &ChildView<'_>) -> Result<IpAddr, Atom2ParseError> {
    if len_check(atom, 4).is_ok() {
        // IPv4
        let ip_u32 = atom.payload().get_u32_le();
        let ip = Ipv4Addr::from(ip_u32);
        Ok(ip.into())
    } else if len_check(atom, 6).is_ok() {
        // IPv6
        todo!()
    } else {
        Err(Atom2ParseError::MalformedData)
    }
}
