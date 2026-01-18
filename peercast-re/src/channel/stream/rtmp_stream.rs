use std::task::{Poll, Waker};

use bytes::{BufMut, BytesMut};
use futures_core::Stream;
use http::Request;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tracing_subscriber::field::debug;

use crate::{channel::stream::rtmp2flv::Rtmp2Flv, prelude::*};
use libpeercast_re::{
    ConnectionNo,
    rtmp::stream_manager::{ConnectionMessage, StreamManagerMessage},
    util::util_mpsc::mpsc_send,
};

enum State {
    Connected,
    PlaybackRequesting,
    Playing,
    // Disconnecting,
    Disconnected,
}
pub struct RtmpStream {
    cno: ConnectionNo,
    rtmp_stream_manager: UnboundedSender<StreamManagerMessage>,
    message_reciever: UnboundedReceiver<ConnectionMessage>,
    disconnect_tx: UnboundedSender<()>,
    state: State,
    request_id_counter: u32,
    caller_sender: UnboundedSender<RtmpCallerMessage>,
    flv_converter: Option<Rtmp2Flv>,
}

enum RtmpCallerMessage {
    Delayed(std::task::Waker, std::time::Duration),
}
async fn rtmp_stream_caller(mut reciever: UnboundedReceiver<RtmpCallerMessage>) {
    while let Some(msg) = reciever.recv().await {
        match msg {
            RtmpCallerMessage::Delayed(waker, dur) => {
                tokio::time::sleep(dur).await;
                waker.wake();
            } // cx: &mut std::task::Context<'_>,
        }
    }
}

impl RtmpStream {
    pub fn new(
        cno: ConnectionNo,
        rtmp_stream_manager: UnboundedSender<StreamManagerMessage>,
        // message_reciever: UnboundedReceiver<ConnectionMessage>,
        // disconnect_tx: UnboundedSender<()>,
    ) -> Self {
        let (caller_tx, caller_reciever) = mpsc::unbounded_channel();
        let (messaeg_tx, message_reciever) = mpsc::unbounded_channel();
        let (disconnect_tx, disconnect_rx) = mpsc::unbounded_channel();
        let msg = libpeercast_re::rtmp::stream_manager::StreamManagerMessage::NewConnection {
            connection_id: cno.0,
            sender: messaeg_tx,
            disconnection: disconnect_rx,
        };
        let mut state = State::Connected;
        if !mpsc_send(&rtmp_stream_manager.clone(), msg) {
            error!("Failed to send NewConnection message to StreamManager");
            state = State::Disconnected
        }

        tokio::spawn(rtmp_stream_caller(caller_reciever));
        RtmpStream {
            cno,
            rtmp_stream_manager,
            message_reciever,
            disconnect_tx,
            state,
            request_id_counter: 1,
            caller_sender: caller_tx,
            flv_converter: None,
        }
    }

    fn handle_playging(
        mut self: std::pin::Pin<&mut Self>,
        msg: ConnectionMessage,
    ) -> Poll<Option<Result<bytes::Bytes, std::io::Error>>> {
        match msg {
            ConnectionMessage::NewMetadata {
                metadata,
            } => {
                debug!("New metadata received: {:#?}", metadata);
                self.flv_converter = Some(Rtmp2Flv::new(metadata));
                if let Some(flv_converter) = &mut self.flv_converter {
                    let flv_header = flv_converter.header();
                    debug!("FLV header generated");
                    Poll::Ready(Some(Ok(flv_header)))
                } else {
                    error!("FLV converter not initialized");
                    self.as_mut().state = State::Disconnected;
                    return Poll::Ready(None);
                }
            }
            ConnectionMessage::NewVideoData {
                timestamp,
                can_be_dropped: _,
                data,
            } => {
                debug!("New video data received: timestamp={:?} len={}", timestamp, data.len());
                if let Some(flv_converter) = &mut self.flv_converter {
                    let data = flv_converter.push_video_data(timestamp, data);
                    Poll::Ready(Some(Ok(data)))
                } else {
                    error!("FLV converter not initialized");
                    self.as_mut().state = State::Disconnected;
                    Poll::Ready(None)
                }
                // let mut newdata = BytesMut::with_capacity(data.len() + 5);
                // newdata.put(&b"VIDEO"[..]);
                // newdata.extend_from_slice(&data);
                // Poll::Ready(Some(Ok(newdata.freeze())))
            }
            ConnectionMessage::NewAudioData {
                timestamp,
                can_be_dropped: _,
                data,
            } => {
                debug!("New audio data received: timestamp={:?} len={}", timestamp, data.len());
                if let Some(flv_converter) = &mut self.flv_converter {
                    let data = flv_converter.push_audio_data(timestamp, data);
                    Poll::Ready(Some(Ok(data)))
                } else {
                    error!("FLV converter not initialized");
                    self.as_mut().state = State::Disconnected;
                    Poll::Ready(None)
                }
                // let mut newdata = BytesMut::with_capacity(data.len() + 5);
                // newdata.put(&b"AUDIO"[..]);
                // newdata.extend_from_slice(&data);
                // Poll::Ready(Some(Ok(newdata.freeze())))
            }
            ConnectionMessage::RequestAccepted {
                request_id,
            } => {
                unimplemented!()
            }
            ConnectionMessage::RequestDenied {
                request_id,
            } => {
                unimplemented!()
            }
        }
    }
}

impl Stream for RtmpStream {
    type Item = Result<bytes::Bytes, std::io::Error>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        match &self.state {
            State::Connected => {
                let message = StreamManagerMessage::PlaybackRequest {
                    request_id: self.request_id_counter,
                    rtmp_app: String::from("live"),
                    stream_key: String::from("livestream"),
                    connection_id: self.cno.0,
                };
                debug!("Request send: {}", self.request_id_counter);
                if !mpsc_send(&self.rtmp_stream_manager, message) {
                    error!("Request send Failed: {}", self.request_id_counter);
                    self.as_mut().state = State::Disconnected;
                    return std::task::Poll::Ready(None);
                }
                self.as_mut().state = State::PlaybackRequesting;
                cx.waker().clone().wake();
                Poll::Pending
            }
            State::PlaybackRequesting => {
                debug!("RtmpStream in PlaybackRequesting cno: {}", self.cno.0);
                return match self.message_reciever.poll_recv(cx) {
                    Poll::Pending => Poll::Pending,
                    Poll::Ready(None) => {
                        self.get_mut().state = State::Disconnected;
                        Poll::Ready(None)
                    }
                    Poll::Ready(Some(r)) => match r {
                        ConnectionMessage::RequestAccepted {
                            request_id,
                        } => {
                            debug!("Request accepted: {}", request_id);
                            self.as_mut().state = State::Playing;
                            self.as_mut().request_id_counter += 1;
                            if !mpsc_send(
                                &self.caller_sender,
                                RtmpCallerMessage::Delayed(cx.waker().clone(), std::time::Duration::from_secs(0)),
                            ) {
                                error!("Failed to send delayed wakeup message to RtmpCaller");
                                self.as_mut().state = State::Disconnected;
                                return Poll::Ready(None);
                            };
                            Poll::Pending
                        }
                        ConnectionMessage::RequestDenied {
                            request_id,
                        } => {
                            debug!("Request denied: {}", request_id);
                            self.as_mut().state = State::Disconnected;
                            Poll::Ready(None)
                        }
                        _ => {
                            unimplemented!("Unexpected message in PlaybackRequesting state");
                        }
                    },
                };
            }
            State::Playing => match self.message_reciever.poll_recv(cx) {
                Poll::Pending => Poll::Pending,
                Poll::Ready(None) => {
                    self.get_mut().state = State::Disconnected;
                    Poll::Ready(None)
                }
                Poll::Ready(Some(msg)) => self.handle_playging(msg),
            },
            // MEMO: いきなり切断してもよいのでこのままにしておく
            // State::Disconnecting => {
            //     let message = StreamManagerMessage::PlaybackFinished {
            //         connection_id: self.cno.0,
            //     };
            //     if !mpsc_send(&self.rtmp_stream_manager, message) {
            //         error!("Failed to send delayed wakeup message to RtmpCaller");
            //         self.as_mut().state = State::Disconnected;
            //         return Poll::Ready(None);
            //     };
            //     self.as_mut().state = State::Disconnected;
            //     cx.waker().clone().wake();
            //     Poll::Pending
            // }
            State::Disconnected => {
                debug!("RtmpStream disconnected: {}", self.cno.0);
                Poll::Ready(None)
            }
        }
    } // fn poll_next()
}

#[cfg(test)]
mod t {
    use super::*;
    use crate::test_helper::assert_unpin;

    #[test]
    fn it_works() {
        assert_unpin::<RtmpStream>();
    }
}
