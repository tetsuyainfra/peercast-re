mod error;
mod gnuid;

pub use error::GnuIdParseError;
pub use gnuid::GnuId;

#[cfg(feature = "serde")]
pub mod ext_serde;
