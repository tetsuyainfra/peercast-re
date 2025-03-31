use std::{collections::VecDeque, fmt::Debug, net::SocketAddr, time::Instant};

use bytes::BytesMut;
use tokio::{
    io::{AsyncRead, AsyncWriteExt, ReadHalf, WriteHalf},
    net::TcpStream,
};

use crate::{
    pcp::{atom, Atom, GnuId},
    ConnectionId,
};

////////////////////////////////////////////////////////////////////////////////
///  Inner
///
#[derive(Debug)]
pub(super) struct Inner {
    connection_id: ConnectionId,
    self_session_id: GnuId,
    stream: TcpStream,
    remote: SocketAddr,
    read_buf: BytesMut,
    //
    read_counts: VecDeque<(Instant, u64)>,
    write_counts: VecDeque<(Instant, u64)>,
}

impl Inner {
    pub(super) fn new(
        connection_id: ConnectionId,
        self_session_id: GnuId,
        stream: TcpStream,
        remote: SocketAddr,
        read_buf: Option<BytesMut>,
    ) -> Self {
        Self {
            connection_id,
            self_session_id,
            stream: stream,
            remote,
            read_buf: read_buf.unwrap_or_else(|| BytesMut::with_capacity(4096)),
            read_counts: Default::default(),
            write_counts: Default::default(),
        }
    }

    #[inline]
    pub(super) fn connection_id(&self) -> ConnectionId {
        self.connection_id
    }

    #[inline]
    pub(super) fn remote_addr(&self) -> &SocketAddr {
        &self.remote
    }

    #[inline]
    pub(super) fn self_session_id(&self) -> &GnuId {
        &self.self_session_id
    }

    pub(super) async fn shutdown(&mut self) -> Result<(), std::io::Error> {
        self.stream.shutdown().await
    }

    #[ignore = "いらないハズ"]
    #[inline]
    pub(super) async fn peek(&self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        self.stream.peek(buf).await
    }

    #[inline]
    pub(super) async fn read_atom(&mut self) -> Result<Atom, std::io::Error> {
        atom::read_atom(&mut self.stream, &mut self.read_buf).await
    }

    #[inline]
    pub(super) async fn write_atom(&mut self, atom: Atom) -> Result<(), std::io::Error> {
        atom.write_stream(&mut self.stream).await
    }

    #[inline]
    pub(super) async fn write_atoms(
        &mut self,
        atoms: &mut VecDeque<Atom>,
    ) -> Result<(), std::io::Error> {
        while let Some(atom) = atoms.pop_front() {
            atom.write_stream(&mut self.stream).await?
        }
        Ok(())
    }

    pub(super) fn split(self) -> (ReadHalfInner, WriteHalfInner) {
        let Self {
            connection_id,
            self_session_id,
            stream,
            remote,
            read_buf,
            read_counts,
            write_counts,
        } = self;

        let (read_half, write_half) = tokio::io::split(stream);
        let read_half = ReadHalfInner::new(connection_id.clone(), read_half, read_buf, read_counts);
        let write_half = WriteHalfInner::new(connection_id, write_half, write_counts);

        (read_half, write_half)
    }
}

////////////////////////////////////////////////////////////////////////////////
//  ReadHalfInner
//

pub struct ReadHalfInner {
    connection_id: ConnectionId,
    stream: ReadHalf<TcpStream>,
    read_buf: BytesMut,
    //
    read_counts: VecDeque<(Instant, u64)>,
}

impl ReadHalfInner {
    pub fn new(
        connection_id: ConnectionId,
        stream: ReadHalf<TcpStream>,
        read_buf: BytesMut,
        read_counts: VecDeque<(Instant, u64)>,
    ) -> Self {
        Self {
            connection_id,
            stream,
            read_buf,
            read_counts,
        }
    }

    #[inline]
    pub fn connection_id(&self) -> ConnectionId {
        self.connection_id
    }

    #[inline]
    pub async fn read_atom(&mut self) -> Result<Atom, std::io::Error> {
        atom::read_atom(&mut self.stream, &mut self.read_buf).await
    }
}

////////////////////////////////////////////////////////////////////////////////
//  ReadWriteInner
//
//

pub struct WriteHalfInner {
    connection_id: ConnectionId,
    stream: WriteHalf<TcpStream>,
    //
    write_counts: VecDeque<(Instant, u64)>,
}

impl WriteHalfInner {
    pub fn new(
        connection_id: ConnectionId,
        stream: WriteHalf<TcpStream>,
        write_counts: VecDeque<(Instant, u64)>,
    ) -> Self {
        Self {
            connection_id,
            stream,
            write_counts,
        }
    }

    #[inline]
    pub fn connection_id(&self) -> ConnectionId {
        self.connection_id
    }

    #[inline]
    pub async fn write_atom(&mut self, atom: Atom) -> Result<(), std::io::Error> {
        atom.write_stream(&mut self.stream).await
    }

    #[inline]
    pub(super) async fn write_atoms(
        &mut self,
        atoms: &mut VecDeque<Atom>,
    ) -> Result<(), std::io::Error> {
        while let Some(atom) = atoms.pop_front() {
            atom.write_stream(&mut self.stream).await?
        }
        Ok(())
    }
}
