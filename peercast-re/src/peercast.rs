use std::{net::SocketAddr, sync::OnceLock};

use crate::{channel::ReChannel, cli::Args, config::Config, repository::ReChannelRepository};
use libpeercast_re::{ConnectionId, pcp::PcpConnectionFactory};

////////////////////////////////////////////////////////////////////////////////
// application initialize
//
pub fn app_init(args: &Args, config: &Config) {
    let self_session_id = libpeercast_re::pcp::GnuId::new();
    let self_socket = (config.server_address, config.server_port).into();

    _CONN_FACTORY.get_or_init(|| PcpConnectionFactory::new(self_session_id, self_socket));
    _REPOSITORY.get_or_init(|| ReChannelRepository::new(&self_session_id));
}

////////////////////////////////////////////////////////////////////////////////
// Global Variables
//
static _CONN_FACTORY: OnceLock<PcpConnectionFactory> = OnceLock::new();
#[inline]
#[allow(non_snake_case)]
pub fn PcpConnectionFactory() -> &'static PcpConnectionFactory {
    _CONN_FACTORY.get().unwrap()
}

static _REPOSITORY: OnceLock<ReChannelRepository<ReChannel>> = OnceLock::new();
#[inline]
#[allow(non_snake_case)]
pub fn Repository() -> &'static ReChannelRepository<ReChannel> {
    _REPOSITORY.get().unwrap()
}

pub async fn task_runner(shutdown_token: tokio_util::sync::CancellationToken) -> anyhow::Result<()> {
    'accept: loop {
        tokio::select! {
            _ = shutdown_token.cancelled() => {
                tracing::debug!("received shutdown signal");
                break 'accept;
            }
        }
    }

    Ok(())
}

pub async fn serve_pcphttp(
    cid: ConnectionId,
    conn: tokio::net::TcpStream,
    remote: SocketAddr,
    _graceful_shutdown: tokio_util::sync::CancellationToken,
) -> anyhow::Result<()> {
    let _read_buf = bytes::BytesMut::new();

    // HandshakeFutureにすればよさそう
    let _handshake = PcpConnectionFactory().accept(cid, conn, remote);

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
