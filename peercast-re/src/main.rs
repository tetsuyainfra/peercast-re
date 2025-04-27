use api::{router, ReStore};
use clap::Parser;
use std::net::SocketAddr;
use tracing::info;


mod cli;
mod api;
mod config;
mod ui;


////////////////////////////////////////////////////////////////////////////////
// MAIN
//
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = cli::Args::parse();
    dbg!(&cli);

    // let Ok((config_path, config)) = config::load_config(cli.config_file.clone()) else {
    //     std::process::exit(exitcode::CONFIG);
    // };
    // let config = cli.merge_with(&config);

    logging_init();
    let (router, api) = router(ReStore{}.into());
    let app = router.merge(ui::router());

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 17145))
        .await
        .unwrap();

    info!(
        "listening on http://{}/",
        listener.local_addr().unwrap(),
    );

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
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
