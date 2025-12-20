use anyhow::Context;
use axum::response::Redirect;
use bytes::BytesMut;
use clap::Parser;
use futures_util::FutureExt;
use libpeercast_re::ConnectionId;
use std::{
    net::{Shutdown, SocketAddr},
    sync::OnceLock,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::info;
use utoipa::openapi::info;

use peercast_re::{
    channel::ReChannel, cli, config, handler, peercast, repository::ReChannelRepository,
};

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

    peercast::app_init(&config);

    enum Command {
        Listen { url: String },
    }
    match args.command {
        Some(cli::Commands::Listen { url }) => {
            info!("Starting PeerCast Re Listen for URL: {}", url);
            let ipc_addr = config.ipc_path;
            let mut ipc_stream = tokio::net::UnixSocket::new_stream()?
                .connect(ipc_addr)
                .await?;
            // Send Listen Command via IPC
            ipc_stream
                .write_all(format!("LISTEN {}\n", url).as_bytes())
                .await
                .context("Failed to send LISTEN command via IPC")?;
            info!("Sent LISTEN(url: {}) command via IPC", url);
            ipc_stream
                .shutdown()
                .await
                .context("Failed to shutdown IPC stream")?;
            return Ok(());
        }
        Some(_) | None => {
            info!("Starting PeerCast Re Server (default command)...");
        }
    }

    // Create Store
    let store = peercast::Store {
        config: config.clone(),
        config_path,
    };
    let store = std::sync::Arc::new(store);

    // Start Server Listeners
    let svr_addr = SocketAddr::from((config.server_address, config.server_port));
    let svr_listener = tokio::net::TcpListener::bind(svr_addr)
        .await
        .with_context(|| format!("Failed to bind Server Address: {}", svr_addr))?;
    let api_addr = SocketAddr::from((config.api_address, config.api_port));
    let api_listener = tokio::net::TcpListener::bind(api_addr)
        .await
        .with_context(|| format!("Failed to bind API Address: {}", api_addr))?;
    let ipc_addr = config.ipc_path;
    let ipc_listener = tokio::net::UnixListener::bind(ipc_addr.clone())
        .with_context(|| format!("Failed to bind IPC Address: {}", ipc_addr))?;

    info!(
        "PeerCast listening on pcp://{}/",
        svr_listener.local_addr().unwrap()
    );
    info!(
        "  UI/API listening on http://{}/ui",
        api_listener.local_addr().unwrap()
    );
    info!(
        "     IPC listening on unix:{}",
        ipc_listener
            .local_addr()?
            .as_pathname()
            .context("cant get pathname")?
            .display()
    );

    // Start Server Tasks
    let shutdown_token = tokio_util::sync::CancellationToken::new();
    let mut set = tokio::task::JoinSet::new();
    set.spawn(
        peercast::task_runner(shutdown_token.child_token()).then(|r| async {
            match r {
                Ok(_) => Ok(ServerThread::PeerCastTask),
                Err(e) => Err(e),
            }
        }),
    );
    set.spawn(peercast_server(
        shutdown_token.child_token(),
        store.clone(),
        svr_listener,
    ));
    set.spawn(api_server(
        shutdown_token.child_token(),
        store.clone(),
        api_listener,
    ));
    set.spawn(ipc_server(
        shutdown_token.child_token(),
        store,
        ipc_listener,
    ));

    // shutdown notifier
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
    PeerCastTask,
    Api,
    Ipc,
    ShutdownNotifier,
    Command,
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
        let cid: ConnectionId = ConnectionId::new();
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
                    // serve_root(cid, conn, remote, child_shutdown).await
                    unimplemented!("PeerCast is not allowed");
                }
                Ok(ConnectionProtocol::PeerCastHttp) => {
                    tracing::error!(?cid, ?remote, "STERAM si PeerCastHttp Protocol");
                    // let _ = conn.shutdown().await;
                    // unimplemented!("PeerCastHttp is not allowed");
                    peercast::serve_pcphttp(cid, conn, remote, child_shutdown).await
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

////////////////////////////////////////////////////////////////////////////////
// API Server
//
async fn ipc_server(
    shutdown_token: tokio_util::sync::CancellationToken,
    store: std::sync::Arc<peercast::Store>,
    ipc_listener: tokio::net::UnixListener,
) -> anyhow::Result<ServerThread> {
    'accept: loop {
        let (mut stream, remote_addr) = tokio::select! {
            _ = shutdown_token.cancelled() => {
                tracing::debug!("received shutdown signal");
                break 'accept;
            }
            r = ipc_listener.accept() => {
                match r {
                    Err(e) => {
                        tracing::error!("Failed to accept IPC connection: {}", e);
                        break 'accept;
                    }
                    Ok((conn, addr)) => {
                        tracing::info!("Accepted IPC connection from {:?}", addr);
                        // Handle the IPC connection here
                        (conn, addr)
                    }
                }
            }
        };
        let mut buf  = BytesMut::with_capacity(4096);
        let n = stream.read_buf(&mut buf).await?;
        let cmd_str = String::from_utf8_lossy(&buf[..n]);
        log::info!("Received IPC command: {}", cmd_str);
    }

    // Shutdown処理
    match ipc_listener.local_addr() {
        Err(e) => {
            tracing::error!("Failed to get IPC socket local address : {}", e);
        }
        Ok(addr) => match addr.as_pathname() {
            None => {
                tracing::error!("IPC socket address is not a valid pathname");
            }
            Some(path) => match std::fs::remove_file(path) {
                Err(e) => {
                    tracing::error!("Failed to remove IPC socket file {:?}: {}", path, e);
                }
                Ok(_) => {
                    tracing::info!("Removed IPC socket file {:?}", path);
                }
            },
        },
    };

    info!("IPC Server has shut down");
    Ok(ServerThread::Ipc)
}

////////////////////////////////////////////////////////////////////////////////
// Initialization Logging
//
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
