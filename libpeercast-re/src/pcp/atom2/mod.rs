/// Atom2
/// Atom2: 不変ビュー(読み取り専用) この構造体は内部のデータの完全性を保証します。
/// AtomMut: 可変ビュー(読み書き可能)
//
use std::{fmt, ops::Range};

use bytes::{
    buf::{Buf, BufMut},
    BytesMut,
};

use crate::pcp::{atom2::atom_mut::AtomMut, Atom, Id4};

pub mod atom_mut;
pub mod codec;
pub mod parser;

#[derive(Debug, PartialEq, Eq)]
enum Kind {
    Parent,
    Child,
}

const ATOM_HEADER_LENGTH: usize = 8;
const ATOM_HEADER_POS_ID: Range<usize> = 0..4;
const ATOM_HEADER_POS_ENCODE_LENGTH: Range<usize> = 4..8;
const ATOM_HEADER_POS_START_PAYLOAD: usize = 8;

trait AtomView {
    fn raw(&self) -> &[u8];

    /// id
    fn id(&self) -> Id4 {
        let mut arr = [0u8; 4];
        arr.copy_from_slice(&self.raw()[ATOM_HEADER_POS_ID]);
        Id4::from(arr)
    }

    /// DON'T USE THIS DIRECTLY. Use len() instead.
    fn raw_length(&self) -> u32 {
        let mut arr = [0u8; 4];
        arr.copy_from_slice(&self.raw()[ATOM_HEADER_POS_ENCODE_LENGTH]);
        u32::from_le_bytes(arr)
    }

    /// payload length
    /// 注意: 親Atomの場合、子Atomの個数となります(バイト数ではありません)
    fn length(&self) -> u32 {
        self.raw_length() & 0x7FFFFFFF
    }

    fn kind(&self) -> Kind {
        if (self.raw_length() & 0x80000000) == 0 {
            Kind::Child
        } else {
            Kind::Parent
        }
    }

    fn payload(&self) -> &[u8] {
        &self.raw()[ATOM_HEADER_POS_START_PAYLOAD..]
    }
}

////////////////////////////////////////////////////////////////////////////////
/// Atom2
/// Atom2が作成された時点で、内部データの完全性は保証されているものとする。
/// 例えば、lengthフィールドが実際のデータ長と一致していることなどを含む。
struct Atom2 {
    raw: bytes::Bytes,
    // verified: bool, // TODO:もしこの構造体内でデータの整合性を検証するならば、このフィールドが必要になる
}

impl Atom2 {
    /// バイト列からAtom2を作成する
    /// 内部データの完全性は呼び出し元が保証するものとする
    fn new(buf: bytes::Bytes) -> Self {
        Self {
            raw: buf,
        }
    }
}

impl Atom2 {
    // view()のほうがいいか？
    fn view(&self) -> Atom2Kind<'_> {
        match self.kind() {
            Kind::Parent => Atom2Kind::Parent(ParentView {
                buf: &self.raw,
            }),
            Kind::Child => Atom2Kind::Child(ChildView {
                buf: &self.raw,
            }),
        }
    }
}

impl AtomView for Atom2 {
    fn raw(&self) -> &[u8] {
        &self.raw
    }
}

impl fmt::Debug for Atom2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Atom2 {{ id: {:?}, length: {}, kind: {:?} }}", self.id(), self.length(), self.kind())
    }
}

////////////////////////////////////////////////////////////////////////////////
/// Atom2Kind
///
enum Atom2Kind<'a> {
    Parent(ParentView<'a>),
    Child(ChildView<'a>),
}

////////////////////////////////////////////////////////////////////////////////
/// ChildView
///
struct ChildView<'a> {
    buf: &'a [u8],
}

impl AtomView for ChildView<'_> {
    fn raw(&self) -> &[u8] {
        self.buf
    }
}

impl ChildView<'_> {
    fn data(&self) -> &[u8] {
        debug_assert_eq!(self.length() as usize, self.raw().len() - 8);
        &self.buf[8..]
    }
}

////////////////////////////////////////////////////////////////////////////////
/// ParentView
///
struct ParentView<'a> {
    buf: &'a [u8],
}

impl AtomView for ParentView<'_> {
    fn raw(&self) -> &[u8] {
        self.buf
    }
}

impl ParentView<'_> {
    /// 子Atomのイテレータを返す
    fn children(&self) -> ChildIter<'_> {
        ChildIter {
            // buf: self.payload(),
            buf: &self.buf[8..],
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
/// ChildIter
///
struct ChildIter<'a> {
    buf: &'a [u8],
}

impl<'a> Iterator for ChildIter<'a> {
    type Item = Atom2Kind<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.buf.is_empty() {
            return None;
        }

        // let id = Id4::from8u32::from_le_bytes(self.buf[4..8].try_into().unwrap())); // 見る必要なし
        let size_and_parent = u32::from_le_bytes(self.buf[4..8].try_into().unwrap());
        let length = size_and_parent & 0x7FFF_FFFF;
        let kind = if (size_and_parent & 0x80000000) == 0 {
            Kind::Child
        } else {
            Kind::Parent
        };

        match kind {
            Kind::Child => {
                // ChildAtom
                let packet_length = ATOM_HEADER_LENGTH + (length as usize);
                let view_buf = &self.buf[0..packet_length];
                self.buf.advance(packet_length);
                Some(Atom2Kind::Child(ChildView {
                    buf: view_buf,
                }))
            }
            Kind::Parent => {
                // ParentAtom の処理（lengthがバイトサイズじゃないので注意）
                // let this_atoms_bytesize = calculate_parent_atom_bytesize(self.buf);
                let this_atoms_bytesize = parser::try_parse_atom(self.buf).unwrap();
                let buf = &self.buf[0..this_atoms_bytesize as usize];
                self.buf.advance(this_atoms_bytesize as usize);
                Some(Atom2Kind::Parent(ParentView {
                    buf,
                }))
            }
        }
    }
}

/*
/// 信用されたバッファから、親Atomのバイトサイズを計算する
fn calculate_parent_atom_bytesize(buf: &[u8]) -> usize {
    let size_and_parent = (&buf[4..8]).get_u32_le();
    let length: u32 = size_and_parent & 0x7FFF_FFFF;
    let kind = if (size_and_parent & 0x80000000) == 0 {
        AtomKind::Child
    } else {
        AtomKind::Parent
    };

    match kind {
        AtomKind::Child => {
            // ChildAtom の場合、lengthはバイトサイズそのもの
            return 8 + length as usize;
        }
        AtomKind::Parent => {
            let mut offset = 8; // 親Atomのヘッダサイズ
            for i in 0..length {
                // 子Atomを順に解析してバイトサイズを合計する
                let child_length = calculate_parent_atom_bytesize(&buf[offset..]);
                offset = offset + child_length;
            }
            return offset;
        }
    }
}
    */

/*
/// 信用されていないバッファから、Atomのバイトサイズを検証しつつ取得する
fn verify_atom_bytes(buf: &[u8]) -> Result<usize, ParseError> {
    if buf.len() < ATOM_HEADER_LENGTH {
        return Err(ParseError::UnexpectedEnd);
    }

    let size_and_parent = (&buf[4..8]).get_u32_le();
    let length: u32 = size_and_parent & 0x7FFF_FFFF;
    let kind = if (size_and_parent & 0x80000000) == 0 {
        AtomKind::Child
    } else {
        AtomKind::Parent
    };

    match kind {
        AtomKind::Child => {
            // ChildAtom の場合、lengthはバイトサイズそのもの
            let expected_size = ATOM_HEADER_LENGTH + length as usize;
            if buf.len() < expected_size {
                return Err(ParseError::UnexpectedEnd);
            }
            Ok(expected_size)
        }
        AtomKind::Parent => {
            let mut offset = ATOM_HEADER_LENGTH; // 初期値はこのAtomのヘッダサイズ
            for _ in 0..length {
                if offset >= buf.len() {
                    return Err(ParseError::UnexpectedEnd);
                }
                // 子Atomを順に解析してバイトサイズを合計する
                let child_size = verify_atom_bytes(&buf[offset..])?;
                offset += child_size;
            }
            Ok(offset)
        }
    }
} */

#[cfg(test)]
mod tests {
    use std::net::IpAddr;

    use ipnet::IpAdd;

    use crate::pcp::{ChildAtom, GnuId, ParentAtom};

    use super::*;

    fn to_bytes(c: ChildAtom) -> bytes::Bytes {
        let mut b = BytesMut::new();
        c.write_bytes(&mut b);
        b.freeze()
    }

    #[test]
    fn test_atom2_creation() {
        let c = ChildAtom::from((Id4::PCP_OK, 1_u8));
        let a = Atom2 {
            raw: to_bytes(c),
        };

        assert_eq!(a.id(), Id4::PCP_OK);
        println!("{:?}", a);

        match a.view() {
            Atom2Kind::Parent(parent_view) => unreachable!(),
            Atom2Kind::Child(child_view) => {
                assert_eq!(child_view.length(), 1);
                assert_eq!(child_view.data(), &[1_u8]);
                assert_eq!(child_view.data().len(), 1);
            }
        }

        let c = ChildAtom::from((Id4::PCP_BCST, GnuId::new()));
        let a = Atom2 {
            raw: to_bytes(c),
        };
        match a.view() {
            Atom2Kind::Parent(parent_view) => unreachable!(),
            Atom2Kind::Child(child_view) => {
                assert_eq!(child_view.length(), 16);
                assert_eq!(child_view.data().len(), 16);
            }
        }
    }

    #[test]
    fn test_childiter_child() {
        let a1 = ChildAtom::from((Id4::PCP_ATOM, 1_u16)).into();

        let mut citr = ChildIter {
            buf: &to_bytes(a1),
        };
        let atom = citr.next().unwrap();
        match atom {
            Atom2Kind::Parent(_) => unreachable!(),
            Atom2Kind::Child(child_view) => {
                assert_eq!(child_view.length(), 2); // SHORTのサイズ
                assert_eq!(child_view.data(), &[1_u8, 0_u8]); // Little Endian
            }
        }
        assert!(citr.next().is_none());
    }

    #[test]
    fn test_childiter_parent() {
        let a1 = ChildAtom::from((Id4::PCP_ATOM, 1_u32)).into();
        let a2_1 = ChildAtom::from((Id4::PCP_CHAN_PKT, "abc".to_string())).into(); // "abc\0"としてエンコードされる
        let a2_2 = ChildAtom::from((Id4::PCP_BCST, "127.1.2.3".parse::<IpAddr>().unwrap())).into();
        let a2 = ParentAtom::from((Id4::PCP_HOST, vec![a2_1, a2_2])).into();
        let a3 = ChildAtom::from((Id4::PCP_BCST, 258_u16)).into();
        let a = ParentAtom::from((Id4::PCP_HELO, vec![a1, a2, a3]));
        let mut bm = BytesMut::new();
        a.write_bytes(&mut bm);
        let bm = bm.freeze();

        let mut atom = Atom2 {
            raw: bm,
        };
        assert_eq!(atom.id(), Id4::PCP_HELO);
        assert_eq!(atom.length(), 3); // 子Atomが2つ

        // 親Atomのビューを取得
        let Atom2Kind::Parent(pv) = atom.view() else {
            unreachable!()
        };
        assert_eq!(pv.id(), Id4::PCP_HELO);
        assert_eq!(pv.length(), 3); // 子Atomが2つ
        let mut citr = pv.children();

        let v1 = citr.next().unwrap();
        match v1 {
            Atom2Kind::Child(child_view) => {
                assert_eq!(child_view.id(), Id4::PCP_ATOM);
                assert_eq!(child_view.length(), 4);
                assert_eq!(child_view.data(), &[1_u8, 0_u8, 0_u8, 0_u8]);
            }
            Atom2Kind::Parent(_) => unreachable!(),
        };
        let v2 = citr.next().unwrap();
        match v2 {
            Atom2Kind::Parent(parent_view) => {
                assert_eq!(parent_view.id(), Id4::PCP_HOST);
                assert_eq!(parent_view.length(), 2); // 子Atomが2つ

                let mut citr2 = parent_view.children();
                let cv1 = citr2.next().unwrap();
                match cv1 {
                    Atom2Kind::Child(child_view) => {
                        assert_eq!(child_view.id(), Id4::PCP_CHAN_PKT);
                        assert_eq!(child_view.length(), 4);
                        assert_eq!(child_view.data(), b"abc\0");
                    }
                    Atom2Kind::Parent(_) => unreachable!(),
                }

                let cv2 = citr2.next().unwrap();
                match cv2 {
                    Atom2Kind::Child(child_view) => {
                        assert_eq!(child_view.id(), Id4::PCP_BCST);
                        assert_eq!(child_view.length(), 4);
                        assert_eq!(child_view.data(), &[3_u8, 2, 1, 127] as &[u8]);
                    }
                    Atom2Kind::Parent(_) => unreachable!(),
                }
                assert!(citr2.next().is_none());
            }
            Atom2Kind::Child(child_view) => unreachable!(),
        }

        let v3 = citr.next().unwrap();
        match v3 {
            Atom2Kind::Child(child_view) => {
                assert_eq!(child_view.id(), Id4::PCP_BCST);
                assert_eq!(child_view.length(), 2);
                assert_eq!(child_view.data(), &[02_u8, 01_u8]);
            }
            Atom2Kind::Parent(_) => unreachable!(),
        };
        assert!(citr.next().is_none());
    }
}
