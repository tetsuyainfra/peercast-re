#![allow(unused)]
use once_cell::sync::Lazy;

pub const PKG_VERSION: &'static str = env!("CARGO_PKG_VERSION");
pub static PKG_AGENT: Lazy<String> = Lazy::new(|| format!("PeerCast/0.1218 (REv{PKG_VERSION})"));

pub mod config;

mod conn;
pub use conn::ConnectionId;

pub mod error;

pub mod util {
    mod identify;
    mod shutdown;
    pub mod util_mpsc;
    pub use identify::identify_protocol;
    pub use identify::{ConnectionProtocol, IdentifierError};
    pub(crate) use shutdown::Shutdown;
}

pub mod codec;

/// Peercast Protocol
pub mod pcp {
    /// Atom Packet Sturct
    mod atom;
    /// atom packet Builder
    mod builder;
    /// Channel Struct
    mod channel;
    mod classify;
    mod connection;
    pub mod error_code;
    mod gnuid;
    mod id4;
    mod node;
    mod procedure;
    mod session;
    mod stream;
    mod util;

    pub use atom::{Atom, ChildAtom, ParentAtom};
    pub use channel::*;
    pub use gnuid::GnuId;
    pub use id4::Id4;
    pub use node::Node;
}

pub mod http;

pub mod rtmp;

pub mod app {
    pub mod cui;
    mod cui_dl;

    pub use cui_dl::CuiDL;
}

#[cfg(test)]
mod test_helper;

#[cfg(test)]
use tokio::test;

// 32bitで動かすためにバッファの長さに明示的にu64を使うようにする？
#[cfg(not(target_pointer_width = "64"))]
compile_error!("compilation is only allowed for 64-bit targets");
