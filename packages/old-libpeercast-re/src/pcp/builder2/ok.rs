use crate::pcp::{Atom, AtomMut, ChildAtom, Id4};

pub struct OkBuilder2 {
    value: u32,
}

impl OkBuilder2 {
    pub fn new(value: u32) -> Self {
        Self {
            value,
        }
    }
    pub fn build(&self) -> AtomMut {
        (Id4::PCP_OK, self.value).into()
    }
}
