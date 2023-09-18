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
#[derive(Debug)]
pub enum TaskStatus {
    Idle,
    Running,
    Stopped,
    Error,
}

////////////////////////////////////////////////////////////////////////////////
/// SourceTaskConfig
///
pub enum SourceTaskConfig {
    Broadcast(BroadcastTaskConfig),
    Relay(RelayTaskConfig),
}

#[async_trait]
pub(crate) trait SourceTaskTrait: Send + Sync + std::fmt::Debug {
    fn connect(&self, config: SourceTaskConfig);
    fn retry(&self);

    fn update_info(&self, info: ChannelInfo);
    fn update_track(&self, info: TrackInfo);

    fn status(&self) -> TaskStatus;
    async fn status_changed(&mut self) -> Result<(), watch::error::RecvError>;

    async fn stop(&self);
}

////////////////////////////////////////////////////////////////////////////////
// RelayTask
//
