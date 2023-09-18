use bytes::{Buf, Bytes};
use tracing::info;

use super::util::atom as _in;
use super::{Atom, ChannelInfo, GnuId, Id4, TrackInfo};

// ChannelPacketのデータ構造
// Parent(PCP_CHAN, vec[
//  Child(PCP_CHAN_ID, GnuID),      // BroadcastID == ChannelID
//  Parent(PCP_CHAN_PKT, vec[
//    Child(PCP_CHAN_PKT_TYPE, PCP_CHAN_PKT_[HEAD|DATA]) // type
//    Child(PCP_CHAN_PKT_POS, u32),
//    Child(PCP_CHAN_PKT_CONTINUATION, true); // YT only
//    Child(PCP_CHAN_PKT_DATA, payload)
//  ]),
// ])

pub enum ClassifyAtom {
    HeadData {
        atom: Atom,
        head_data: Bytes,
        pos: u32,
        info: Option<ChannelInfo>,
        track: Option<TrackInfo>,
    },
    Data {
        atom: Atom,
        data: Bytes,
        pos: u32,
        continuation: Option<bool>,
    },
    Unknown {
        atom: Atom,
    },
}

// パケットデータタイプを表すだけのデータ
#[derive(Debug)]
pub enum ChanPktDataType {
    Head,
    Data,
}
impl ChanPktDataType {
    const HEAD: u32 = Id4::PCP_CHAN_PKT_HEAD.0;
    const DATA: u32 = Id4::PCP_CHAN_PKT_DATA.0;
}

impl ClassifyAtom {
    // #[instrument(level = "trace")]
    pub fn classify(atom: Atom) -> ClassifyAtom {
        // debug!("classify: {}", atom.len());
        match atom.id() {
            // CHANNEL DATA
            Id4::PCP_CHAN => {
                let atoms = atom.as_parent().childs();

                let id = get_id(atoms);
                let Some((typ_, pos, data, continuation)) = split_pkt(atoms) else {
                    panic!("入ってるはず・・・")
                };
                let info = get_info(atoms);
                let track = get_track(atoms);
                return match typ_ {
                    //
                    ChanPktDataType::Head => {
                        info!(info=?info, track=?track);
                        ClassifyAtom::HeadData {
                            atom,
                            head_data: data,
                            pos,
                            info,
                            track,
                        }
                    }
                    //
                    ChanPktDataType::Data => {
                        //
                        // info!("atom {:#?}", &atom);
                        ClassifyAtom::Data {
                            atom: atom,
                            data: data,
                            pos: pos,
                            continuation,
                        }
                    }
                };
            }
            _ => {
                panic!("not implement! atom classify");
            }
        }

        // trace!("atom: {:#?}", &atom);
        Self::Unknown { atom }
    }
}

fn _get_by_id(id: Id4, atoms: &Vec<Atom>) -> Option<&Atom> {
    atoms
        .iter()
        .find_map(|a| if a.id() == id { Some(a) } else { None })
}

fn get_id(atoms: &Vec<Atom>) -> Option<GnuId> {
    _get_by_id(Id4::PCP_CHAN_ID, atoms).map(|a| {
        let id_n = a.as_child().payload().get_u128();
        GnuId::from(id_n)
    })
}

//
// in CHAN ATOM
//
fn get_chan_pkt(atoms: &Vec<Atom>) -> Option<&Atom> {
    _get_by_id(Id4::PCP_CHAN_PKT, atoms)
}
fn get_info(atoms: &Vec<Atom>) -> Option<ChannelInfo> {
    _get_by_id(Id4::PCP_CHAN_INFO, atoms).map(|a| ChannelInfo::from(a))
}
fn get_track(atoms: &Vec<Atom>) -> Option<TrackInfo> {
    _get_by_id(Id4::PCP_CHAN_TRACK, atoms).map(|a| TrackInfo::from(a))
}
//
// in PKT ATOM
//
fn get_pkt_type(atoms: &Vec<Atom>) -> Option<ChanPktDataType> {
    _get_by_id(Id4::PCP_CHAN_PKT_TYPE, atoms).map(|a| {
        match Id4::from(_in::to_u32_be(&a.as_child().payload())) {
            Id4::PCP_CHAN_PKT_DATA => ChanPktDataType::Data,
            Id4::PCP_CHAN_PKT_HEAD => ChanPktDataType::Head,
            _ => panic!("error"),
        }
    })
}
fn get_pkt_pos(atoms: &Vec<Atom>) -> Option<u32> {
    _get_by_id(Id4::PCP_CHAN_PKT_POS, atoms).map(|a| _in::to_u32(&a.as_child().payload()))
}
fn get_pkt_data(atoms: &Vec<Atom>) -> Option<Bytes> {
    _get_by_id(Id4::PCP_CHAN_PKT_DATA, atoms).map(|a| a.as_child().payload())
}
fn get_pkt_continuing(atoms: &Vec<Atom>) -> Option<bool> {
    let r = _get_by_id(Id4::PCP_CHAN_PKT_CONTINUATION, atoms)
        .map(|a| _in::to_u8(&a.as_child().payload()));
    r.map(|v| !(v == 0))
}

fn split_pkt(atoms: &Vec<Atom>) -> Option<(ChanPktDataType, u32, Bytes, Option<bool>)> {
    let Some(pkt_atom) = get_chan_pkt(atoms) else {
        return None;
    };
    let atoms = pkt_atom.as_parent().childs();

    let typ = get_pkt_type(atoms).unwrap();
    let pos = get_pkt_pos(atoms).unwrap();
    let data = get_pkt_data(atoms).unwrap();
    let continuing = get_pkt_continuing(atoms);

    Some((typ, pos, data, continuing))
}
