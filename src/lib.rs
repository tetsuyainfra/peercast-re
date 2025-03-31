#![allow(unused)]
use std::str::FromStr;

use num::traits::AsPrimitive;
use once_cell::sync::Lazy;

pub const PKG_VERSION: &'static str = env!("CARGO_PKG_VERSION");
pub const PKG_VERSION_MAJOR: &'static str = env!("CARGO_PKG_VERSION_MAJOR");
pub const PKG_VERSION_MINOR: &'static str = env!("CARGO_PKG_VERSION_MINOR");
pub const PKG_VERSION_PATCH: &'static str = env!("CARGO_PKG_VERSION_PATCH");
pub const PKG_AGENT: &'static str = concat!("PeerCast/0.1218 (REv", env!("CARGO_PKG_VERSION"), ")");
pub const PKG_SERVANT_VERSION: u32 = 1218;
pub const PKG_SERVANT_VERSION_VP: u32 = 27;
pub const PKG_SERVANT_VERSION_EX_PREFIX: bytes::Bytes = bytes::Bytes::from_static(b"RE");
pub const PKG_SERVANT_VERSION_EX_NUMBER: Lazy<u16> = Lazy::new(|| {
    let major = PKG_VERSION_MAJOR.parse::<u16>().unwrap();
    let minor = PKG_VERSION_MINOR.parse::<u16>().unwrap();
    assert!(major < 10 && minor < 100);
    major * 100 + minor
});

pub mod config;

mod conn;
pub use conn::ConnectionId;

pub mod error;

pub mod codec;

/// Peercast Protocol
pub mod pcp;

pub mod http;

pub mod rtmp;

pub mod app {
    pub mod cui;
    mod cui_dl;

    pub use cui_dl::CuiDL;
}

pub mod util;

#[cfg(test)]
mod test_helper;

use rtmp::State;
#[cfg(test)]
use tokio::test;

// 32bitで動かすためにバッファの長さに明示的にu64を使うようにする？
#[cfg(not(target_pointer_width = "64"))]
compile_error!("compilation is only allowed for 64-bit targets");
