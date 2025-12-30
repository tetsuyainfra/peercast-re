use std::net::SocketAddr;

use libpeercast_re::{ConnectionId, pcp::GnuId};
use tokio::sync::{
    mpsc::{self, UnboundedReceiver},
    oneshot,
};

use crate::config::Config;
use crate::repository::{Channel, ReChannelRepository};

pub enum ConnectionMessage<C: Channel> {
    NewConnection(ConnectionId, tokio::net::TcpStream, SocketAddr, GnuId),
    GetChannels(oneshot::Sender<Vec<C>>),
    GetOrCreateChannel {
        channel_id: GnuId,
        channel_info: Option<libpeercast_re::pcp::ChannelInfo>,
        track_info: Option<libpeercast_re::pcp::TrackInfo>,
        config: Option<C::Config>,
        responder: oneshot::Sender<C>,
    },
    GetStream {
        channel_id: GnuId,
        responder: GnuId,
    },
}

#[derive(Debug, Clone)]
pub struct PeCaServerAPI<C: Channel> {
    sender: mpsc::UnboundedSender<ConnectionMessage<C>>,
}

impl<C: Channel> PeCaServerAPI<C> {
    pub async fn get_channels(&self) -> anyhow::Result<Vec<C>> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(ConnectionMessage::GetChannels(tx))
            .map_err(|e| anyhow::anyhow!("Failed to send GetChannels message: {}", e))?;
        let r = rx.await.map_err(|e| anyhow::anyhow!("Failed to receive channels: {}", e))?;
        Ok(r)
    }

    pub async fn get_or_create_channel(
        &self,
        channel_id: GnuId,
        channel_info: Option<libpeercast_re::pcp::ChannelInfo>,
        track_info: Option<libpeercast_re::pcp::TrackInfo>,
        config: Option<C::Config>,
    ) -> anyhow::Result<C> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(ConnectionMessage::GetOrCreateChannel {
                channel_id,
                channel_info,
                track_info,
                config,
                responder: tx,
            })
            .map_err(|e| anyhow::anyhow!("Failed to send GetChannels message: {}", e))?;

        let r = rx.await.map_err(|e| anyhow::anyhow!("Failed to receive channels: {}", e))?;
        Ok(r)
    }
}

pub struct PeCaServer<C: Channel> {
    config: Config,
    self_session_id: GnuId,
    receiver: UnboundedReceiver<ConnectionMessage<C>>,
    repo: ReChannelRepository<C>,
}

impl<C> PeCaServer<C>
where
    C: Channel + Clone + Send + Sync + std::fmt::Debug + 'static,
{
    pub fn new(config: Config) -> (Self, PeCaServerAPI<C>) {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let self_session_id = GnuId::new();
        let svr = Self {
            config,
            self_session_id,
            receiver,
            repo: ReChannelRepository::new(&self_session_id),
        };
        let api = PeCaServerAPI {
            sender,
        };
        (svr, api)
    }

    pub async fn start(self, shutdown_token: tokio_util::sync::CancellationToken) -> anyhow::Result<()> {
        let Self {
            self_session_id,
            mut receiver,
            mut repo,
            config,
        } = self;

        if config.create_dummy {
            let dummy_channel_id = GnuId::from(0x123456789ABCDEF_u128);
            let dummy_channel_info = libpeercast_re::pcp::ChannelInfo {
                name: "Dummy Channel".to_string(),
                url: "http://example.com".to_string(),
                genre: "Various".to_string(),
                desc: "This is a dummy channel.".to_string(),
                comment: "No comments.".to_string(),
                stream_type: "video/x-flv".to_string(),
                stream_ext: ".flv".to_string(),
                bitrate: 128,
                typ: "FLV".to_string(),
            };
            let dummy_track_info = libpeercast_re::pcp::TrackInfo {
                title: "Dummy Track".to_string(),
                creator: "Dummy Artist".to_string(),
                url: "http://example.com/track".to_string(),
                album: "Dummy Album".to_string(),
                genre: "Various".to_string(),
                ..Default::default()
            };

            let _ = repo.create_or_get(dummy_channel_id, Some(dummy_channel_info), Some(dummy_track_info), None).await;
        }

        'accept: loop {
            tokio::select! {
                    _ = shutdown_token.cancelled() => {
                        tracing::debug!("received shutdown signal");
                        break 'accept;
                    }
                    msg = receiver.recv() => {
                        match msg {
                            Some(msg) => {
                                Self::handle_message(&config, &self_session_id, &mut repo, msg).await;
                            },
                            None => {
                                tracing::debug!("ConnectionMessage channel closed");
                                break 'accept;
                            },
                        }
                }
            }
        }

        Ok(())
    }

    async fn handle_message(
        _config: &Config,
        _self_session_id: &GnuId,
        repo: &mut ReChannelRepository<C>,
        msg: ConnectionMessage<C>,
    ) {
        match msg {
            ConnectionMessage::NewConnection(_, _, _, _) => {}
            ConnectionMessage::GetChannels(sender) => {
                let channels = repo.get_channels();
                let _ = sender.send(channels);
            }
            ConnectionMessage::GetOrCreateChannel {
                channel_id,
                channel_info,
                track_info,
                config,
                responder,
            } => {
                let ch = repo.create_or_get(channel_id, channel_info, track_info, config).await;
                let _ = responder.send(ch);
            }
            ConnectionMessage::GetStream {
                ..
            } => {}
        }
    }
}

pub async fn serve_pcphttp(
    cid: ConnectionId,
    conn: tokio::net::TcpStream,
    remote: SocketAddr,
    _graceful_shutdown: tokio_util::sync::CancellationToken,
) -> anyhow::Result<()> {
    let _read_buf = bytes::BytesMut::new();

    // HandshakeFutureにすればよさそう
    // let _handshake = PcpConnectionFactory().accept(cid, conn, remote);

    // let mut conn = match handshake.incoming(root_atom.into()).await {
    //     Err(e) => {
    //         todo!();
    //         #[allow(unused)]
    //         return Ok(());
    //     }
    //     Ok(HandshakeType::Ping) => return Ok(()),
    //     Ok(HandshakeType::YellowPage(conn)) => conn,
    // };

    // // RootならTrackerに次の情報を送って、情報のアップデートを求める(Broadcastを遅らせる)
    // let root_atom = RootBuilder::build_update_request();
    // conn.write_atom(root_atom).await;

    // // 最初のAtomはBroadcastが確定する
    // let first_atom = match conn.read_atom().await {
    //     Ok(a) => a,
    //     Err(_) => return,
    // };
    // dbg!(&first_atom);

    // let bcst = match PcpBroadcast::parse(&first_atom) {
    //     Ok(b) => b,
    //     Err(_) => return,
    // };
    // dbg!(&bcst);

    // // パケットの中身が適正か確認する
    // let PcpBroadcast {
    //     channel_id,
    //     channel_packet,
    //     host,
    //     ..
    // } = &bcst;
    // let (channel_id_in_bcst, channel_packet) = match (channel_id, channel_packet) {
    //     (Some(chid), Some(chpkt)) => (chid, chpkt),
    //     _ => return,
    // };
    // // TODO: HostのIPチェックを行う？

    // // Hostの接続先を確定
    // let tracker_host = host
    //     .as_ref()
    //     .and_then(|pcp_host| get_tracker_addr(&remote, &pcp_host.addresses));

    // let PcpChannel {
    //     channel_id,
    //     broadcast_id,
    //     channel_info,
    //     track_info,
    //     ..
    // } = channel_packet;

    // let (channel_id_in_chpkt, braodcast_id) = match (channel_id, broadcast_id) {
    //     (Some(chid), Some(bcid)) => (chid, bcid),
    //     _ => return,
    // };

    // // 不正チェック
    // if channel_id_in_bcst != channel_id_in_chpkt {
    //     return;
    // }

    // // チャンネル情報の変換
    // let channel_info = channel_info.as_ref().map(|i| i.into());
    // let track_info = track_info.as_ref().map(|t| t.into());
    // //
    // let config = RootConfig { tracker_host };

    // // 対象チャンネルを取得
    // let repo = REPOSITORY();
    // let ch = repo.create_or_get(*channel_id_in_bcst, channel_info, track_info, Some(config));

    // // Channelにコネクションを接続
    // let attach_task = ch.attach_connection(conn, graceful_shutdown, closed_send);
    // attach_task.await;

    // conn.shutdown().await?;
    Ok(())
}

////////////////////////////////////////////////////////////////////////////////
// application initialize
//
// pub fn app_init(args: &Args, config: &Config) {
//     // let self_session_id = libpeercast_re::pcp::GnuId::new();
//     // let self_socket = (config.server_address, config.server_port).into();

//     // _CONN_FACTORY.get_or_init(|| PcpConnectionFactory::new(self_session_id, self_socket));
//     // _REPOSITORY.get_or_init(|| ReChannelRepository::new(&self_session_id));

//     if args.create_dummy {
//         let dummy_channel_id = GnuId::from(0x123456789ABCDEF_u128);
//         let dummy_channel_info = libpeercast_re::pcp::ChannelInfo {
//             name: "Dummy Channel".to_string(),
//             url: "http://example.com".to_string(),
//             genre: "Various".to_string(),
//             desc: "This is a dummy channel.".to_string(),
//             comment: "No comments.".to_string(),
//             stream_type: "video/x-flv".to_string(),
//             stream_ext: ".flv".to_string(),
//             bitrate: 128,
//             typ: "FLV".to_string(),
//         };
//         let dummy_track_info = libpeercast_re::pcp::TrackInfo {
//             title: "Dummy Track".to_string(),
//             creator: "Dummy Artist".to_string(),
//             url: "http://example.com/track".to_string(),
//             album: "Dummy Album".to_string(),
//             genre: "Various".to_string(),
//             ..Default::default()
//         };

//         Repository().create_or_get(dummy_channel_id, Some(dummy_channel_info), Some(dummy_track_info), None);
//     }
// }
