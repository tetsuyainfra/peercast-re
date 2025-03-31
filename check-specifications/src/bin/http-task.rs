use std::{net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};

use anyhow::anyhow;
use axum::{extract::ws::Message, routing, Router};
use futures_util::{SinkExt, StreamExt};
use tokio::{net::TcpListener, signal, sync::watch};
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use tower_http::{
    add_extension::AddExtensionLayer,
    services::ServeDir,
    trace::{DefaultMakeSpan, TraceLayer},
};
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};

struct ShutdownState {
    graceful: CancellationToken,
    force: CancellationToken,
    tracker: TaskTracker,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logger();
    println!("process number: {}", std::process::id());
    let assets_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
    println!("asset_dir: {:?}", &assets_dir);

    let listener = tokio::net::TcpListener::bind(("localhost", 7143)).await?;
    info!(
        "HTTP listening on http://{}",
        listener.local_addr().unwrap(),
    );

    let axum_handle = axum_server::Handle::new();
    let tracker = tokio_util::task::TaskTracker::new();

    let graceful_token = CancellationToken::new();
    let force_token = CancellationToken::new();

    let child_graceful_token = graceful_token.child_token(); // for http server
    let child_force_token = CancellationToken::new();
    let _shutdown_handle = tracker.spawn(shutdown(graceful_token));

    let tracker = TaskTracker::new();

    let shutdown_state = ShutdownState {
        graceful: child_graceful_token.clone(),
        force: child_force_token.clone(),
        tracker: tracker.clone(),
    };

    let app = Router::new()
        .fallback_service(ServeDir::new(assets_dir).append_index_html_on_directories(true))
        .route("/ws", routing::any(ws_handler))
        // logging so we can see what's going on
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        )
        .with_state(Arc::new(shutdown_state));

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async move {
        child_graceful_token.cancelled().await;

        info!("graceful start!");
    })
    .await
    .unwrap();

    tracker.close();
    tokio::select! {
        _ = tracker.wait() => {
            println!("all spawner is closed")
        }
        _ = child_force_token.cancelled() => {
            // http_server_handle.abort();
        }
    }

    Ok(())
}

fn init_logger() {
    let fmt_filter = tracing_subscriber::filter::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_e| "info".into());

    let fmt_layer = tracing_subscriber::fmt::layer();
    tracing_subscriber::registry()
        .with(fmt_layer.with_filter(fmt_filter))
        // .with(access_log.with_filter(access_log_filter_fn))
        // .with(access_log)
        .init();
}

async fn shutdown(graceful_token: CancellationToken) {
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = signal::ctrl_c() => {
            println!("catch ctrl_c");
            graceful_token.cancel();
        }
        _ = terminate =>  {
            println!("catch terminate");
        }
    }
}

async fn ws_handler(
    ws: axum::extract::WebSocketUpgrade,
    axum::extract::ConnectInfo(addr): axum::extract::ConnectInfo<SocketAddr>,
    axum::extract::State(state): axum::extract::State<Arc<ShutdownState>>,
) -> impl axum::response::IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, addr, state))
}

async fn handle_socket(
    mut socket: axum::extract::ws::WebSocket,
    who: SocketAddr,
    shutdown: Arc<ShutdownState>,
) {
    let (mut sender, mut receiver) = socket.split();

    let graceful_token = shutdown.graceful.clone();
    let send_task = shutdown.tracker.spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        let mut i = 0;
        'sendloop: loop {
            tokio::select! {
                r = sender.send(Message::Text(format!("Server message {i}...").into())) => {
                    if r.is_err() { error!("sender error"); break }
                },
                _ = graceful_token.cancelled() => {
                    info!("sender token_child cancelled()");
                    break 'sendloop
                }
            }
            info!("send messege {i}");
            i += 1;
            interval.tick().await;
        }
        i
    });

    let graceful_token = shutdown.graceful.clone();
    let recv_task = shutdown.tracker.spawn(async move {
        let mut cnt = 0;
        loop {
            tokio::select! {
                msg = receiver.next() => {
                    match msg {
                        Some(msg) => {
                            info!("recieve messege: {msg:?}");
                        },
                        None => {
                            info!("recive None");
                            break
                        },
                    }
                }
                _ = graceful_token.cancelled() => {
                    info!("reciever token_child cancelled()");
                    break
                }
            }
            cnt += 1;
        }
        cnt
    });

    futures_util::future::join_all(vec![send_task, recv_task]).await;

    // tokio::select! {
    //     send_cnt = (&mut send_task) => {
    //         match send_cnt {
    //             Ok(a) => { info!("Sended {a}")},
    //             Err(a) => { error!("Sended Error {a:?}") },
    //         }
    //         token.cancel();
    //     }
    //     recv_cnt = (&mut recv_task) => {
    //         match recv_cnt {
    //             Ok(a) => { info!("Recv {a}")},
    //             Err(a) => { error!("Recv Error {a:?}") },
    //         }
    //         token.cancel();
    //     }
    // }
    info!("finished handle_socket")
}
