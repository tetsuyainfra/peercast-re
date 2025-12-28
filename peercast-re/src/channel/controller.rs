use libpeercast_re::pcp::GnuId;
use tokio::sync::mpsc;

//
enum ControllerMessage {
    Connect,
    Disconnect,
}

//
enum FutureResult {
    Disconnection {
        connection_id: i32,
    },
    MessageReceived {
        receiver: mpsc::UnboundedReceiver<ControllerMessage>,
        message: Option<ControllerMessage>,
    },
}

pub struct ChannelController {
    valid_channel_id: GnuId,
}

impl ChannelController {
    pub fn new(channel_id: GnuId) -> Self {
        Self {
            valid_channel_id: channel_id,
        }
    }

    /// let (sender, reciever) = tokio::sync::mpsc::unbounded_channel();
    /// let controller = ChannelController::new()
    /// tokio::spawn(controller.run(reciever))
    /// # use sender to send message to manager on ReChannel
    pub(super) fn run(self, mut reciever: tokio::sync::mpsc::UnboundedReceiver<ControllerMessage>) {
        async fn new_receiver_future(mut receiver: mpsc::UnboundedReceiver<ControllerMessage>) -> FutureResult {
            let result = receiver.recv().await;
            FutureResult::MessageReceived {
                receiver,
                message: result,
            }
        }

        let ChannelController {
            valid_channel_id,
        } = self;
    }
}
