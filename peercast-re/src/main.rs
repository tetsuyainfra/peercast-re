use anyhow::Context;
use axum::response::Redirect;
use clap::Parser;
use libpeercast_re::ConnectionId;
use std::net::SocketAddr;
use tracing::info;

use peercast_re::{peercast, cli, config, handler};

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
    // Handle the PeerCast connection here
    tokio::select! {
        _ = shutdown_token.cancelled() => {
            // Shutdown signal received
            tracing::info!("Connection({}) from {} is being closed due to shutdown", cid, remote);
        }
        _ = async {
            // Handle the connection
            tracing::info!("Accepted connection({}) from {}", cid, remote);
            // Here you would handle the connection, e.g., read/write data
            match libpeercast_re::util::identify_protocol(&conn).await {
                Ok(ConnectionProtocol::PeerCast) => {
                    // serve_root(cid, stream, remote, graceful_shutdown, closed_send).await
                }
                Ok(ConnectionProtocol::PeerCastHttp) => {
                    tracing::error!("PeerCastHttp is not allowed");
                    // let _ = conn.shutdown().await;
                }
                Ok(ConnectionProtocol::Http) => {
                    tracing::warn!(?cid, ?remote, "STREAM is HTTP Protocol");
                    // serve_http(cid, stream, remote, graceful_shutdown, force_shutdown).await
                    let _ = conn.shutdown().await;
                }
                Ok(ConnectionProtocol::Unknown) => {
                    tracing::warn!(?cid, ?remote, "STREAM is Unkwon Protocol");
                    let _ = conn.shutdown().await;
                }
                Err(e) => {
                    tracing::error!(?cid, ?remote, "Failed: identify_protocol: {}", e);
                    let _ = conn.shutdown().await;
                }
            }
        } => {
            tracing::info!("Connection({}) from {} closed", cid, remote);
        }
    }

    tracing::debug!("Connection({}) handler for {} has exited", cid, remote);
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
