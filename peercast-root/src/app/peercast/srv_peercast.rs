// app/peercast/server.rs

use std::time::Duration;

use libpeercast_re::{
    connection::{Connection, ConnectionFactory, ConnectionNo, HandshakeConnection},
    io::IoStream,
};
use peercast_root::connection::{Handshaked, RootHandshakeConfig};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tracing::info;

use crate::app::state::ArcState;

///
/// PCPサーバーを起動する関数
/// - handshake中のタスクはここで管理する
pub async fn server_peercast(
    state: ArcState,
    listener: TcpListener,
    graceful_shutdown: CancellationToken,
) -> anyhow::Result<()> {
    info!("START PCP SERVER");
    let (closed_tx, closed_rx) = tokio::sync::watch::channel(());
    let tracker = tokio_util::task::TaskTracker::new();

    'accept: loop {
        let cno = ConnectionNo::new();
        let name = format!("Handshake-{}", cno);
        let spawner = tokio::task::Builder::new().name(&name);
        let child_graceful_shutdown = graceful_shutdown.child_token();
        let state = state.clone();

        // Dropすることで、全ての接続終了を確認する
        let closed_rx = closed_rx.clone();

        tokio::select! {
            accept = listener.accept() => {
               match accept {
                    Ok((stream, remote)) => {
                        let fut = tracker.track_future(handle_handshake(cno, stream, remote, state, child_graceful_shutdown, closed_rx));
                        spawner.spawn(fut).expect("Failed to spawn handshake task");
                    }
                    Err(e) => {
                        info!("Failed to accept connection: {:?}", e);
                    }
                }
            }
            _ = graceful_shutdown.cancelled() => {
                info!("GRACEFUL SHUTDOWN REQUESTED");
                break 'accept;
            }
        }
    }

    // 自身で持っている接続を閉じる
    drop(closed_rx);
    // 全ての接続が閉じるまで待つ
    let _ = closed_tx.closed().await;

    // trackerを閉じて、新規にSpawnできないようにし、全てのスレッドが終了するのを待つ
    tracker.close();
    tracker.wait().await;

    Ok(())
}

/// ハンドシェイクを処理する関数
/// - ハンドシェイクが成功したら、コネクションのrun()を新しいタスクで実行する
pub async fn handle_handshake(
    cno: ConnectionNo,
    stream: tokio::net::TcpStream,
    remote: std::net::SocketAddr,
    state: ArcState,
    graceful_shutdown: CancellationToken,
    closed_rx: tokio::sync::watch::Receiver<()>,
) -> anyhow::Result<()> {
    info!(cno = ?cno, "Accepted connection from {}", remote);

    let config = RootHandshakeConfig {
        self_session_id: state.self_session_id,
        shutdown: graceful_shutdown,
    };
    let handshake = state.conns.factory.create_accepted_connection(cno, IoStream::Tcp(stream), remote, Some(config));

    let handshaked = tokio::time::timeout(Duration::from_secs(5), handshake.handshake()).await??;

    match handshaked {
        Handshaked::Pcp(conn) => {
            info!(cno = ?cno, "Handshake succeeded with {}", remote);
            info!(cno = ?cno, "Starting PCP connection with {}", remote);
            let _ = tokio::task::Builder::new()
                .name(format!("Conn-{}", cno).as_str())
                .spawn(conn.run())
                .expect("Failed to spawn PCP connection task");
        }
    }

    drop(closed_rx);
    Ok(())
}
