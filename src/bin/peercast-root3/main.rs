use std::{net::SocketAddr, sync::OnceLock};

use anyhow::bail;
use axum::Router;
use channel::RootChannel;
use clap::Parser;
use connection::PcpConnectionFactory;
use peercast_re::{
    pcp::GnuId,
    util::{identify_protocol, ConnectionProtocol},
    ConnectionId,
};
use store::ChannelRepository;
use tokio::{io::AsyncWriteExt, net::TcpStream};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, trace, warn};

// use crate::channel::{tracker_channel::TrackerChannel, ChannelStore};

mod channel;
mod cli;
mod connection;
mod logging;
mod shutdown;
mod store;

#[cfg(test)]
mod test_helper;

// Don't use directly. SEE: REPOSITORY()
static _REPOSITORY: OnceLock<ChannelRepository<RootChannel>> = OnceLock::new();
// Don't use directly. SEE: CONN_FACTORY()
static _CONN_FACTORY: OnceLock<PcpConnectionFactory> = OnceLock::new();
// Don't use directly. SEE: HTTP_API()
// static _HTTP_API_INTO_MAKE_WITH: OnceLock<IntoMakeServiceWithConnectInfo<Router, MyConnectInfo>> =
//     OnceLock::new();
// static _HTTP_API_INTO_MAKE: OnceLock<IntoMakeService<Router>> = OnceLock::new();
static _HTTP_API: OnceLock<Router> = OnceLock::new();

#[derive(Debug, Clone)]
struct ApiState {}

#[inline]
#[allow(non_snake_case)]
pub fn REPOSITORY() -> &'static ChannelRepository<RootChannel> {
    _REPOSITORY.get().unwrap()
}

#[inline]
#[allow(non_snake_case)]
pub fn CONN_FACTORY() -> &'static PcpConnectionFactory {
    _CONN_FACTORY.get().unwrap()
}

#[inline]
#[allow(non_snake_case)]
pub fn HTTP_API() -> &'static Router {
    _HTTP_API.get().unwrap()
}

fn INIT_APP(self_session_id: GnuId, self_ipaddr: std::net::IpAddr, self_port: u16) {
    _REPOSITORY.get_or_init(|| ChannelRepository::new());
    //
    _CONN_FACTORY
        .get_or_init(|| PcpConnectionFactory::new(self_session_id, self_ipaddr, self_port));
    //
    _HTTP_API.get_or_init(|| {
        axum::Router::new()
            .route("/", axum::routing::get(root))
            .with_state(ApiState {})
    });
}

async fn root() -> &'static str {
    "Hello, World!"
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = cli::Args::parse();

    cli::version_print(&args)?;
    logging::init(&args)?;

    INIT_APP(GnuId::new(), args.bind, args.port);

    let (shutdown_task, graceful, force) = shutdown::create_task_anyhow();

    // let shutdown_task = tokio::spawn(shutdown_task);
    let server_task = tokio::spawn(server(args, graceful, force));
    // let spawner = tokio::task::Builder::new().name(&"main");
    // let server_task = spawner.spawn(server(args, graceful, force)).unwrap();

    // futures_util::future::join_all(vec![shutdown_task, server_task]).await;
    futures_util::future::join_all(vec![server_task]).await;

    Ok(())
}

async fn server(
    args: cli::Args,
    graceful_shutdown: CancellationToken,
    force_shutdown: CancellationToken,
) -> anyhow::Result<()> {
    let listener = tokio::net::TcpListener::bind((args.bind, args.port)).await?;
    info!("PCP listening on pcp://{}", listener.local_addr().unwrap(),);

    let tracker = tokio_util::task::TaskTracker::new();
    info!("START PCP SERVER");

    let app: axum::Router =
        axum::Router::new().route("/", axum::routing::get(|| async { "/ path" }));

    let factory = PcpConnectionFactory::new(GnuId::new(), args.bind, args.port);

    loop {
        let connection_id = ConnectionId::new();
        let name = format!("tcp({})", connection_id.0);
        let spawner = tokio::task::Builder::new().name(&name);
        let child_graceful_shutdown = graceful_shutdown.child_token();
        let child_force_shutdown = force_shutdown.child_token();

        tokio::select! {
            accept = listener.accept() => {
                match accept {
                    Ok((stream, addr)) => {
                        let _handle = spawner.spawn(tracker.track_future(serve( connection_id, stream, addr, child_graceful_shutdown, child_force_shutdown)));
                    }
                    Err(e) => {
                        error!(?e, "something is occured in listener.accept()");
                        break;
                    }
                }
            },
            _ = graceful_shutdown.cancelled() => {
                info!("GRACEFUL SHUTDOWN REQUESTED");
                break;
            }
        }
    }

    tokio::select! {
        _ = force_shutdown.cancelled() => {
            // trackerが存在することでくれるありがたい
            tracker.close();
            tracker.wait().await;

            // MEMO: こうやってshutdownのタイムアウトを設定してもいいよな・・・
            // timeout(tracker.wait()).await
        }
    }

    Ok(())
}

async fn serve(
    connection_id: ConnectionId,
    mut stream: TcpStream,
    addr: SocketAddr,
    graceful_shutdown: CancellationToken,
    force_shutdown: CancellationToken,
) {
    info!(?connection_id, ?addr, "SPAWN SERVE");

    match identify_protocol(&stream).await {
        Ok(ConnectionProtocol::PeerCast) => {
            serve_pcp(
                connection_id,
                stream,
                addr,
                graceful_shutdown,
                force_shutdown,
            )
            .await
        }
        Ok(ConnectionProtocol::PeerCastHttp) => {
            todo!("PeerCastHttp Protocol");
        }
        Ok(ConnectionProtocol::Http) => {
            serve_http(
                connection_id,
                stream,
                addr,
                graceful_shutdown,
                force_shutdown,
                // app.into_make_service_with_connect_info::<MyConnectInfo>(),
            )
            .await
        }
        Ok(ConnectionProtocol::Unknown) => {
            warn!(?connection_id, ?addr, "STREAM is Unkwon Protocol");
            let _ = stream.shutdown().await;
        }
        Err(e) => {
            error!(?connection_id, ?addr, "Error in identify_protocol: {}", e);
            let _ = stream.shutdown().await;
        }
    }
}

async fn serve_pcp(
    connection_id: ConnectionId,
    mut stream: TcpStream,
    addr: SocketAddr,
    graceful_shutdown: CancellationToken,
    force_shutdown: CancellationToken,
) {
    let handshake = CONN_FACTORY().accept(connection_id, stream, addr);

    let connection = match handshake.incoming_pcp(true).await {
        Ok(conn) => conn,
        Err(e) => {
            debug!("Handshake is error: {:#}", e);
            return;
        }
    };

    connection.run().await;
}

async fn serve_http(
    connection_id: ConnectionId,
    tcp_stream: TcpStream,
    remote_addr: SocketAddr,
    graceful_shutdown: CancellationToken,
    force_shutdown: CancellationToken,
) {
    let socket = hyper_util::rt::TokioIo::new(tcp_stream);

    let hyper_service = hyper_util::service::TowerToHyperService::new(HTTP_API().clone());

    let builder =
        hyper_util::server::conn::auto::Builder::new(hyper_util::rt::TokioExecutor::new());
    // builder.http2().enable_connect_protocol(); // ENABLE HTTP2
    let conn = builder.serve_connection_with_upgrades(socket, hyper_service);

    futures_util::pin_mut!(conn);
    loop {
        tokio::select! {
            // HTTP1.1以降の接続の使い回しができるようになっている？
            result = conn.as_mut() => {
                if let Err(_err) = result {
                    trace!("failed to serve connection: {_err:#}");
                }
                break;
            }
        }
    }
}
