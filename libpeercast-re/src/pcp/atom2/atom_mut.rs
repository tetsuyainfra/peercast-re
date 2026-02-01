use bytes::{BufMut, Bytes, BytesMut};
use daemonize::Parent;
use nom::combinator::into;

use crate::pcp::{
    atom2::{Atom2, Atom2Kind, AtomView, ChildView, Kind, ParentView},
    Id4,
};

////////////////////////////////////////////////////////////////////////////////
/// AtomMutの実装
pub struct AtomMut {
    id: Id4,
    data: AtomDataMut,
}

enum AtomDataMut {
    Parent(Vec<AtomMut>),
    Child(BytesMut),
}

impl AtomMut {
    /// Atomのバイト長を取得する(ヘッダ部分を除く)
    fn payload_length_byte(&self) -> u32 {
        match &self.data {
            AtomDataMut::Child(payload) => payload.len() as u32,
            AtomDataMut::Parent(children) => children.iter().fold(0, |acc, c| acc + c.payload_length_byte()),
        }
    }

    fn write(&self, buf: &mut BytesMut) {
        // IDを書き込む
        buf.put_u32(self.id.0);
        // buf.put_u32_le(enable_msb_1(self.childs.len() as u32));

        match &self.data {
            AtomDataMut::Child(payload) => {
                // ChildAtomの場合、長さはペイロードの長さ
                let length = payload.len() as u32;
                buf.put_u32_le(length & 0x7FFF_FFFF); // 最上位ビットは0
                buf.put_slice(&payload[..]);
            }
            AtomDataMut::Parent(children) => {
                // ParentAtomの場合、長さは子Atomの数
                let length = children.len() as u32;
                buf.put_u32_le(length | 0x80000000); // 最上位ビットは1
                for child in children {
                    child.write(buf);
                }
            }
        }
    }
}

impl From<AtomMut> for Atom2 {
    fn from(value: AtomMut) -> Atom2 {
        let mut buf = BytesMut::with_capacity(value.payload_length_byte() as usize + 8); // ヘッダ分の容量を確保
        value.write(&mut buf);
        Atom2 {
            raw: buf.freeze(),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
// From<()> for AtomMut の実装
//

/// ChildAtom変換の実装
impl From<(Id4, BytesMut)> for AtomMut {
    fn from(value: (Id4, BytesMut)) -> Self {
        AtomMut {
            id: value.0,
            data: AtomDataMut::Child(value.1),
        }
    }
}

/// ParentAtom変換の実装
impl From<(Id4, Vec<AtomMut>)> for AtomMut {
    fn from(value: (Id4, Vec<AtomMut>)) -> Self {
        AtomMut {
            id: value.0,
            data: AtomDataMut::Parent(value.1),
        }
    }
}

/// Payload部が空のChildAtom
impl From<(Id4, ())> for AtomMut {
    fn from(value: (Id4, ())) -> Self {
        AtomMut {
            id: value.0,
            data: AtomDataMut::Child(BytesMut::new()),
        }
    }
}

pub struct EmptyPayload;

/// (Id4, ())とどちらの方がわかりやすいかな？
impl From<(Id4, EmptyPayload)> for AtomMut {
    fn from(value: (Id4, EmptyPayload)) -> Self {
        AtomMut::from((value.0, ()))
    }
}
////////////////////////////////////////////////////////////////////////////////
/// From<Atom2> for AtomMut の実装
///
impl From<Atom2> for AtomMut {
    fn from(value: Atom2) -> Self {
        AtomMut::from(value.view())
    }
}

impl From<Atom2Kind<'_>> for AtomMut {
    fn from(value: Atom2Kind<'_>) -> Self {
        match value {
            Atom2Kind::Parent(parent_view) => AtomMut::from(parent_view),
            Atom2Kind::Child(child_view) => AtomMut::from(child_view),
        }
    }
}

impl From<ChildView<'_>> for AtomMut {
    fn from(value: ChildView<'_>) -> Self {
        let mut child_bytes_mut = BytesMut::from(&value.payload()[..]);
        AtomMut {
            id: value.id(),
            data: AtomDataMut::Child(child_bytes_mut),
        }
    }
}

impl From<ParentView<'_>> for AtomMut {
    fn from(value: ParentView<'_>) -> Self {
        let mut children_mut = Vec::with_capacity(value.length() as usize);
        for child_view in value.children() {
            children_mut.push(child_view.into());
        }
        AtomMut {
            id: value.id(),
            data: AtomDataMut::Parent(children_mut),
        }
    }
}

#[cfg(test)]
mod t {
    use bytes::BytesMut;

    use crate::pcp::{
        atom2::{
            atom_mut::{AtomMut, EmptyPayload},
            Atom2, AtomView,
        },
        Id4,
    };

    #[test]
    fn test_atom_emtpy_payload() {
        let a: AtomMut = (Id4::PCP_HELO, EmptyPayload).into();

        let atom: Atom2 = a.into();
        assert_eq!(atom.id(), Id4::PCP_HELO);
        assert_eq!(atom.length(), 0);
        assert_eq!(atom.payload(), &[] as &[u8]);
    }
}
