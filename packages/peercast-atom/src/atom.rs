use crate::AtomView;

////////////////////////////////////////////////////////////////////////////////
/// Atom<S>
/// 内部データの型をジェネリクスで指定できるAtom
/// 内部データの型はAsRef<[u8]>を実装している必要がある(例: Vec<u8>, bytes::Bytesなど)
/// AtomView::raw()を実装しているため、AtomViewの機能を利用できる
/// 内部データの完全性は作成元が保証するものとする
#[derive(Debug, Clone)]
pub struct Atom<S>
where
    S: AsRef<[u8]>,
{
    raw: S,
}

impl<S> Atom<S>
where
    S: AsRef<[u8]>,
{
    #[allow(unused)]
    /// バイト列からAtom<S>を作成する
    /// 内部データの完全性は作成元が保証するものとする
    pub fn new(raw: S) -> Self {
        Self {
            raw,
        }
    }
}

impl<S> AtomView for Atom<S>
where
    S: AsRef<[u8]>,
{
    fn raw(&self) -> &[u8] {
        self.raw.as_ref()
    }
}

#[cfg(test)]
pub(crate) mod test {
    use crate::{AtomKind, AtomView, KindView};

    use super::*;

    pub fn tmpl_atom_child_payload_none<S>()
    where
        S: AsRef<[u8]>,
        S: From<Vec<u8>>,
    {
        let atom = Atom::<S>::new(vec![1, 0, 0, 0, 0, 0, 0, 0].into());
        assert_eq!(atom.raw(), &[1, 0, 0, 0, 0, 0, 0, 0]);

        assert_eq!(atom.id(), [1, 0, 0, 0]);
        assert_eq!(atom.raw_length(), 0);
        assert_eq!(atom.length(), 0);
        assert_eq!(atom.kind(), AtomKind::Child);
        assert_eq!(atom.payload(), &[]);
    }

    pub fn tmpl_atom_child_payload_2<S>()
    where
        S: AsRef<[u8]>,
        S: From<Vec<u8>>,
    {
        let atom = Atom::<S>::new(vec![1, 0, 0, 0, 2, 0, 0, 0, 0xFF, 0xFE].into());
        assert_eq!(atom.raw(), &[1, 0, 0, 0, 2, 0, 0, 0, 0xFF, 0xFE]);

        assert_eq!(atom.id(), [1, 0, 0, 0]);
        assert_eq!(atom.raw_length(), 2);
        assert_eq!(atom.length(), 2);
        assert_eq!(atom.kind(), AtomKind::Child);
        assert_eq!(atom.payload(), &[0xFF, 0xFE]);
    }

    pub fn tmpl_atom_parent_childs_none<S>()
    where
        S: AsRef<[u8]>,
        S: From<Vec<u8>>,
    {
        let mut buf = Vec::<u8>::new();
        buf.extend_from_slice(b"pcp\n");
        buf.extend_from_slice(&0x8000_0000_u32.to_le_bytes());

        let atom = Atom::<S>::new(buf.clone().into());
        assert_eq!(atom.raw(), &buf);

        assert_eq!(atom.id(), [b'p', b'c', b'p', b'\n']);
        assert_eq!(atom.raw_length(), 0x8000_0000);
        assert_eq!(atom.length(), 0);
        assert_eq!(atom.kind(), AtomKind::Parent);
        assert_eq!(atom.payload(), &[]);
    }

    pub fn tmpl_atom_parent_childs_2<S>()
    where
        S: AsRef<[u8]>,
        S: From<Vec<u8>>,
    {
        let mut buf = Vec::<u8>::new();
        buf.extend_from_slice(b"pcp\n");
        buf.extend_from_slice(&0x8000_0002_u32.to_le_bytes());
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
                buf.extend_from_slice(b"cafe");
                buf.extend_from_slice(&0x8000_0000_u32.to_le_bytes());
            }
            // Child3
            {
                buf.extend_from_slice(b"dead");
                buf.extend_from_slice(&0x8000_0001_u32.to_le_bytes());
                {
                    buf.extend_from_slice(b"beef");
                    buf.extend_from_slice(&1_u32.to_le_bytes());
                    buf.extend_from_slice(&[0xAB]);
                }
            }
        }

        let atom = Atom::<S>::new(buf.clone().into());
        assert_eq!(atom.raw(), &buf);

        assert_eq!(atom.id(), [b'p', b'c', b'p', b'\n']);
        assert_eq!(atom.raw_length(), 0x8000_0000);
        assert_eq!(atom.length(), 0);
        assert_eq!(atom.kind(), AtomKind::Parent);
        assert_eq!(atom.payload().len(), 10 + 10);
        let parent_view = match atom.view() {
            KindView::Child(_) => unreachable!(),
            KindView::Parent(parent_view) => parent_view,
        };
        let mut children = parent_view.children();

        let child0 = children.next().unwrap();
        assert_eq!(child0.id(), [b'a', b'b', b'c', b'd']);
        assert_eq!(child0.payload(), &[0xFF, 0xFE]);
        assert_eq!(child0.kind(), AtomKind::Child);

        let child1 = children.next().unwrap();
        assert_eq!(child1.id(), [b'b', b'e', b'e', b'f']);
        assert_eq!(child1.payload(), &[0xFD, 0xFC]);
        assert_eq!(child1.kind(), AtomKind::Child);

        let child2 = children.next().unwrap();
        assert_eq!(child2.id(), [b'c', b'a', b'f', b'e']);
        assert_eq!(child2.payload(), &[]);
        assert_eq!(child2.kind(), AtomKind::Parent);

        let child3 = children.next().unwrap();
        assert_eq!(child3.id(), [b'd', b'e', b'a', b'd']);
        assert_eq!(child3.payload().len(), 10);
        assert_eq!(child3.kind(), AtomKind::Parent);
        {
            let KindView::Parent(child3_view) = child3.view() else {
                unreachable!()
            };
            let mut child3_children = child3_view.children();
            let child3_child0 = child3_children.next().unwrap();
            assert_eq!(child3_child0.id(), [b'b', b'e', b'e', b'f']);
            assert_eq!(child3_child0.payload(), &[0xAB]);
            assert_eq!(child3_child0.kind(), AtomKind::Child);
        }

        assert_eq!(children.next(), None);
    }

    macro_rules! call_atom_test_tmpl {
        ($t:ty) => {
            crate::atom::test::tmpl_atom_child_payload_none::<$t>();
            crate::atom::test::tmpl_atom_child_payload_2::<$t>();
            crate::atom::test::tmpl_atom_parent_childs_none::<$t>();
        };
    }

    /// 同じパターンのテストを実行するためのマクロ
    /// usage:
    /// #[test]
    /// fn test_vec_atom_payload_none() {
    ///     call_atom_test_tmpl!(Vec<u8>);
    /// }
    pub(crate) use call_atom_test_tmpl;
}
