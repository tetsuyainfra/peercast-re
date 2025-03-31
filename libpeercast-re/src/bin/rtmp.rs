use peercast_re::{
    rtmp::{
        self, connection,
        rtmp_connection::{RtmpConnection, RtmpConnectionEvent},
    },
    ConnectionId,
};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    logging_init();

    let manager_sender = rtmp::stream_manager::start();
    let manager_sender_cloned = manager_sender.clone();

    let handle = tokio::spawn(async move {
        println!("Listening for connections on port 11935");
        let listener = tokio::net::TcpListener::bind("0.0.0.0:11935").await?;

        loop {
            let (stream, connection_info) = listener.accept().await?;

            let connection_id = ConnectionId::new().0;
            let connection =
                connection::Connection::new(connection_id, manager_sender_cloned.clone());
            println!(
                "Connection {}: Connection received from {}",
                connection_id,
                connection_info.ip()
            );

            tokio::spawn(connection.start_handshake(stream));
        }
        #[allow(unreachable_code)]
        Ok::<_, std::io::Error>(())
    });

    let mut conn = RtmpConnection::new(manager_sender.clone(), ConnectionId::new(), "req1", "");
    let r = conn.connect().await;
    println!("connect: {r}");

    loop {
        let msg = match conn.recv().await {
            Some(msg) => msg,
            None => break,
        };
        match msg {
            RtmpConnectionEvent::NewVideoData {
                // timestamp,
                // data,
                // can_be_dropped,
                ..
            } => todo!(),
            RtmpConnectionEvent::NewAudioData {
                // timestamp,
                // data,
                // can_be_dropped,
                ..
            } => todo!(),
            RtmpConnectionEvent::NewMetadata { metadata: _ } => todo!(),
        }
    }

    handle.await.unwrap()
}

fn logging_init() {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_file(true)
                .with_line_number(true)
                .with_target(false),
        )
        .with(EnvFilter::from("trace,hyper=info"))
        .init();
}
