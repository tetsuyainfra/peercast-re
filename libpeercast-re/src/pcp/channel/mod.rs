use paste::paste;
use serde::Serialize;
use tracing::error;

mod broker;
mod channel;
mod channel_info;
mod channel_stream;
mod manager;
mod node_pool;
mod src_task;
mod track_info;

pub(self) use broker::ChannelBrokerMessage;
pub use broker::{ChannelMessage, ChannelReciever};
pub use channel::{Channel, ChannelType};
pub use channel_info::ChannelInfo;
pub use manager::ChannelManager;
pub use node_pool::{Node, NodePool};
pub use src_task::{BroadcastTaskConfig, RelayTaskConfig, SourceTaskConfig, TaskStatus};
pub use track_info::TrackInfo;

use crate::pcp::{atom, Id4};

use super::Atom;


macro_rules! merge_field {
    ($self:ident, $value:ident, $field:ident) => {
        if let Some(v) = $value.$field {
            $self.$field = v;
        }
    };
}
pub(self) use merge_field;