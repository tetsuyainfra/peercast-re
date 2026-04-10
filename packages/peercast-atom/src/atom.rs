use crate::{AtomMut, AtomView};

////////////////////////////////////////////////////////////////////////////////
/// Atom
/// 内部データの完全性は作成元が保証するものとする
#[derive(Debug, Clone)]
pub struct Atom {
    raw: bytes::Bytes,
}

impl Atom {
    #[allow(unused)]
    /// バイト列からAtom<S>を作成する
    /// 内部データの完全性は作成元が保証するものとする
    pub fn new(raw: bytes::Bytes) -> Self {
        Self {
            raw,
        }
    }

    pub fn write_to(&self, dst: &mut bytes::BytesMut) {
        dst.extend_from_slice(&self.raw);
    }
}

impl AtomView for Atom {
    fn raw(&self) -> &[u8] {
        self.raw.as_ref()
    }
}

/////////////////////////////////////////////////////////////////////////////////
/// From<AtomMut> for Atom の実装
///
impl From<Atom> for AtomMut {
    fn from(atom: Atom) -> Self {
        atom.view().into()
    }
}

#[cfg(test)]
pub(crate) mod test {
    use crate::{AtomKind, AtomView, KindView};

    use super::*;

    #[test]
    pub fn tmpl_atom_child_payload_none() {
        let atom = Atom::new(vec![1, 0, 0, 0, 0, 0, 0, 0].into());
        assert_eq!(atom.raw(), &[1, 0, 0, 0, 0, 0, 0, 0]);

        assert_eq!(atom.id(), [1, 0, 0, 0].into());
        assert_eq!(atom.raw_length(), 0);
        assert_eq!(atom.length(), 0);
        assert_eq!(atom.kind(), AtomKind::Child);
        assert_eq!(atom.raw_payload(), &[]);
    }

    #[test]
    pub fn tmpl_atom_child_payload_2() {
        let atom = Atom::new(vec![1, 0, 0, 0, 2, 0, 0, 0, 0xFF, 0xFE].into());
        assert_eq!(atom.raw(), &[1, 0, 0, 0, 2, 0, 0, 0, 0xFF, 0xFE]);

        assert_eq!(atom.id(), [1, 0, 0, 0].into());
        assert_eq!(atom.raw_length(), 2);
        assert_eq!(atom.length(), 2);
        assert_eq!(atom.kind(), AtomKind::Child);
        assert_eq!(atom.raw_payload(), &[0xFF, 0xFE]);
    }

    #[test]
    pub fn tmpl_atom_parent_childs_none() {
        let mut buf = Vec::<u8>::new();
        buf.extend_from_slice(b"pcp\n");
        buf.extend_from_slice(&0x8000_0000_u32.to_le_bytes());

        let atom = Atom::new(buf.clone().into());
        assert_eq!(atom.raw(), &buf);

        assert_eq!(atom.id(), [b'p', b'c', b'p', b'\n'].into());
        assert_eq!(atom.raw_length(), 0x8000_0000);
        assert_eq!(atom.length(), 0);
        assert_eq!(atom.kind(), AtomKind::Parent);
        assert_eq!(atom.raw_payload(), &[]);
    }

    #[test]
    pub fn tmpl_atom_parent_childs_2() {
        let mut buf = Vec::<u8>::new();
        buf.extend_from_slice(b"pcp\n");
        buf.extend_from_slice(&0x8000_0004_u32.to_le_bytes());
        {
            // Child0
            buf.extend_from_slice(b"abcd");
            buf.extend_from_slice(&2_u32.to_le_bytes());
            buf.extend_from_slice(&[0xFF, 0xFE]);
            // Child1
            buf.extend_from_slice(b"beef");
            buf.extend_from_slice(&2_u32.to_le_bytes());
            buf.extend_from_slice(&[0xFD, 0xFC]);
            // Child2
            {
                // Parent
                buf.extend_from_slice(b"cafe");
                buf.extend_from_slice(&0x8000_0000_u32.to_le_bytes());
            }
            // Child3
            {
                // Parent
                buf.extend_from_slice(b"dead");
                buf.extend_from_slice(&0x8000_0001_u32.to_le_bytes());
                {
                    buf.extend_from_slice(b"beef");
                    buf.extend_from_slice(&1_u32.to_le_bytes());
                    buf.extend_from_slice(&[0xAB]);
                }
            }
        }

        let atom = Atom::new(buf.clone().into());
        assert_eq!(atom.raw(), &buf);

        assert_eq!(atom.id(), [b'p', b'c', b'p', b'\n'].into());
        assert_eq!(atom.raw_length(), 0x8000_0004);
        assert_eq!(atom.length(), 4);
        assert_eq!(atom.kind(), AtomKind::Parent);
        assert_eq!(atom.raw_payload().len(), 10 + 10 + 8 + (8 + (8 + 1)));
        let parent_view = match atom.view() {
            KindView::Child(_) => unreachable!(),
            KindView::Parent(parent_view) => parent_view,
        };
        let mut children = parent_view.children();

        let child0 = children.next().unwrap();
        assert_eq!(child0.id(), [b'a', b'b', b'c', b'd'].into());
        assert_eq!(child0.raw_payload(), &[0xFF, 0xFE]);
        assert_eq!(child0.kind(), AtomKind::Child);

        let child1 = children.next().unwrap();
        assert_eq!(child1.id(), [b'b', b'e', b'e', b'f'].into());
        assert_eq!(child1.raw_payload(), &[0xFD, 0xFC]);
        assert_eq!(child1.kind(), AtomKind::Child);

        let child2 = children.next().unwrap();
        assert_eq!(child2.id(), [b'c', b'a', b'f', b'e'].into());
        assert_eq!(child2.raw_payload(), &[]);
        assert_eq!(child2.kind(), AtomKind::Parent);

        let child3 = children.next().unwrap();
        assert_eq!(child3.id(), [b'd', b'e', b'a', b'd'].into());
        assert_eq!(child3.raw_payload().len(), 9);
        assert_eq!(child3.kind(), AtomKind::Parent);
        {
            let KindView::Parent(child3_view) = child3.view() else {
                unreachable!()
            };
            let mut child3_children = child3_view.children();
            let child3_child0 = child3_children.next().unwrap();
            assert_eq!(child3_child0.id(), [b'b', b'e', b'e', b'f'].into());
            assert_eq!(child3_child0.raw_payload(), &[0xAB]);
            assert_eq!(child3_child0.kind(), AtomKind::Child);
        }

        assert_eq!(children.next(), None);
    }
}
