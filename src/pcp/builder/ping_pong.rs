use bytes::Buf;

use crate::{
    error::AtomParseError,
    pcp::{atom::decode::decode_gnuid, Atom, GnuId, Id4},
};

////////////////////////////////////////////////////////////////////////////////
//  Ping
//

#[derive(Debug)]
pub struct PingBuilder {
    self_session_id: GnuId,
}

impl PingBuilder {
    pub fn new(self_session_id: GnuId) -> Self {
        Self { self_session_id }
    }

    pub fn build(self) -> Vec<Atom> {
        let magic_atom = Atom::Child((Id4::PCP_CONNECT, 1_u32).into());

        let session_atom = Atom::Child((Id4::PCP_HELO_SESSIONID, self.self_session_id).into());
        let ping_atom = Atom::Parent((Id4::PCP_HELO, vec![session_atom]).into());

        vec![magic_atom, ping_atom]
    }
}

#[derive(Debug)]
pub struct PingInfo {
    pub session_id: GnuId,
}

impl PingInfo {
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

        Ok(PingInfo {
            session_id: GnuId::from(session_id.unwrap()),
        })
    }
}

////////////////////////////////////////////////////////////////////////////////
//  Pong
//

#[derive(Debug)]
pub struct PongBuilder {
    session_id: GnuId,
}

impl PongBuilder {
    pub fn new(session_id: GnuId) -> Self {
        Self { session_id }
    }

    pub fn build(self) -> Atom {
        let mut vec: Vec<Atom> = Vec::new();
        vec.push(Atom::Child(
            (Id4::PCP_HELO_SESSIONID, self.session_id).into(),
        ));

        Atom::Parent((Id4::PCP_OLEH, vec).into())
    }
}

#[derive(Debug)]
pub struct PongInfo {
    pub session_id: GnuId,
}

impl PongInfo {
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

        Ok(PongInfo {
            session_id: GnuId::from(session_id.unwrap()),
        })
    }
}
