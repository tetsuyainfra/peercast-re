use std::{fmt, ops::Range};

use peercast_gnuid::GnuId;
use peercast_id4::Id4;

use crate::{AtomMut, atom_mut::AtomData, parser::AtomParser};

pub(crate) const ATOM_HEADER_LENGTH: usize = 8;
pub(crate) const ATOM_HEADER_POS_ID: Range<usize> = 0..4;
pub(crate) const ATOM_HEADER_POS_ENCODE_LENGTH: Range<usize> = 4..8;
pub(crate) const ATOM_HEADER_START_PAYLOAD: usize = 8;

////////////////////////////////////////////////////////////////////////////////
/// AtomView
///
pub trait AtomView {
    /// Atomの生データを返す。これを型で実装することで、AtomViewの機能を利用できるようになる
    fn raw(&self) -> &[u8];

    fn id(&self) -> Id4 {
        let mut arr = [0u8; 4];
        arr.copy_from_slice(&self.raw()[ATOM_HEADER_POS_ID]);
        Id4::from(arr)
    }

    /// DON'T USE THIS DIRECTLY. Use length() instead.
    fn raw_length(&self) -> u32 {
        let mut arr = [0u8; 4];
        arr.copy_from_slice(&self.raw()[ATOM_HEADER_POS_ENCODE_LENGTH]);
        u32::from_le_bytes(arr)
    }

    /// DONT USE THIS DIRECTLY. Use view() and try_decode_* instead.
    fn raw_payload(&self) -> &[u8] {
        &self.raw()[ATOM_HEADER_START_PAYLOAD..]
    }

    /// payload length
    /// 注意: 親Atomの場合、子Atomの個数となります(バイト数ではありません)
    fn length(&self) -> u32 {
        self.raw_length() & 0x7FFFFFFF
    }

    fn kind(&self) -> AtomKind {
        if (self.raw_length() & 0x80000000) == 0 {
            AtomKind::Child
        } else {
            AtomKind::Parent
        }
    }

    fn view(&'_ self) -> KindView<'_> {
        match self.kind() {
            AtomKind::Child => KindView::Child(ChildView {
                buf: self.raw(),
            }),
            AtomKind::Parent => KindView::Parent(ParentView {
                buf: self.raw(),
            }),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
/// AtomKind
/// Atomの種類を表す列挙型
#[derive(Debug, PartialEq, Eq)]
pub enum AtomKind {
    Parent,
    Child,
}

////////////////////////////////////////////////////////////////////////////////
/// AtomTryDecode
/// Atomのpayloadを特定の型にデコードするためのトレイト
/// このトレイトを実装することで、AtomViewのtry_decode_*メソッドを利用できるようになる
pub trait AtomTryDecode: AtomView {
    fn try_decode_u8(&self) -> Option<u8> {
        if self.kind() != AtomKind::Child || self.length() != 1 {
            return None;
        }
        Some(self.raw_payload()[0])
    }

    fn try_decode_i8(&self) -> Option<i8> {
        if self.kind() != AtomKind::Child || self.length() != 1 {
            return None;
        }
        Some(self.raw_payload()[0] as i8)
    }

    fn try_decode_u16(&self) -> Option<u16> {
        if self.kind() != AtomKind::Child || self.length() != 2 {
            return None;
        }
        let mut arr = [0u8; 2];
        let payload = self.raw_payload();
        arr.copy_from_slice(payload);
        Some(u16::from_le_bytes(arr))
    }
    fn try_decode_u16_be(&self) -> Option<u16> {
        if self.kind() != AtomKind::Child || self.length() != 2 {
            return None;
        }
        let mut arr = [0u8; 2];
        let payload = self.raw_payload();
        arr.copy_from_slice(payload);
        Some(u16::from_be_bytes(arr))
    }

    fn try_decode_u32(&self) -> Option<u32> {
        if self.kind() != AtomKind::Child || self.length() != 4 {
            return None;
        }
        let mut arr = [0u8; 4];
        let payload = self.raw_payload();
        arr.copy_from_slice(payload);
        Some(u32::from_le_bytes(arr))
    }
    fn try_decode_u32_be(&self) -> Option<u32> {
        if self.kind() != AtomKind::Child || self.length() != 4 {
            return None;
        }
        let mut arr = [0u8; 4];
        let payload = self.raw_payload();
        arr.copy_from_slice(payload);
        Some(u32::from_be_bytes(arr))
    }

    fn try_decode_i32(&self) -> Option<i32> {
        if self.kind() != AtomKind::Child || self.length() != 4 {
            return None;
        }
        let mut arr = [0u8; 4];
        let payload = self.raw_payload();
        arr.copy_from_slice(payload);
        Some(i32::from_le_bytes(arr))
    }
    fn try_decode_i32_be(&self) -> Option<i32> {
        if self.kind() != AtomKind::Child || self.length() != 4 {
            return None;
        }
        let mut arr = [0u8; 4];
        let payload = self.raw_payload();
        arr.copy_from_slice(payload);
        Some(i32::from_be_bytes(arr))
    }

    fn try_decode_vec(&self) -> Option<Vec<u8>> {
        if self.kind() != AtomKind::Child || self.length() == 0 {
            return None;
        }
        Some(self.raw_payload().to_vec())
    }

    fn try_decode_bytes(&self) -> Option<bytes::Bytes> {
        if self.kind() != AtomKind::Child || self.length() == 0 {
            return None;
        }
        Some(bytes::Bytes::copy_from_slice(self.raw_payload()))
    }

    fn try_decode_ipaddr(&self) -> Option<std::net::IpAddr> {
        if self.kind() != AtomKind::Child {
            return None;
        }
        let payload = self.raw_payload();
        if payload.len() == 4 {
            Some(std::net::IpAddr::V4(std::net::Ipv4Addr::from(u32::from_le_bytes(payload.try_into().ok()?))))
        } else if payload.len() == 16 {
            Some(std::net::IpAddr::V6(std::net::Ipv6Addr::from(u128::from_be_bytes(payload.try_into().ok()?)))) // BE
        } else {
            None
        }
    }

    fn try_decode_ipv4addr(&self) -> Option<std::net::Ipv4Addr> {
        if self.kind() != AtomKind::Child {
            return None;
        }
        let payload = self.raw_payload();
        if payload.len() == 4 {
            Some(std::net::Ipv4Addr::from(u32::from_le_bytes(payload.try_into().ok()?)))
        } else {
            None
        }
    }
    fn try_decode_ipv6addr(&self) -> Option<std::net::Ipv6Addr> {
        if self.kind() != AtomKind::Child {
            return None;
        }
        let payload = self.raw_payload();
        if payload.len() == 16 {
            Some(std::net::Ipv6Addr::from(u128::from_be_bytes(payload.try_into().ok()?))) // BE
        } else {
            None
        }
    }

    fn try_decode_gnuid(&self) -> Option<GnuId> {
        if self.kind() != AtomKind::Child || self.length() != 16 {
            return None;
        }
        let mut arr = [0u8; 16];
        let payload = self.raw_payload();
        arr.copy_from_slice(payload);
        Some(GnuId::from(arr))
    }
}

////////////////////////////////////////////////////////////////////////////////
/// KindView
/// Atomの種類に応じたビューを表す列挙型
#[derive(PartialEq, Eq)]
pub enum KindView<'a> {
    Parent(ParentView<'a>),
    Child(ChildView<'a>),
}

impl AtomView for KindView<'_> {
    fn raw(&self) -> &[u8] {
        match self {
            KindView::Parent(view) => view.raw(),
            KindView::Child(view) => view.raw(),
        }
    }
}

impl AtomTryDecode for KindView<'_> {}

impl From<KindView<'_>> for AtomMut {
    fn from(view: KindView<'_>) -> Self {
        match view {
            KindView::Parent(parent_view) => {
                let children: Vec<AtomMut> = parent_view.children().map(|child| child.into()).collect();
                AtomMut::new(parent_view.id(), AtomData::Parent(children))
            }
            KindView::Child(child_view) => {
                let payload = bytes::Bytes::copy_from_slice(child_view.data());
                AtomMut::new(child_view.id(), AtomData::Child(payload))
            }
        }
    }
}
impl fmt::Debug for KindView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KindView::Parent(view) => f.debug_tuple("KindView::Parent").field(view).finish(),
            KindView::Child(view) => f.debug_tuple("KindView::Child").field(view).finish(),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
/// ChildView
/// ChildAtomのビュー
#[derive(PartialEq, Eq)]
pub struct ChildView<'a> {
    pub(crate) buf: &'a [u8],
}

impl ChildView<'_> {
    pub fn data(&self) -> &[u8] {
        debug_assert_eq!(self.length() as usize, self.raw().len() - 8);
        &self.buf[8..]
    }
}

impl AtomView for ChildView<'_> {
    fn raw(&self) -> &[u8] {
        self.buf
    }
}

impl AtomTryDecode for ChildView<'_> {}

impl fmt::Debug for ChildView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ChildView")
            //
            .field("id", &self.id())
            .field("length", &self.length())
            .field("data", &self.data())
            //
            .finish()
    }
}

////////////////////////////////////////////////////////////////////////////////
/// ParentView
/// ParentAtomのビュー
#[derive(PartialEq, Eq)]
pub struct ParentView<'a> {
    pub(crate) buf: &'a [u8],
}

impl ParentView<'_> {
    /// 子Atomのイテレータを返す
    pub fn children(&self) -> AtomIter<'_> {
        AtomIter {
            // buf: self.payload(),
            buf: &self.buf[8..],
        }
    }
}

impl AtomView for ParentView<'_> {
    fn raw(&self) -> &[u8] {
        self.buf
    }
}

impl fmt::Debug for ParentView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let childrens: Vec<KindView<'_>> = self.children().collect();
        f.debug_struct("ParentView")
            .field("id", &self.id())
            .field("length", &self.length())
            .field("children", &childrens)
            .finish()
    }
}

////////////////////////////////////////////////////////////////////////////////
/// ParentIter
///
#[derive(Clone)]
pub struct AtomIter<'a> {
    buf: &'a [u8],
}

impl<'a> Iterator for AtomIter<'a> {
    type Item = KindView<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.buf.is_empty() {
            return None;
        }

        // let id = Id4::from8u32::from_le_bytes(self.buf[4..8].try_into().unwrap())); // 見る必要なし
        let size_and_parent = u32::from_le_bytes(self.buf[4..8].try_into().unwrap());
        let length = size_and_parent & 0x7FFF_FFFF;
        let kind = if (size_and_parent & 0x80000000) == 0 {
            AtomKind::Child
        } else {
            AtomKind::Parent
        };

        match kind {
            AtomKind::Child => {
                // ChildAtom
                let packet_length = ATOM_HEADER_LENGTH + (length as usize);
                let (view_buf, rest) = self.buf.split_at(packet_length);

                self.buf = rest;

                Some(KindView::Child(ChildView {
                    buf: view_buf,
                }))
            }
            AtomKind::Parent => {
                // ParentAtom の処理（lengthがバイトサイズじゃないので注意）
                // MEMO: View内のバッファは正常であることが保証されているのでunwrapしてよい
                let this_atoms_bytesize = AtomParser::default_try_parse(self.buf).unwrap();
                let (view_buf, rest) = self.buf.split_at(this_atoms_bytesize);
                self.buf = rest;

                Some(KindView::Parent(ParentView {
                    buf: view_buf,
                }))
            }
        }
    }
}

impl fmt::Debug for AtomIter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

#[cfg(test)]
mod t {
    use crate::Atom;

    use super::*;

    #[test]
    fn test_atom_view_child() {
        let mut buf = Vec::<u8>::new();
        buf.extend_from_slice(b"pcp\n");
        buf.extend_from_slice(&0x0000_0004_u32.to_le_bytes());
        buf.extend_from_slice(b"abcd");

        let atom = Atom::new(buf.clone().into());
        assert_eq!(atom.raw(), &buf);

        let view = atom.view();
        match view {
            KindView::Child(child_view) => {
                assert_eq!(child_view.id(), [b'p', b'c', b'p', b'\n'].into());
                assert_eq!(child_view.length(), 4);
                assert_eq!(child_view.data(), b"abcd");
                dbg!(child_view);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_atom_view_parent() {
        let mut buf = Vec::<u8>::new();
        buf.extend_from_slice(b"pcp\n");
        buf.extend_from_slice(&0x8000_0002_u32.to_le_bytes());
        {
            buf.extend_from_slice(b"pcpa");
            buf.extend_from_slice(&0x0000_0001_u32.to_le_bytes());
            buf.extend_from_slice(b"a");
        }
        {
            buf.extend_from_slice(b"pcpb");
            buf.extend_from_slice(&0x0000_0001_u32.to_le_bytes());
            buf.extend_from_slice(b"b");
        }

        let atom = Atom::new(buf.clone().into());
        let view = atom.view();
        assert_eq!(view.try_decode_bytes(), None); // ParentAtomはデコードできないことを確認

        match view {
            KindView::Parent(parent_view) => {
                assert_eq!(parent_view.id(), [b'p', b'c', b'p', b'\n'].into());
                assert_eq!(parent_view.length(), 2);

                let mut children = parent_view.children();
                let child0 = children.next().unwrap();
                assert_eq!(child0.id(), [b'p', b'c', b'p', b'a'].into());
                assert_eq!(child0.raw_payload(), b"a");
                assert_eq!(child0.try_decode_bytes(), Some(bytes::Bytes::from_static(b"a")));
                assert_eq!(child0.try_decode_u8(), Some(b'a'));

                let child1 = children.next().unwrap();
                assert_eq!(child1.id(), [b'p', b'c', b'p', b'b'].into());
                assert_eq!(child1.raw_payload(), b"b");
                assert_eq!(child1.try_decode_bytes(), Some(bytes::Bytes::from_static(b"b")));
                assert_eq!(child1.try_decode_u8(), Some(b'b'));

                dbg!(parent_view);
            }
            _ => unreachable!(),
        }
    }
}
