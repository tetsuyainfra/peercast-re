mod api;
mod http_svc;
mod middleware;

use std::{
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    sync::{Arc, Mutex},
};

use axum::extract::{connect_info::Connected, ConnectInfo};
use axum_core::response::IntoResponse;
use bytes::Bytes;
use http::StatusCode;
pub use http_svc::HttpSvc;
use hyper_util::rt::TokioIo;
use ipnet::IpNet;
use pbkdf2::password_hash::errors::B64Error;
use sync_wrapper::SyncWrapper;
use tokio::{net::TcpStream, sync::mpsc};

use crate::{
    config::Config,
    pcp::{ChannelManager, GnuId},
    rtmp::stream_manager::StreamManagerMessage,
    util::Shutdown,
    ConnectionId,
};

pub use api::Api;

pub(crate) type ShutdownAndNotifySet = (Shutdown, tokio::sync::mpsc::Sender<()>);

#[derive(Debug)]
// pub struct MyIncomingStream<'a> {
pub struct MyIncomingStream {
    pub connection_id: ConnectionId,
    // pub tcp_stream: &'a TokioIo<TcpStream>,
    pub remote_addr: SocketAddr,
    pub(crate) shutdown: Arc<Mutex<Option<ShutdownAndNotifySet>>>,
}

// impl MyIncomingStream<'_> {
impl MyIncomingStream {
    /// Returns the local address that this stream is bound to.
    pub fn local_addr(&self) -> std::io::Result<SocketAddr> {
        // self.tcp_stream.inner().local_addr()
        todo!()
    }

    /// Returns the remote address that this stream is bound to.
    pub fn remote_addr(&self) -> SocketAddr {
        self.remote_addr
    }
}

#[derive(Clone, Debug)]
pub struct MyConnectInfo {
    // pub local: SocketAddr,
    pub remote: SocketAddr,
    pub connection_id: ConnectionId,
    // 長い通信があり、シャットダウンを綺麗にしたいならの変数を取得する
    pub(crate) shutdown: Arc<Mutex<Option<ShutdownAndNotifySet>>>,
}

/*
// impl Connected<MyIncomingStream<'_>> for MyConnectInfo {
//     fn connect_info(mut target: MyIncomingStream<'_>) -> Self {
impl Connected<MyIncomingStream> for MyConnectInfo {
    fn connect_info(mut target: MyIncomingStream) -> Self {
        MyConnectInfo {
            local: target.local_addr().unwrap(),
            remote: target.remote_addr(),
            connection_id: target.connection_id,
            shutdown: target.shutdown.clone(),
        }
    }
} */

impl Connected<MyConnectInfo> for MyConnectInfo {
    fn connect_info(target: MyConnectInfo) -> Self {
        target
    }
}

////////////////////////////////////////////////////////////////////////////////
// AppStatej
//
#[derive(Clone)]
pub(self) struct AppState {
    config_path: PathBuf,
    config: Config,
    //
    session_id: GnuId,
    channel_manager: Arc<ChannelManager>,
    //
    manager_sender: Arc<mpsc::UnboundedSender<StreamManagerMessage>>,

    #[cfg(debug_assertions)]
    proxy_mode: UiProxyMode,
}

////////////////////////////////////////////////////////////////////////////////
// Check Ip
//
// fn is_ip_internal(remote_addr: &SocketAddr, permit_address: &Vec<IpNet>) -> bool {
//     let remote_ip = &remote_addr.ip();
//     let hit_net = permit_address
//         .iter()
//         .find(|ipnet| ipnet.contains(remote_ip));
//     match hit_net {
//         Some(_) => {
//             tracing::trace!("ip IS local");
//             true
//         }
//         None => {
//             tracing::trace!("ip IS NOT local");
//             false
//         }
//     }
// }

////////////////////////////////////////////////////////////////////////////////
// Proxy
//
#[cfg(debug_assertions)]
#[derive(Debug, Clone)]
pub(self) enum UiProxyMode {
    Embed,
    Proxy,
}
