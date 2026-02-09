use std::future::Future;

use crate::pcp::{ChannelInfo, GnuId, TrackInfo};

pub mod dummy_channel;
pub mod local_repository;
pub mod shared_repository;
pub(self) mod typical_repository;

/// チャンネルの状態を表す列挙型
pub enum ChannelState {
    /// 未使用の状態
    Idle,
    /// リスニング・配信中の状態
    Active,
    /// 完全に終了した状態
    Finished,
}

/// Peercast-re, およびそのクライアントでのみ使われるチャンネル型
pub enum ChannelType {
    /// RootはYPのチャンネルタイプである。Peercast-Reでは使われない
    Root,
    /// Trackerは配信者の立てるチャンネルタイプである。
    Tracker,
    /// Relayはリスナーが中継しているチャンネルタイプである。
    Relay,
}

/// チャンネルの振る舞いを定義するトレイト
/// Channelトレイトは、チャンネルの基本的な操作や情報取得のためのメソッドを提供します。
/// このトレイトを実装することで、様々な種類のチャンネルを扱うことができます。
/// Send と Sync トレイトを継承しているため、チャンネルインスタンスが複数のスレッド間で安全に共有および移動できることを保証します。
#[rustfmt::skip]
pub trait Channel : Clone + Send + Sync + PartialEq + Eq + std::fmt::Debug  + 'static{
    type Config ;

    fn new(
        id: GnuId,
        channel_info: Option<ChannelInfo>,
        track_info: Option<TrackInfo>,
        config: Option<Self::Config>
    ) -> Self;

    /// if use create_or_get method, after_create will be called after creation.
    fn after_create(&mut self) -> impl Future<Output=()> + Send {async {}}

    /// This method is called BEFORE deleting the channel FROM Repository.
    fn before_delete(&mut self) {}

    fn id(&self) -> GnuId { unimplemented!() }
    fn state(&self) -> ChannelState{ unimplemented!() }
    fn channel_type(&self) -> ChannelType { unimplemented!() }

    fn config(&self) -> Option<&Self::Config> { unimplemented!() }
    fn update_config(&mut self, config: Self::Config) { unimplemented!() }

    fn channel_info(&self) -> Option<ChannelInfo> { unimplemented!()}
    fn track_info(&self) -> Option<TrackInfo> { unimplemented!()}

    fn created_at(&self) -> chrono::DateTime<chrono::Utc> { unimplemented!() }
    fn updated_at(&self) -> chrono::DateTime<chrono::Utc> { unimplemented!() }
    fn viewed_at(&self) -> chrono::DateTime<chrono::Utc> { unimplemented!() }

}

/// Repository トレイトは、チャンネルの管理と操作を行うためのインターフェースを提供します。
/// ところで、PeerCastを実装する際には配信用チャンネルとリスニング用チャンネルのロジック分ける必要がある。
/// 無理に一つの構造体で実装しようとすると、ジェネリクスを多用することになり、コードが複雑になる。
/// そのため、リポジトリのインスタンスを複数用意し、用途に応じて使い分ける設計にした。
/// もちろんenumを使って一つの構造体で実装することも可能ではあるが、コードが煩雑になるため避けよう。
#[rustfmt::skip]
pub trait Repository<C: Channel> {
    fn get(&self, id: GnuId) -> Option<C>;
    fn get_all(&self) -> Vec<C>;

    fn create(
        &self,
        id: GnuId,
        channel_info: Option<ChannelInfo>,
        track_info: Option<TrackInfo>,
        config: Option<C::Config>,
    ) -> (C, bool);
    fn create_or_get(
        &self,
        id: GnuId,
        channel_info: Option<ChannelInfo>,
        track_info: Option<TrackInfo>,
        config: Option<C::Config>,
    ) -> impl Future<Output = C> + Send;

    fn delete_channel(& self, id: GnuId) -> bool;
    fn delete_all(& self);

    fn filter_map_collect<F, G, R>(&self, f: F, g: G) -> Vec<R>
    where
        F: FnMut(&GnuId, &C) -> bool,
        G: FnMut(&GnuId, &C) -> R,
    {
        unimplemented!()
    }

    fn filter_collect<F>(&self, f: F) -> Vec<C>
    where
        F: FnMut(&GnuId, &C) -> bool,
    {
        self.filter_map_collect(f, |k, v| v.clone())
    }

    fn map_collect<G, R>(&self, g: G) -> Vec<R>
    where
        G: FnMut(&GnuId, &C) -> R,
    {
        self.filter_map_collect(|_, _| true, g)
    }
}
