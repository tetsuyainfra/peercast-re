use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use peercast_re::{
    pcp::GnuId,
    util::{rwlock_read_poisoned, rwlock_write_poisoned},
};

pub trait Channel {
    type Config: Clone;
    fn new(id: GnuId, config: Self::Config) -> Self;
}

#[derive(Debug, Clone)]
pub struct ChannelRepository<T> {
    channels_: Arc<RwLock<HashMap<GnuId, T>>>,
}

impl<T> ChannelRepository<T>
where
    T: Channel + Clone,
{
    pub fn new() -> Self {
        Self {
            channels_: Default::default(),
        }
    }

    pub fn get(&self, id: &GnuId) -> Option<T> {
        self.channels_
            .read()
            .unwrap_or_else(rwlock_read_poisoned)
            .get(id)
            .map(|ch| ch.clone())
    }

    pub fn get_or_create(&self, id: GnuId, config: T::Config) -> T {
        self.channels_
            .write()
            .unwrap_or_else(rwlock_write_poisoned)
            .entry(id)
            .or_insert_with_key(|id| T::new(id.clone(), config))
            .clone()
    }
}

////////////////////////////////////////////////////////////////////////////////
// Disable
////////////////////////////////////////////////////////////////////////////////
// #[derive(Debug, Clone)]
// pub struct Channel {
//     id: GnuId,
// }
// impl Channel {
//     fn new(id: GnuId) -> Self {
//         Self { id }
//     }
// }

// #[derive(Debug, Clone)]
// pub struct ChannelRepository {
//     channels_: Arc<RwLock<HashMap<GnuId, Channel>>>,
// }

// impl ChannelRepository {
//     pub fn new() -> Self {
//         Self {
//             channels_: Default::default(),
//         }
//     }

//     pub fn get(&self, id: &GnuId) -> Option<Channel> {
//         self.channels_
//             .read()
//             .unwrap_or_else(rwlock_read_poisoned)
//             .get(id)
//             .map(|ch| ch.clone())
//     }

//     pub fn get_or_create(&self, id: GnuId) -> Channel {
//         self.channels_
//             .write()
//             .unwrap_or_else(rwlock_write_poisoned)
//             .entry(id)
//             .or_insert_with_key(|id| Channel::new(id.clone()))
//             .clone()
//     }
// }
