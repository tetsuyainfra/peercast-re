use anyhow::Context;
use axum::response::Redirect;
use clap::Parser;
use futures_util::FutureExt;
use libpeercast_re::ConnectionId;
use std::net::SocketAddr;
use tracing::{error, info};

use peercast_re::{cli, config, handler, peercast};

////////////////////////////////////////////////////////////////////////////////
// main
//
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    logging_init();

    let args = cli::Args::parse();
    dbg!(&args);

    let (config, config_path) = config::load_config(args.clone()).context("Failed to Load configuration")?;

    peercast::app_init(&config);

    match args.command {
        Some(cli::Commands::Listen { url }) => {
            // TODO: DELETE ME after implement internal API client
            info!("Starting LISTEN Request {}", url);
            tokio::spawn(async move {
                // HACKME: http で requestする
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                let mut command = tokio::process::Command::new("curl");
                command.arg("-v").arg(url.as_str());
                let output = command.output().await;
                match output {
                    Ok(r) => {
                        info!("LISTEN Request Stdout: \n{}", String::from_utf8_lossy(&r.stdout));
                        info!("LISTEN Request Stderr: \n{}", String::from_utf8_lossy(&r.stderr));
                    }
                    Err(e) => {
                        error!("LISTEN Request Failed: {}", e);
                    }
                }
            });
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

    info!("PeerCast listening on pcp://{}/", svr_listener.local_addr().unwrap());
    info!("      UI listening on http://{}/ui", api_listener.local_addr().unwrap());
    info!("     API listening on http://{}/api", api_listener.local_addr().unwrap());

    // Start Server Tasks
    let shutdown_token = tokio_util::sync::CancellationToken::new();
    let mut set = tokio::task::JoinSet::new();
    set.spawn(peercast::task_runner(shutdown_token.child_token()).then(|r| async {
        match r {
            Ok(_) => Ok(ServerThread::PeerCastTask),
            Err(e) => Err(e),
        }
    }));
    set.spawn(peercast_server(shutdown_token.child_token(), store.clone(), svr_listener));
    set.spawn(api_server(shutdown_token.child_token(), store.clone(), api_listener));

    // shutdown notifier
    set.spawn(async move {
        tokio::signal::ctrl_c().await.expect("Failed to listen for ctrl-c signal");
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

///////////////////////////////////////////////////////////////////////////////
// API Server
//
async fn api_server(
    wait_signal: tokio_util::sync::CancellationToken,
    store: std::sync::Arc<peercast::Store>,
    api_listener: tokio::net::TcpListener,
) -> anyhow::Result<ServerThread> {
    let router = axum::Router::new()
        .route("/", axum::routing::get(|| async { Redirect::to("/ui") }))
        .nest("/ui", handler::ui::build_router(&store))
        .nest("/api", handler::api::build_router(&store));

    let router = if cfg!(debug_assertions) {
        info!(
            "Swagger listening on http://{}{}",
            api_listener.local_addr().unwrap(),
            peercast_re::SWAGGER_PATH
        );
        router.merge(handler::api::build_swagger())
    } else {
        router
    };

    // debug!("API Router: {:#?}", router);
    axum::serve(api_listener, router.into_make_service_with_connect_info::<SocketAddr>())
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
// Initialization Logging
//
fn logging_init() {
    use tracing_subscriber::{EnvFilter, fmt};

    // `log` クレートを `tracing` に統合
    tracing_log::LogTracer::init().expect("failed to initialize tracing");

    // `tracing` のSubscriberを初期化
    let subscriber = fmt()
        .with_file(true)
        .with_line_number(true)
        .with_target(true)
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            println!("RUST_LOG=debug");
            "debug".into()
        }))
        .finish();

    // グローバルのデフォルトに設定
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set subscriber");
}
