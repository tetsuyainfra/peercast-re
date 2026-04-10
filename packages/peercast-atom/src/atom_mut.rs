use bytes::{BufMut, Bytes, BytesMut};
use peercast_gnuid::GnuId;
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

////////////////////////////////////////////////////////////////////////////////
// From<>による変換
//
/// ChildAtom変換の実装
impl From<(Id4, bytes::Bytes)> for AtomMut {
    fn from(value: (Id4, bytes::Bytes)) -> Self {
        AtomMut {
            id: value.0,
            data: AtomData::Child(value.1),
        }
    }
}

/// ParentAtom変換の実装
impl From<(Id4, Vec<AtomMut>)> for AtomMut {
    fn from(value: (Id4, Vec<AtomMut>)) -> Self {
        AtomMut {
            id: value.0,
            data: AtomData::Parent(value.1),
        }
    }
}

/// Payload部が空のChildAtom
impl From<(Id4, ())> for AtomMut {
    fn from(value: (Id4, ())) -> Self {
        AtomMut {
            id: value.0,
            data: AtomData::Child(bytes::Bytes::new()),
        }
    }
}

// 空タプルで実装しているので、以下は無効化
// pub struct EmptyPayload;
// /// (Id4, ())とどちらの方がわかりやすいかな？
// impl From<(Id4, EmptyPayload)> for AtomMut {
//     fn from(value: (Id4, EmptyPayload)) -> Self {
//         AtomMut::from((value.0, ()))
//     }
// }

////////////////////////////////////////////////////////////////////////////////
// 各データ型からAtomMutへの変換の実装
//

impl From<(Id4, GnuId)> for AtomMut {
    fn from((id, value): (Id4, GnuId)) -> Self {
        assert_eq!(std::mem::size_of::<GnuId>(), 16);
        let payload = Bytes::copy_from_slice(&value.0.to_be_bytes()[..]); // BE

        AtomMut {
            id: id,
            data: AtomData::Child(payload),
        }
    }
}

impl From<(Id4, u8)> for AtomMut {
    fn from((id, value): (Id4, u8)) -> Self {
        let payload = bytes::Bytes::copy_from_slice(&value.to_be_bytes()[..]);
        debug_assert_eq!(payload.len(), 1);

        AtomMut {
            id: id,
            data: AtomData::Child(payload),
        }
    }
}

impl From<(Id4, u16)> for AtomMut {
    fn from((id, value): (Id4, u16)) -> Self {
        // let mut payload = BytesMut::with_capacity(2);
        // payload.put_u16_le(value); // LE
        let payload = bytes::Bytes::copy_from_slice(&value.to_le_bytes()[..]);
        debug_assert_eq!(payload.len(), 2);

        AtomMut {
            id: id,
            data: AtomData::Child(payload),
        }
    }
}

impl From<(Id4, u32)> for AtomMut {
    fn from((id, value): (Id4, u32)) -> Self {
        // let mut payload = BytesMut::with_capacity(4);
        // payload.put_u32_le(value); // LE
        let payload = bytes::Bytes::copy_from_slice(&value.to_le_bytes()[..]);
        debug_assert_eq!(payload.len(), 4);

        AtomMut {
            id: id,
            data: AtomData::Child(payload),
        }
    }
}

impl From<(Id4, i32)> for AtomMut {
    fn from((id, value): (Id4, i32)) -> Self {
        // let mut payload = BytesMut::with_capacity(4);
        // payload.put_i32_le(value); // LE
        let payload = bytes::Bytes::copy_from_slice(&value.to_le_bytes()[..]);
        debug_assert_eq!(payload.len(), 4);

        AtomMut {
            id: id,
            data: AtomData::Child(payload),
        }
    }
}

/// Vec<u8>AtomMutに変換する
/// ※文字列であれば呼び出し側が\0を最後に付加すること
impl From<(Id4, Vec<u8>)> for AtomMut {
    fn from((id, value): (Id4, Vec<u8>)) -> Self {
        let payload = bytes::Bytes::copy_from_slice(&value[..]);

        AtomMut {
            id: id,
            data: AtomData::Child(payload),
        }
    }
}

/// String(内部はUTF-8)をAtomMutに変換する
/// 自動的に文字列末尾に\0を挿入する
impl From<(Id4, String)> for AtomMut {
    fn from((id, value): (Id4, String)) -> Self {
        let mut payload = BytesMut::with_capacity(value.len() + 1);
        payload.extend_from_slice(value.as_bytes());
        payload.put_u8(b'\0');

        AtomMut {
            id: id,
            data: AtomData::Child(payload.freeze()),
        }
    }
}

impl From<(Id4, std::net::IpAddr)> for AtomMut {
    fn from((id, value): (Id4, std::net::IpAddr)) -> Self {
        let payload = match value {
            std::net::IpAddr::V4(ip) => {
                // LEでエンコードする
                let mut oct = ip.octets();
                oct.reverse();
                bytes::Bytes::copy_from_slice(&oct)
            }
            std::net::IpAddr::V6(ip) => {
                // BEでエンコードする
                bytes::Bytes::copy_from_slice(&ip.octets())
            }
        };

        AtomMut {
            id: id,
            data: AtomData::Child(payload),
        }
    }
}

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

    #[test]
    fn test_mut_to_immutable() {
        let atom_mut = AtomMut::new(Id4::PCP_ATOM, AtomData::Child(bytes::Bytes::from_static(b"hello")));
        let atom: Atom = atom_mut.try_into().unwrap();
        assert_eq!(atom.raw(), bytes::Bytes::from_static(b"atom\x05\x00\x00\x00hello"));
    }

    #[test]
    fn test_from_tuple_empty() {
        let atom_mut: AtomMut = (Id4::PCP_ATOM, ()).into();
        assert_eq!(atom_mut.id(), Id4::PCP_ATOM);
        assert_eq!(atom_mut.kind(), AtomKind::Child);
        assert_eq!(atom_mut.payload().unwrap().len(), 0);
    }

    #[test]
    fn test_from_tuple_gnuid() {
        let gnu_id = GnuId::new();
        let atom_mut: AtomMut = (Id4::PCP_HELO_SESSIONID, gnu_id).into();
        assert_eq!(atom_mut.id(), Id4::PCP_HELO_SESSIONID);
        assert_eq!(atom_mut.kind(), AtomKind::Child);
        assert_eq!(atom_mut.payload().unwrap(), &bytes::Bytes::copy_from_slice(&gnu_id.0.to_be_bytes()[..]));
    }

    #[test]
    fn test_from_tuple_u8() {
        let atom_mut: AtomMut = (Id4::PCP_HELO_PORT, 123_u8).into();
        assert_eq!(atom_mut.id(), Id4::PCP_HELO_PORT);
        assert_eq!(atom_mut.kind(), AtomKind::Child);
        assert_eq!(atom_mut.payload().unwrap(), &bytes::Bytes::copy_from_slice(&0x7b_u8.to_le_bytes()[..]));
        let atom = atom_mut.freeze().unwrap();
        assert_eq!(atom.raw(), &bytes::Bytes::from_static(b"port\x01\x00\x00\x00\x7b")[..]);
    }

    #[test]
    fn test_from_tuple_u16() {
        let atom_mut: AtomMut = (Id4::PCP_HELO_PORT, 12345_u16).into();
        assert_eq!(atom_mut.id(), Id4::PCP_HELO_PORT);
        assert_eq!(atom_mut.kind(), AtomKind::Child);
        assert_eq!(atom_mut.payload().unwrap(), &bytes::Bytes::copy_from_slice(&12345_u16.to_le_bytes()[..]));
        let atom = atom_mut.freeze().unwrap();
        assert_eq!(atom.raw(), &bytes::Bytes::from_static(b"port\x02\x00\x00\x00\x39\x30")[..]);
    }

    #[test]
    fn test_from_tuple_i32() {
        assert_eq!(-123456789_i32, 0xF8A432EB_u32 as i32);
        let atom_mut: AtomMut = (Id4::PCP_HELO_PORT, -123456789_i32).into();
        assert_eq!(atom_mut.id(), Id4::PCP_HELO_PORT);
        assert_eq!(atom_mut.kind(), AtomKind::Child);
        assert_eq!(atom_mut.payload().unwrap(), &bytes::Bytes::copy_from_slice(&(-123456789_i32).to_le_bytes()[..]));
        assert_eq!(atom_mut.payload().unwrap(), &bytes::Bytes::copy_from_slice(&(0xF8A432EB_u32).to_le_bytes()[..]));
        let atom = atom_mut.freeze().unwrap();
        assert_eq!(atom.raw(), &bytes::Bytes::from_static(b"port\x04\x00\x00\x00\xEB\x32\xA4\xF8")[..]);
    }

    #[test]
    fn test_from_tuple_u32() {
        let atom_mut: AtomMut = (Id4::PCP_HELO_PORT, 123456789_u32).into();
        assert_eq!(atom_mut.id(), Id4::PCP_HELO_PORT);
        assert_eq!(atom_mut.kind(), AtomKind::Child);
        assert_eq!(atom_mut.payload().unwrap(), &bytes::Bytes::copy_from_slice(&123456789_u32.to_le_bytes()[..]));
        let atom = atom_mut.freeze().unwrap();
        assert_eq!(atom.raw(), &bytes::Bytes::from_static(b"port\x04\x00\x00\x00\x15\xCD\x5B\x07")[..]);
    }

    #[test]
    fn test_from_tuple_string() {
        let atom_mut: AtomMut = (Id4::PCP_ATOM, "hello".to_string()).into();
        assert_eq!(atom_mut.id(), Id4::PCP_ATOM);
        assert_eq!(atom_mut.kind(), AtomKind::Child);
        assert_eq!(atom_mut.payload().unwrap(), &bytes::Bytes::from_static(b"hello\0"));
        let atom = atom_mut.freeze().unwrap();
        assert_eq!(atom.raw(), &bytes::Bytes::from_static(b"atom\x06\x00\x00\x00hello\0")[..]);
    }

    #[test]
    fn test_from_tuple_ipaddr_v4() {
        let ip_v4 = std::net::IpAddr::V4(std::net::Ipv4Addr::new(192, 168, 0, 1));
        let atom_mut_v4: AtomMut = (Id4::PCP_ATOM, ip_v4).into();
        assert_eq!(atom_mut_v4.id(), Id4::PCP_ATOM);
        assert_eq!(atom_mut_v4.kind(), AtomKind::Child);
        assert_eq!(atom_mut_v4.payload().unwrap(), &bytes::Bytes::from_static(&[1, 0, 168, 192])); // LEでエンコード
        let atom = atom_mut_v4.freeze().unwrap();
        assert_eq!(atom.raw(), &bytes::Bytes::from_static(b"atom\x04\x00\x00\x00\x01\x00\xA8\xC0")[..]);
    }

    #[test]
    fn test_from_tuple_ipaddr_v6() {
        let ip_v6 = std::net::IpAddr::V6(std::net::Ipv6Addr::new(
            0x2001, 0x0db8, 0x85a3, 0x0000, 0x0000, 0x8a2e, 0x0370, 0x7334,
        ));
        let atom_mut_v6: AtomMut = (Id4::PCP_ATOM, ip_v6).into();
        assert_eq!(atom_mut_v6.id(), Id4::PCP_ATOM);
        assert_eq!(atom_mut_v6.kind(), AtomKind::Child);
        assert_eq!(
            atom_mut_v6.payload().unwrap(),
            &bytes::Bytes::from_static(&[
                0x20, 0x01, 0x0d, 0xb8, 0x85, 0xa3, 0x00, 0x00, 0x00, 0x00, 0x8a, 0x2e, 0x03, 0x70, 0x73, 0x34
            ])
        ); // BEでエンコード
        let atom = atom_mut_v6.freeze().unwrap();
        assert_eq!(
            atom.raw(),
            &bytes::Bytes::from_static(
                b"atom\x10\x00\x00\x00\x20\x01\x0d\xb8\x85\xa3\x00\x00\x00\x00\x8a\x2e\x03\x70\x73\x34"
            )[..]
        );
    }
}
