use std::{collections::HashMap, future::Future};

use crate::{
    pcp::{ChannelInfo, GnuId, TrackInfo, ValidChannelInfo, ValidTrackInfo},
    repository::{Channel, Repository},
};

#[derive(Debug)]
pub(super) struct TypicalRepository<C> {
    channels: HashMap<GnuId, C>,
}

impl<C> TypicalRepository<C>
where
    C: Channel,
{
    pub fn new() -> Self {
        Self {
            channels: HashMap::new(),
        }
    }
}

impl<C> TypicalRepository<C>
where
    C: Channel,
{
    pub fn get(&self, id: crate::pcp::GnuId) -> Option<C> {
        self.channels.get(&id).cloned()
    }

    pub fn get_all(&self) -> Vec<C> {
        self.channels.values().cloned().collect()
    }

    pub fn create(
        &mut self,
        id: GnuId,
        channel_info: Option<ValidChannelInfo>,
        track_info: Option<ValidTrackInfo>,
        config: Option<<C as Channel>::Config>,
    ) -> (C, bool) {
        match self.channels.get(&id) {
            Some(ch) => (ch.clone(), false),
            None => {
                // let ch =
                // C::new(self.session_id.clone(), id.clone(), channel_info, track_info, rtmp_stream_manager, config);
                let ch: C = C::new(id.clone(), channel_info, track_info, config);
                tracing::info!("Created new channel: {:?}", ch);
                self.channels.insert(id, ch.clone());
                (ch, true)
            }
        }
    }

    pub fn delete_channel(&mut self, id: crate::pcp::GnuId) -> bool {
        if let Some(mut ch) = self.channels.remove(&id) {
            ch.before_delete();
            tracing::info!("Deleted channel: {:?}", ch);
            true
        } else {
            false
        }
    }

    pub fn delete_all(&mut self) {
        let keys_to_remove: Vec<GnuId> = self.channels.iter().map(|(k, _)| *k).collect();
        for id in keys_to_remove {
            self.delete_channel(id);
        }
    }

    pub fn filter_map_collect<F, G, R>(&self, mut f: F, mut g: G) -> Vec<R>
    where
        F: FnMut(&GnuId, &C) -> bool,
        G: FnMut(&GnuId, &C) -> R,
    {
        self.channels.iter().filter(|(k, v)| f(k, v)).map(|(k, v)| g(k, v)).collect()
    }
}

#[cfg(test)]
pub(crate) async fn test_repository<C: Channel>(mut repo: impl Repository<C>) {
    // TODO: channel_info, track_info, config を使ったテストも追加する
    let cid = GnuId::new();
    let channel = repo.create_or_get(cid.clone(), None, None, None).await;
    assert_eq!(channel.cid(), cid);

    let fetched_channel = repo.get(cid.clone());
    assert!(fetched_channel.is_some());
    assert_eq!(channel, fetched_channel.unwrap());

    let (channel_be_cloned, is_created) = repo.create(cid, None, None, None);
    assert_eq!(channel_be_cloned, channel);
    assert_eq!(is_created, false);

    let all_channels = repo.get_all();
    assert_eq!(all_channels.len(), 1);

    let _ = repo.create_or_get(GnuId::new(), None, None, None).await;
    let _ = repo.create_or_get(GnuId::new(), None, None, None).await;
    let all_channels = repo.get_all();
    assert_eq!(all_channels.len(), 3);

    let filter_channels = repo.filter_collect(|i, c| c.cid() == channel.cid());
    assert_eq!(filter_channels.len(), 1);

    let mapped_channels = repo.map_collect(|i, c| i.clone());
    assert_eq!(mapped_channels.len(), 3);

    let filter_mapped_channels = repo.filter_map_collect(|i, c| c.cid() == channel.cid(), |i, c| i.clone());
    assert_eq!(filter_mapped_channels.len(), 1);
    assert_eq!(filter_mapped_channels[0], channel.cid());

    let deleted = repo.delete_channel(cid.clone());
    assert!(deleted);
    let all_channels = repo.get_all();
    assert_eq!(all_channels.len(), 2);

    let fetched_channel_after_delete = repo.get(cid);
    assert!(fetched_channel_after_delete.is_none());

    let deleted = repo.delete_all();
    assert_eq!(repo.get_all().len(), 0);
}
