use crate::prelude::*;
use futures::Stream;
use libpeercast_re::pcp::GnuId;

pub struct ReStream {
    channel_id: GnuId,
    // receiver: ChannelReciever,
    is_sent_header: bool,
    is_sent_keyframe: bool,
}
impl ReStream {
    // pub fn new(channel_id: GnuId, receiver: ChannelReciever) -> Self {
    pub fn new(channel_id: GnuId) -> Self {
        trace!("ChannelStream::new()");
        Self {
            channel_id,
            // receiver,
            is_sent_header: false,
            is_sent_keyframe: false,
        }
    }
}

impl Stream for ReStream {
    type Item = Result<bytes::Bytes, std::io::Error>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        use std::task::Poll;
        todo!()
        // match self.receiver.poll_recv(cx) {
        //     Poll::Pending => {
        //         // polling
        //         Poll::Pending
        //     }
        //     Poll::Ready(None) => {
        //         info!("FINISH ChannelStream CID:{}", self.channel_id);
        //         Poll::Ready(Some(Err("something error".into())))
        //     }
        //     Poll::Ready(Some(msg)) => {
        //         match msg {
        //             ChannelMessage::RelayChannelHead {
        //                 pos,
        //                 payload,
        //                 ..
        //             } => {
        //                 self.is_sent_header = true;
        //                 Poll::Ready(Some(Ok(payload)))
        //             }
        //             ChannelMessage::RelayChannelData {
        //                 atom,
        //                 payload,
        //                 pos,
        //                 continuation,
        //             } => {
        //                 if self.is_sent_keyframe {
        //                     // keyframeを送った後はガンガン送信してよい
        //                     Poll::Ready(Some(Ok(payload)))
        //                 } else {
        //                     // keyframe未送信
        //                     if continuation {
        //                         Poll::Ready(None)
        //                     } else {
        //                         self.is_sent_keyframe = true;
        //                         Poll::Ready(Some(Ok(payload)))
        //                     }
        //                 }
        //             }
        //         }
        //     }
        // }
    }
}
