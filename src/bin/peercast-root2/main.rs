use std::net::SocketAddr;

use clap::Parser;
use hyper::body::Incoming;
use peercast_re::ConnectionId;
use tokio::net::TcpStream;
use tokio_util::sync::CancellationToken;
use tracing::info;

// use crate::channel::{tracker_channel::TrackerChannel, ChannelStore};

mod channel;
mod cli;
mod logging;
mod manager;

#[cfg(test)]
mod test_helper;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = cli::Args::parse();

    cli::version_print(&args)?;

    logging::init(&args)?;
    // let _store = ChannelStore::<TrackerChannel>::new(None, None);

    info!("logging");

    Ok(())
}
// serve_http2
// crate::extract::connect_info::IntoMakeServiceWithConnectInfo みたいなのを実装しないといけない・・・
// だるくない？
struct MyIncomingStream {}

async fn serve_http2<'a, T, S>(
    connection_id: ConnectionId,
    stream: TcpStream,
    remote_addr: SocketAddr,
    graceful_shutdown: CancellationToken,
    force_shutdown: CancellationToken,
    // mut make_service: IntoMakeServiceWithConnectInfo<axum::Router, MyConnectInfo>,
    mut make_service: T,
) where
    // T: tower::Service<IncomingStream<'a, tokio::net::TcpListener>, Error = Infallible>,
    T: tower::Service<MyIncomingStream, Error = std::convert::Infallible, Response = S>,
    S: tower::Service<
            axum_core::extract::Request,
            Response = axum_core::response::Response,
            Error = std::convert::Infallible,
        > + Clone
        + Send
        + 'static,
    S::Future: Send,
{
    use tower::util::ServiceExt;
    let io = hyper_util::rt::TokioIo::new(stream);

    std::future::poll_fn(|cx| make_service.poll_ready(cx))
        .await
        .unwrap_or_else(|err| match err {});
    let tower_service = make_service
        .call(MyIncomingStream {})
        .await
        .unwrap_or_else(|err| match err {})
        .map_request(|req: axum_core::extract::Request<Incoming>| {
            req.map(axum_core::body::Body::new)
        });

    let hyper_service = hyper_util::service::TowerToHyperService::new(tower_service);

    let mut builder =
        hyper_util::server::conn::auto::Builder::new(hyper_util::rt::TokioExecutor::new());
    // builder.http2().enable_connect_protocol(); // ENABLE HTTP2
    let conn = builder.serve_connection_with_upgrades(io, hyper_service);
    futures_util::pin_mut!(conn);

    loop {
        // tokio::select! {
        //     result = conn.as_mut() => {
        //         if let Err(_err) = result {
        //             trace!("failed to serve connection: {_err:#}");
        //         }
        //         break;
        //     }
        //     // _ = &mut signal_closed => {
        //     //     trace!("signal received in task, starting graceful shutdown");
        //     //     conn.as_mut().graceful_shutdown();
        //     // }
        // };
    }
}
