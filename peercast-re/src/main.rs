use anyhow::{Context, bail};
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

    let config = config::load_config(args.clone()).context("Failed to Load configuration")?;


    let store = app::Store {
        config,
        // config_path: Some(config_path.clone()),
    };
    let store = std::sync::Arc::new(store);
    let router = axum::Router::new()
        .route("/",  axum::routing::get(|| async { "/" }))
        .nest("/api", handler::api::build_router(store))
        .nest("/ui", handler::ui::build_router());

    let addr  = SocketAddr::from(([127, 0, 0, 1], 17145));

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| { format!("Failed to bind Address: {}", addr)})?;

    info!(
        "listening on http://{}/",
        listener.local_addr().unwrap(),
    );

    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await.context("Serving Application Error")?;
    // match cui::CuiApp::run(config_path, config) {
    //     Ok(_) => std::process::exit(exitcode::OK),
    //     Err(e) => {
    //         println!("{e}");
    //         match e {
    //             CuiError::LoadConfiguration => std::process::exit(exitcode::CONFIG),
    //             CuiError::ApplicationError => std::process::exit(exitcode::SOFTWARE),
    //             CuiError::ShutdownFailed(_) => std::process::exit(exitcode::SOFTWARE),
    //             CuiError::Io(_) => std::process::exit(exitcode::IOERR),
    //         }
    //     }
    // }

    Ok(())
}

/// initialize logging
fn logging_init() {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

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
