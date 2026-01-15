use anyhow::Context;
use axum::response::Redirect;
use clap::Parser;
use futures_util::future::FutureExt;
use http::Uri;
use libpeercast_re::ConnectionNo;
use libpeercast_re::pcp::GnuId;
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;
use tokio::time::timeout;
use tower_http::trace::DefaultOnFailure;

use peercast_re::{AppState, prelude::*};
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

    // peercast::app_init(&args, &config);

    match args.command {
        Some(cli::Commands::Listen {
            url,
        }) => {
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

    // Start Server Listeners
    let svr_addr = SocketAddr::from((config.server_address, config.server_port));
    let svr_listener = tokio::net::TcpListener::bind(svr_addr)
        .await
        .with_context(|| format!("Failed to bind Server Address: {}", svr_addr))?;
    let rtmp_addr = SocketAddr::from((config.rtmp_address, config.rtmp_port));
    let rtmp_listener = tokio::net::TcpListener::bind(rtmp_addr)
        .await
        .with_context(|| format!("Failed to bind Rtmp Address: {}", svr_addr))?;
    let api_addr = SocketAddr::from((config.api_address, config.api_port));
    let api_listener = tokio::net::TcpListener::bind(api_addr)
        .await
        .with_context(|| format!("Failed to bind API Address: {}", api_addr))?;

    info!("PeerCast listening on pcp://{}/", svr_listener.local_addr().unwrap());
    info!("RTMP(FLV)listening on rtmp://{}/", rtmp_listener.local_addr().unwrap());
    info!("      UI listening on http://{}/ui", api_listener.local_addr().unwrap());
    info!("     API listening on http://{}/api", api_listener.local_addr().unwrap());

    // Start Server Tasks
    let shutdown_token = tokio_util::sync::CancellationToken::new();
    // start Rtmp StreamManager
    let manager_sender = libpeercast_re::rtmp::stream_manager::start();

    // Create Store
    let repository = peercast::init(config.clone(), &manager_sender).await;
    let store = peercast_re::State {
        config: config.clone(),
        config_path,
        repository: repository,
        rtmp_manager_sender: manager_sender,
    };
    let store: AppState = std::sync::Arc::new(store);

    // let tb = tokio::task::Builder::new();
    // let tt = tokio_util::task::TaskTracker::new();
    // tt.spawn(peercast_server(shutdown_token.child_token(), store.clone(), svr_listener));
    // tt.spawn(api_server(shutdown_token.child_token(), store.clone(), api_listener));
    // tt.spawn(async move {
    //     tokio::signal::ctrl_c().await.expect("Failed to listen for ctrl-c signal");
    //     shutdown_token.cancel();
    //     info!("Shutdown signal sent");
    //     Ok::<_, anyhow::Error>(ServerThread::ShutdownNotifier)
    // });
    // tt.close();
    // tt.wait().await;

    let mut set = tokio::task::JoinSet::new();
    set.build_task().name("PCP Listen").spawn(peercast_server(
        shutdown_token.child_token(),
        store.clone(),
        svr_listener,
    ))?;
    set.build_task().name("API Listen").spawn(api_server(shutdown_token.child_token(), store.clone(), api_listener))?;
    set.build_task().name("RTMP Listen").spawn(rtmp_server(
        shutdown_token.child_token(),
        store.clone(),
        rtmp_listener,
    ))?;

    // temporary task for test
    let shutdown_token_child = shutdown_token.child_token();
    set.build_task().name("Init time Temporary Task").spawn(async move {
        let ch = store.repository.get(&GnuId::from_str("00000000000000000123456789ABCDEF").unwrap());
        if let Some(ch) = ch {
            println!("Adding dummy source stream to dummy channel");
            let _ = ch.add_source_stream(Uri::from_static("rtmp://example.com/live/stream")).await;
        }
        // tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        let _ = timeout(Duration::from_secs(5), shutdown_token_child.cancelled()).await;
        Ok(ServerThread::Temporary)
    })?;
    // shutdown notifier
    set.build_task().name("Shutdown Notifier").spawn(async move {
        tokio::signal::ctrl_c().await.expect("Failed to listen for ctrl-c signal");
        shutdown_token.cancel();
        info!("Shutdown signal sent");
        Ok(ServerThread::ShutdownNotifier)
    })?;

    while let Some(res) = set.join_next().await {
        let r = res.context("A server thread has panicked")?;
        info!("A server thread has shut down : {:?}", r);
    }

    info!("PeerCast Re Application has shut down");
    Ok(())
}

#[derive(Debug)]
enum ServerThread {
    PeerCastServer,
    RtmpServer,
    ApiServer,
    ShutdownNotifier,
    Temporary,
}

////////////////////////////////////////////////////////////////////////////////
// PeerCast Server
//
async fn peercast_server(
    wait_signal: tokio_util::sync::CancellationToken,
    _store: AppState,
    svr_listener: tokio::net::TcpListener,
) -> anyhow::Result<ServerThread> {
    let listener = svr_listener;

    let tracker = tokio_util::task::TaskTracker::new();
    let token = tokio_util::sync::CancellationToken::new();

    'accept: loop {
        let child_token = token.child_token();

        tokio::select! {
            _ = wait_signal.cancelled() => {
                tracing::debug!("received shutdown signal on PeerCast server");
                break 'accept;
            }
            r = listener.accept() => {
                let cid: ConnectionNo = ConnectionNo::new();

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
    Ok::<_, anyhow::Error>(ServerThread::PeerCastServer)
}

async fn spawned_peercast_connection_handler(
    cid: ConnectionNo,
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
    store: AppState,
    api_listener: tokio::net::TcpListener,
) -> anyhow::Result<ServerThread> {
    use tower_http::{
        trace::TraceLayer,
        trace::{DefaultMakeSpan, DefaultOnResponse},
    };

    // LOGの制御はtower_http::traceで行う
    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(DefaultMakeSpan::new().level(Level::INFO).include_headers(true))
        .on_failure(DefaultOnFailure::new().level(Level::ERROR))
        .on_response(DefaultOnResponse::new().level(Level::INFO).latency_unit(tower_http::LatencyUnit::Millis));

    let router = axum::Router::new()
        .route("/", axum::routing::get(|| async { Redirect::to("/ui") }))
        .nest("/ui", handler::ui::build_router())
        .nest("/api", handler::api::build_router())
        .merge(handler::peercast::router())
        .fallback(async || http::StatusCode::NOT_FOUND) // ← これが重要
        .layer(trace_layer)
        .with_state(store.clone());

    let router = if cfg!(debug_assertions) {
        info!("Swagger listening on http://{}{}", api_listener.local_addr().unwrap(), peercast_re::SWAGGER_PATH);
        router.merge(handler::build_swagger())
    } else {
        router
    };

    // debug!("API Router: {:#?}", router);
    axum::serve(api_listener, router.into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(async move {
            wait_signal.cancelled_owned().await;
            tracing::debug!("received shutdown signal on API Server");
        })
        .await
        .context("Serving Application Error")?;

    info!("API Server has shut down");
    Ok(ServerThread::ApiServer)
}

////////////////////////////////////////////////////////////////////////////////
// RTMP Server
//
async fn rtmp_server(
    wait_signal: tokio_util::sync::CancellationToken,
    store: AppState,
    rtmp_listener: tokio::net::TcpListener,
) -> anyhow::Result<ServerThread> {
    use libpeercast_re::rtmp;
    let manager_sender = store.rtmp_manager_sender.clone();
    let listener = rtmp_listener;

    let tracker = tokio_util::task::TaskTracker::new();
    let token = tokio_util::sync::CancellationToken::new();

    'accept: loop {
        let child_token = token.child_token();

        tokio::select! {
            _ = wait_signal.cancelled() => {
                tracing::debug!("received shutdown signal on RTMP server");
                break 'accept;
            }
            r = listener.accept() => {
                let cno: ConnectionNo = ConnectionNo::new();

                match r {
                    Err(e) => {
                        tracing::error!("Failed to accept connection: {}", e);
                        break 'accept;
                    }
                    Ok((conn, addr)) => {
                        let connection = rtmp::connection::Connection::new(cno.0 as i32, manager_sender.clone());
                        tracker.spawn(connection.start_handshake(conn));
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

    info!("RTMP Server has shut down");
    Ok::<_, anyhow::Error>(ServerThread::RtmpServer)
}

////////////////////////////////////////////////////////////////////////////////
// Initialization Logging
//
fn logging_init() {
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::{EnvFilter, fmt};
    // use tracing_subscriber::layer::SubscriberExt;

    // subscriberの構築
    let subscriber = tracing_subscriber::registry();

    // tokio-conosole を有効化
    #[cfg(debug_assertions)]
    let subscriber = {
        println!("Console Subscriber listening on http://127.0.0.1:6699");
        let console_layer = console_subscriber::ConsoleLayer::builder()
            // オプション
            .with_default_env()
            .spawn();
        subscriber.with(console_layer)
    };

    // コンソール出力のフォーマットレイヤー
    let subscriber = {
        // 環境変数ベースのフィルターレイヤー
        let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            // axum logs rejections from built-in extractors with the `axum::rejection`
            // target, at `TRACE` level. `axum::rejection=trace` enables showing those events
            // let logfilter = "debug";
            let logfilter = format!("{}=debug,tower_http=debug,axum::rejection=trace", env!("CARGO_CRATE_NAME")).into();
            println!("Default Log Filter: RUST_LOG={}", logfilter);
            logfilter
        });

        // デフォルトのフォーマットレイヤー
        let fmt_layer = fmt::layer()
            // オプション
            .with_ansi(true)
            .with_file(true)
            .with_line_number(true)
            .with_target(false)
            .with_filter(env_filter);

        subscriber.with(fmt_layer)
    };

    subscriber.init();

    /* どうやってもログが出ないときに試す
    // tracing_subscriber::fmt::init();
     */
}
