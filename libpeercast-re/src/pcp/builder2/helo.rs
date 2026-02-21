use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    str,
};

use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use bytes::Buf;
use futures_util::SinkExt;

use crate::pcp::{
    atom2::{Atom2Kind, AtomView},
    builder2::{self, ParseError},
    Atom2, AtomMut, GnuId, Id4,
};
use crate::prelude::*;

pub struct HeloBuilder2 {
    agent: Option<String>,
    version: Option<u32>,
    session_id: GnuId,
    broadcast_id: Option<GnuId>,
    os_type: Option<i32>,
    port: Option<u16>,
    port_check: Option<u16>,
}

impl HeloBuilder2 {
    fn new(session_id: GnuId) -> Self {
        Self {
            agent: None,
            version: None,
            session_id,
            broadcast_id: None,
            os_type: None,
            port: None,
            port_check: None,
        }
    }
    pub fn broadcast_id(mut self, broadcast_id: GnuId) -> Self {
        self.broadcast_id = Some(broadcast_id);
        self
    }

    // 解放が確認されているポート番号
    // Confirmed Open Port Number
    pub fn port(mut self, open_confiremed_port_no: u16) -> Self {
        self.port = Some(open_confiremed_port_no);
        self
    }

    // 相手に解放してるか確認してほしいポート番号
    // I would like you to confirm Port Number as Open
    pub fn port_check(mut self, ping_port_no: u16) -> Self {
        self.port_check = Some(ping_port_no);
        self
    }

    pub fn build(&self) -> AtomMut {
        let mut vec = Vec::with_capacity(7);

        vec.push(builder2::AGENT.clone().into());
        vec.push(builder2::VERSION.clone().into());

        vec.push(AtomMut::from((Id4::PCP_HELO_SESSIONID, self.session_id)));

        if let Some(port) = self.port {
            vec.push(AtomMut::from((Id4::PCP_HELO_PORT, port)));
        }

        if let Some(port_check) = self.port_check {
            // portはu16だがAtomパケットではu32で送られる・・・
            vec.push(AtomMut::from((Id4::PCP_HELO_PING, port_check)))
        }

        if let Some(bcid) = self.broadcast_id {
            vec.push(AtomMut::from((Id4::PCP_HELO_BCID, bcid)));
        }

        AtomMut::from((Id4::PCP_HELO, vec))
    }
}

#[derive(Debug, Default)]
pub struct HeloInfo {
    pub agent: Option<String>,
    pub version: Option<u32>,
    pub session_id: Option<GnuId>,
    pub broadcast_id: Option<GnuId>,
    pub port: Option<u16>,
    pub port_check: Option<u16>,
    pub os_type: i32,
}

impl TryFrom<&Atom2> for HeloInfo {
    type Error = ParseError;

    fn try_from(atom: &Atom2) -> Result<Self, Self::Error> {
        if atom.id() != Id4::PCP_HELO {
            return Err(ParseError::TargetNotFound);
        }

        let pv = match atom.view() {
            Atom2Kind::Parent(pv) => pv,
            Atom2Kind::Child(_) => return Err(ParseError::TargetNotFound),
        };

        let mut helo = HeloInfo::default();
        for child in pv.children() {
            let cv = match child {
                Atom2Kind::Parent(parent_view) => continue,
                Atom2Kind::Child(cv) => cv,
            };
            match cv.id() {
                Id4::PCP_HELO_SESSIONID => {
                    let v = cv.data().read_u128::<BigEndian>().map_err(|_| ParseError::InvalidPayload)?;
                    helo.session_id = Some(GnuId::from(v));
                }
                Id4::PCP_HELO_BCID => {
                    let v = cv.data().read_u128::<BigEndian>().map_err(|_| ParseError::InvalidPayload)?;
                    helo.session_id = Some(GnuId::from(v));
                }
                Id4::PCP_HELO_AGENT => {
                    // HACKME: Vec<u8>で受けた方がいいか？
                    let v = String::from_utf8_lossy(cv.payload());
                    helo.agent = Some(v.to_string());
                }
                Id4::PCP_HELO_VERSION => {
                    let v = cv.data().read_u32::<LittleEndian>().map_err(|_| ParseError::InvalidPayload)?;
                    helo.version = Some(v);
                }
                Id4::PCP_HELO_PORT => {
                    let v = cv.data().read_u16::<LittleEndian>().map_err(|_| ParseError::InvalidPayload)?;
                    helo.port = Some(v);
                }
                Id4::PCP_HELO_PING => {
                    let v = cv.data().read_u16::<LittleEndian>().map_err(|_| ParseError::InvalidPayload)?;
                    helo.port_check = Some(v);
                }
                _ => {
                    continue;
                }
            }
        }
        Ok(helo)
    }
}

#[cfg(test)]
mod tests {
    use crate::pcp::{
        atom2::{Atom2Kind, AtomView},
        builder2::helo::HeloBuilder2,
        Atom2, GnuId, Id4,
    };

    #[test]
    fn test_helo_builder() {
        let session_id = GnuId::new();
        let bcid = GnuId::new();
        let helo_mut = HeloBuilder2::new(session_id).broadcast_id(bcid).port(1).port_check(2).build();
        assert_eq!(helo_mut.id(), Id4::PCP_HELO);
        let helo: Atom2 = helo_mut.into();
        assert_eq!(helo.id(), Id4::PCP_HELO);
        assert_eq!(helo.length(), 6);
    }
}
