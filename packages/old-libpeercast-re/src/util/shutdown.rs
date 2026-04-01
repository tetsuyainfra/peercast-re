// original code: https://github.com/tokio-rs/mini-redis/blob/master/src/shutdown.rs
// LICENSE: MIT
use tokio::sync::{broadcast, mpsc};

/// Listens for the server shutdown signal.
///
/// Shutdown is signalled using a `broadcast::Receiver`. Only a single value is
/// ever sent. Once a value has been sent via the broadcast channel, the server
/// should shutdown.
///
/// The `Shutdown` struct listens for the signal and tracks that the signal has
/// been received. Callers may query for whether the shutdown signal has been
/// received or not.
#[derive(Debug)]
pub(crate) struct Shutdown {
    /// `true` if the shutdown signal has been received
    is_shutdown: bool,

    /// The receive half of the channel used to listen for shutdown.
    notify: broadcast::Receiver<()>,
}

impl Shutdown {
    /// Create a new `Shutdown` backed by the given `broadcast::Receiver`.
    pub(crate) fn new(notify: broadcast::Receiver<()>) -> Shutdown {
        Shutdown {
            is_shutdown: false,
            notify,
        }
    }

    /// Returns `true` if the shutdown signal has been received.
    pub(crate) fn is_shutdown(&self) -> bool {
        self.is_shutdown
    }

    /// Receive the shutdown notice, waiting if necessary.
    pub(crate) async fn recv(&mut self) {
        // If the shutdown signal has already been received, then return
        // immediately.
        if self.is_shutdown {
            return;
        }

        // Cannot receive a "lag error" as only one value is ever sent.
        let _ = self.notify.recv().await;

        // Remember that the signal has been received.
        self.is_shutdown = true;
    }
}

#[cfg(test)]
mod t {
    use super::*;
    use futures_util::FutureExt;
    use tokio::sync::watch;

    #[crate::test]
    async fn test_watch() {
        let (tx, mut rx) = watch::channel(());

        tx.send(());

        let r = rx.changed().await;
        assert_eq!(*rx.borrow(), ());

        tx.send(());

        let r = rx.changed().await;
        assert_eq!(*rx.borrow(), ());
    }

    #[crate::test]
    async fn test_shutdown() {
        let (tx, mut rx) = watch::channel(0);

        tx.send(1);

        let r = rx.changed().await;
        assert_eq!(*rx.borrow(), 1);

        tx.send(1);

        let r = rx.changed().await;
        assert_eq!(*rx.borrow(), 1);
    }
}
