/// peercast-port-checkerd
/// PeerCastのポートが開いているか確認してくれるAPIサーバー
/// IPv4/IPv6の両方のポートを開いて待つ
///
use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    num::ParseIntError,
    sync::Arc,
    time::Duration,
};

use axum::{
    extract::{ConnectInfo, Query, State},
    response::{Html, Redirect},
    routing::get,
    Json, Router,
};
use axum_core::{extract::Request, response::IntoResponse};
use bytes::BytesMut;
use clap::Parser;
use hyper::StatusCode;
use libpeercast_re::{
    pcp::{procedure::PcpHandshake, GnuId},
    ConnectionId,
};
use serde_json::json;
use thiserror::Error;
use tokio::net::TcpStream;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::{error, info, Level};
use tracing_subscriber::{prelude::*, EnvFilter};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    // #[arg(short, long, default_value = "0.0.0.0")]
    #[arg(short, long, default_value = "0.0.0.0")]
    bind: std::net::IpAddr,

    /// Name of the person to greet
    #[arg(short, long, default_value_t = 7145)]
    port: u16,

    #[arg(long, default_value = "/ppc")]
    path: String,

    #[arg(long, default_value_t = 3000)]
    connect_timeout: u64,
}

#[tokio::main]
async fn main() {
    let registry = tracing_subscriber::registry()
        // .with(tracing_subscriber::fmt::layer().with_target(false))
        .with(tracing_subscriber::fmt::layer().with_target(true))
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            println!("RUST_LOG=info");
            "info".into()
        }));

    match tracing_journald::layer() {
        Ok(layer) => {
            registry.with(layer).init();
        }
        Err(e) => {
            registry.init();
            error!("couldn't connect to journald: {}", e);
        }
    }

    let exename = std::env::current_exe()
        .unwrap()
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    info!("START {}", exename);

    let args = Args::parse();

    let state = Arc::new(AppConf {
        path: args.path.clone(),
        connect_timeout: args.connect_timeout,
    });

    let ppc_app = Router::new()
        .route("/", get(handler))
        .route("/ip", get(ppc_ip_handler))
        .route("/portcheck", get(ppc_portcheck_handler));

    let ppc_without_slash = args.path.clone();
    let ppc_path = format!("{}/", &args.path);
    let ppc_path_cloned = ppc_path.clone();

    let app = Router::new()
        .route("/", get(handler))
        .route(
            &ppc_without_slash,
            get(|| async move { Redirect::permanent(&ppc_path_cloned) }),
        )
        .nest(&ppc_path, ppc_app)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .with_state(state);

    let listener = tokio::net::TcpListener::bind((args.bind, args.port))
        .await
        .unwrap();
    info!(
        "listening on http://{}{}/",
        listener.local_addr().unwrap(),
        args.path
    );
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

#[derive(Debug, Clone)]
struct AppConf {
    path: String,
    connect_timeout: u64,
}

type AppState = Arc<AppConf>;

// async fn handler() -> Html<&'static str> {
async fn handler(State(conf): State<AppState>) -> Html<String> {
    let path = conf.path.clone();
    Html(format!(
        "<h1>PeerCast-Port-Checker</h1>
        <div>
        <ul>
          <li><a href='{path}/ip'>{path}/ip</a></li>
          <li><a href='{path}/portcheck?port=7144'>{path}/portcheck?port=7144</a></li>
          <li><a href='{path}/portcheck?port=17144'>{path}/portcheck?port=17144</a></li>
          <li>{path}/portcheck?port=
              <form action='{path}/portcheck' method='get'>
                <input type=number name='port' value='7144' />
                <button>GO</button>
              </form>
          </li>
        </ul>
        </div>
    ",
    ))
}

async fn ppc_ip_handler(ConnectInfo(info): ConnectInfo<SocketAddr>) -> impl IntoResponse {
    let ip = info.ip();
    Json(json!({ "ip": ip }))
}

async fn ppc_portcheck_handler(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    _req: Request,
) -> impl IntoResponse {
    let connection_id = ConnectionId::new();
    match port_check(connection_id, addr, params, state.connect_timeout).await {
        Err(e) => match e {
            PortCheckError::VariableError(_) | PortCheckError::ParamNotFound => (
                StatusCode::NOT_ACCEPTABLE,
                Json(json!({
                    "error": e.to_string(),
                    "result": false
                })),
            ),
            PortCheckError::FailedConnectRemote(_) | PortCheckError::IoError(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": e.to_string(),
                    "result": false
                })),
            ),
        },

        Ok((remote, port, result)) => {
            //
            (
                StatusCode::ACCEPTED,
                Json(json!({
                    "ip": remote,
                    "port": port,
                    "result": result}
                )),
            )
        }
    }
}

#[derive(Debug, Error)]
enum PortCheckError {
    #[error("Query Param is invalid")]
    VariableError(#[from] ParseIntError),

    #[error("Query Param is invalid")]
    ParamNotFound,

    #[error("Could not connect in time")]
    FailedConnectRemote(#[from] tokio::time::error::Elapsed),

    #[error("Something io error occured")]
    IoError(#[from] std::io::Error),
}

async fn port_check(
    connection_id: ConnectionId,
    addr: SocketAddr,
    params: HashMap<String, String>,
    connect_timeout: u64,
) -> Result<(IpAddr, u16, bool), PortCheckError> {
    let ip = addr.ip();
    let port = params.get("port").ok_or(PortCheckError::ParamNotFound)?;
    let port = port.parse::<u16>()?;

    let remote: SocketAddr = (ip, port).into();
    let stream = tokio::time::timeout(
        Duration::from_millis(connect_timeout),
        TcpStream::connect(remote),
    )
    .await??;

    let handshake = PcpHandshake::new(
        connection_id,
        stream,
        None,
        remote,
        BytesMut::with_capacity(4096),
        GnuId::new(),
    );

    let result = match handshake.outgoing_ping().await {
        Ok(_session_id) => true,
        Err(_) => false,
    };

    Ok((ip, port, result))
}
