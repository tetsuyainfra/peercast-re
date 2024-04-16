use std::{net::SocketAddr, path::PathBuf};

use futures_util::{task::SpawnExt, Future};
use tracing::info;

use crate::{
    config::Config,
    pcp::{ChannelInfo, ChannelManager, ChannelType, GnuId, RelayTaskConfig, SourceTaskConfig},
    ConnectionId,
};

pub struct CuiDL {}

impl CuiDL {
    pub async fn run(
        config_path: PathBuf,
        config: Config,
        shutdown: impl Future,
        channel_id: GnuId,
        connect_addr: SocketAddr,
    ) {
        let session_id = GnuId::new();
        let ch_manager = ChannelManager::new(&session_id);

        let ch_type = ChannelType::Relay; //("localhost:7144".parse().unwrap());
        let ch = ch_manager.create_or_get(channel_id, ch_type, None, None);

        let task_config = SourceTaskConfig::Relay(RelayTaskConfig {
            addr: connect_addr,
            self_addr: None,
        });
        let _r = ch.connect(ConnectionId::new(), task_config);

        // Connectingになるのを待つ

        let mut reciever = ch.channel_reciever(ConnectionId::new());

        let recieve_fut = async {
            loop {
                let data = reciever.recv().await;
                match data {
                    Some(msg) => info!("msg: {msg:?}"),
                    None => break,
                };
            }
        };
        let ctrl_c = async {
            shutdown.await;
        };

        tokio::pin!(recieve_fut, ctrl_c);

        futures_util::future::select(recieve_fut, ctrl_c).await;
    }
}
