use tokio::io::{AsyncRead, AsyncWrite, DuplexStream};
use tokio_util::codec::Framed;

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
        self.shutdown().await;
    }
}

#[async_trait::async_trait]
impl Shutdownable for DuplexStream {
    async fn shutdown(&mut self) -> () {
        self.shutdown().await;
    }
}
