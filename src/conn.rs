use std::{fmt, sync::atomic::AtomicI32};

use bytes::Bytes;
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};
use tracing::{warn, Dispatch};

use crate::error::{self, HandshakeError};

static GLOBAL_CONNECTION_COUNT: AtomicI32 = AtomicI32::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConnectionId(pub i32);
impl ConnectionId {
    pub fn new() -> Self {
        let count = GLOBAL_CONNECTION_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if count < 0 {
            warn!("connection id counter is MINUS value, it's overflow. you should reboot this application.")
        }
        Self(count)
    }
}

impl fmt::Display for ConnectionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Connection({})", self.0))
    }
}

impl From<i32> for ConnectionId {
    fn from(value: i32) -> Self {
        Self(value)
    }
}

#[cfg(test)]
mod t {
    use crate::show_size;

    use super::*;

    #[test]
    fn test_connection_id() {
        let id = ConnectionId::new();
        assert_eq!(id.0, 1);
        let id = ConnectionId::new();
        assert_eq!(id.0, 2);

        let x_max = AtomicI32::new(i32::MAX);
        let count = x_max.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        assert_eq!(count, i32::MAX);

        let count = x_max.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        assert_eq!(count, i32::MIN);
    }

    #[ignore = "this is show size. not test"]
    #[test]
    fn test_size() {
        show_size!(i32);
        show_size!(ConnectionId);
    }
}
