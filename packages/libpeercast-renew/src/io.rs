use std::{
    pin::Pin,
    task::{Context, Poll},
};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

pub trait Io: AsyncRead + AsyncWrite + Shutdownable + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Shutdownable + Unpin + Send> Io for T {}

////////////////////////////////////////////////////////////////////////////////
// Shutdownable
//
#[async_trait::async_trait]
pub trait Shutdownable {
    async fn shutdown(&mut self) -> ();
}

#[async_trait::async_trait]
impl Shutdownable for tokio::net::TcpStream {
    async fn shutdown(&mut self) -> () {
        tokio::net::TcpStream::shutdown(self).await;
    }
}

#[async_trait::async_trait]
impl Shutdownable for tokio::io::DuplexStream {
    async fn shutdown(&mut self) -> () {
        tokio::io::DuplexStream::shutdown(self).await;
    }
}

////////////////////////////////////////////////////////////////////////////////
// IoStream
//

/// TcpStreamとDuplexStreamを一つの型に閉じ込める
/// ConnectionのSpec::Ioで簡単に使えるようにするためのラッパー
#[derive(Debug)]
pub enum IoStream {
    Tcp(tokio::net::TcpStream),
    Duplex(tokio::io::DuplexStream),
}

impl From<tokio::net::TcpStream> for IoStream {
    fn from(s: tokio::net::TcpStream) -> Self {
        IoStream::Tcp(s)
    }
}

impl From<tokio::io::DuplexStream> for IoStream {
    fn from(s: tokio::io::DuplexStream) -> Self {
        IoStream::Duplex(s)
    }
}

impl AsyncRead for IoStream {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            IoStream::Tcp(s) => Pin::new(s).poll_read(cx, buf),
            IoStream::Duplex(s) => Pin::new(s).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for IoStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<std::io::Result<usize>> {
        match self.get_mut() {
            IoStream::Tcp(s) => Pin::new(s).poll_write(cx, buf),
            IoStream::Duplex(s) => Pin::new(s).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            IoStream::Tcp(s) => Pin::new(s).poll_flush(cx),
            IoStream::Duplex(s) => Pin::new(s).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            IoStream::Tcp(s) => Pin::new(s).poll_shutdown(cx),
            IoStream::Duplex(s) => Pin::new(s).poll_shutdown(cx),
        }
    }
}

#[async_trait::async_trait]
impl Shutdownable for IoStream {
    async fn shutdown(&mut self) -> () {
        match self {
            IoStream::Tcp(s) => s.shutdown().await,
            IoStream::Duplex(s) => s.shutdown().await,
        }
    }
}

#[cfg(test)]
mod t {
    use crate::{
        io::{Io, IoStream},
        test_helper::{assert_send, assert_sync},
    };

    fn test_io() {
        fn assert_io<T: Io>() {}

        assert_io::<IoStream>();
        assert_sync::<IoStream>();
        assert_send::<IoStream>();
    }

    #[test]
    fn test() {
        test_io();
    }
}
