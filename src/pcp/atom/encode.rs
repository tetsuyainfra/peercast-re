//! encode Native Literal to Atom

use std::net::IpAddr;

use bytes::Bytes;

use crate::pcp::{GnuId, Id4};

use super::{Atom, ChildAtom, ParentAtom};

//--------------------------------------------------------------------------------
// From<>  for Atom
//
impl From<ChildAtom> for Atom {
    fn from(value: ChildAtom) -> Self {
        Self::Child(value)
    }
}
impl From<ParentAtom> for Atom {
    fn from(value: ParentAtom) -> Self {
        Self::Parent(value)
    }
}

//--------------------------------------------------------------------------------
// From<>  for ParentAtom
//
impl From<(Id4, Vec<Atom>)> for ParentAtom {
    fn from((id, childs): (Id4, Vec<Atom>)) -> Self {
        ParentAtom { id, childs }
    }
}

//--------------------------------------------------------------------------------
// From<> for ChildArom
//

impl From<(Id4, u8)> for ChildAtom {
    fn from((id, value): (Id4, u8)) -> Self {
        let payload = Bytes::copy_from_slice(&value.to_le_bytes()); // LE
        debug_assert_eq!(payload.len(), 1);
        Self::new(id, &payload)
    }
}
impl From<(Id4, u16)> for ChildAtom {
    fn from((id, value): (Id4, u16)) -> Self {
        let payload = Bytes::copy_from_slice(&value.to_le_bytes()); // LE
        debug_assert_eq!(payload.len(), 2);
        Self::new(id, &payload)
    }
}

impl From<(Id4, i32)> for ChildAtom {
    fn from((id, value): (Id4, i32)) -> Self {
        let payload = Bytes::copy_from_slice(&value.to_le_bytes()); // LE
        debug_assert_eq!(payload.len(), 4);
        Self::new(id, &payload)
    }
}

impl From<(Id4, u32)> for ChildAtom {
    fn from((id, value): (Id4, u32)) -> Self {
        let payload = Bytes::copy_from_slice(&value.to_le_bytes()); // LE
        debug_assert_eq!(payload.len(), 4);
        Self::new(id, &payload)
    }
}
impl From<(Id4, GnuId)> for ChildAtom {
    fn from((id, gnu_id): (Id4, GnuId)) -> Self {
        let value_u128: u128 = gnu_id.0;
        let payload = Bytes::copy_from_slice(&value_u128.to_be_bytes()); // BigEndianみたいですねぇ
        debug_assert_eq!(payload.len(), 16);
        Self::new(id, &payload)
    }
}

impl From<(Id4, IpAddr)> for ChildAtom {
    fn from((id, ip): (Id4, IpAddr)) -> Self {
        let payload = match ip {
            IpAddr::V4(ip) => {
                // LEでエンコードする
                let bytes = Into::<u32>::into(ip).to_le_bytes();
                Bytes::copy_from_slice(&bytes)
            }
            IpAddr::V6(ip) => {
                // BEでエンコードする？
                let bytes = Into::<u128>::into(ip).to_be_bytes();
                Bytes::copy_from_slice(&bytes)
            }
        };
        Self::new(id, &payload)
    }
}

impl From<(Id4, String)> for ChildAtom {
    fn from((id, s): (Id4, String)) -> Self {
        let payload = Bytes::from(s + "\0");
        Self::new(id, &payload)
    }
}

impl From<(Id4, Bytes)> for ChildAtom {
    fn from((id, payload): (Id4, Bytes)) -> Self {
        Self::new(id, &payload)
    }
}
impl From<(Id4, &Bytes)> for ChildAtom {
    fn from((id, payload): (Id4, &Bytes)) -> Self {
        Self::new(id, payload)
    }
}
