use std::{cell::RefCell, collections::HashMap, future::Future, marker::PhantomData};

use crate::{
    model::{ValidChannelInfo, ValidTrackInfo},
    pcp::{ChannelInfo, GnuId, TrackInfo},
    repository::typical_repository::TypicalRepository,
};

use super::{Channel, Repository};

/// LocalRepository は スレッドローカルでのみ使用されるリポジトリ実装です。
/// このリポジトリは Send(スレッド間の移動禁止) Sync(スレッド間の共有禁止)を実装していないため
/// ローカルスレッド内でのみ使用されることを意図しています。
pub struct LocalRepository<C> {
    impl_: RefCell<TypicalRepository<C>>,
    // / This marker ensures that LocalRepository is !Send and !Sync
    // _marker: PhantomData<std::rc::Rc<()>>,
}

impl<C: Channel> LocalRepository<C> {
    pub fn new() -> Self {
        LocalRepository {
            impl_: RefCell::new(TypicalRepository::<C>::new()),
            // _marker: PhantomData,
        }
    }
}

impl<C> Repository<C> for LocalRepository<C>
where
    C: Channel,
{
    fn get(&self, id: crate::pcp::GnuId) -> Option<C> {
        self.impl_.borrow().get(id)
    }

    fn get_all(&self) -> Vec<C> {
        self.impl_.borrow().get_all()
    }

    fn create(
        &self,
        id: GnuId,
        channel_info: Option<ValidChannelInfo>,
        track_info: Option<ValidTrackInfo>,
        config: Option<<C as Channel>::Config>,
    ) -> (C, bool) {
        self.impl_.borrow_mut().create(id, channel_info, track_info, config)
    }

    fn create_or_get(
        &self,
        id: crate::pcp::GnuId,
        channel_info: Option<ValidChannelInfo>,
        track_info: Option<ValidTrackInfo>,
        config: Option<C::Config>,
    ) -> impl Future<Output = C> {
        let (mut ch, is_create) = self.impl_.borrow_mut().create(id, channel_info, track_info, config);
        async move {
            if is_create {
                ch.after_create().await;
            }
            ch
        }
    }

    fn delete_channel(&self, id: crate::pcp::GnuId) -> bool {
        self.impl_.borrow_mut().delete_channel(id)
    }

    fn delete_all(&self) {
        self.impl_.borrow_mut().delete_all();
    }

    fn filter_map_collect<F, G, R>(&self, f: F, g: G) -> Vec<R>
    where
        F: FnMut(&GnuId, &C) -> bool,
        G: FnMut(&GnuId, &C) -> R,
    {
        self.impl_.borrow().filter_map_collect(f, g)
    }
}

/// コンパイルエラーになることを確認するテストコード
/// ```compile_fail
/// use std::rc::Rc;
///
/// struct NotSend(Rc<()>);
/// fn assert_send<T: Send>() {}
/// assert_send::<NotSend>();
/// ```
fn _doc_example() {}

/// Syncが実装されていないことを確認するテストコード
/// ```compile_fail
/// use libpeercast_re::repository::local_repository::LocalRepository;
/// use libpeercast_re::repository::dummy_channel::DummyChannel;
/// fn assert_send<T: Sync>() {}
/// assert_send::<LocalRepository::<DummyChannel>>();
/// ```
fn _doc_local_repository() {}

/// コンパイルが通ることを確認するテストコード
/// ```
/// use libpeercast_re::repository::local_repository::LocalRepository;
/// use libpeercast_re::repository::dummy_channel::DummyChannel;
/// let _ = LocalRepository::<DummyChannel>::new();
/// ```
fn _doc_local_repository_ok() {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::{dummy_channel::DummyChannel, typical_repository::test_repository};

    #[tokio::test]
    async fn test_local_repository() {
        let mut repo: LocalRepository<DummyChannel> = LocalRepository::new();
        test_repository(repo).await;
    }
}
