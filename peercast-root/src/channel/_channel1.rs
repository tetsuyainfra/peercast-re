use std::{
    net::SocketAddr,
    sync::{Arc, Mutex, RwLock},
};

use chrono::{DateTime, Utc};
use futures_util::{FutureExt, future::BoxFuture};
use libpeercast_re::{
    pcp::{
        ChannelInfo, GnuId, TrackInfo,
        builder::{QuitBuilder, QuitReason},
        connection::PcpConnection,
        decode::{PcpBroadcast, PcpChannel, PcpHost},
    },
    util::{mutex_poisoned, rwlock_read_poisoned, rwlock_write_poisoned},
};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

use crate::prelude::*;
use crate::repository::Channel;

pub mod json_model;

#[derive(Debug, Clone)]
pub struct RootChannel {
    cid: GnuId,
    tracker_addr: Arc<RwLock<Option<SocketAddr>>>,
    channel_info: Arc<RwLock<libpeercast_re::pcp::ChannelInfo>>,
    track_info: Arc<RwLock<libpeercast_re::pcp::TrackInfo>>,
    number_of_listener: Arc<RwLock<i32>>,
    number_of_relay: Arc<RwLock<i32>>,
    last_update: Arc<Mutex<DateTime<Utc>>>,
    created_at: Arc<DateTime<Utc>>,
}
#[derive(Debug)]
pub struct RootConfig {
    pub broadcast_id: GnuId,
    pub tracker_host: Option<SocketAddr>,
}

impl Channel for RootChannel {
    type Config = RootConfig;
    fn new(
        _self_session_id: GnuId,
        cid: GnuId,
        channel_info: Option<libpeercast_re::pcp::ChannelInfo>,
        track_info: Option<libpeercast_re::pcp::TrackInfo>,
        config: Option<RootConfig>,
    ) -> Self {
        let tracker_addr = config.and_then(|c| c.tracker_host);
        let now_ = Utc::now();

        Self {
            cid,
            tracker_addr: Arc::new(RwLock::new(tracker_addr)),
            channel_info: RwLock::new(channel_info.unwrap_or_default()).into(),
            track_info: RwLock::new(track_info.unwrap_or_default()).into(),
            number_of_listener: RwLock::new(0).into(),
            number_of_relay: RwLock::new(0).into(),
            last_update: Arc::new(Mutex::new(now_.clone())),
            created_at: Arc::new(now_),
        }
    }

    fn last_update(&self) -> DateTime<Utc> {
        self.last_update.lock().unwrap_or_else(mutex_poisoned).clone()
    }
}
impl RootChannel {
    fn arrived_broadcast(&self, bcst: PcpBroadcast, remote_addr: &SocketAddr) {
        info!(cid = ?self.cid, "ArrivedBroadcast");
        debug!(?bcst);
        // TODO: 不正チェックした方がいいかも

        let PcpBroadcast {
            channel_packet,
            host,
            ..
        } = bcst;
        if channel_packet.is_none() {
            return;
        }

        // Host情報の更新
        if let Some(pcp_host) = host {
            let PcpHost {
                addresses,
                number_listener,
                number_relay,
                ..
            } = pcp_host;
            // TrackerのIPアドレスを更新
            {
                let tracker_addr = get_tracker_addr(&remote_addr, &addresses);
                // MEMO: tracker_addr=Noneが帰ってきたらどする？
                let mut tracker_host_locked = self.tracker_addr.write().unwrap_or_else(rwlock_write_poisoned);
                *tracker_host_locked = tracker_addr;
            }
            // listener数を更新
            {
                let mut listner_locked = self.number_of_listener.write().unwrap_or_else(rwlock_write_poisoned);
                if let Some(listner) = number_listener {
                    *listner_locked = listner;
                }
            }
            // relay数を更新
            {
                let mut relay_locked = self.number_of_relay.write().unwrap_or_else(rwlock_write_poisoned);
                if let Some(relay) = number_relay {
                    *relay_locked = relay;
                }
            }
        }

        //
        let PcpChannel {
            channel_info,
            track_info,
            ..
        } = channel_packet.unwrap();
        {
            let mut channel_info_unlocked = self.channel_info.write().unwrap_or_else(rwlock_write_poisoned);
            let mut track_info_unlocked = self.track_info.write().unwrap_or_else(rwlock_write_poisoned);
            let mut last_update_locked = self.last_update.lock().unwrap_or_else(mutex_poisoned);

            match channel_info {
                Some(new_channel_info) => {
                    debug!(?new_channel_info);
                    channel_info_unlocked.merge_pcp(new_channel_info);
                }
                None => (),
            };
            match track_info {
                Some(new_track_info) => {
                    debug!(?new_track_info);
                    track_info_unlocked.merge_pcp(new_track_info);
                }
                None => (),
            };
            *last_update_locked = Utc::now();
        }
    }

    fn id(&self) -> GnuId {
        self.cid
    }

    fn channel_info(&self) -> ChannelInfo {
        self.channel_info.read().unwrap_or_else(rwlock_read_poisoned).clone()
    }
    fn track_info(&self) -> TrackInfo {
        self.track_info.read().unwrap_or_else(rwlock_read_poisoned).clone()
    }

    fn tracker_addr(&self) -> Option<SocketAddr> {
        self.tracker_addr.read().unwrap_or_else(rwlock_read_poisoned).clone()
    }
    fn number_of_listener(&self) -> i32 {
        self.number_of_listener.read().unwrap_or_else(rwlock_read_poisoned).clone()
    }
    fn number_of_relay(&self) -> i32 {
        self.number_of_relay.read().unwrap_or_else(rwlock_read_poisoned).clone()
    }
    fn created_at(&self) -> DateTime<Utc> {
        self.created_at.as_ref().clone()
    }
}

impl RootChannel {
    pub fn attach_connection(
        self,
        pcp_connection: PcpConnection,
        graceful_shutdown: CancellationToken,
        closed_send: watch::Receiver<()>,
    ) -> AttachTaskFuture {
        info!("ATTACH CONNECTION TO CHANNEL ");
        async move {
            info!("START");
            let remote_addr = pcp_connection.remote_addr();
            let (mut read_inner, mut write_inner) = pcp_connection.split();

            'main: loop {
                tokio::select! {
                    atom = read_inner.read_atom() => {
                        match atom {
                            Ok(a) => {
                                // *self.last_update.lock().unwrap_or_else(mutex_poisoned) = Utc::now();
                                match PcpBroadcast::parse(&a) {
                                    Ok(pcp) => self.arrived_broadcast(pcp, &remote_addr),
                                    Err(e) => error!(?e, "error"),
                                };
                            },
                            Err(e) => {
                                error!("Read Error: {:?}", e);
                                break 'main
                            }
                        }
                    }
                    _ = graceful_shutdown.cancelled() => {
                        warn!("Shutdown signal catched");
                        break 'main
                    }
                };
            } // loop 'main

            let quit = QuitBuilder::new(QuitReason::Any).build();
            write_inner.write_atom(quit).await;
            info!("QUIT Channel");

            drop(read_inner);
            drop(write_inner);

            // スレッドの終了を知らせる
            drop(closed_send);
        }
        .boxed()
    }
}

type AttachTaskFuture = BoxFuture<'static, ()>;

pub fn get_tracker_addr(remote_addr: &SocketAddr, addresses: &Vec<SocketAddr>) -> Option<SocketAddr> {
    // Hostの接続先を確定
    // TODO: firewall checkが必要
    let host = addresses.iter().find(|addr| addr.ip() == remote_addr.ip());
    let tracker_host = host.map(|h| h.clone());

    tracker_host
}

#[cfg(test)]
mod t {
    use crate::{channel::RootChannel, test_helper::*};

    #[test]
    fn check_types() {
        assert_sized::<RootChannel>();
    }
}
