// use channel::{Channel, ChannelConfig};

// pub use self::store::ChannelStore;
// use self::store::ChannelWatcherMessage;

mod channel;
// mod implement;
// mod store;
// pub mod tracker_channel;

// //------------------------------------------------------------------------------
// // Repository
// //
// pub trait Repository {
//     fn new() -> Self;

//     fn session_id(&self) -> GnuId;
//     fn broadcast_id(&self) -> GnuId;

//     fn get(&self, channel_id: &GnuId) -> Option<Channel>;

//     fn get_or_create(&self, channel_id: &GnuId, config: ChannelConfig) -> Channel;
//     fn remove(&self, channel_id: &GnuId);
// }

// //------------------------------------------------------------------------------
// // RepositoryWatcher
// //
// pub trait RepositoryWatcher {
//     fn new() -> Self;

//     fn subscribe(channel: ());
//     fn stop(&mut self, id: GnuId);

//     async fn stop_wait(&mut self, id: GnuId);
// }

//------------------------------------------------------------------------------
// ConnectionManager : 接続を管理するマネージャー
//
