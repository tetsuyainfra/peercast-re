pub use peercast_atom as atom;
pub use peercast_gnuid::GnuId;
pub use peercast_id4::Id4;

pub mod connection;
pub mod model;
pub mod pcp;
pub mod utils;

pub const PKG_VERSION: &'static str = env!("CARGO_PKG_VERSION");
pub const PKG_VERSION_MAJOR: &'static str = env!("CARGO_PKG_VERSION_MAJOR");
pub const PKG_VERSION_MINOR: &'static str = env!("CARGO_PKG_VERSION_MINOR");
pub const PKG_VERSION_PATCH: &'static str = env!("CARGO_PKG_VERSION_PATCH");
pub const PKG_AGENT: &'static str = concat!("PeerCast/0.1218 (REv", env!("CARGO_PKG_VERSION"), ")");
pub const PKG_SERVANT_VERSION: u32 = 1218;
pub const PKG_SERVANT_VERSION_VP: u32 = 27;
// pub const PKG_SERVANT_VERSION_EX_PREFIX: bytes::Bytes = bytes::Bytes::from_static(b"RE");
// pub const PKG_SERVANT_VERSION_EX_NUMBER: Lazy<u16> = Lazy::new(|| {
//     let major = PKG_VERSION_MAJOR.parse::<u16>().unwrap();
//     let minor = PKG_VERSION_MINOR.parse::<u16>().unwrap();
//     assert!(major < 10 && minor < 100);
//     major * 100 + minor
// });

#[cfg(test)]
mod test_helper;
