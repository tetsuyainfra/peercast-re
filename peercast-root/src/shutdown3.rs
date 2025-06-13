use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::pin::pin;
use tokio::{signal, spawn};
use tokio::time::{Duration, sleep};
use tokio_util::sync::CancellationToken;

pub fn create_task() -> (
    impl Future<Output = ()>,
    CancellationToken,
) {
    let graceful_shutdown = CancellationToken::new();

    let child_graceful_shutdown = graceful_shutdown.child_token();

    let handler = async move {
        tokio::select! {
            _ = signal::ctrl_c() => {
                println!("\ncatched Ctrl+C");
                graceful_shutdown.cancel();
            }
        };
    };
    (handler, child_graceful_shutdown)
}

pub fn create_task_anyhow() -> (
    impl Future<Output = anyhow::Result<()>>,
    CancellationToken,
) {
    let (handler, graceful) = create_task();

    let new_handler = async move {
        handler.await;
        anyhow::Ok(())
    };

    (new_handler, graceful)
}

#[cfg(test)]
mod tests {
    use futures_util::FutureExt;
    use hyper_util::server::graceful;
    use tokio::sync::watch;
    use tokio_util::task::TaskTracker;
    use tracing::info;

    use super::*;
    async fn wait_for_cancellation(no: usize, token: CancellationToken) {
        println!("no: {no} started");
        tokio::select! {
            _ = token.cancelled() => {
                println!("no: {no} cancelled");
            }
        }
        println!("no: {no} finished");
    }

    #[tokio::test]
    async fn test_wait_for_cancellation() {
        let token = CancellationToken::new();

        let graceful_shutdown_token = token.child_token();
        let (signal_tx, signal_rx) = watch::channel(());

        tokio::spawn(async move {
            //
            graceful_shutdown_token.cancelled().await;
            info!("received graceful shutdown signal. Telling tasks to shutdown");
            drop(signal_rx);
        });

        let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
        let (close_tx, close_rx) = watch::channel(());

        loop {
            let (io, remote_addr) = tokio::select! {
                conn = listener.accept() => conn.unwrap(),
                _ = signal_tx.closed() => {
                    info!("signal received, not accepting new connections");
                    break;
                }
            };
            handle_connection(&signal_tx, &close_rx, io, remote_addr).await;
        }
        // 自分が持っている終了通知チャンネルをDrop
        drop(close_rx);
        drop(listener);

        // 子が開いている終了通知チャンネルがすべて終了されるのを待つ
        close_tx.closed().await;
    }

    async fn handle_connection(
        signal_tx: &watch::Sender<()>,
        close_rx: &watch::Receiver<()>,
        io: tokio::net::TcpStream,
        remote_addr: std::net::SocketAddr,
    ) {
        // 親からの終了を検出する
        let signal_tx = signal_tx.clone();
        // 親へ終了を教える(drop(close_rx)で終了を通知する)
        let close_rx = close_rx.clone();

        tokio::spawn(async move {
            // let mut conn = pin!(builder.serve_connection_with_upgrades(io, hyper_service));
            let mut signal_closed = pin!(signal_tx.closed().fuse());

            loop {
                tokio::select! {
            //         result = conn.as_mut() => {
            //             if let Err(_err) = result {
            //                 trace!("failed to serve connection: {_err:#}");
            //             }
            //             break;
            //         }
                    _ = &mut signal_closed => {
                        info!("signal received in task, starting graceful shutdown");
                        // conn.as_mut().graceful_shutdown();
                    }
                }
            }
            drop(close_rx);
        });
    }
}
