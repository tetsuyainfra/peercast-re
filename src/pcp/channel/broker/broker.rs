//------------------------------------------------------------------------------
// ChannelBroker
//

use core::panic;
use std::{
    collections::{HashMap, VecDeque},
    fmt::Debug,
    sync::{Arc, RwLock},
};

use axum::routing::head;
use byteorder::WriteBytesExt;
use bytes::{Buf, BufMut, Bytes, BytesMut};
use futures_util::{
    future::{select_all, BoxFuture},
    FutureExt,
};
use nom::AsBytes;
use pbkdf2::hmac::digest::core_api::Buffer;
use peercast_re_api::models::channel_track;
use rml_rtmp::{sessions::StreamMetadata, time::RtmpTimestamp};
use serde::{Deserialize, Serialize};
use tokio::{
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};
use tracing::{debug, error, trace, warn};

use crate::{
    codec::rtmp::flv::{self, TaggedData},
    pcp::{
        builder::{ChannelInfoBuilder, TrackInfoBuilder},
        classify::{self, ChanPktDataType},
        Atom, ChildAtom, GnuId, Id4, ParentAtom,
    },
    rtmp::rtmp_connection::RtmpConnectionEvent,
    util::util_mpsc::send,
    ConnectionId,
};

use super::{ChannelInfo, TrackInfo, ChannelBrokerMessage, ChannelReciever, ChannelMessage};




/// Channelで扱いやすくするためのクラス
#[derive(Debug)]
pub(crate) struct ChannelBroker {
    manager_tx: mpsc::UnboundedSender<ChannelBrokerMessage>,
    task: JoinHandle<()>,
    task_shutdown_tx: mpsc::UnboundedSender<()>,
}

impl ChannelBroker {
    pub fn new(
        channel_id: GnuId,
        channel_info: Arc<RwLock<Option<ChannelInfo>>>,
        track_info: Arc<RwLock<Option<TrackInfo>>>,
    ) -> Self {
        let (manager_tx, manager_rx) = mpsc::unbounded_channel();
        let (task_shutdown_tx, task_shutdown_rx) = mpsc::unbounded_channel();
        let broker =
            ChannelBrokerWoker::new(channel_id, channel_info, track_info, task_shutdown_rx);

        let task = tokio::spawn(broker.start(manager_rx));

        Self {
            manager_tx,
            task,
            task_shutdown_tx,
        }
    }

    pub fn sender(&self) -> UnboundedSender<ChannelBrokerMessage> {
        self.manager_tx.clone()
    }

    pub fn channel_reciever(&self, connection_id: ConnectionId) -> ChannelReciever {
        ChannelReciever::create(self.sender(), connection_id)
    }
}

pub struct HeadAtom {
    atom: Atom,
    pos: u32,
    //
    magic_with_data: Bytes,
    data: Bytes,
}

/// ChannelBrokerの実処理が行われるWoker
struct ChannelBrokerWoker {
    channel_id: GnuId,
    shutdown_rx: UnboundedReceiver<()>,
    //
    sender_by_connection_id: HashMap<ConnectionId, mpsc::UnboundedSender<ChannelMessage>>,
    relays_ids: Vec<ConnectionId>,
    //
    new_disconnect_futures: Vec<BoxFuture<'static, FutureResult>>,
    new_disconnections: Vec<(ConnectionId, mpsc::UnboundedReceiver<()>)>,
    //
    head_atom: Option<HeadAtom>,
    channel_info: Arc<RwLock<Option<ChannelInfo>>>,
    track_info: Arc<RwLock<Option<TrackInfo>>>,

    // Rtmp -> Flv Stream (packet)
    flv_position: u32,
    flvnizer: RtmpFlvnizer,
    // Flv -> Atom
}

// Broker分けた方がシンプルになると思う
impl ChannelBrokerWoker {
    fn new(
        channel_id: GnuId,
        channel_info: Arc<RwLock<Option<ChannelInfo>>>,
        track_info: Arc<RwLock<Option<TrackInfo>>>,
        shutdown_rx: UnboundedReceiver<()>,
    ) -> Self {
        Self {
            channel_id,
            shutdown_rx,
            //
            sender_by_connection_id: HashMap::new(),
            relays_ids: Default::default(),
            //
            new_disconnect_futures: Vec::new(),
            new_disconnections: Vec::new(),
            //
            head_atom: None,
            channel_info,
            track_info,
            //
            flvnizer: RtmpFlvnizer::new(),
            flv_position: 0,
        }
    }

    fn cleanup_connection(&mut self, connection_id: ConnectionId) {
        println!("Stream manager is removing connection id {}", connection_id);

        self.sender_by_connection_id.remove(&connection_id);
        // if let Some(key) = self.key_by_connection_id.remove(&connection_id) {
        //     if let Some(players) = self.players_by_key.get_mut(&key) {
        //         players.remove(&connection_id);
        //     }

        //     if let Some(details) = self.publish_details.get_mut(&key) {
        //         if details.connection_id == connection_id {
        //             self.publish_details.remove(&key);
        //         }
        //     }
        // }
    }

    async fn start(mut self, receiver: mpsc::UnboundedReceiver<ChannelBrokerMessage>) {
        debug!("CID:{} ChannelBrokerWorker START", self.channel_id);
        async fn new_receiver_future(
            mut receiver: UnboundedReceiver<ChannelBrokerMessage>,
        ) -> FutureResult {
            let result = receiver.recv().await;
            FutureResult::MessageReceived {
                receiver,
                message: result,
            }
        }

        // select_allはFutureのリストを処理して、最初にreadyになったfutureの値とindexを返す(loop内 futures.await)
        // https://docs.rs/futures/latest/futures/future/fn.select_all.html
        // messageの受信とdisconnectionを並列して処理しなくてはならず、messageの受信はともかく、connection切断は複数あり得るのでこうなってる
        let mut futures = select_all(vec![new_receiver_future(receiver).boxed()]);

        loop {
            let (result, _index, remaining_futures) = futures.await;
            let mut new_futures = Vec::from(remaining_futures);

            // trace!(message = ?result);
            match result {
                FutureResult::MessageReceived { receiver, message } => {
                    match message {
                        Some(message) => self.handle_message(message),
                        None => break,
                    };
                    new_futures.push(new_receiver_future(receiver).boxed()); // メッセージを処理したら、新たにリストに処理待ちする
                }

                FutureResult::Disconnection { connection_id } => {
                    self.cleanup_connection(connection_id)
                }
            }

            for future in self.new_disconnect_futures.drain(..) {
                new_futures.push(future);
            }

            futures = select_all(new_futures);
        }

        // Shutdown(終了処理)
        debug!("CID:{} ChannelBrokerWorker FINISH", self.channel_id);
    }

    fn handle_message(&mut self, message: ChannelBrokerMessage) {
        match message {
            ChannelBrokerMessage::NewConnection {
                connection_id,
                sender,
                disconnection,
            } => self.handle_new_connection(connection_id, sender, disconnection),
            ChannelBrokerMessage::UpdateChannelInfo { info, track } => {
                // これは主にBroadcast側で実行される
                let mut lock_info = self.channel_info.write().unwrap();
                let mut lock_track = self.track_info.write().unwrap();
                *lock_info = Some(info);
                *lock_track = Some(track);
            }
            ChannelBrokerMessage::AtomHeadData {
                atom,
                payload,
                pos,
                info,
                track,
            } => {
                self.handle_head_data(atom, payload, pos, info, track);
            }
            ChannelBrokerMessage::AtomData {
                atom,
                data,
                pos,
                continuation,
            } => {
                self.handle_data(atom, data, pos, continuation);
            }
            ChannelBrokerMessage::AtomBroadcast { direction, atom } => todo!(),
            ChannelBrokerMessage::BroadcastEvent(event) => {
                //
                self.handle_rtmp_event(event)
            }
        }
    }

    fn handle_new_connection(
        &mut self,
        connection_id: ConnectionId,
        mut sender: UnboundedSender<ChannelMessage>,
        disconnection: UnboundedReceiver<()>,
    ) {
        // metadataが有れば送っておく
        if (self.head_atom.is_some()) {
            let HeadAtom {
                atom,
                pos,
                magic_with_data,
                data,
            } = self.head_atom.as_ref().unwrap();
            send(
                &mut sender,
                ChannelMessage::AtomChanHead {
                    atom: atom.clone(),
                    pos: *pos,
                    data: magic_with_data.clone(),
                },
            );
        }
        match self.sender_by_connection_id.insert(connection_id, sender) {
            Some(_sender) => {
                error!(?connection_id, "connection id never overlap.");
                panic!("connection id never overlap.");
            }
            None => {}
        };
        self.new_disconnect_futures
            .push(Self::wait_for_client_disconnection(connection_id, disconnection).boxed());
    }

    fn handle_head_data(
        &mut self,
        atom: Atom,
        payload: Bytes,
        pos: u32,
        info: Option<ChannelInfo>,
        track: Option<TrackInfo>,
    ) {
        // FIXME: 到着したPayloadに含まれるTaggedFLVデータをパースしてCodecのHeaderを更新しなくてはいけない
        {
            let mut lock_info = self.channel_info.write().unwrap();
            let mut lock_track = self.track_info.write().unwrap();
            if info.is_some() {
                *lock_info = info.clone();
            }
            if track.is_some() {
                *lock_track = track.clone();
            }
        }
        if self.head_atom.is_none() {
            // 初回更新なのでmagic_withも更新する
            let head_atom = HeadAtom {
                atom: atom,
                pos: pos,
                magic_with_data: payload.clone(),
                data: payload,
            };
            self.head_atom = Some(head_atom);
        } else {
            // 2回目以降なのでdataだけ更新する
            // FIXME: Relayの場合magic_with_dataを更新してくれないので自分で中身解析する必要がある
            let mut head_atom = self.head_atom.take().unwrap();
            head_atom.atom = atom;
            head_atom.pos = pos;
            head_atom.data = payload;
            self.head_atom = Some(head_atom);
        }
        let HeadAtom {
            atom,
            pos,
            magic_with_data,
            data,
        } = self.head_atom.as_ref().unwrap();
        self.send_listener(ChannelMessage::AtomChanHead {
            atom: atom.clone(),
            pos: pos.clone(),
            data: data.clone(),
        })
    }

    fn handle_data(&self, atom: Atom, data: Bytes, pos: u32, continuation: Option<bool>) {
        // TODO: continuationの扱いをどうするか
        let can_be_dropped = match continuation {
            Some(cont) => cont,
            None => false,
        };
        let msg = ChannelMessage::AtomChanData {
            data,
            pos,
            can_be_dropped,
        };
        self.send_listener(msg)
    }

    // BroadcastTaskからRtmpに関するメッセージを受け取りAtomに変換してRecieverに送り出す
    fn handle_rtmp_event(&mut self, event: RtmpConnectionEvent) {
        // trace!(?event);
        let flv_tagged = match event {
            RtmpConnectionEvent::NewMetadata { metadata } => self.flvnizer.write_meta(metadata),
            RtmpConnectionEvent::NewVideoData {
                timestamp,
                data,
                can_be_dropped,
            } => self
                .flvnizer
                .write_video(timestamp.value, data, can_be_dropped),
            RtmpConnectionEvent::NewAudioData {
                timestamp,
                data,
                can_be_dropped,
            } => self
                .flvnizer
                .write_audio(timestamp.value, data, can_be_dropped),
        };

        // trace!(?flv_tagged);
        match flv_tagged {
            // まだヘッダーがそろっていない
            None => {
                trace!(flv_tagged = "None");
            }
            // データを送信する
            Some(FlvnizedData::UpdateHeader {
                tagged_data,
                magic_with_data,
            }) => {
                trace!(flv_tagged = "UpdateHeader");
                // 情報が無いので自分をコピーして送るしかない
                let mut info = None;
                let mut track = None;
                {
                    let info_lock = self.channel_info.read().unwrap();
                    let track_lock = self.track_info.read().unwrap();
                    info = info_lock.clone();
                    track = track_lock.clone();
                }

                // tagged_data.len(), magic_with_data.len()は当然違う長さになる。
                // それは、この後のパケットがあったとしたら同一の位置になることを保障するため
                // ※ 要はFLVファイル先頭のb"FLV\1..."は長さの計算に入れない
                if self.head_atom.is_none() {
                    // 一番最初はb"FLV\1"を含む長さを入れる
                    assert_eq!(self.flv_position, 0);
                    self.flv_position = magic_with_data.len() as u32;

                    let atom = create_atom(
                        self.channel_id,
                        ChanPktDataType::Head,
                        info.clone(),
                        track.clone(),
                        self.flv_position,
                        None,
                        &magic_with_data,
                    );
                    self.handle_head_data(atom, magic_with_data, self.flv_position, info, track);
                } else {
                    self.flv_position = self.flv_position + tagged_data.len() as u32;

                    // 初回接続時のデータを書き換え
                    let mut head_atom = self.head_atom.take().unwrap();
                    head_atom.magic_with_data = magic_with_data;
                    self.head_atom = Some(head_atom);
                    debug_assert!(self.head_atom.is_some());

                    // atomを送ってもらう
                    let atom = create_atom(
                        self.channel_id,
                        ChanPktDataType::Head,
                        info,
                        track,
                        self.flv_position,
                        None,
                        &tagged_data,
                    );
                    self.handle_head_data(atom, tagged_data, self.flv_position, None, None);
                }
            }
            Some(FlvnizedData::Data {
                tagged_data,
                can_be_dropped,
            }) => {
                trace!(flv_tagged = "Data");
                self.flv_position = self.flv_position + tagged_data.len() as u32;

                let atom = create_atom(
                    self.channel_id,
                    ChanPktDataType::Data,
                    None,
                    None,
                    self.flv_position,
                    None,
                    &tagged_data,
                );
                self.handle_data(atom, tagged_data, self.flv_position, Some(can_be_dropped))
            }
        }
    }

    fn send_listener(&self, message: ChannelMessage) {
        for (id, sender) in &self.sender_by_connection_id {
            send(sender, message.clone());
        }
    }

    // 送られてきたrecieverをラップするselect_allできるようにする
    async fn wait_for_client_disconnection(
        connection_id: ConnectionId,
        mut receiver: UnboundedReceiver<()>,
    ) -> FutureResult {
        // The channel should only be closed when the client has disconnected
        while let Some(()) = receiver.recv().await {}

        FutureResult::Disconnection { connection_id }
    }
}

fn create_atom(
    broadcast_id: GnuId,
    chan_data_type: ChanPktDataType,
    info: Option<ChannelInfo>,
    track: Option<TrackInfo>,
    pos: u32,
    continuation: Option<bool>,
    data: &Bytes,
) -> Atom {
    // See: Channel Atom structure -> pcp/classify.rs
    // https://github.com/plonk/peercast-yt/blob/787be6405cc2d82a5d26c0023aaa5d1973c13802/core/common/servent.cpp#L1883
    let chan_pkt_childs: Vec<Atom> = match (&chan_data_type, continuation) {
        (&ChanPktDataType::Head, _) => {
            vec![
                ChildAtom::from((Id4::PCP_CHAN_PKT_TYPE, Id4::PCP_CHAN_PKT_HEAD.0)).into(),
                ChildAtom::from((Id4::PCP_CHAN_PKT_POS, pos)).into(),
                ChildAtom::from((Id4::PCP_CHAN_PKT_DATA, data)).into(),
            ]
        }
        (&ChanPktDataType::Data, Some(true)) => {
            vec![
                ChildAtom::from((Id4::PCP_CHAN_PKT_TYPE, Id4::PCP_CHAN_PKT_DATA.0)).into(),
                ChildAtom::from((Id4::PCP_CHAN_PKT_POS, pos)).into(),
                ChildAtom::from((Id4::PCP_CHAN_PKT_CONTINUATION, 1_u8)).into(),
                ChildAtom::from((Id4::PCP_CHAN_PKT_DATA, data)).into(),
            ]
        }
        (&ChanPktDataType::Data, Some(false)) | (&ChanPktDataType::Data, None) => {
            vec![
                ChildAtom::from((Id4::PCP_CHAN_PKT_TYPE, Id4::PCP_CHAN_PKT_DATA.0)).into(),
                ChildAtom::from((Id4::PCP_CHAN_PKT_POS, pos)).into(),
                ChildAtom::from((Id4::PCP_CHAN_PKT_DATA, data)).into(),
            ]
        }
    };

    match (chan_data_type) {
        ChanPktDataType::Head => {
            //
            ParentAtom::from((
                Id4::PCP_CHAN,
                vec![
                    ChildAtom::from((Id4::PCP_CHAN_ID, broadcast_id)).into(),
                    ChannelInfoBuilder::new(info.unwrap()).build(),
                    TrackInfoBuilder::new(track.unwrap()).build(),
                    ParentAtom::from((Id4::PCP_CHAN_PKT, chan_pkt_childs)).into(),
                ],
            ))
            .into()
        }
        ChanPktDataType::Data => {
            //
            ParentAtom::from((
                Id4::PCP_CHAN,
                vec![
                    ChildAtom::from((Id4::PCP_CHAN_ID, broadcast_id)).into(),
                    ParentAtom::from((Id4::PCP_CHAN_PKT, chan_pkt_childs)).into(),
                ],
            ))
            .into()
        }
    }
}

#[derive(Debug)]
enum FutureResult {
    Disconnection {
        connection_id: ConnectionId,
    },
    MessageReceived {
        receiver: UnboundedReceiver<ChannelBrokerMessage>,
        message: Option<ChannelBrokerMessage>,
    },
}

////////////////////////////////////////////////////////////////////////////////
// RtmpToFLVAtom Serializer
//

// struct FlvnizedData {
//     // 単に入力されたFVLのTagをつけたデータに変換したバッファ
//     tagged_data: Bytes,
//     // FLVのヘッダ、Audio/Videoのヘッダが付いたバッファ (再接続してきたリモートに渡して途中から再生できるようにする）
//     magic_with_data: Option<Bytes>,
// }
#[derive(Debug)]
enum FlvnizedData {
    UpdateHeader {
        tagged_data: Bytes,
        magic_with_data: Bytes,
    },
    Data {
        tagged_data: Bytes,
        can_be_dropped: bool,
    },
}

struct RtmpFlvnizer {
    metadata: Option<StreamMetadata>,
    audio_header: Option<(u32, Bytes)>, // (timestamp, data)
    video_header: Option<(u32, Bytes)>, // (timestamp, data)
    magic_with_header: Option<Bytes>,
}

impl RtmpFlvnizer {
    fn new() -> Self {
        Self {
            metadata: None,
            audio_header: None,
            video_header: None,
            magic_with_header: None,
        }
    }

    fn write_meta(&mut self, metadata: StreamMetadata) -> Option<FlvnizedData> {
        self.metadata = Some(metadata);
        self.audio_header = None; // TODO: 消すべきかなんとも分らん
        self.video_header = None; //
        None
    }

    // Noneが帰ってくるのはHeaderの準備ができていないため
    fn write_video(
        &mut self,
        timestamp: u32,
        data: Bytes,
        can_be_dropped: bool,
    ) -> Option<FlvnizedData> {
        if data.len() < 5 {
            warn!("rtmp video payload should be more bigger... {:?}", data);
            return None;
        }

        let mut magic_with_data = None;
        if Self::is_avc_header(&data[0..5]) {
            self.video_header = Some((timestamp, data.clone()));
            if !self.set_header() {
                return None;
            }
            // Headerの更新
            magic_with_data = Some(self.magic_with_header.as_ref().unwrap().clone());
        }

        let tagged_data = Self::flved(1, flv::DataType::VIDEO, timestamp, &data);

        if let Some(magic_with_data) = magic_with_data {
            return Some(FlvnizedData::UpdateHeader {
                tagged_data,
                magic_with_data,
            });
        } else {
            return Some(FlvnizedData::Data {
                tagged_data,
                can_be_dropped,
            });
        }
    }

    // See: write_video
    fn write_audio(
        &mut self,
        timestamp: u32,
        data: Bytes,
        can_be_dropped: bool,
    ) -> Option<FlvnizedData> {
        if data.len() < 2 {
            warn!("rtmp audio payload should be more bigger... {:?}", &data);
            return None;
        }

        let tagged_data = Self::flved(1, flv::DataType::AUDIO, timestamp, &data);
        let mut magic_with_data = None;
        if Self::is_aac_header(&data[0..2]) {
            self.audio_header = Some((timestamp, data.clone()));
            if !self.set_header() {
                return None;
            } else {
                // Headerの更新
                magic_with_data = Some(self.magic_with_header.as_ref().unwrap().clone());
            }
        }

        if let Some(magic_with_data) = magic_with_data {
            return Some(FlvnizedData::UpdateHeader {
                tagged_data,
                magic_with_data,
            });
        } else {
            return Some(FlvnizedData::Data {
                tagged_data,
                can_be_dropped,
            });
        }
    }

    // headerが作成されたらtrueを返す
    fn set_header(&mut self) -> bool {
        if (self.metadata.is_none()) {
            return false;
        }
        let has_audio = self.metadata.as_ref().unwrap().audio_codec_id.is_some();
        if (!has_audio || self.audio_header.is_none()) {
            return false;
        }
        let has_video = self.metadata.as_ref().unwrap().video_codec_id.is_some();
        if (!has_video || self.video_header.is_none()) {
            return false;
        }

        let mut magic_buf = Self::write_magic(1, 9, has_audio, has_video);

        let (timestamp, data) = self.video_header.as_ref().unwrap();
        let video_header_buf = Self::flved(1, flv::DataType::VIDEO, *timestamp, data);
        magic_buf.put(&video_header_buf[..]);
        let (timestamp, data) = self.audio_header.as_ref().unwrap();
        let audio_header_buf = Self::flved(1, flv::DataType::AUDIO, *timestamp, data);
        magic_buf.put(&audio_header_buf[..]);

        self.magic_with_header = Some(magic_buf.freeze());
        return true;
    }

    fn write_magic(version: u8, offset: u32, has_audio: bool, has_video: bool) -> BytesMut {
        let mut buf = BytesMut::new();
        buf.put(&b"FLV"[..]);
        buf.put_u8(version);
        let mut flags = 0_u8;
        if has_video {
            flags |= 0b0000_0001
        };
        if has_audio {
            flags |= 0b0000_0100
        };
        buf.put_u8(flags);
        buf.put_u32(offset);
        buf.put_u32(0_u32); // write empty previous tag
        buf
    }

    fn flved(stream_id: u32, data_type: flv::DataType, timestamp: u32, data: &Bytes) -> Bytes {
        let mut buf = BytesMut::new();
        //
        let tag_type = data_type.0;
        let timestamps = timestamp.to_be_bytes();
        let stream_ids = stream_id.to_be_bytes();
        let data_size = (data.len() as u32).to_be_bytes();
        let tag_size = (data.len() + 11) as u32;
        //
        let tag = [
            // tag type
            tag_type,
            // datasize (24bit)
            data_size[1],
            data_size[2],
            data_size[3],
            // timestamp (LSB 24bit)
            timestamps[1],
            timestamps[2],
            timestamps[3],
            // timestamp extend (MSB 8bit)
            timestamps[0],
            // stream_id 24bit
            stream_ids[1],
            stream_ids[2],
            stream_ids[3],
        ];
        // write header
        buf.put(&tag[..]);
        // payload
        buf.put(&data[..]);
        // tag_size
        debug_assert_eq!(buf.len() as u32, tag_size);
        buf.put_u32(tag_size);

        buf.freeze()
    }

    // fn tag(data_type: Data) -> TaggedChunk {}
    fn is_avc_header(head: &[u8]) -> bool {
        trace!(is_avc_header=?head);
        // FLV Video Tag [E.4.3 Video Tags, pp72]
        // Field           ValueType
        // FrameType       UB[4]
        // CodecID         UB[4]
        // AVCPacketType   if Codec ID == 7 UI8  // 1byte
        // CompotionTime   if Codec ID == 7 SI24 // 3byte
        // Field
        let frame_type = (head[0] & 0xF0) >> 4;
        let codec_id = (head[0] & 0x0F);
        let avc_packet_type = if codec_id == 7 { Some(head[1]) } else { None };
        let compotion_time = if codec_id == 7 {
            let t = [0, head[2], head[3], head[4]].as_bytes().get_u32();
            Some(t)
        } else {
            None
        };

        match avc_packet_type {
            Some(0x00) => true,  // AVC seqence
            Some(0x01) => false, // AVC NAUL
            Some(0x02) => false, // AVC end sequence
            Some(_) => false,
            None => false,
        }
    }

    fn is_aac_header(head: &[u8]) -> bool {
        match head[0] {
            0xAF => {
                //debug!("AAC,44 kHz,16bit samples,Stereo sound");
            }
            _ => {
                debug!("AUDIO CODEC is unknown")
            }
        };
        match head[1] {
            0x00 => {
                debug!("aac header");
                true
            }
            0x01 => {
                // trace!("aac packet");
                false
            }
            _ => false,
        }
    }
}


#[cfg(test)]
mod t {
    use std::time::Duration;
    use tokio::time;

    use super::*;
    use crate::{
        pcp::{ChildAtom, Id4},
        test_helper::*,
    };

    #[crate::test]
    async fn test_broker_worker() {
        init_logger("trace");

        let (shutdown_tx, shutdown_rx) = mpsc::unbounded_channel();
        let (broker_sender, broker_receiver) = mpsc::unbounded_channel();

        let worker = ChannelBrokerWoker::new(
            GnuId::new(),
            Default::default(),
            Default::default(),
            shutdown_rx,
        );
        let h = tokio::spawn(async move {
            worker.start(broker_receiver).await;
        });

        drop(broker_sender);

        h.await;
    }

    #[crate::test]
    async fn test_broker() {
        init_logger("trace");

        let broker = ChannelBroker::new(GnuId::new(), Default::default(), Default::default());

        let mut reciever = broker.channel_reciever(ConnectionId::new());
        let handle = tokio::spawn(async move { reciever.recv().await });

        let atom: Atom = Atom::Child(ChildAtom::from((Id4::PCP_HELO, 1_u8)));
        let payload = Bytes::new();
        broker.sender().send(ChannelBrokerMessage::AtomHeadData {
            atom,
            payload,
            pos: 0,
            info: Some(ChannelInfo::new()),
            track: Some(TrackInfo::new()),
        });

        let r = handle.await.unwrap();
        assert!(r.is_some());
        println!("{r:#?}");
    }

    #[crate::test]
    async fn test_channel_reciever() {
        is_send::<ChannelReciever>();
        is_sync::<ChannelReciever>();
    }
}
