use std::{
    clone,
    convert::Infallible,
    net::SocketAddr,
    path::PathBuf,
    process::exit,
    str::FromStr,
    sync::{
        atomic::{AtomicI8, Ordering},
        mpsc::channel,
        Arc, Mutex,
    },
    task::Poll,
    time::Duration,
};

use axum_core::{extract::Request, response::Response};
use futures_util::{stream, task::SpawnExt, Future, FutureExt};
use hyper_util::client::connect;
use ipnet::IpNet;
use thiserror::Error;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{
        broadcast,
        mpsc::{self, UnboundedSender},
    },
    time::{sleep, Instant},
};
use tower::Service;
use tracing::{info, warn};

use crate::{
    config::Config,
    error,
    http::{HttpSvc, MyConnectInfo, MyIncomingStream, ShutdownAndNotifySet},
    pcp::{BroadcastTaskConfig, ChannelInfo, ChannelManager, GnuId, TrackInfo},
    rtmp::{
        connection,
        stream_manager::{self, StreamManagerMessage},
    },
    util::{identify_protocol, ConnectionProtocol, Shutdown},
    ConnectionId,
};

#[derive(Debug, Error)]
pub enum CuiError {
    #[error("failed config file loading")]
    LoadConfiguration,
    #[error("application something error occured.")]
    ApplicationError,

    #[error("application is finish but failed to gracefull shutdown: {0}")]
    ShutdownFailed(String),

    #[error("IoError: {0}")]
    Io(#[from] std::io::Error),
}
pub struct CuiApp {
    config_path: PathBuf,
    config: Config,
    notify_shutdown_tx: broadcast::Sender<()>,
    shutdown_complete_tx: mpsc::Sender<()>,
}

impl CuiApp {
    const WAIT_FORCE_SHUTDOWN_SEC: u64 = 60;
    const WAIT_FORCE_SHUTDOWN_CTRLC_TIMES: usize = 3;

    pub fn run(config_path: PathBuf, config: Config) -> Result<(), CuiError> {
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

            let CuiApp { mut notify_shutdown_tx, mut shutdown_complete_tx ,..} = app;
            drop(notify_shutdown_tx); // シャットダウンをspawnしたタスクへ通知する
            drop(shutdown_complete_tx); //

            enum GarcefullShutdownReason {
                Success,
                AfterPeriod,
                UserForce,
            }
            let gracefull_reason = tokio::select! {
                // gracefull シャットダウンを待つ
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
                GarcefullShutdownReason::AfterPeriod => Err(CuiError::ShutdownFailed(format!("Wait {}seconds, but can't shutdowned", Self::WAIT_FORCE_SHUTDOWN_SEC))),
                GarcefullShutdownReason::UserForce =>Err(CuiError::ShutdownFailed("User send ctrl+c".into())),
            }
        }) // rt.block()
    }

    // TODO: ApplicationをRestartする機能を付けるならResult<ShutdownOrRestart, TuiError>っていう変数を返せばよさそう
    async fn main(&mut self) -> Result<(), CuiError> {
        let session_id = GnuId::new();
        let channel_manager = ChannelManager::new();
        let manager_sender = stream_manager::start();
        let http_svc = HttpSvc::new(
            self.config_path.clone(),
            self.config.clone(),
            session_id,
            Arc::clone(&channel_manager),
            Arc::new(manager_sender.clone()),
        );

        // RTMP
        let c = &self.config;
        let server_addr: String = format!("{}:{}", c.server_address.to_ipaddr(), c.server_port);
        info!("bind server -> {server_addr}");
        info!("UI          -> http://localhost:{}/ui/", c.server_port);
        let listener = tokio::net::TcpListener::bind(server_addr).await?;

        // RTMP
        let rtmp_addr = format!("{}:{}", "127.0.0.1", c.rtmp_port); // FIXME: config.rtmp_addressの追加が必要かな？
        info!("rtmp server -> rtmp://localhost:{}", c.rtmp_port);
        let rtmp_listener = tokio::net::TcpListener::bind(rtmp_addr.clone()).await?;
        let _rtmp_handle = tokio::spawn(Self::spawn_rtmp_server(
            manager_sender.clone(),
            rtmp_listener,
            rtmp_addr,
        ));

        // debugging
        let ch = channel_manager.create(
            GnuId::from_str("00112233445566778899AABBCCDDEEFF").unwrap(),
            crate::pcp::ChannelType::Broadcast {
                app: "req1".into(),
                pass: "".into(),
            },
            ChannelInfo::new().name("Test".into()).into(),
            TrackInfo::default().into(),
        );
        ch.unwrap().connect(
            ConnectionId::new(),
            session_id,
            BroadcastTaskConfig {
                app_key: "req1".into(),
                stream_key: "".into(),
                rtmp_manager: manager_sender.clone(),
            }
            .into(),
        );

        'accept_loop: loop {
            let mut connection_id = ConnectionId::new();
            let shutdown = Shutdown::new(self.notify_shutdown_tx.subscribe());
            let shutdown_complete_tx = self.shutdown_complete_tx.clone();
            let shutdown_set = (shutdown, shutdown_complete_tx);
            // pcp
            let cloned_channel_manager = Arc::clone(&channel_manager);
            // http
            let cloned_http_service = http_svc.clone();
            // rtmp
            let cloned_manager_sender = manager_sender.clone();
            // let cloned_local_address = Arc::clone(&local_address);

            let (mut tcp_stream, remote_addr) = listener.accept().await?;

            tokio::spawn(async move {
                info!("Next is Connection: {connection_id} {remote_addr:?}");
                let protocol = match identify_protocol(&tcp_stream).await {
                    Ok(p) => p,
                    Err(e) => {
                        warn!("comming protocol identify error :{e}");
                        ConnectionProtocol::Unknown
                    }
                };
                match &protocol {
                    ConnectionProtocol::PeerCast | ConnectionProtocol::PeerCastHttp => {
                        Self::spawn_pcp_server(
                            cloned_channel_manager,
                            cloned_manager_sender,
                            // cloned_local_address,
                            connection_id,
                            tcp_stream,
                            remote_addr,
                            protocol,
                            shutdown_set,
                        )
                        .await;
                    }
                    ConnectionProtocol::Http | ConnectionProtocol::Unknown => {
                        Self::spawn_http_server(
                            // cloned_local_address,
                            connection_id,
                            tcp_stream,
                            remote_addr,
                            shutdown_set,
                            cloned_http_service
                                .into_make_service_with_connect_info::<MyConnectInfo>(),
                        )
                        .await;
                    }
                };
            });
        }
        Ok(())
    }
    async fn spawn_rtmp_server(
        manager_sender: UnboundedSender<StreamManagerMessage>,
        listener: TcpListener,
        rtmp_addr: String,
    ) -> Result<(), std::io::Error> {
        println!("Listening for connections on {}", rtmp_addr);

        loop {
            let (stream, connection_info) = listener.accept().await?;
            let current_id = ConnectionId::new();

            let connection = connection::Connection::new(current_id.0, manager_sender.clone());
            println!(
                "Connection {}: Connection received from {}",
                current_id.0,
                connection_info.ip()
            );

            tokio::spawn(connection.start_handshake(stream));
        }
    }

    async fn spawn_pcp_server(
        channel_manager: Arc<ChannelManager>,
        rtmp_stream_manager: UnboundedSender<StreamManagerMessage>,
        // local_address: Arc<Vec<IpNet>>,
        //
        connection_id: ConnectionId,
        tcp_stream: TcpStream,
        remote_addr: SocketAddr,
        protocol: ConnectionProtocol,
        shutdown_set: ShutdownAndNotifySet,
    ) {
        let channel_manager = channel_manager.clone();

        let handle = tokio::task::spawn(async move {
            info!("incomming PCP Protocol");
            // connection.start_negotiation(stream).await
            drop(shutdown_set)
        });
    }

    async fn spawn_http_server<M, S>(
        // local_address: Arc<Vec<IpNet>>,
        //
        connection_id: ConnectionId,
        tcp_stream: TcpStream,
        remote_addr: SocketAddr,
        shutdown_set: ShutdownAndNotifySet,
        mut make_service: M,
    ) where
        M: for<'a> Service<MyIncomingStream<'a>, Error = Infallible, Response = S>,
        S: Service<Request, Response = Response, Error = Infallible> + Clone + Send + 'static,
        S::Future: Send,
    {
        use axum_core::body::Body;
        use futures_util::future::poll_fn;
        use hyper::{self as hyper1, server::conn::http1};
        use hyper_util::rt::TokioIo;
        use tower_hyper_http_body_compat::TowerService03HttpServiceAsHyper1HttpService;
        use tower_hyper_http_body_compat::{HttpBody04ToHttpBody1, HttpBody1ToHttpBody04};

        let tcp_stream = TokioIo::new(tcp_stream);
        poll_fn(|cx| make_service.poll_ready(cx))
            .await
            .unwrap_or_else(|err| match err {});

        let service = make_service
            .call(MyIncomingStream {
                connection_id,
                tcp_stream: &tcp_stream,
                remote_addr,
                shutdown: Arc::new(Mutex::new(Some(shutdown_set))),
            })
            .await
            .unwrap_or_else(|err| match err {});

        let service = hyper1::service::service_fn(move |req: Request<hyper1::body::Incoming>| {
            // `hyper1::service::service_fn` takes an `Fn` closure. So we need an owned service in
            // order to call `poll_ready` and `call` which need `&mut self`
            let mut service = service.clone();

            let req = req.map(|body| {
                // wont need this when axum uses http-body 1.0
                let http_body_04 = HttpBody1ToHttpBody04::new(body);
                Body::new(http_body_04)
            });

            // doing this saves cloning the service just to await the service being ready
            //
            // services like `Router` are always ready, so assume the service
            // we're running here is also always ready...
            match poll_fn(|cx| service.poll_ready(cx)).now_or_never() {
                Some(Ok(())) => {}
                Some(Err(err)) => match err {},
                None => {
                    // ...otherwise load shed
                    let mut res = Response::new(HttpBody04ToHttpBody1::new(Body::empty()));
                    *res.status_mut() = http::StatusCode::SERVICE_UNAVAILABLE;
                    return std::future::ready(Ok(res)).left_future();
                }
            }

            let future = service.call(req);

            async move {
                let response = future
                    .await
                    .unwrap_or_else(|err| match err {})
                    // wont need this when axum uses http-body 1.0
                    .map(HttpBody04ToHttpBody1::new);

                Ok::<_, Infallible>(response)
            }
            .right_future()
        }); // let service = hyper1::service::service_fn(...)

        let _handle = tokio::task::spawn(async move {
            match http1::Builder::new()
                .serve_connection(tcp_stream, service)
                // for websockets
                // .with_upgrades()
                .await
            {
                Ok(()) => {}
                Err(_err) => {
                    // This error only appears when the client doesn't send a request and
                    // terminate the connection.
                    //
                    // If client sends one request then terminate connection whenever, it doesn't
                    // appear.
                }
            }
        });
    }
}

struct TuiAppState {
    pub session_id: GnuId,
    pub channel_manager: Arc<ChannelManager>,

    // TODO: いずれConfigManagerで変更通知が行くようにする
    pub config: Config,
    pub config_path: PathBuf,
}
