// app/http/server.rs

use axum::{http::HeaderValue, routing};
use axum_client_ip::ClientIpSource;
use hyper::Method;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tower_http::{
    cors::CorsLayer,
    set_header::SetResponseHeaderLayer,
    trace::{DefaultMakeSpan, TraceLayer},
};
use tracing::info;

use crate::app::{http::handler, state::ArcState};

pub async fn server_http(
    state: ArcState,
    listener: TcpListener,
    graceful_shutdown: CancellationToken,
) -> anyhow::Result<()> {
    let use_outer_html_dir = state.api_config.use_outer_html_dir.clone();
    info!("use_outer_html_dir: {:?}", &use_outer_html_dir);

    let cor_origins: Vec<_> =
        state.api_config.allow_cors.iter().map(|origin| origin.parse::<HeaderValue>().unwrap()).collect();
    info!("cor_origins: {:?}", &cor_origins);

    let cache_control_value = format!("max-age={}, public, mustrevalidate", &state.api_config.cache_max_age);
    info!("cache-control: {}", &cache_control_value);

    let client_ip_source: ClientIpSource = state.api_config.client_ip_source.clone().into();
    info!("client-ip-source: {:?}", &client_ip_source);

    let _tracker = tokio_util::task::TaskTracker::new();
    info!("START HTTP SERVER");

    let app = axum::Router::new()
        .route("/index.txt", routing::get(handler::index_txt))
        .route("/index.json", routing::get(handler::index_json))
        .fallback_service(handler::static_router(use_outer_html_dir, state.api_config.embed_tmpl_ctx.clone()))
        //
        .layer(TraceLayer::new_for_http().make_span_with(DefaultMakeSpan::default().include_headers(true)))
        .layer(CorsLayer::new().allow_origin(cor_origins).allow_methods([Method::GET]))
        .layer(SetResponseHeaderLayer::if_not_present(
            axum::http::header::CACHE_CONTROL,
            HeaderValue::from_bytes(cache_control_value.as_bytes()).unwrap(),
        ))
        .layer(client_ip_source.into_extension())
        .with_state(state);

    let _ = axum::serve(listener, app.into_make_service_with_connect_info::<std::net::SocketAddr>())
        .with_graceful_shutdown(graceful_shutdown.cancelled_owned())
        .await;

    Ok(())
}
