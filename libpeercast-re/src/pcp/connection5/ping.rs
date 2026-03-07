use std::{future::Future, io::Error, net::TcpStream, process::id, time::Duration};

use bytes::BytesMut;
use futures_util::{SinkExt, StreamExt};
use nom::combinator::Opt;
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_util::codec::Framed;
use uuid::Bytes;

use crate::{
    error::ConnectionError,
    pcp::{
        atom2::parser::ParseError,
        builder2::{InfoParseError, OlehInfo, PingBuilder2, QuitBuilder2, QuitInfo, QuitReason},
        connection5::{Connection, ConnectionSpec, OutgoingConnection},
        Atom2, AtomCodec, AtomView, GnuId, Id4,
    },
    ConnectionNo,
};

#[derive(Debug, Error)]
pub enum PingError {
    #[error("atom parse error")]
    ParseError(#[from] ParseError),

    #[error("atom to *Info parse error")]
    InfoParseError(#[from] InfoParseError),

    #[error("timeout")]
    Timeout(#[from] tokio::time::error::Elapsed),

    #[error("io error")]
    Io(#[from] std::io::Error),
}

pub struct PingConfig {
    timeout_msec: u64,
    self_session_id: GnuId,
}

impl Default for PingConfig {
    fn default() -> Self {
        Self {
            timeout_msec: 5000,
            self_session_id: GnuId::new(),
        }
    }
}

pub struct Ping<S> {
    cno: ConnectionNo,
    remote: std::net::SocketAddr,
    config: PingConfig,
    _marker: std::marker::PhantomData<S>,
}

#[async_trait::async_trait]
impl<S: ConnectionSpec> Connection for Ping<S> {
    type Spec = S;

    fn cno(&self) -> crate::ConnectionNo {
        self.cno
    }

    fn handle(&self) -> <Self::Spec as super::ConnectionSpec>::Handle {
        todo!()
    }

    fn run(self) -> impl Future<Output = Result<(), ConnectionError>> {
        async { unimplemented!("DONT USE THIS") }
    }
}

impl<S: ConnectionSpec> OutgoingConnection for Ping<S> {
    type Config = PingConfig;
    type Output = Result<GnuId, PingError>;

    fn new(
        cno: crate::ConnectionNo,
        remote: std::net::SocketAddr,
        config: Option<Self::Config>,
        manager: <Self::Spec as super::ConnectionSpec>::Manager,
    ) -> Self {
        Self {
            cno,
            remote,
            config: config.unwrap_or_default(),
            _marker: std::marker::PhantomData,
        }
    }

    /// 成功したら接続先のSessionIDを返す
    fn connect(self) -> impl Future<Output = Self::Output> {
        async move {
            let Self {
                cno,
                remote,
                config,
                _marker,
            } = self;

            let mut conn = tokio::time::timeout(
                Duration::from_millis(config.timeout_msec),
                tokio::net::TcpStream::connect(remote),
            )
            .await??;

            let mut framed = Framed::new(conn, AtomCodec::new());

            let ping_atoms = PingBuilder2::new(config.self_session_id).port(Some(7144)).build();
            for a in ping_atoms {
                let _ = framed.send(a).await?;
            }

            let oleh_atom = get_atom_timeout(Duration::from_millis(config.timeout_msec), &mut framed).await?;
            dbg!(&oleh_atom);
            let oleh = OlehInfo::try_from(&oleh_atom)?;
            dbg!(&oleh);

            let quit_atom = QuitBuilder2::new(QuitReason::UserShutdown).build();
            let _ = framed.send(quit_atom).await?;

            // MEMO: 相手からQuitは帰って来ない場合があるので・・・取得しないことにする
            // let quit_atom = get_atom_timeout(Duration::from_millis(config.timeout_msec), &mut framed).await?;
            // dbg!(&quit_atom);
            // let recv_quit = QuitInfo::try_from(&quit_atom)?;
            // dbg!(recv_quit);

            // drop(framed);

            Ok(oleh.session_id)
        }
    }
}

async fn get_atom_timeout(
    dur: Duration,
    framed: &mut Framed<tokio::net::TcpStream, AtomCodec>,
) -> Result<Atom2, PingError> {
    let Some(ret_atom) = tokio::time::timeout(dur, framed.next()).await? else {
        // Noneが帰って来た時のエラー
        return Err(Error::new(std::io::ErrorKind::UnexpectedEof, "remote don't send atom").into());
    };
    let atom = ret_atom?;

    Ok(atom)
}

#[cfg(test)]
mod t {
    use std::{net::SocketAddr, pin::Pin};

    use tokio_util::sync::CancellationToken;

    use crate::{
        pcp::connection5::{
            ping::Ping,
            shared::{self, SharedManager},
            ConnectionFactory, ConnectionHandle, ConnectionSpec, HandshakeConnection, OutgoingConnection,
        },
        ConnectionNo,
    };

    #[derive(Debug)]
    pub struct MySpec {}
    impl ConnectionSpec for MySpec {
        type Handshake = ();

        type HandshakeConfig = ();

        type State = ();

        type Stats = ();

        type Handle = ();
        type Manager = SharedManager<Self>;
    }

    #[async_trait::async_trait]
    impl HandshakeConnection for () {
        type Spec = MySpec;

        type HandshakeResult = ();

        fn new(
            cno: ConnectionNo,
            remote: SocketAddr,
            shutdown_token: Option<CancellationToken>,
            config: Option<<Self::Spec as ConnectionSpec>::HandshakeConfig>,
            manager: <Self::Spec as ConnectionSpec>::Manager,
        ) -> Self {
            todo!()
        }

        fn manager(&self) -> &<Self::Spec as ConnectionSpec>::Manager {
            todo!()
        }

        fn cno(&self) -> ConnectionNo {
            todo!()
        }

        async fn handshake(self) -> Self::HandshakeResult {
            todo!()
        }
    }

    impl ConnectionHandle for () {
        type Spec = MySpec;

        fn cno(&self) -> crate::ConnectionNo {
            todo!()
        }

        fn task_name(&self) -> std::borrow::Cow<'static, str> {
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

    #[tokio::test]
    async fn t() {
        let (factory, manager) = shared::connection_factory::<MySpec>();
        // let conn = factory.create_outgoing::<Ping<MySpec>>("127.0.0.1:7144".parse().unwrap(), None);
        let conn = factory.create_outgoing::<Ping<MySpec>>("10.10.10.11:61744".parse().unwrap(), None);
        // let conn = factory.create_outgoing::<Ping<MySpec>>("10.10.10.11:61745".parse().unwrap(), None);

        let r = conn.connect().await;
        dbg!(r);
    }
}
