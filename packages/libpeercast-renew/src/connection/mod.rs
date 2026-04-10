mod connection_no;
pub mod error;
mod traits;

pub use connection_no::ConnectionNo;
pub use traits::{ConnectionHandle, ConnectionManager, ConnectionSpec, HandshakeConnection};
