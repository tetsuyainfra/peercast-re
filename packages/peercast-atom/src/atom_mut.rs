use crate::AtomView;

pub trait AtomWritable {
    fn set_raw_length(&mut self, length: u32);
    fn set_payload(&mut self, payload: &[u8]);
}

////////////////////////////////////////////////////////////////////////////////
/// AtomMut<S>
/// 内部データの型をジェネリクスで指定できるAtom
/// 内部データの型はAsRef<[u8]>を実装している必要がある(例: Vec<u8>, bytes::Bytesなど)
/// AtomView::raw()を実装しているため、AtomViewの機能を利用できる
/// 内部データの完全性は作成元が保証するものとする
#[derive(Debug, Clone)]
pub struct AtomMut<S> {
    pub raw: S,
}

impl<S> AtomMut<S>
where
    S: AsRef<[u8]>,
    S: AsMut<[u8]>,
{
    #[allow(unused)]
    /// バイト列からAtom<S>を作成する
    /// 内部データの完全性は作成元が保証するものとする
    pub fn new(raw: S) -> Self {
        Self {
            raw,
        }
    }

    pub fn set_id(&mut self, id: [u8; 4]) {
        self.raw.as_mut()[0..4].copy_from_slice(&id);
    }
}

impl<S> AtomView for AtomMut<S>
where
    S: AsRef<[u8]>,
    S: AsMut<[u8]>,
{
    fn raw(&self) -> &[u8] {
        self.raw.as_ref()
    }
}
