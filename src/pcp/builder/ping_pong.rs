use bytes::Buf;

use crate::{
    error::AtomParseError,
    pcp::{classify::get_by_id, Atom, GnuId, Id4},
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
        if atom.id() != Id4::PCP_HELO {
            return Err(AtomParseError::IdError);
        }
        let p = match atom {
            Atom::Child(_) => return Err(AtomParseError::NotFoundValue),
            Atom::Parent(p) => p,
        };
        let Some(id_atom) = get_by_id(Id4::PCP_SESSIONID, p.childs()) else {
            return Err(AtomParseError::NotFoundValue);
        };
        let session_id = id_atom.as_child().payload().get_u128();

        Ok(PingInfo {
            session_id: GnuId::from(session_id),
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
        if atom.id() != Id4::PCP_OLEH {
            return Err(AtomParseError::IdError);
        }
        let p = match atom {
            Atom::Child(_) => return Err(AtomParseError::NotFoundValue),
            Atom::Parent(p) => p,
        };
        let Some(id_atom) = get_by_id(Id4::PCP_HELO_SESSIONID, p.childs()) else {
            return Err(AtomParseError::NotFoundValue);
        };
        let session_id = id_atom.as_child().payload().get_u128();

        Ok(PongInfo {
            session_id: GnuId::from(session_id),
        })
    }
}
