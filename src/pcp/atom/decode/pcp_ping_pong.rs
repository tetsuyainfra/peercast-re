use crate::{
    error::AtomParseError,
    pcp::{Atom, GnuId, Id4},
};

use super::decode_gnuid;

#[derive(Debug)]
pub struct PcpPing {
    pub session_id: GnuId,
}

impl PcpPing {
    pub fn parse(atom: &Atom) -> Result<Self, AtomParseError> {
        if !(atom.id() == Id4::PCP_HELO && atom.is_parent()) {
            return Err(AtomParseError::IdError);
        }
        let mut session_id = None;
        for a in atom.as_parent().childs() {
            if a.is_parent() {
                return Err(AtomParseError::ValueError);
            }
            let a = a.as_child();
            match a.id() {
                Id4::PCP_SESSIONID => session_id = Some(decode_gnuid(a)?),
                _ => {}
            }
        }
        if session_id.is_none() {
            return Err(AtomParseError::NotFoundValue);
        }

        Ok(Self {
            session_id: GnuId::from(session_id.unwrap()),
        })
    }
}

#[derive(Debug)]
pub struct PcpPong {
    pub session_id: GnuId,
}

impl PcpPong {
    pub fn parse(atom: &Atom) -> Result<Self, AtomParseError> {
        if !(atom.id() == Id4::PCP_OLEH && atom.is_parent()) {
            return Err(AtomParseError::IdError);
        }

        let mut session_id = None;
        for a in atom.as_parent().childs() {
            if a.is_parent() {
                return Err(AtomParseError::ValueError);
            }
            let a = a.as_child();
            match a.id() {
                Id4::PCP_HELO_SESSIONID => session_id = Some(decode_gnuid(a)?),
                _ => {}
            }
        }
        if session_id.is_none() {
            return Err(AtomParseError::NotFoundValue);
        }

        Ok(Self {
            session_id: GnuId::from(session_id.unwrap()),
        })
    }
}
