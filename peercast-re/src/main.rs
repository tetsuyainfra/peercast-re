use anyhow::Context;
use axum::response::Redirect;
use clap::Parser;
use libpeercast_re::{ConnectionId, pcp::PcpConnectionFactory};
use std::{net::SocketAddr, sync::OnceLock};
use tracing::info;

use peercast_re::{channel::ReChannel, cli, config, handler, peercast, repository::ReChannelRepository};

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

////////////////////////////////////////////////////////////////////////////////
// main
//
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    logging_init();

    let args = cli::Args::parse();
    dbg!(&args);

    let (config, config_path) =
        config::load_config(args.clone()).context("Failed to Load configuration")?;

    let store = peercast::Store {
        config: config.clone(),
        config_path,
    };
    let store = std::sync::Arc::new(store);

    let self_session_id = libpeercast_re::pcp::GnuId::new();
    let self_socket =  (config.server_address, config.server_port).into();
    _CONN_FACTORY.get_or_init(|| PcpConnectionFactory::new(self_session_id, self_socket));


    let svr_addr = SocketAddr::from((config.server_address, config.server_port));
    let svr_listener = tokio::net::TcpListener::bind(svr_addr)
        .await
        .with_context(|| format!("Failed to bind Server Address: {}", svr_addr))?;

    let api_addr = SocketAddr::from((config.api_address, config.api_port));
    let api_listener = tokio::net::TcpListener::bind(api_addr)
        .await
        .with_context(|| format!("Failed to bind API Address: {}", api_addr))?;

    info!(
        "PeerCast listening on pcp://{}/",
        svr_listener.local_addr().unwrap()
    );
    info!(
        "  UI/API listening on http://{}/ui",
        api_listener.local_addr().unwrap()
    );

    let shutdown_token = tokio_util::sync::CancellationToken::new();
    let mut set = tokio::task::JoinSet::new();
    set.spawn(peercast_server(
        shutdown_token.child_token(),
        store.clone(),
        svr_listener,
    ));
    set.spawn(api_server(
        shutdown_token.child_token(),
        store,
        api_listener,
    ));
    set.spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for ctrl-c signal");
        shutdown_token.cancel();
        info!("Shutdown signal sent");
        Ok(ServerThread::ShutdownNotifier)
    });

    while let Some(res) = set.join_next().await {
        let r = res.context("A server thread has panicked")?;
        info!("A server thread has shut down : {:?}", r);
    }

    info!("PeerCast Re Application has shut down");
    Ok(())
}

#[derive(Debug)]
enum ServerThread {
    PeerCast,
    Api,
    ShutdownNotifier,
}

////////////////////////////////////////////////////////////////////////////////
// PeerCast Server
//
async fn peercast_server(
    wait_signal: tokio_util::sync::CancellationToken,
    _store: std::sync::Arc<peercast::Store>,
    svr_listener: tokio::net::TcpListener,
) -> anyhow::Result<ServerThread> {
    let listener = svr_listener;

    let tracker = tokio_util::task::TaskTracker::new();
    let token = tokio_util::sync::CancellationToken::new();

    'accept: loop {
        let cid = ConnectionId::new();
        let child_token = token.child_token();

        tokio::select! {
            _ = wait_signal.cancelled() => {
                tracing::debug!("received shutdown signal");
                break 'accept;
            }
            r = listener.accept() => {
                match r {
                    Err(e) => {
                        tracing::error!("Failed to accept connection: {}", e);
                        break 'accept;
                    }
                    Ok((conn, addr)) => {
                        tracker.spawn(spawned_peercast_connection_handler(cid, conn, addr, child_token));
                    }
                }
            }
        }
    }

    // Shutdown処理
    {
        tracker.close();
        token.cancel();
    }

    tracker.wait().await;

    info!("PeerCast Server has shut down");
    Ok::<_, anyhow::Error>(ServerThread::PeerCast)
}

async fn spawned_peercast_connection_handler(
    cid: ConnectionId,
    mut conn: tokio::net::TcpStream,
    remote: SocketAddr,
    shutdown_token: tokio_util::sync::CancellationToken,
) -> anyhow::Result<()> {
    use libpeercast_re::util::ConnectionProtocol;
    use tokio::io::AsyncWriteExt;

    let child_shutdown = shutdown_token.child_token();
    // Handle the PeerCast connection here
    tokio::select! {
        _ = shutdown_token.cancelled() => {
            tracing::info!("Connection({}) from {} is being closed due to shutdown", cid, remote);
        }
        r = async {
            // Handle the connection
            tracing::info!("Accepted connection({}) from {}", cid, remote);

            match libpeercast_re::util::identify_protocol(&conn).await {
                Ok(ConnectionProtocol::PeerCast) => {
                    tracing::info!(?cid, ?remote, "STREAM is PeerCast Protocol");
                    // serve_root(cid, stream, remote, graceful_shutdown, closed_send).await
                    serve_root(cid, conn, remote, child_shutdown).await
                }
                Ok(ConnectionProtocol::PeerCastHttp) => {
                    tracing::error!(?cid, ?remote, "STERAM si PeerCastHttp Protocol");
                    // let _ = conn.shutdown().await;
                    unimplemented!("PeerCastHttp is not allowed");
                }
                Ok(ConnectionProtocol::Http) => {
                    tracing::error!(?cid, ?remote, "STREAM is HTTP Protocol");
                    // serve_http(cid, stream, remote, graceful_shutdown, force_shutdown).await
                    conn.shutdown().await.context("Failed to shutdown HTTP connection")
                }
                Ok(ConnectionProtocol::Unknown) => {
                    tracing::warn!(?cid, ?remote, "STREAM is Unkwon Protocol");
                    conn.shutdown().await.context("Failed to shutdown Unknown connection")
                }
                Err(e) => {
                    tracing::error!(?cid, ?remote, "Failed: identify_protocol: {}", e);
                    conn.shutdown().await.context("Failed to shutdown on identify_protocol error")
                }
            }
            // MEMO: matchの返り値をimpl Future<Output = anyhow::Result<(), Error>>で一旦受けて.awaitしたほうが早い？
        } => {
            tracing::info!("Connection({}) from {} closed, reason({:?})", cid, remote, r);
        }
    }

    tracing::debug!("Connection({}) handler for {} has exited", cid, remote);
    Ok(())
}



async fn serve_root(
    cid: ConnectionId,
    mut conn: tokio::net::TcpStream,
    remote: SocketAddr,
    _graceful_shutdown: tokio_util::sync::CancellationToken,
) -> anyhow::Result<()> {
    use libpeercast_re::pcp::{
        builder::RootBuilder,
        connection::HandshakeType,
    };
    let read_buf = bytes::BytesMut::new();

    // HandshakeFutureにすればよさそう
    let handshake = PcpConnectionFactory().accept(cid, conn, remote);

    // // Handshake時に送ってもらうAtomを作成する
    // let root_atom = RootBuilder::default()
    //     .set_update_interval(10)
    //     .set_next_update_interval(10)
    //     .build();

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
// API Server
//
async fn api_server(
    wait_signal: tokio_util::sync::CancellationToken,
    store: std::sync::Arc<peercast::Store>,
    api_listener: tokio::net::TcpListener,
) -> anyhow::Result<ServerThread> {
    let router = axum::Router::new()
        .route("/", axum::routing::get(|| async { Redirect::to("/ui") }))
        .nest("/api", handler::api::build_router(store))
        .nest("/ui", handler::ui::build_router());
    axum::serve(
        api_listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async move {
        wait_signal.cancelled_owned().await;
        tracing::debug!("received shutdown signal");
    })
    .await
    .context("Serving Application Error")?;

    info!("API Server has shut down");
    Ok(ServerThread::Api)
}

/// initialize logging
fn logging_init() {
    use tracing_subscriber::{EnvFilter, fmt, prelude::*};

    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_file(true)
                .with_line_number(true)
                .with_target(false),
        )
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            println!("RUST_LOG=debug");
            "debug".into()
        }))
        .init();
}
