use bytes::{BufMut, BytesMut};
use peercast_id4::Id4;

use crate::{
    Atom, AtomKind,
    atom_view::ATOM_HEADER_LENGTH,
    error::{AtomFreezeError, AtomMutError},
};

////////////////////////////////////////////////////////////////////////////////
/// AtomMut
#[derive(Debug, Clone)]
pub struct AtomMut {
    id: Id4,
    data: AtomData,
}

#[derive(Debug, Clone)]
pub enum AtomData {
    Parent(Vec<AtomMut>),
    Child(bytes::Bytes),
}

impl AtomMut {
    const MAX_CHILDREN: u32 = 0x7FFF_FFFF;
    const MAX_PAYLOAD: u32 = 0x7FFF_FFFF;

    #[allow(unused)]
    /// バイト列からAtom<S>を作成する
    /// 内部データの完全性は作成元が保証するものとする
    pub fn new(id: Id4, data: AtomData) -> Self {
        Self {
            id,
            data,
        }
    }

    pub fn id(&self) -> Id4 {
        self.id
    }

    pub fn set_id(&mut self, id: Id4) {
        self.id = id
    }

    pub fn kind(&self) -> AtomKind {
        match self.data {
            AtomData::Child(_) => AtomKind::Child,
            AtomData::Parent(_) => AtomKind::Parent,
        }
    }

    pub fn data(&self) -> &AtomData {
        &self.data
    }

    pub fn set_data(&mut self, data: AtomData) {
        self.data = data
    }

    pub fn data_mut(&mut self) -> &mut AtomData {
        &mut self.data
    }

    //
    // ここからペイロードの内容にアクセスするための便利なメソッドを追加していく
    //
    ///  子Atomのスライスを返す
    pub fn children(&self) -> Option<&[AtomMut]> {
        match &self.data {
            AtomData::Parent(children) => Some(children),
            AtomData::Child(_) => None,
        }
    }

    ///  子Atomの可変スライスを返す
    pub fn children_mut(&mut self) -> Option<&mut Vec<AtomMut>> {
        match &mut self.data {
            AtomData::Parent(children) => Some(children),
            AtomData::Child(_) => None,
        }
    }

    // ChildAtomのペイロードを返す
    pub fn payload(&self) -> Option<&bytes::Bytes> {
        match &self.data {
            AtomData::Child(bytes) => Some(bytes),
            AtomData::Parent(_) => None,
        }
    }

    //
    // ここから書き込み向け機能の実装
    //
    /// Atomのペイロード部のバイト長を返す
    pub fn payload_bytes(&self) -> usize {
        match &self.data {
            AtomData::Child(bytes) => bytes.len(),
            AtomData::Parent(childs) => childs.iter().map(|child| ATOM_HEADER_LENGTH + child.payload_bytes()).sum(),
        }
    }

    /// Atomをバイト列に書き込む
    pub fn write_to(&self, dst: &mut BytesMut) -> Result<(), AtomFreezeError> {
        match &self.data {
            AtomData::Child(bytes) => {
                dst.put_slice(&self.id.0.to_be_bytes()); // IDは常に4バイトでビッグエンディアン
                let length = u32::try_from(bytes.len())
                    .ok()
                    .filter(|&n| n <= Self::MAX_PAYLOAD)
                    .ok_or(AtomFreezeError::PayloadTooLarge(bytes.len()))?;
                dst.put_u32_le(length); // 最上位ビットは0
                dst.put_slice(bytes);
            }
            AtomData::Parent(childs) => {
                dst.put_slice(&self.id.0.to_be_bytes()); // IDは常に4バイトでビッグエンディアン
                let length = u32::try_from(childs.len())
                    .ok()
                    .filter(|&n| n <= Self::MAX_CHILDREN)
                    .ok_or(AtomFreezeError::TooManyChildren(childs.len()))?;
                dst.put_u32_le(length | 0x80000000); // 最上位ビットは1
                for child in childs {
                    let _ = child.write_to(dst)?;
                }
            }
        };
        Ok(())
    }

    /// AtomMutをAtomに変換する
    pub fn freeze(self) -> Result<Atom, AtomFreezeError> {
        let mut buf = BytesMut::with_capacity(self.payload_bytes() as usize + 8); // ヘッダ分の容量を確保
        self.write_to(&mut buf)?;
        Ok(Atom::new(buf.freeze()))
    }

    //
    // ここからビルダーパターンの実装
    //
    /// ParentAtomを作成するためのビルダーメソッド
    pub fn parent(id: Id4, children: Vec<AtomMut>) -> Self {
        Self::new(id, AtomData::Parent(children))
    }

    /// ChildAtomを作成するためのビルダーメソッド
    pub fn child(id: Id4, data: bytes::Bytes) -> Self {
        Self::new(id, AtomData::Child(data))
    }

    /// ParentAtomに子Atomを追加するためのビルダーメソッド
    pub fn push_child(&mut self, child: AtomMut) -> Result<(), AtomMutError> {
        match &mut self.data {
            AtomData::Parent(children) => {
                children.push(child);
                Ok(())
            }
            AtomData::Child(_) => Err(AtomMutError::NotAParent),
        }
    }
}

impl TryFrom<AtomMut> for Atom {
    type Error = AtomFreezeError;

    fn try_from(value: AtomMut) -> Result<Self, Self::Error> {
        value.freeze()
    }
}

////////////////////////////////////////////////////////////////////////////////
// AtomData向けの実装
// MEMO:
// バグの原因にならないか？
// 例えばVec<Atom>をBytesにした後、bytes.into()するとAtomData::Childにしてしまうようなケースが出そう

/// AtomData::Parent変換の実装
/// usage:
/// let childs = vec![AtomMut::new(id1, data1), AtomMut::new(id2, data2)];
/// AtomMut::new(id, childs.into()) // <-- ここが楽になる
// impl From<Vec<AtomMut>> for AtomData {
//     fn from(children: Vec<AtomMut>) -> Self {
//         AtomData::Parent(children)
//     }
// }

/// AtomData::Child変換の実装
/// usage: AtomMut::new(id, bytes.into())
// impl From<bytes::Bytes> for AtomData {
//     fn from(bytes: bytes::Bytes) -> Self {
//         AtomData::Child(bytes)
//     }
// }

#[cfg(test)]
mod t {
    use super::*;
    use crate::AtomView;

    #[test]
    fn test_freeze_child() {
        let atom_mut = AtomMut::new(Id4::PCP_ATOM, AtomData::Child(bytes::Bytes::from_static(b"hello")));
        let atom = atom_mut.freeze().unwrap();
        assert_eq!(atom.raw(), bytes::Bytes::from_static(b"atom\x05\x00\x00\x00hello"));
    }

    #[test]
    fn test_freeze_parent() {
        let atom_mut = AtomMut::new(
            Id4::PCP_ATOM,
            AtomData::Parent(vec![
                AtomMut::new(Id4::from(*b"chl1"), AtomData::Child(bytes::Bytes::from_static(b"hell1"))).into(),
                AtomMut::new(Id4::from(*b"chl2"), AtomData::Child(bytes::Bytes::from_static(b"hell2"))).into(),
            ]),
        );

        let mut buf = Vec::new();
        buf.put_u32(Id4::PCP_ATOM.0);
        buf.put_u32_le(0x80000002); // ParentAtomで子Atomが2つ
        // 子Atom1
        buf.put_u32(Id4::from(*b"chl1").0);
        buf.put_u32_le(5); // ChildAtomでペイロードが5バイト
        buf.put_slice(b"hell1");
        // 子Atom2
        buf.put_u32(Id4::from(*b"chl2").0);
        buf.put_u32_le(5); // ChildAtomでペイロードが5バイト
        buf.put_slice(b"hell2");

        let atom = atom_mut.freeze().unwrap();
        assert_eq!(atom.raw(), &buf);
    }

    #[test]
    fn test_immutable_to_mut() {
        // let atom = Atom::new(bytes::Bytes::from_static(b"atom\x05\x00\x00\x00hello"));
        // let atom_mut: AtomMut = atom.try_into().unwrap();
        // assert_eq!(atom_mut.id(), Id4::from(*b"atom"));
        // assert_eq!(atom_mut.kind(), AtomKind::Child);
        // assert_eq!(atom_mut.payload().unwrap(), &bytes::Bytes::from_static(b"hello"));
    }
}
