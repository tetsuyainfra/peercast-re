// Original source from : rml_rtmph
// LICENSE : MIT

use tokio::sync::mpsc;

/// Sends a message over an unbounded receiver and returns true if the message was sent
/// or false if the channel has been closed.
pub fn mpsc_send<T>(sender: &mpsc::UnboundedSender<T>, message: T) -> bool {
    match sender.send(message) {
        Ok(_) => true,
        Err(_) => false,
    }
}
