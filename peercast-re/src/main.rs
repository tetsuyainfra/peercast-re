use anyhow::Context;
use axum::response::Redirect;
use clap::Parser;
use std::net::SocketAddr;
use tracing::info;

use peercast_re::{app, cli, config, handler};

////////////////////////////////////////////////////////////////////////////////
// MAIN
//
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    logging_init();

    let args = cli::Args::parse();
    dbg!(&args);

    let (config, config_path) =
        config::load_config(args.clone()).context("Failed to Load configuration")?;

    let store = app::Store {
        config: config.clone(),
        config_path,
    };
    let store = std::sync::Arc::new(store);
    let router = axum::Router::new()
        .route("/", axum::routing::get(|| async { Redirect::to("/ui") }))
        .nest("/api", handler::api::build_router(store))
        .nest("/ui", handler::ui::build_router());

    let svr_addr = SocketAddr::from((config.server_address, config.server_port));
    let svr_listener = tokio::net::TcpListener::bind(svr_addr)
        .await
        .with_context(|| format!("Failed to bind Server Address: {}", svr_addr))?;

    let api_addr = SocketAddr::from((config.api_address, config.api_port));
    let api_listener = tokio::net::TcpListener::bind(api_addr)
        .await
        .with_context(|| format!("Failed to bind API Address: {}", api_addr))?;

    info!("PeerCast listening on pcp://{}/", svr_listener.local_addr().unwrap());
    info!("  UI/API listening on http://{}/ui", api_listener.local_addr().unwrap());

    axum::serve(
        api_listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .context("Serving Application Error")?;

    Ok(())
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
