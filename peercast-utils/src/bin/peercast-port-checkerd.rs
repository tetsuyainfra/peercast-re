/// peercast-port-checkerd
/// PeerCastのポートが開いているか確認してくれるAPIサーバー
/// IPv4/IPv6の両方のポートを開いて待つ
///
use std::{net::SocketAddr, process::exit};

use anyhow::Context;
use utoipa::{OpenApi, openapi};
use utoipa_axum::router::OpenApiRouter;

use clap::Parser;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::{Level, error, info};
use tracing_subscriber::{EnvFilter, prelude::*};
use utoipa_swagger_ui::SwaggerUi;

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

    #[arg(long, default_value = "/api/v1/ppc")]
    path: String,

    #[arg(long, default_value_t = 3000, value_name="CONNECT_TIMEOUT_MILLI_SECS")]
    connect_timeout: u64,

    #[arg(long, default_value = "http://localhost:7145")]
    servers: String,

    #[cfg(debug_assertions)]
    #[arg(long, default_value_t=true, action = clap::ArgAction::Set)]
    enable_swagger: bool,

    #[cfg(not(debug_assertions))]
    #[arg(long, default_value_t=false, action = clap::ArgAction::Set)]
    enable_swagger: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, clap::Subcommand, Clone)]
pub enum Commands {
    ShowApiYaml {},
}

const PPC_TAG: &str = "peercast-port-checkerd";

#[derive(OpenApi)]
#[openapi(
    tags(
        (name = PPC_TAG, description = "PeerCast Port Checker API")
    )
)]
struct ApiDoc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let servers: Vec<_> = args
        .servers
        .split(",")
        .map(|s| openapi::server::ServerBuilder::new().url(s).build())
        .collect();

    let (router, mut api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest(&args.path, ppc::router(args.path.clone(), args.connect_timeout))
        .split_for_parts();

    api.servers = Some(servers);

    match args.command {
        Some(Commands::ShowApiYaml {}) => {
            let api_doc = serde_json::to_string_pretty(&api).unwrap();
            println!("{api_doc}");
            return Ok(());
        }
        _ => {}
    }

    let registry = tracing_subscriber::registry()
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

    let app = if args.enable_swagger {
        router.merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api.clone()))
    } else {
        router
    };

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
    .await?;

    Ok(())
}

mod ppc {
    use std::{
        net::{IpAddr, SocketAddr},
        sync::Arc,
        time::Duration,
    };

    use axum::{
        Json,
        extract::{ConnectInfo, Query, Request, State},
        response::Html,
    };
    use bytes::BytesMut;
    use hyper::StatusCode;
    use libpeercast_re::{
        ConnectionId,
        pcp::{GnuId, procedure::PcpHandshake},
    };
    use serde::{Deserialize, Serialize};
    use thiserror::Error;
    use tokio::net::TcpStream;
    use utoipa::{IntoParams, ToSchema};
    use utoipa_axum::{router::OpenApiRouter, routes};

    use crate::PPC_TAG;

    #[derive(Debug, Clone)]
    struct Store {
        path: String,
        connect_timeout_mills: u64,
    }

    pub(super) fn router(path: String, connect_timeout_mills: u64) -> OpenApiRouter {
        let store = Arc::new(Store{ path, connect_timeout_mills });
        OpenApiRouter::new()
            .routes(routes!(api_root))
            .routes(routes!(ip_check))
            .routes(routes!(port_check))
            .with_state(store)
    }

    #[utoipa::path(
        get,
        path = "",
        tag = PPC_TAG,
        responses(
            (status = 200, description = "api root document",)
        )
    )]
    async fn api_root(State(store): State<Arc<Store>>) -> Html<String> {
        let path = &store.path;

        Html(format!("<h1>PeerCast-Port-Checker</h1>
<div>
  <ul>
    <li><a href='{path}/ip_check'>{path}/ip</a></li>
    <li><a href='{path}/port_check?port=7144'>{path}/port_check?port=7144</a></li>
    <li><a href='{path}/port_check?port=17144'>{path}/port_check?port=17144</a></li>
    <li>{path}/port_check?port=
      <form action='{path}/port_check' method='get'>
        <input type=number name='port' value='7144' />
        <button>GO</button>
      </form>
    </li>
  </ul>
</div>",
        ))
    }

    /// response Ip check
    #[derive(Serialize, Deserialize, ToSchema, Clone)]
    struct CheckedIp {
        #[schema(format=Ipv4)]
        ip: String,
        done: bool,
    }

    #[utoipa::path(
        get,
        path = "/ip_check",
        tag = PPC_TAG,
        responses(
            (status = 200, description = "success to ip check", body=CheckedIp)
        )
    )]
    async fn ip_check(ConnectInfo(info): ConnectInfo<SocketAddr>) -> Json<CheckedIp> {
        let ip = info.ip();
        Json(CheckedIp {
            ip: ip.to_string(),
            done: true,
        })
    }

    #[derive(Debug, Error)]
    enum PortCheckError {
        #[error("Could not connect in time")]
        FailedConnectRemote(#[from] tokio::time::error::Elapsed),

        #[error("Something io error occured")]
        IoError(#[from] std::io::Error),
    }

    /// params for Port check
    #[derive(Deserialize, IntoParams)]
    struct PortCheckQuery {
        /// Search by value. Search is incase sensitive.
        port: u16,
    }

    /// response Port check
    #[derive(Serialize, Deserialize, ToSchema, Clone)]
    struct CheckedPort {
        #[schema(format=Ipv4)]
        ip: String,

        /// checked port
        #[schema(maximum = 65535)]
        port: u16,

        /// success
        result: bool,
    }

    /// response Port check
    #[derive(Serialize, ToSchema, Clone)]
    struct CheckedPortError {
        #[schema(format=Ipv4)]
        reason: String,

        // always false
        result: bool,
    }

    #[utoipa::path(
        get,
        path = "/port_check",
        tag = PPC_TAG,
        params(PortCheckQuery),
        responses(
            (status = 200, description = "Success to check port", body=CheckedPort),
            (status = 400, description = "Failed to check port, because parameter is wrong"),
            (status = 500, description = "Failed to check port, because can't connect to you", body=CheckedPortError)
        )
    )]
    async fn port_check(
        State(state): State<Arc<Store>>,
        query: Query<PortCheckQuery>,
        ConnectInfo(addr): ConnectInfo<SocketAddr>,
        _req: Request,
    ) -> Result<Json<CheckedPort>, (StatusCode, Json<CheckedPortError>)> {
        let connection_id = ConnectionId::new();

        match outgoing_port_check(connection_id, addr, query.port, state.connect_timeout_mills).await {
            Err(e) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(CheckedPortError {
                    reason: e.to_string(),
                    result: false,
                }),
            )),
            Ok((remote, port, result)) => Ok(Json(CheckedPort {
                ip: remote.to_string(),
                port,
                result,
            })),
        }
    }

    async fn outgoing_port_check(
        connection_id: ConnectionId,
        addr: SocketAddr,
        port: u16,
        connect_timeout: u64,
    ) -> Result<(IpAddr, u16, bool), PortCheckError> {
        let ip = addr.ip();

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
}
