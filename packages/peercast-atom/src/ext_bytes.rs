use crate::{Atom, atom_view::AtomTryParse, error::AtomParseError, parser};

////////////////////////////////////////////////////////////////////////////////
/// BytesAtom
/// bytes::Bytesを内部データとするAtom
pub type AtomBytes = Atom<bytes::Bytes>;

impl AtomTryParse for AtomBytes {
    fn try_parse(buf: &[u8]) -> Result<(Self, &[u8]), AtomParseError> {
        let length = parser::default_try_parse(buf)?;
        let (buf_atom, rest) = buf.split_at(length as usize);
        let atom = AtomBytes::new(bytes::Bytes::copy_from_slice(buf_atom));

        Ok((atom, rest))
    }
}

#[cfg(test)]
mod t_bytes {
    use crate::atom::test::call_atom_test_tmpl;

    #[test]
    fn test_atom_bytes() {
        call_atom_test_tmpl!(bytes::Bytes);
    }
}
