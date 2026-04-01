use std::collections::HashMap;

use crate::config::Config;

use super::GnuId;

/// TrackerChannelを管理するクラス(RootServerで利用する)
struct TrackerChannel {}

impl TrackerChannel {}

trait Channel {
    type Config;
    fn new(option: Config);
}

struct TrackerChannelManager<CHANNEL = TrackerChannel> {
    channels: HashMap<GnuId, CHANNEL>,
}

impl<C, CONFIG> TrackerChannelManager<C>
where
    C: Channel<Config = CONFIG>,
{
    fn new() -> Self {
        Self {
            channels: Default::default(),
        }
    }

    fn create_channel(option: Config) {
        C::new(option)
    }
}
