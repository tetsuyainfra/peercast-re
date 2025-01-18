use std::{
    collections::{HashMap, VecDeque},
    marker::PhantomData,
    net::SocketAddr,
    path::PathBuf,
    str::FromStr,
    sync::{Arc, Mutex, MutexGuard},
    time::Duration,
};

use bytes::BytesMut;
use thiserror::Error;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{broadcast, mpsc},
    time::Instant,
};
use tracing::{debug, info};

use crate::{
    config::Config,
    error::HandshakeError,
    pcp::{
        builder::{OkBuilder, OlehBuilder, RootBuilder},
        Atom, ChannelManager, GnuId, PcpConnectType, PcpConnectionFactory,
    },
    ConnectionId,
};

#[derive(Debug, Error)]
pub enum RootError {
    #[error("failed config file loading")]
    LoadConfiguration,
    #[error("application something error occured.")]
    ApplicationError,

    #[error("application is finish but failed to gracefull shutdown: {0}")]
    ShutdownFailed(String),

    #[error("IoError: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Error)]
pub enum MainError {
    #[error("Handshake Error")]
    HandshakeError(#[from] HandshakeError),
}

pub struct RootApp {
    config_path: PathBuf,
    config: Config,
    notify_shutdown_tx: broadcast::Sender<()>,
    shutdown_complete_tx: mpsc::Sender<()>,
}

impl RootApp {
    const WAIT_FORCE_SHUTDOWN_SEC: u64 = 60;
    const WAIT_FORCE_SHUTDOWN_CTRLC_TIMES: usize = 3;

    pub fn run(config_path: PathBuf, config: Config) -> Result<(), RootError> {
        debug!(?config);
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(4)
            .build()
            .unwrap();

        rt.block_on(async {
            let (notify_shutdown_tx, _) = broadcast::channel(1);
            let (shutdown_complete_tx, mut shutdown_complete_rx) = mpsc::channel(1);

            let mut app = Self {
                config_path, config,
                notify_shutdown_tx,
                shutdown_complete_tx
            };
            // アプリケーションの実行
            tokio::select! {
                r = app.main() => {
                    if let Err(err) = r {
                        return Err(err);
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    // この時点でSelf::main内のインスタンスは全部Dropされる
                    println!(
                        "Exit 'after {} seconds' or 'press Ctrl+c 3 times'",
                        Self::WAIT_FORCE_SHUTDOWN_SEC
                    );
                }
            };

            let RootApp { mut notify_shutdown_tx, mut shutdown_complete_tx ,..} = app;
            drop(notify_shutdown_tx); // シャットダウンをspawnしたタスクへ通知する
            drop(shutdown_complete_tx); //

            enum GarcefullShutdownReason {
                Success,
                AfterPeriod,
                UserForce,
            }
            let gracefull_reason = tokio::select! {
                _ = async move {
                     // シャットダウン通知を待つ(正確には全ての送信者が閉じられるのを待ち、Noneが帰って来るのを待つ)
                    let r = shutdown_complete_rx.recv().await;
                    match r {
                        Some(_) => panic!("shutdown_complete_rx never send values"),
                        None => {},
                    };
                } => {
                    println!("Gracefull shutdown completly.");
                    GarcefullShutdownReason::Success
                }
                // 5秒毎に経過時間を表示(終わることはない)
                _ = async {
                    let start = Instant::now() + Duration::from_secs(5);
                    let mut interval = tokio::time::interval_at(start, Duration::from_secs(5));
                    let mut count = 0;
                    loop {
                        count +=1;
                        interval.tick().await;
                        println!("{} secs...", count * 5);
                    }
                }=> {
                    unreachable!()
                }
                // 指定時間終了を待つ
                _ = async {
                    tokio::time::sleep(Duration::from_secs(Self::WAIT_FORCE_SHUTDOWN_SEC)).await;
                }=> {
                    println!("{}sec pass. will force shutdown", Self::WAIT_FORCE_SHUTDOWN_SEC);
                    GarcefullShutdownReason::AfterPeriod
                }
                // Ctrl+c 3回
                _ = async {
                    for i in 0..Self::WAIT_FORCE_SHUTDOWN_CTRLC_TIMES {
                        tokio::signal::ctrl_c().await;
                        println!("Ctrl+c detected {}/{}", i+1, Self::WAIT_FORCE_SHUTDOWN_CTRLC_TIMES);
                    }
                }=> {
                    println!("Ctrl+c {} times detected. will force shutdown", Self::WAIT_FORCE_SHUTDOWN_CTRLC_TIMES);
                    GarcefullShutdownReason::UserForce
                },
            };


            match gracefull_reason {
                GarcefullShutdownReason::Success => Ok(()),
                GarcefullShutdownReason::AfterPeriod => Err(RootError::ShutdownFailed(format!("Wait {}seconds, but can't shutdowned", Self::WAIT_FORCE_SHUTDOWN_SEC))),
                GarcefullShutdownReason::UserForce =>Err(RootError::ShutdownFailed("User send ctrl+c".into())),
            }
        }) // rt.block()
    }

    async fn main(&mut self) -> Result<(), RootError> {
        let self_session_id = match self.config.root_session_id {
            None => GnuId::new(),
            Some(id) => id,
        };

        let ipaddr = self.config.server_address.to_ipaddr();
        let port = self.config.server_port;
        let addr = (ipaddr, port);
        let listener = TcpListener::bind(addr).await?;

        let factory = PcpConnectionFactory::new(self_session_id);
        loop {
            let connection_id = ConnectionId::new();
            let (stream, remote) = listener.accept().await?;

            let pcp_handshake = factory.accept(stream, remote);

            let connection_handle = tokio::spawn(async move {
                let mut pcp_connection = pcp_handshake.incoming_pcp().await?;
                match &pcp_connection.con_type {
                    PcpConnectType::Outgoing => unreachable!(),
                    PcpConnectType::IncomingPing(ping_info) => {
                        todo!("retrun oleh and quit")
                    }
                    PcpConnectType::IncomingBroadcast(helo_info) => {
                        //
                        info!("Incomming Broadcast")
                        // let atom = pcp_connection.read_atom().await;
                    }
                };

                Ok::<_, MainError>(())
            });
        }

        Ok(())
    }
}

struct Broker<MSG> {
    channel_manager: ChannelManager,
    _a: PhantomData<MSG>,
}

//-----
// PeerCastStation
// Client -> Root (YP)への接続
// https://github.com/kumaryu/peercaststation/blob/6184647e600ec3a388462169ab7118314114252e/PeerCastStation/PeerCastStation.PCP/PCPYellowPageClient.cs#L370C44-L370C58
