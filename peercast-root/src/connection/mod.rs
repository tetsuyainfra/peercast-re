use clap::error;
use libpeercast_re::{
    atom::codec::AtomCodec,
    connection::{
        Connection, ConnectionHandle, ConnectionNo, ConnectionSpec, HandshakeConnection,
        shared::{SharedFactory, SharedManager},
    },
    io::{self, IoStream},
    pcp::builder::{HeloInfo, MagicInfo, OlehBuilder},
    service::{PortChecker, PortCheckerService},
};
use tokio_stream::StreamExt;
use tokio_util::codec::Framed;

pub type RootConnectionManager = SharedManager<RootConnectionSpec>;
pub type RootConnectionFactory = SharedFactory<RootConnectionSpec>;

#[derive(Debug)]
pub struct RootConnectionSpec {}

impl ConnectionSpec for RootConnectionSpec {
    type Handshake = RootHandshake;

    type HandshakeConfig = RootHandshakeConfig;

    type State = RootState;

    type Stats = RootStats;

    type Handle = RootHandle;

    type Manager = SharedManager<Self>;
}

pub struct RootHandshake {
    cno: ConnectionNo,
    stream: IoStream,
    remote: std::net::SocketAddr,
    self_session_id: libpeercast_re::GnuId,
}

#[async_trait::async_trait]
impl HandshakeConnection for RootHandshake {
    type Spec = RootConnectionSpec;

    type HandshakeResult = Result<Handshaked, HandshakeError>;

    fn new(
        cno: ConnectionNo,
        stream: IoStream,
        remote: std::net::SocketAddr,
        config: Option<<Self::Spec as ConnectionSpec>::HandshakeConfig>,
        manager: <Self::Spec as ConnectionSpec>::Manager,
    ) -> Self {
        Self {
            cno,
            stream,
            remote,
            self_session_id: config.unwrap().self_session_id,
        }
    }

    fn manager(&self) -> &<Self::Spec as ConnectionSpec>::Manager {
        todo!()
    }

    fn cno(&self) -> ConnectionNo {
        todo!()
    }

    async fn handshake(self) -> Self::HandshakeResult {
        let Self {
            cno,
            stream,
            remote,
            self_session_id,
        } = self;

        let mut framed = Framed::new(stream, AtomCodec::default());
        let magic = framed.try_next().await?.ok_or_else(|| {
            HandshakeError::Io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Stream closed before receiving MAGIC",
            ))
        })?;

        let magic = MagicInfo::try_from(magic)?;
        if magic.ip_mode != libpeercast_re::model::IpMode::IpV4 {
            return Err(HandshakeError::UnsupportedIpMode(magic.ip_mode));
        }

        // PCP_HELO
        let helo = framed.try_next().await?.ok_or_else(|| {
            HandshakeError::Io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Stream closed before receiving HELO",
            ))
        })?;
        let helo = HeloInfo::try_from(helo)?;
        dbg!("Received HELO: {:?}", &helo);

        // HELO must contain session_id
        let Some(remote_session_id) = helo.session_id else {
            return Err(HandshakeError::MissingField("session_id".to_string()));
        };

        // Check port accessibility if ping port is provided
        let mut remote_accessable_port = None;
        if let Some(ping_port) = helo.ping {
            // TODO: PortCheckをキャッシュする
            let r = PortCheckerService::new(self_session_id, std::time::Duration::from_secs(5))
                .check_with_session_id(remote.ip(), ping_port, remote_session_id)
                .await;
            if matches!(r, Ok(true)) {
                remote_accessable_port = Some(ping_port);
            }
        }

        // Respond with PCP_OLEH
        let oleh = OlehBuilder::new(self_session_id, remote.ip(), remote_accessable_port.unwrap_or(0)).build();

        Ok(Handshaked::Pcp(RootConnection {
            framed,
        }))
    }
}

#[derive(Debug)]
pub struct RootHandshakeConfig {
    pub self_session_id: libpeercast_re::GnuId,
    pub shutdown: tokio_util::sync::CancellationToken,
}

#[derive(Debug, thiserror::Error)]
pub enum HandshakeError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Timeout")]
    Timeout,

    #[error("Failed to parse magic info: {0}")]
    ParseError(#[from] libpeercast_re::atom::error::AtomCodecError),

    #[error("Invalid magic info: {0}")]
    InfoParseError(#[from] libpeercast_re::pcp::builder::error::InfoParseError),

    #[error("Unsupported IP mode: {0:?}")]
    UnsupportedIpMode(libpeercast_re::model::IpMode),

    #[error("Missing field in Atom: {0}")]
    MissingField(String),
}

pub enum Handshaked {
    Pcp(RootConnection),
}

#[derive(Debug)]
pub struct RootConnection {
    framed: Framed<IoStream, AtomCodec>,
}
impl Connection for RootConnection {
    type Spec = RootConnectionSpec;

    fn cno(&self) -> ConnectionNo {
        todo!()
    }

    fn remote(&self) -> std::net::SocketAddr {
        todo!()
    }

    fn handle(&self) -> <Self::Spec as ConnectionSpec>::Handle {
        todo!()
    }

    fn run(self) -> impl Future<Output = Result<(), libpeercast_re::connection::error::ConnectionError>> {
        async { Ok(()) }
    }
}
#[derive(Debug, Clone)]
pub struct RootHandle {}

impl ConnectionHandle for RootHandle {
    type Spec = RootConnectionSpec;

    fn cno(&self) -> ConnectionNo {
        todo!()
    }

    fn manager(&self) -> &<Self::Spec as ConnectionSpec>::Manager {
        todo!()
    }

    fn task_name(&self) -> std::sync::Arc<String> {
        todo!()
    }

    fn state(&self) -> <Self::Spec as ConnectionSpec>::State {
        todo!()
    }

    fn stats(&self) -> <Self::Spec as ConnectionSpec>::Stats {
        todo!()
    }

    fn shutdown(&self) -> () {
        todo!()
    }
}

pub struct RootState {}
pub struct RootStats {}

pub struct RootConn {}
