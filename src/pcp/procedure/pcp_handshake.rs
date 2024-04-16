use std::{io::Bytes, net::SocketAddr, sync::Arc};

use bytes::{Buf, BufMut, BytesMut};
use minijinja::__context::build;
use thiserror::Error;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use tracing::{debug, error, info, instrument, trace};

use crate::{
    error::{AtomParseError, HandshakeError},
    pcp::{
        atom::{self, read_atom},
        builder::{
            HelloBuilder, HostInfo, OlehBuilder, OlehInfo, PingBuilder, PingInfo, PongBuilder,
            PongInfo, QuitBuilder, QuitInfo, QuitReason,
        },
        Atom, ChannelManager, GnuId, Id4,
    },
    ConnectionId,
};

use super::http_req::{create_channel_request, parse_pcp_http_response};

#[derive(Debug)]
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
pub struct PcpHandshake {
    connection_id: ConnectionId,
    stream: TcpStream,
    self_addr: Option<SocketAddr>,
    remote: SocketAddr,
    read_buf: BytesMut,
    self_session_id: GnuId,
}

impl PcpHandshake {
    pub fn new(
        connection_id: ConnectionId,
        stream: TcpStream,
        self_addr: Option<SocketAddr>,
        remote: SocketAddr,
        read_buf: BytesMut,
        self_session_id: GnuId,
    ) -> Self {
        Self {
            connection_id,
            stream,
            self_addr,
            remote,
            read_buf,
            self_session_id,
        }
    }

    #[instrument(fields(connection_id = self.connection_id.0))]
    pub async fn outgoing(
        mut self,
        broadcast_id: GnuId,
    ) -> Result<HandshakeReturn<TcpStream>, HandshakeError> {
        let mut req_buf = create_channel_request(broadcast_id);

        // ヘッダーの送信
        while req_buf.has_remaining() {
            debug!(CID=?&self.connection_id, req = ?&req_buf);
            self.stream.write_buf(&mut req_buf).await?;
        }

        // Parse HTTP response
        let (response, http_header_bytes_len) = loop {
            let r = self.stream.read_buf(&mut self.read_buf).await; // appendされる
            trace!(CID=?&self.connection_id, read_buf = ?&self. read_buf);

            // Bytesの処理をすること
            let resp = parse_pcp_http_response(&self.read_buf)
                .map_err(|e| HandshakeError::HttpResponse)?;
            match resp {
                Some(r) => break r,
                None => continue, // 途中までしかレスポンスが帰ってきていないので継続して読み取る
            }
        };
        let _header_buf: BytesMut = self.read_buf.split_to(http_header_bytes_len); // ヘッダー分のバッファを解放
        trace!(CID=?&self.connection_id, response=?response, read_buf_len=?&self.read_buf.len());

        // https://github.com/kumaryu/peercaststation/blob/6184647e600ec3a388462169ab7118314114252e/PeerCastStation/PeerCastStation.PCP/PCPSourceStream.cs#L284
        // FIXME: IPv6対応 GetPCPVersionの値が100ならIp V6

        match response.status().as_u16() {
            200 => {
                // 200: チャンネルはあってリレー可能？
                // PCPのハンドシェイク接続, PCP_OKが来て、その後PCP_CHAN_PKTでチャンネルの情報とストリームがだらだら来る
                // あとは適当な間隔でPCP_HOSTをPCP_BCSTにつけて流してあげればよい
                let oleh = self.send_hello(broadcast_id).await?;
                let _ = self.recv_ok().await?;

                let Self {
                    connection_id,
                    stream,
                    self_addr,
                    remote,
                    read_buf,
                    self_session_id,
                } = self;
                Ok(HandshakeReturn::Success {
                    stream,
                    read_buf,
                    oleh,
                })
            }
            503 => {
                // 503: チャンネルはあるけどリレーできない
                // 次に接続すべきノードがPCP_HOSTで最大8個流れてきてPCP_QUITで終了
                // Self::send_helo().await;
                let oleh = self.send_hello(broadcast_id).await?;
                let (hosts, quit) = self.recv_hosts_and_quit().await?;

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

    //
    #[instrument(fields(connection_id = self.connection_id.0))]
    pub async fn outgoing_ping(mut self) -> Result<GnuId, HandshakeError> {
        let mut buf = BytesMut::new();
        let ping_atoms = PingBuilder::new(self.self_session_id).build();
        for atom in ping_atoms {
            atom.write_bytes(&mut buf);
        }
        // Send Magic/Ping Atom
        let _ = self.stream.write_all_buf(&mut buf).await?;

        // Receive oleh Atom
        let oleh_atom = self.read_atom().await?;
        let pong_info = PongInfo::parse(&oleh_atom)?;

        // Send Quit
        let quit_atom = QuitBuilder::new(QuitReason::ConnectionError).build();
        self.send_atom(quit_atom).await?;

        self.stream.flush().await?;
        self.stream.shutdown().await?;

        Ok(pong_info.session_id)
    }

    #[instrument(fields(connection_id = self.connection_id.0))]
    pub async fn incoming(
        &mut self,
        channel_manager: Arc<ChannelManager>,
    ) -> Result<(), HandshakeError> {
        trace!("Incoming PCP");
        // TCP Streamの冒頭 pcp\nもAtomのヘッダーとして扱う
        let connect_atom = self.read_atom().await?;
        trace!(
            "incomming connection CID:{}, atom: {:#?}",
            self.connection_id,
            &connect_atom
        );
        if connect_atom.id() != Id4::PCP_CONNECT {
            return Err(AtomParseError::IdError.into());
        }

        let ping_atom = self.read_atom().await?;
        trace!("ping_atom: {:#?}", &ping_atom);
        let r = PingInfo::parse(&ping_atom)?;
        trace!("PingInfo: {:#?}", &r);

        // send pong
        // OlehBuilder::new(self.session_id, remote_ip, remote_port)
        // self.stream.write_all_buf(src)
        let pong_atom = PongBuilder::new(self.self_session_id).build();
        let _ = self.send_atom(pong_atom).await?;

        let quit_atom = self.read_atom().await?;
        let quit_info = QuitInfo::parse(&quit_atom);
        info!(?quit_info);
        info!("OK Incoming PCP Port check");

        Ok(())
    }

    async fn read_atom(&mut self) -> Result<Atom, HandshakeError> {
        // todo: bufferの入れ替え方法考えないとナー
        let atom = atom::read_atom(&mut self.stream, &mut self.read_buf).await?;
        trace!("atom arrived. {}", &atom);
        Ok(atom)
    }
    async fn send_atom(&mut self, atom: Atom) -> Result<(), HandshakeError> {
        let mut buf = BytesMut::new();
        atom.write_bytes(&mut buf);

        self.stream.write_all_buf(&mut buf).await?;
        Ok(())
    }

    /// Send Hello then Recv OLEH
    async fn send_hello(&mut self, broadcast_id: GnuId) -> Result<OlehInfo, HandshakeError> {
        let mut payload = BytesMut::new();

        // HELOを送信
        let mut builder = HelloBuilder::new(self.self_session_id, broadcast_id);
        if (self.self_addr.is_some()) {
            let port = self.self_addr.as_ref().unwrap().port();
            builder = builder.port(port).ping(port);
        }
        builder.build().write_bytes(&mut payload);
        self.stream.write_all_buf(&mut payload).await?;

        // PCP_OLEHが帰って来る
        let atom = self.read_atom().await?;
        trace!(atom = ?&atom);
        let id4 = atom.id();
        if id4 != Id4::PCP_OLEH {
            return Err(HandshakeError::Failed);
        }
        let oleh_info = OlehInfo::parse(&atom);

        Ok(oleh_info)
    }

    /// Recieve Ok Atom
    async fn recv_ok(&mut self) -> Result<(), HandshakeError> {
        let ok_atom = self.read_atom().await?;
        if ok_atom.id() != Id4::PCP_OK {
            return Err(HandshakeError::Failed);
        }

        Ok(())
    }

    /// Recv Hosts and Quit
    async fn recv_hosts_and_quit(
        &mut self,
    ) -> Result<(Vec<HostInfo>, Option<QuitInfo>), HandshakeError> {
        let mut hosts = vec![];
        let mut quit = None;
        loop {
            let Ok(atom) = self.read_atom().await else {
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

        Ok((hosts, quit))
    }

    // async fn read_next_host(
    //     stream: &mut TcpStream,
    //     read_buf: &mut BytesMut,
    //     self_session_id: GnuId,
    //     broadcast_id: GnuId,
    // ) -> Result<(OlehInfo, Vec<HostInfo>, Option<QuitInfo>), PcpHandshakeError> {
    //     let oleh = Self::send_helo_get_oleh(read_buf).await?;

    //     let mut hosts = vec![];
    //     let mut quit = None;
    //     loop {
    //         let Ok(atom) = read_atom(&mut self.stream, read_buf).await else {
    //             break;
    //         };
    //         if atom.id() == Id4::PCP_HOST {
    //             let host = HostInfo::parse(&atom);
    //             hosts.push(host);
    //         } else if atom.id() == Id4::PCP_QUIT {
    //             // const int error = PCP_ERROR_QUIT + PCP_ERROR_UNAVAILABLE; これが帰ってきているはず
    //             let quit = Some(QuitInfo::parse(&atom));
    //             break;
    //         }
    //     }
    //     trace!("HostInfo: {hosts:#?}");

    //     Ok((oleh, hosts, quit))
    // }
}

impl std::fmt::Debug for PcpHandshake {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PcpHandshake")
            .field("connection_id", &self.connection_id)
            // .field("stream", &self.stream)
            .field("remote", &self.remote)
            // .field("read_buf", self.read_buf.len())
            .field("self_session_id", &self.self_session_id)
            .finish_non_exhaustive()
        // .finish()
    }
}
