use std::{fmt::Debug, net::SocketAddr, sync::Arc};

use futures_util::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use libpeercast_re::{
    ConnectionNo,
    io::IoStream,
    pcp::{
        Atom2, AtomCodec, AtomMut, AtomView,
        builder2::{QuitBuilder2, QuitReason},
        connection5::{
            Connection, ConnectionHandle, ConnectionSpec, HandshakeConnection,
            shared::{SharedFactory, SharedManager},
        },
        procedure::handshake::{HandshakeProtocol, IncomingProtocolDiscriminator, Parts},
    },
    util::util_mpsc::mpsc_send,
};
use tokio::sync::{mpsc, watch};
use tokio_util::codec::{Framed, FramedParts};
use tracing::{error, info};

pub type RootConnectionManager = SharedManager<RootSpec>;
pub type RootConnectionFactory = SharedFactory<RootSpec>;

#[derive(Debug)]
pub struct RootSpec();

impl ConnectionSpec for RootSpec {
    type Handshake = RootHandshake;
    type HandshakeConfig = Config;

    type State = State;

    type Stats = Stats;

    type Handle = RootHandle;

    type Manager = SharedManager<Self>;
}

////////////////////////////////////////////////////////////////////////////////
//  RootHandshake
//
pub struct Config {
    io: IoStream,
}
pub struct RootHandshake {
    cno: ConnectionNo,
    remote: SocketAddr,
    stream: IoStream,
    shutdown_token: tokio_util::sync::CancellationToken,
    manager: SharedManager<RootSpec>,
}

impl RootHandshake {}

#[async_trait::async_trait]
impl HandshakeConnection for RootHandshake {
    type Spec = RootSpec;
    type HandshakeResult = Result<RootHandshakeResult, libpeercast_re::error::HandshakeError>;

    fn new(
        cno: ConnectionNo,
        remote: SocketAddr,
        shutdown_token: Option<tokio_util::sync::CancellationToken>,
        config: Option<<Self::Spec as ConnectionSpec>::HandshakeConfig>,
        manager: <Self::Spec as ConnectionSpec>::Manager,
    ) -> Self {
        let Config {
            io,
        } = config.unwrap();

        Self {
            cno,
            remote,
            stream: io,
            shutdown_token: shutdown_token.unwrap_or_else(|| tokio_util::sync::CancellationToken::new()),
            manager,
        }
    }

    fn manager(&self) -> &<Self::Spec as ConnectionSpec>::Manager {
        &self.manager
    }

    fn cno(&self) -> ConnectionNo {
        self.cno
    }

    async fn handshake(self) -> Self::HandshakeResult {
        let Self {
            cno,
            remote,
            stream,
            shutdown_token,
            manager: _,
        } = self;

        let discrimer = IncomingProtocolDiscriminator::new();

        let handshake_protocol = discrimer.identify(cno, stream, remote).await;
        let Ok(protocol) = handshake_protocol else {
            return Err(libpeercast_re::error::HandshakeError::Failed);
        };
        match protocol {
            HandshakeProtocol::Pcp(incoming_pcp_handshake) => {
                let Parts::<IoStream> {
                    cno,
                    remote,
                    stream,
                    read_buf,
                    write_buf,
                } = incoming_pcp_handshake.into_parts();
                let establishd = RootEstablished::new(cno, remote, stream, read_buf, write_buf, shutdown_token);

                Ok(RootHandshakeResult::Pcp(establishd))
            }
            HandshakeProtocol::HttpPcp(_incoming_http_pcp_handshake) => {
                error!("Not Implementing HTTP PCP PLEASE HACK ME");
                todo!()
            }
            HandshakeProtocol::Http(incoming_http_handshake) => {
                let _ = incoming_http_handshake.shutdown().await;
                return Ok(RootHandshakeResult::Http);
            }
            HandshakeProtocol::Unknown(incoming_unknown_handshake) => {
                let _ = incoming_unknown_handshake.shutdown().await;
                return Ok(RootHandshakeResult::Unknown);
            }
        }
    }
}

pub enum RootHandshakeResult {
    Pcp(RootEstablished),
    PcpHttp,
    Http,
    Unknown,
}

////////////////////////////////////////////////////////////////////////////////
//  RootEstablish
//
pub struct RootEstablished {
    cno: ConnectionNo,
    #[allow(dead_code)]
    remote: SocketAddr,
    stream: IoStream,
    read_buf: bytes::BytesMut,
    write_buf: bytes::BytesMut,

    shutdown_token: tokio_util::sync::CancellationToken,
    //
    state_tx: watch::Sender<State>,
    // state_rx: watch::Receiver<State>,
    //
    #[allow(dead_code)]
    stats_tx: watch::Sender<Stats>,
    #[allow(dead_code)]
    stats_rx: watch::Receiver<Stats>,
    //
    #[allow(dead_code)]
    command_tx: mpsc::UnboundedSender<Command>,
    #[allow(dead_code)]
    command_rx: mpsc::UnboundedReceiver<Command>,
    //
    handle_inner: Arc<RHandleInner>,
}

impl RootEstablished {
    fn new(
        cno: ConnectionNo,
        remote: SocketAddr,
        stream: IoStream,
        read_buf: bytes::BytesMut,
        write_buf: bytes::BytesMut,
        shutdown_token: tokio_util::sync::CancellationToken,
    ) -> Self {
        let (state_tx, state_rx) = watch::channel(State::Init);
        let (stats_tx, stats_rx) = watch::channel(Stats {});
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let handle_inner = Arc::new(RHandleInner {
            cno,
            task_name: Arc::new(format!("Conn-{}", cno)),
            state_rx: state_rx,
            stats_rx: stats_rx.clone(),
            command_tx: command_tx.clone(),
        });

        Self {
            cno,
            remote,
            stream,
            read_buf,
            write_buf,
            shutdown_token,
            //
            state_tx,
            // state_rx,
            //
            stats_tx,
            stats_rx,
            //
            command_tx,
            command_rx,
            //
            handle_inner,
        }
    }

    fn handle(&self) -> RootHandle {
        RootHandle(Arc::clone(&self.handle_inner))
    }
}

impl Connection for RootEstablished {
    type Spec = RootSpec;

    fn cno(&self) -> ConnectionNo {
        self.cno
    }

    fn handle(&self) -> <Self::Spec as ConnectionSpec>::Handle {
        self.handle()
    }

    fn run(self) -> impl Future<Output = Result<(), libpeercast_re::error::ConnectionError>> {
        // MEMO 本当はコード分ける必要ないんだけど、Someとかの変換考えるのに分けて書いた。整理が終わったら統合しても良い
        let task = ConnectionTask::new(self);
        task.run()
    }
}

struct ConnectionTask {
    stream: Option<IoStream>,
    read_buf: Option<bytes::BytesMut>,
    write_buf: Option<bytes::BytesMut>,
    shutdown_token: tokio_util::sync::CancellationToken,
    //
    state_tx: Option<watch::Sender<State>>,
    // state_rx: watch::Receiver<State>,
    //
    handle_inner: Arc<RHandleInner>,
    // R/W thread
    reader_tx: Option<mpsc::UnboundedSender<Atom2>>,
    reader_rx: Option<mpsc::UnboundedReceiver<Atom2>>,
    writer_tx: Option<mpsc::UnboundedSender<AtomMut>>,
    writer_rx: Option<mpsc::UnboundedReceiver<AtomMut>>,
}

impl ConnectionTask {
    fn new(established: RootEstablished) -> Self {
        let (reader_tx, reader_rx) = mpsc::unbounded_channel();
        let (writer_tx, writer_rx) = mpsc::unbounded_channel();
        Self {
            stream: Some(established.stream),
            read_buf: Some(established.read_buf),
            write_buf: Some(established.write_buf),
            shutdown_token: established.shutdown_token,
            //
            state_tx: Some(established.state_tx),
            // state_rx: established.state_rx,
            //
            handle_inner: established.handle_inner,
            // R/W thread
            reader_tx: Some(reader_tx),
            reader_rx: Some(reader_rx),
            writer_tx: Some(writer_tx),
            writer_rx: Some(writer_rx),
        }
    }

    async fn run(mut self) -> Result<(), libpeercast_re::error::ConnectionError> {
        let mut now_state = self.state_tx.as_ref().unwrap().borrow().clone();
        loop {
            let new_state = match now_state {
                State::Init => self.on_init().await,
                State::Running => self.on_running().await,
                State::ShuttingDown => self.on_shutting_down().await,
                State::Draining => self.on_draining().await,
                State::Closed => {
                    self.on_closed().await;
                    break;
                }
            };
            if now_state != new_state {
                if let Some(state_tx) = self.state_tx.as_mut() {
                    let _ = state_tx.send(new_state);
                }
                now_state = new_state;
            }
        }

        Ok(())
    }

    async fn on_init(&mut self) -> State {
        // ストリームの初期化
        let stream = self.stream.take().unwrap();
        let mut parts = FramedParts::new::<Atom2>(stream, AtomCodec::new());
        parts.read_buf = self.read_buf.take().unwrap();
        parts.write_buf = self.write_buf.take().unwrap();
        let framed = Framed::from_parts(parts);

        // 認証の実行
        let is_authenticate_success = false;

        if !is_authenticate_success {
            return State::ShuttingDown;
        }

        // preapre to create thread
        let (framed_writer, framed_reader): (
            SplitSink<Framed<IoStream, AtomCodec>, AtomMut>,
            SplitStream<Framed<IoStream, AtomCodec>>,
        ) = framed.split();

        // create Read Thread
        let reader_tx = self.reader_tx.take().unwrap();
        let task_name = format!("{}-R", self.handle_inner.task_name);
        let _ = tokio::task::Builder::new().name(&task_name.clone()).spawn(async move {
            let span = tracing::info_span!("{}", task_name);
            let _enter = span.enter();
            read_loop(framed_reader, reader_tx).await
        });

        // create Write Thread
        let writer_rx = self.writer_rx.take().unwrap();
        let task_name = format!("{}-W", self.handle_inner.task_name);
        let _ = tokio::task::Builder::new().name(&task_name.clone()).spawn(async move {
            let span = tracing::info_span!("{}", task_name);
            let _enter = span.enter();
            write_loop(framed_writer, writer_rx).await
        });

        assert!(self.stream.is_none());
        assert!(self.read_buf.is_none());
        assert!(self.write_buf.is_none());
        State::Running
    }

    async fn on_running(&mut self) -> State {
        let Some(ref mut reader_rx) = self.reader_rx.as_mut() else {
            return State::ShuttingDown;
        };

        tokio::select! {
            atom = reader_rx.recv() => {
                let Some(atom) = atom else {
                    return State::ShuttingDown;
                };
                self.on_atom(atom).await;

                return State::Running
            }
            _ = self.shutdown_token.cancelled() => {
                return State::ShuttingDown;
            }
        }
    }

    async fn on_shutting_down(&mut self) -> State {
        // send quit
        if let Some(writer_tx) = self.writer_tx.take() {
            let quit: AtomMut = QuitBuilder2::new(QuitReason::Any).build();
            let _r = writer_tx.send(quit);
        }
        let _drop = self.reader_rx.take();

        State::Draining
    }

    async fn on_draining(&mut self) -> State {
        State::Closed
    }

    async fn on_closed(&mut self) -> State {
        let _drop = self.state_tx.take();

        // check
        assert!(self.state_tx.is_none());
        assert!(self.reader_tx.is_none());
        assert!(self.reader_rx.is_none());
        assert!(self.writer_tx.is_none());
        assert!(self.writer_rx.is_none());

        State::Closed
    }

    // HERE TO PROCEDURE
    async fn on_atom(&mut self, atom: Atom2) {
        let _id = atom.id();
    }
}

////////////////////////////////////////////////////////////////////////////////
//  ConnectionTask -> spawn(Read)
//
async fn read_loop(
    //
    mut reader: SplitStream<Framed<IoStream, AtomCodec>>,
    sender: mpsc::UnboundedSender<Atom2>,
) {
    info!("read_loop start");
    loop {
        match reader.next().await {
            Some(Ok(atom)) => {
                if false == mpsc_send(&sender, atom) {
                    info!("Failed to send message to main task");
                    break;
                };
            }
            Some(Err(err)) => {
                error!("Error: {}", err);
                break;
            }
            None => break,
        };
    }
    info!("read_loop finish");
}

////////////////////////////////////////////////////////////////////////////////
//  ConnectionTask -> spawn(Write)
//
async fn write_loop(
    //
    mut writer: SplitSink<Framed<IoStream, AtomCodec>, AtomMut>,
    mut reciever: mpsc::UnboundedReceiver<AtomMut>,
) {
    info!("write_loop start");
    loop {
        match reciever.recv().await {
            Some(atom) => {
                let _ = writer.send(atom).await;
            }
            None => break,
        }
    }
    info!("write_loop finish");
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Shutdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Init,
    Running,
    ShuttingDown,
    Draining,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stats {}

////////////////////////////////////////////////////////////////////////////////
//  RootHandle
//
#[derive(Debug)]
pub struct RootHandle(Arc<RHandleInner>);

impl Clone for RootHandle {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl ConnectionHandle for RootHandle {
    type Spec = RootSpec;

    fn cno(&self) -> ConnectionNo {
        self.0.cno
    }

    fn task_name(&self) -> std::borrow::Cow<'static, str> {
        todo!()
    }

    fn state(&self) -> <Self::Spec as ConnectionSpec>::State {
        self.0.state_rx.borrow().clone()
    }

    fn stats(&self) -> <Self::Spec as ConnectionSpec>::Stats {
        self.0.stats_rx.borrow().clone()
    }

    fn shutdown(&self) -> () {
        let _ = self.0.command_tx.send(Command::Shutdown);
    }
}

#[derive(Debug)]
struct RHandleInner {
    cno: ConnectionNo,
    task_name: Arc<String>,
    state_rx: watch::Receiver<State>,
    stats_rx: watch::Receiver<Stats>,
    command_tx: mpsc::UnboundedSender<Command>,
}

/*
どちらがいいかな・・・
    fn run(self) -> impl Future<Output = Result<(), libpeercast_re::error::ConnectionError>> {
        RootConnectionTask::new(self)
    }
struct RootConnectionTask {
    inner: RootEstablished,
}

impl RootConnectionTask {
    fn new(inner: RootEstablished) -> Self {
        Self {
            inner,
        }
    }
}

impl Future for RootConnectionTask {
    type Output = Result<(), libpeercast_re::error::ConnectionError>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        todo!()
    }
}
    */

// MEMO: Framedをpartsから作る方法もある
// let mut parts = FramedParts::new::<Atom2>(stream, AtomCodec::new()); // Encoderのanotationを与える必要がある
// parts.read_buf = read_buf;    // <-- バッファーを外部から与えられる
// parts.write_buf = self.write_buf.take().unwrap();
// let framed = Framed::from_parts(parts);

// MEMO: Framedをsplitする方法もある
// let framed = Framed::new(stream, AtomCodec::new());
// let (mut tx, tr): (
//     futures_util::stream::SplitSink<Framed<IoStream, AtomCodec>, AtomMut>,
//     futures_util::stream::SplitStream<Framed<IoStream, AtomCodec>>,
// ) = framed.split();
// let item: AtomMut = (Id4::PCP_ATOM, 1_u8).into();
// let x = tx.send(item).await;

// MEMO: こうすれば、増え続けるrecieverを処理できる (つまりRootサーバーでは使わない)
// async fn new_receiver_future(mut receiver: UnboundedReceiver<StreamManagerMessage>) -> FutureResult {
//     let result = receiver.recv().await;
//     FutureResult::MessageReceived {
//         receiver,
//         message: result,
//     }
// }
// select_allはFutureのリストを処理して、最初にreadyになったfutureの値とindexを返す(loop内 futures.await)
// https://docs.rs/futures/latest/futures/future/fn.select_all.html
// let mut futures = select_all(vec![new_receiver_future(receiver).boxed()]);
// select_allはtrait Futureを実装してるのでawaitできて・・・
// let (result, _index, remaining_futures) = futures.await;
// let mut new_futures = Vec::from(remaining_futures);
