use bytes::{Buf, BytesMut};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tracing::{debug, trace};
use url::form_urlencoded::parse;

use crate::{
    error::HandshakeError,
    pcp::{
        atom::read_atom,
        builder::{HelloBuilder, HostInfo, OlehInfo, QuitInfo},
        procedure::http_req::parse_pcp_http_response,
        Atom, GnuId, Id4,
    },
    ConnectionId,
};

use super::http_req::create_channel_request;

pub enum HandshakeReturn<T> {
    Success {
        stream: T,
        read_buf: BytesMut,
        oleh: OlehInfo,
    },
    NextHost {
        oleh: OlehInfo,
        hosts: Vec<HostInfo>,
        quit: Option<QuitInfo>, // これも返さなくていいような・・・
    },
    ChannelNotFound,
}

pub struct BothHandshake<T> {
    connection_id: ConnectionId,
    stream: T,
    session_id: GnuId,
    broadcast_id: GnuId,
}

impl<T> BothHandshake<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    pub fn new(
        connection_id: ConnectionId,
        stream: T,
        session_id: GnuId,
        broadcast_id: GnuId,
    ) -> Self {
        Self {
            connection_id,
            stream,
            session_id,
            broadcast_id,
        }
    }
    pub async fn outgoing(mut self) -> Result<HandshakeReturn<T>, HandshakeError> {
        let mut req_buf = create_channel_request(self.broadcast_id);

        // ヘッダーの送信
        while req_buf.has_remaining() {
            debug!(req = ?&req_buf);
            self.stream.write_buf(&mut req_buf).await?;
        }

        // Parse HTTP response
        let mut read_buf = BytesMut::with_capacity(4096);
        let (response, http_header_bytes_len) = loop {
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
        let _header_buf: BytesMut = read_buf.split_to(http_header_bytes_len); // ヘッダー分のバッファを解放
        trace!(connection_id=?self.connection_id, response=?response, read_buf_len=?read_buf.len() );

        match response.status().as_u16() {
            200 => {
                // 200: チャンネルはあってリレー可能？
                // PCPのハンドシェイク接続, PCP_OKが来て、その後PCP_CHAN_PKTでチャンネルの情報とストリームがだらだら来る
                // あとは適当な間隔でPCP_HOSTをPCP_BCSTにつけて流してあげればよい
                let oleh = self.hello_pcp(&mut read_buf).await?;
                Ok(HandshakeReturn::Success {
                    stream: self.stream,
                    read_buf,
                    oleh,
                })
            }
            503 => {
                // 503: チャンネルはあるけどリレーできない
                // 次に接続すべきノードがPCP_HOSTで最大8個流れてきてPCP_QUITで終了
                let (oleh, hosts, quit) = self.next_host(&mut read_buf).await?;
                Ok(HandshakeReturn::NextHost { oleh, hosts, quit })
            }
            404 => {
                // 配信終了後はこれになるっぽいんだよね
                Err(HandshakeError::ChannelNotFound)
            }
            _ => {
                // something occured
                todo!()
            }
        }
    }

    async fn hello_pcp(&mut self, read_buf: &mut BytesMut) -> Result<OlehInfo, HandshakeError> {
        let oleh_info = self.send_helo_get_oleh(read_buf).await?;

        // Ok Packet
        let atom = read_atom(&mut self.stream, read_buf).await?;
        if !atom.is_child() || (atom.id() != Id4::PCP_OK) {
            panic!("dont arrive  Ok packet")
        }

        debug!("Connection Handshaked ID:{}", self.connection_id);

        Ok(oleh_info)
    }

    async fn next_host(
        &mut self,
        read_buf: &mut BytesMut,
    ) -> Result<(OlehInfo, Vec<HostInfo>, Option<QuitInfo>), HandshakeError> {
        let oleh = self.send_helo_get_oleh(read_buf).await?;

        let mut hosts = vec![];
        let mut quit = None;
        loop {
            let Ok(atom) = read_atom(&mut self.stream, read_buf).await else {
                break;
            };
            if atom.id() == Id4::PCP_HOST {
                let host = HostInfo::parse(&atom);
                hosts.push(host);
            } else if atom.id() == Id4::PCP_QUIT {
                // const int error = PCP_ERROR_QUIT + PCP_ERROR_UNAVAILABLE; これが帰ってきているはず
                let quit = Some(QuitInfo::parse(&atom));
                break;
            }
        }
        trace!("HostInfo: {hosts:#?}");

        Ok((oleh, hosts, quit))
    }

    async fn send_helo_get_oleh(
        &mut self,
        read_buf: &mut BytesMut,
    ) -> Result<OlehInfo, HandshakeError> {
        let mut payload = BytesMut::new();

        // HELOを送信
        HelloBuilder::new(self.session_id, self.broadcast_id)
            .build()
            .write_bytes(&mut payload);
        self.stream.write_all_buf(&mut payload).await.unwrap();

        // PCP_OLEHが帰って来る
        let atom = read_atom(&mut self.stream, read_buf).await?;
        trace!(atom = ?&atom);
        let id4 = atom.id();
        if id4 != Id4::PCP_OLEH {
            return Err(HandshakeError::Failed);
        }
        let oleh_info = OlehInfo::parse(&atom);
        debug!("OlehInfo: {:?}", &oleh_info);

        Ok(oleh_info)
    }
}

#[cfg(test)]
mod t {
    use super::*;

    #[crate::test]
    async fn test_outgoing() {
        // let result =
        //     bothhandshake::outgoing(connection_id, stream, read_buf, session_id, broadcast_id);
    }

    #[crate::test]
    async fn test_incoming() {
        // let result =
        //     bothhandshake::incoming(connection_id, stream, read_buf, session_id, broadcast_id);
    }
}
