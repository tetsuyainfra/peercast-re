use anyhow::Error;
use byteorder::{LittleEndian, ReadBytesExt};
use bytes::Buf;

use crate::pcp::{builder2::InfoParseError, Atom2, Atom2Kind, AtomMut, AtomView, Id4};

#[derive(Debug, Copy, Clone)]
#[repr(u32)]
pub enum IpMode {
    IpV4 = 1_u32,
    IpV6 = 100_u32,
}
pub struct MagicBuilder2 {
    ip_mode: IpMode,
}

impl MagicBuilder2 {
    fn new(ip_mode: IpMode) -> Self {
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
pub struct MagicInfo {
    pub ip_mode: IpMode,
}

impl TryFrom<&Atom2> for MagicInfo {
    type Error = InfoParseError;

    fn try_from(atom: &Atom2) -> Result<Self, Self::Error> {
        if atom.id() != Id4::PCP_CONNECT {
            return Err(InfoParseError::TargetNotFound);
        }

        let cv = match atom.view() {
            Atom2Kind::Parent(_) => return Err(InfoParseError::TargetNotFound),
            Atom2Kind::Child(cv) => cv,
        };

        let val = cv.data().read_u32::<LittleEndian>().map_err(|_| InfoParseError::InvalidPayload)?;
        let ip_mode = match val {
            1 => IpMode::IpV4,
            100 => IpMode::IpV6,
            v => return Err(InfoParseError::UnknownType),
        };

        Ok(MagicInfo {
            ip_mode,
        })
    }
}

#[cfg(test)]
mod t {
    use crate::pcp::builder2::IpMode;

    #[test]
    fn test_ip_mode() {
        assert_eq!(1_u32, IpMode::IpV4 as u32);
        assert_eq!(100_u32, IpMode::IpV6 as u32);
    }
}
