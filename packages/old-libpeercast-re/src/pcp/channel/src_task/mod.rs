use async_trait::async_trait;
use tokio::sync::watch;

use super::{ChannelInfo, TrackInfo};

pub(super) use broadcast_task::BroadcastTask;
pub use broadcast_task::BroadcastTaskConfig;
pub(super) use relay_task::RelayTask;
pub use relay_task::RelayTaskConfig;

mod broadcast_task;
mod relay_task;

////////////////////////////////////////////////////////////////////////////////
/// TaskState
///
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TaskStatus {
    Init,
    Searching { searched: u32, all: u32 },
    // Handshake { retry: u8 },
    Receiving,
    Idle,
    Finish,
    Error,
}

////////////////////////////////////////////////////////////////////////////////
/// SourceTaskConfig
///
#[derive(Debug)]
pub enum SourceTaskConfig {
    Broadcast(BroadcastTaskConfig),
    Relay(RelayTaskConfig),
}

#[async_trait]
pub(crate) trait SourceTask: Send + Sync + std::fmt::Debug {
    // fn new(config: Self::Config) -> Self;
    // async fn start(self);

    /// データソースとなるホストに接続する
    fn connect(&mut self, config: SourceTaskConfig) -> bool;

    /// 同じ設定で再接続する
    fn retry(&mut self) -> bool;

    /// ChannelInfoを更新する
    fn update_info(&self, info: ChannelInfo) {}
    /// TrackInfoを更新する
    fn update_track(&self, info: TrackInfo) {}

    fn status(&self) -> TaskStatus;
    async fn status_changed(&mut self) -> Result<(), watch::error::RecvError>;

    fn stop(&self);
}

////////////////////////////////////////////////////////////////////////////////
// RelayTask
//
