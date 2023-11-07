use std::{
    net::SocketAddr,
    str::FromStr,
    sync::{Arc, RwLock},
};

use bytes::BytesMut;
use peercast_re::{
    pcp::{
        procedure::PcpHandshake, ChannelManager, ChannelType, GnuId, RelayTaskConfig,
        SourceTaskConfig,
    },
    ConnectionId,
};
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() {
    logging_init();

    let session_id = GnuId::new();
    let channel_manager = ChannelManager::new(&session_id);
    let is_port_open = Arc::new(RwLock::new(false));

    let self_addr: SocketAddr = "192.168.10.231:61751".parse().unwrap();
    let listener = TcpListener::bind(self_addr).await.unwrap();
    let cloned_channel_manager: Arc<ChannelManager> = Arc::clone(&channel_manager);
    let handle = tokio::spawn(async move {
        let (stream, remote) = listener.accept().await.unwrap();
        let connection_id = ConnectionId::new();
        let read_buf = BytesMut::with_capacity(4096);
        info!("Incomming {:?}", remote);

        let _handshaked = PcpHandshake::new(
            connection_id,
            stream,
            Some(self_addr),
            remote,
            read_buf,
            session_id,
        )
        .incoming(cloned_channel_manager)
        .await;
    });

    let server_port = self_addr.port();
    let is_port_open_cloned = Arc::clone(&is_port_open);
    let _port_check_handle: tokio::task::JoinHandle<Result<(), reqwest::Error>> =
        tokio::spawn(async move {
            info!("port check");
            let res = reqwest::get(format!(
                "http://ppc-v4.tetsuyainfra.dev:7145/ppc/portcheck?port={server_port}"
            ))
            .await?;

            let _body = res.bytes().await?;

            let mut is_port_open = is_port_open_cloned.write().unwrap();
            *is_port_open = true;
            Ok::<_, _>(())
        });

    // debugging(Relay)
    let url = match std::env::var("PEERCAST_RE_DEBUG_URL") {
        Ok(s) => url::Url::parse(&s).unwrap(),
        Err(_) => todo!(),
    };
    let id = GnuId::from_str(url.path().split("/").last().unwrap()).unwrap();
    // let (key, val) = url.query_pairs().find(|(k, v)| k == "tip").unwrap();

    let addr = "192.168.10.230:61744".parse().unwrap();
    let ch = channel_manager
        .create(id, ChannelType::Relay, None, None)
        .unwrap();
    ch.connect(
        ConnectionId::new(),
        SourceTaskConfig::Relay(RelayTaskConfig {
            addr: addr,
            self_addr: Some(self_addr),
            // self_addr: None,
        }),
    );

    // Connectingになるのを待つ
    let mut reciever = ch.channel_reciever(ConnectionId::new());
    loop {
        match reciever.recv().await {
            Some(msg) => match msg {
                peercast_re::pcp::ChannelMessage::RelayChannelHead { .. } => {
                    println!("head");
                }
                peercast_re::pcp::ChannelMessage::RelayChannelData { .. } => {
                    // println!("data");
                }
            },
            None => break,
        }
    }

    let _ = handle.await;
}

/// initialize logging
fn logging_init() {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};
    // tracing_subscriber::fmt()
    //     // enable everything
    //     .with_max_level(tracing::Level::TRACE)
    //     // display source code file paths
    //     .with_file(true)
    //     // display source code line numbers
    //     .with_line_number(true)
    //     // disable targets
    //     .with_target(false)
    //     // sets this to be the default, global collector for this application.
    //     .init();

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
