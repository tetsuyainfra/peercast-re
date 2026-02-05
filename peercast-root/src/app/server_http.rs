use std::{net::SocketAddr, path::PathBuf};

use axum::{Router, http::HeaderValue, routing};
use hyper::Method;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tower_http::{cors::CorsLayer, services::ServeDir, set_header::SetResponseHeaderLayer};

use crate::app::{ArcState, cli, handler};
use peercast_root::prelude::*;

pub async fn serve(
    args: cli::Args,
    state: ArcState,
    listener: TcpListener,
    graceful_shutdown: CancellationToken,
) -> anyhow::Result<()> {
    
    use tower_http::trace::{DefaultMakeSpan, TraceLayer};

    let assets_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
    info!("asset_dir: {:?}", &assets_dir);

    let cor_origins: Vec<_> = args.allow_cors.iter().map(|origin| origin.parse::<HeaderValue>().unwrap()).collect();
    info!("cor_origins: {:?}", &cor_origins);

    let cache_control_value = format!("max-age={}, public, mustrelvalidate", &args.cache_max_age);
    info!("cache-control: {}", &cache_control_value);

    let _tracker = tokio_util::task::TaskTracker::new();
    info!("START HTTP SERVER");

    let app = Router::new()
        .fallback_service(ServeDir::new(assets_dir).append_index_html_on_directories(true))
        .route("/index.txt", routing::get(handler::index_txt))
        .route("/api/index.json", routing::get(handler::index_json))
        .layer(TraceLayer::new_for_http().make_span_with(DefaultMakeSpan::default().include_headers(true)))
        .layer(CorsLayer::new().allow_origin(cor_origins).allow_methods([Method::GET]))
        .layer(SetResponseHeaderLayer::if_not_present(
            axum::http::header::CACHE_CONTROL,
            HeaderValue::from_bytes(cache_control_value.as_bytes()).unwrap(),
        ))
        .layer(args.ip_source.into_extension())
        .with_state(state);

    let _ = axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(graceful_shutdown.cancelled_owned())
        .await;

    Ok(())
}
