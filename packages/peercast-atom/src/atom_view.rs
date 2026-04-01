use std::{fmt, ops::Range};

use crate::error::AtomParseError;

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

    fn id(&self) -> [u8; 4] {
        let mut arr = [0u8; 4];
        arr.copy_from_slice(&self.raw()[ATOM_HEADER_POS_ID]);
        arr
    }

    /// DON'T USE THIS DIRECTLY. Use length() instead.
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

    fn payload(&self) -> &[u8] {
        &self.raw()[ATOM_HEADER_START_PAYLOAD..]
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
/// KindView
/// Atomの種類に応じたビューを表す列挙型
#[derive(Debug, PartialEq, Eq)]
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

impl fmt::Debug for ChildView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ChildView")
            //
            .field("id", &self.id())
            .field("len", &self.length())
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
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // let childrens: Vec<AtomView<'_>> = self.children().collect();
        // f.debug_struct("ParentView")
        //     .field("id", &self.id())
        //     .field("length", &self.length())
        //     .field("children", &childrens)
        //     //
        //     .finish()
        todo!()
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
                let this_atoms_bytesize = crate::parser::default_try_parse(self.buf).unwrap();
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
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // f.debug_list()
        //     //
        //     .entries(self.clone())
        //     .finish()
        todo!()
    }
}

////////////////////////////////////////////////////////////////////////////////
/// TryParse
///
pub trait AtomTryParse: AtomView {
    fn try_parse(buf: &[u8]) -> Result<(Self, &[u8]), AtomParseError>
    where
        Self: Sized;
}
