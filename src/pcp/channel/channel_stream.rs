use std::task::Poll;

use axum_core::BoxError;
use bytes::Bytes;
use futures_util::Stream;
use rml_rtmp::sessions::StreamMetadata;
use tokio::sync::mpsc;
use tracing::{info, trace};

use crate::pcp::GnuId;

use super::{ChannelMessage, ChannelReciever};

#[derive(Debug)]
pub struct ChannelStream {
    channel_id: GnuId,
    receiver: ChannelReciever,
    is_sent_header: bool,
    is_sent_keyframe: bool,
}

impl ChannelStream {
    pub(super) fn new(channel_id: GnuId, receiver: ChannelReciever) -> Self {
        trace!("ChannelStream::new()");
        Self {
            channel_id,
            receiver,
            is_sent_header: false,
            is_sent_keyframe: false,
        }
    }
}

impl Stream for ChannelStream {
    type Item = Result<Bytes, BoxError>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        match self.receiver.poll_recv(cx) {
            Poll::Pending => {
                // polling
                Poll::Pending
            }
            Poll::Ready(None) => {
                info!("FINISH ChannelStream CID:{}", self.channel_id);
                Poll::Ready(Some(Err("something error".into())))
            }
            Poll::Ready(Some(msg)) => {
                match msg {
                    ChannelMessage::AtomChanHead {
                        atom,
                        pos,
                        data,
                        //
                    } => {
                        self.is_sent_header = true;
                        Poll::Ready(Some(Ok(data)))
                    }
                    ChannelMessage::AtomChanData {
                        data,
                        pos,
                        can_be_dropped,
                    } => {
                        if self.is_sent_keyframe {
                            // keyframeを送った後はガンガン送信してよい
                            Poll::Ready(Some(Ok(data)))
                        } else {
                            // keyframe未送信
                            if can_be_dropped {
                                Poll::Ready(None)
                            } else {
                                self.is_sent_keyframe = true;
                                Poll::Ready(Some(Ok(data)))
                            }
                        }
                    }
                }
            }
        } // poll_recv(cx)
    }
}

#[cfg(test)]
mod t {
    use std::time::Duration;

    use super::*;

    #[crate::test]
    async fn test() {
        // let (tx, rx) = mpsc::unbounded_channel();
        // tokio::spawn(async move {
        //     //
        //     loop {
        //         match tx.send(ChannelMessage::NewData {
        //             data: Bytes::from_static(b"1"),
        //         }) {
        //             Ok(_) => {}
        //             Err(e) => break,
        //         };
        //         tokio::time::sleep(Duration::from_secs(1)).await;
        //     }
        //     info!("shutdown spawn");
        // });

        // let stream = ChannelStream {
        //     receiver: ChannelReciever,
        //     is_send_metadata: false,
        // };
    }
}
