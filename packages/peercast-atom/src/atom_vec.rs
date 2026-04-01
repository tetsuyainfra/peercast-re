use crate::{
    Atom, AtomMut,
    atom_mut::AtomWritable,
    atom_view::{ATOM_HEADER_LENGTH, AtomTryParse},
    error::AtomParseError,
    parser,
};

////////////////////////////////////////////////////////////////////////////////
/// AtomVec
/// std::vec::Vec<u8>を内部データとするAtom
pub type AtomVec = Atom<Vec<u8>>;

impl AtomTryParse for AtomVec {
    fn try_parse(buf: &[u8]) -> Result<(Self, &[u8]), AtomParseError> {
        let length = parser::default_try_parse(buf)?;
        let (buf_atom, rest) = buf.split_at(length as usize);
        let atom = AtomVec::new(buf_atom.to_vec());

        Ok((atom, rest))
    }
}

#[cfg(test)]
mod t {
    use crate::atom::test::call_atom_test_tmpl;

    #[test]
    fn test_atom_vec() {
        call_atom_test_tmpl!(Vec<u8>);
    }
}

////////////////////////////////////////////////////////////////////////////////
/// AtomMutVec
/// std::vec::Vec<u8>を内部データとするAtomMut
pub type AtomMutVec = AtomMut<Vec<u8>>;

impl AtomWritable for AtomMutVec {
    fn set_payload(&mut self, payload: &[u8]) {
        self.raw.truncate(ATOM_HEADER_LENGTH); // ヘッダ部分を残してペイロードを上書き
        self.raw.extend_from_slice(payload);

        // debug_assert!()
    }

    fn set_raw_length(&mut self, length: u32) {
        todo!()
    }
}

#[cfg(test)]
mod t_mut {
    use super::*;

    #[test]
    fn test_atom_mut_vec() {
        let atom = AtomMutVec::new(vec![1, 2, 3]);
    }
}
