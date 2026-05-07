use peercast_atom::{Atom, AtomMut, AtomTryDecode, AtomView, KindView};
use peercast_id4::Id4;

use crate::{model::IpMode, pcp::builder::error::InfoParseError};

pub struct MagicBuilder {
    ip_mode: IpMode,
}

impl MagicBuilder {
    pub fn new(ip_mode: IpMode) -> Self {
        Self {
            ip_mode,
        }
    }

    pub fn build(&self) -> AtomMut {
        let ip_version_num = self.ip_mode as u32;
        let magic_atom = AtomMut::from((Id4::PCP_CONNECT, ip_version_num));
        magic_atom
    }
}

#[derive(Debug)]
pub struct MagicInfo<A> {
    pub ip_mode: IpMode,
    pub atom: A,
}

impl<'a> TryFrom<&'a Atom> for MagicInfo<&'a Atom> {
    type Error = InfoParseError;

    fn try_from(atom: &'a Atom) -> Result<Self, Self::Error> {
        if atom.id() != Id4::PCP_CONNECT {
            return Err(InfoParseError::TargetNotFound);
        }

        let ip_mode = match atom.view() {
            KindView::Parent(_) => return Err(InfoParseError::TargetNotFound),
            KindView::Child(cv) => match cv.try_decode_i32() {
                Some(1) => IpMode::IpV4,
                Some(100) => IpMode::IpV6,
                Some(_v) => {
                    return Err(InfoParseError::UnknownValueType);
                }
                None => return Err(InfoParseError::UnknownValueType),
            },
        };

        Ok(MagicInfo {
            ip_mode,
            atom,
        })
    }
}

impl<'a> TryFrom<Atom> for MagicInfo<Atom> {
    type Error = InfoParseError;

    fn try_from(atom: Atom) -> Result<Self, Self::Error> {
        let MagicInfo {
            ip_mode,
            ..
        } = MagicInfo::try_from(&atom)?;

        Ok(MagicInfo {
            ip_mode,
            atom,
        })
    }
}

#[cfg(test)]
mod t {
    use super::*;
}
