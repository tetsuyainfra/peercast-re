use std::{
    collections::VecDeque,
    fmt::Write,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpStream},
    str,
    time::Duration,
};

use bytes::{Buf, BufMut, BytesMut};
use http::{Request, StatusCode, Version};
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tracing::{debug, log, trace};

use crate::{
    error::{self, AtomParseError, HandshakeError},
    pcp::{
        atom::read_atom,
        builder::{HelloBuilder, OlehInfo},
        procedure::http_req::parse_pcp_http_response,
        Atom, GnuId, Id4,
    },
};

use super::http_req::RequestHead;

#[derive(Debug)]
pub struct Handshake<T> {
    connection_id: u64,
    stream: T,
    read_buf: BytesMut,
    broadcast_id: GnuId, // 配信IDともいう
    session_id: GnuId,   // クライアント毎のランダムID
    oleh_info: Option<OlehInfo>,
}

impl<T> Handshake<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    pub fn new(
        connection_id: u64,
        stream: T,
        read_buf: BytesMut,
        session_id: GnuId,
        broadcast_id: GnuId,
    ) -> Self {
        Handshake {
            connection_id,
            stream,
            read_buf,
            broadcast_id,
            session_id,
            oleh_info: None,
        }
    }

    pub async fn hello(&mut self) -> Result<(), HandshakeError> {
        let req = Request::builder()
            .method("GET")
            .uri(format!("/channel/{}", &self.broadcast_id))
            .header("x-peercast-pcp", "1")
            .body(())
            .unwrap();

        let (parts, body) = req.into_parts();
        let mut req_buf: BytesMut = RequestHead::new(parts).into();

        // ヘッダを送る
        while req_buf.has_remaining() {
            debug!(req = ?&req_buf);
            self.stream.write_buf(&mut req_buf).await?;
        }

        // 大抵バッファは全部帰ってきてるけど・・・PeercastYTはどうかな？
        let mut read_buf = BytesMut::with_capacity(4096);
        let (response, len) = loop {
            let r = self.stream.read_buf(&mut read_buf).await; // appendされる
            trace!(read_buf = ?&read_buf);

            // Bytesの処理をすること
            let resp =
                parse_pcp_http_response(&read_buf).map_err(|e| HandshakeError::HttpResponse)?;
            match resp {
                Some(r) => break r,
                None => continue, // 途中までしかレスポンスが帰ってきていないので継続して読み取る
            }
        };
        let _header_buf: BytesMut = read_buf.split_to(len); // ヘッダー分のバッファを解放

        match response.status().as_u16() {
            200 => {
                // 200: チャンネルはあってリレー可能？
                // PCPのハンドシェイク接続, PCP_OKが来て、その後PCP_CHAN_PKTでチャンネルの情報とストリームがだらだら来る
                // あとは適当な間隔でPCP_HOSTをPCP_BCSTにつけて流してあげればよい
                self.hello_pcp().await
            }
            503 => {
                // 503: チャンネルはあるけどリレーしてない？
                // 次に接続すべきノードがPCP_HOSTで最大8個流れてきてPCP_QUITで終了
                self.next_host().await;
                todo!()
            }
            404 => Err(HandshakeError::ChannelNotFound),
            _ => {
                // something occured
                todo!()
            }
        }
    }

    async fn hello_pcp(&mut self) -> Result<(), HandshakeError> {
        self.send_helo_get_oleh().await;

        // Ok Packet
        let atom = self
            .read_atoms()
            .await
            .map_err(|e| HandshakeError::Failed)?;
        if !atom.is_child() || (atom.id() != Id4::PCP_OK) {
            panic!("dont arrive  Ok packet")
        }

        log::debug!("Connection Handshaked ID:{}", self.connection_id);

        Ok(())
    }

    async fn send_helo_get_oleh(&mut self) {
        let mut payload = BytesMut::new();

        // HELOを送信
        HelloBuilder::new(self.session_id, self.broadcast_id)
            .build()
            .write_bytes(&mut payload);
        self.stream.write_all_buf(&mut payload).await.unwrap();

        // PCP_OLEHが帰って来るはず
        let atom = read_atom(&mut self.stream, &mut self.read_buf)
            .await
            .unwrap();
        trace!(atom = ?&atom);
        let id4 = atom.id();
        if id4 != Id4::PCP_OLEH {
            panic!("packet is not hello")
        }
        let info = OlehInfo::parse(&atom);
        log::debug!("OlehInfo: {:?}", &info);
        self.oleh_info = Some(info);
    }

    async fn next_host(&mut self) {
        self.send_helo_get_oleh().await;

        tokio::time::sleep(Duration::from_secs(1)).await;
        let mut atoms = vec![];
        loop {
            let Ok(atom) = self.read_atoms().await else {
                break;
            };
            atoms.push(atom);
        }
        debug!("atoms: {atoms:#?}");
    }

    async fn read_atoms(&mut self) -> Result<Atom, HandshakeError> {
        // todo: bufferの入れ替え方法考えないとナー
        let atom = read_atom(&mut self.stream, &mut self.read_buf).await?;
        log::trace!("atom arrived. {}", &atom);
        Ok(atom)
    }

    pub fn raw_parts(self) -> (T, BytesMut, Option<OlehInfo>) {
        let Handshake {
            stream,
            read_buf,
            oleh_info,
            ..
        } = self;
        (stream, read_buf, oleh_info)
    }
}

#[derive(Debug)]
enum HandshakeResult {
    Success {
        stream: TcpStream,
        read_buf: BytesMut,
        oleh: OlehInfo,
    },
    FailedButHaveNext {
        next_host: VecDeque<SocketAddr>,
    },
    FailedCompletely {},
}
