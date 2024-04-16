use crate::pcp::{Atom, ChildAtom, Id4};

pub struct OkBuilder {
    value: u32,
}

impl OkBuilder {
    pub fn new(value: u32) -> Self {
        Self { value }
    }
    pub fn build(&self) -> Atom {
        ChildAtom::from((Id4::PCP_OK, self.value)).into()
    }
}
