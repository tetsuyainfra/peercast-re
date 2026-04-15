mod atom;
mod atom_mut;
mod atom_view;
pub mod error;
// mod codec;
pub mod parser;

pub use atom::Atom;
pub use atom_mut::AtomMut;
pub use atom_view::{AtomIter, AtomKind, AtomTryDecode, AtomView, ChildView, KindView, ParentView};

#[cfg(feature = "codec")]
pub mod codec;
