use std::convert::Infallible;
use std::net::SocketAddr;

use axum::extract::ConnectInfo;
use axum::{extract::Request, routing::get, Router};
use hyper::body::Incoming;
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server;
use tokio::net::TcpListener;
use tower::{Service, ServiceExt};

#[tokio::main]
async fn main() {
    serve_with_connect_info().await;
}

// Similar setup to `serve_plain` but captures the remote address and exposes it through the
// `ConnectInfo` extractor
async fn serve_with_connect_info() {
    let app = Router::new().route(
        "/",
        get(
            |ConnectInfo(remote_addr): ConnectInfo<SocketAddr>| async move {
                format!("Hello {remote_addr}")
            },
        ),
    );

    let mut make_service = app.into_make_service_with_connect_info::<SocketAddr>();

    let listener = TcpListener::bind("0.0.0.0:3001").await.unwrap();

    loop {
        let (socket, remote_addr) = listener.accept().await.unwrap();

        // We don't need to call `poll_ready` because `IntoMakeServiceWithConnectInfo` is always
        // ready.
        let tower_service = unwrap_infallible(make_service.call(remote_addr).await);

        tokio::spawn(async move {
            let socket = TokioIo::new(socket);

            let hyper_service = hyper::service::service_fn(move |request: Request<Incoming>| {
                tower_service.clone().oneshot(request)
            });

            if let Err(err) = server::conn::auto::Builder::new(TokioExecutor::new())
                .serve_connection_with_upgrades(socket, hyper_service)
                .await
            {
                eprintln!("failed to serve connection: {err:#}");
            }
        });
    }
}

fn unwrap_infallible<T>(result: Result<T, Infallible>) -> T {
    match result {
        Ok(value) => value,
        Err(err) => match err {},
    }
}
