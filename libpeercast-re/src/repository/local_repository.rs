use std::{collections::HashMap, future::Future, marker::PhantomData};

use crate::{pcp::GnuId, repository::inner_repository::InnerRepository};

use super::{Channel, Repository};

/// LocalRepository は スレッドローカルでのみ使用されるリポジトリ実装です。
/// このリポジトリは Send(スレッド間の移動禁止) Sync(スレッド間の共有禁止)を実装していないため
/// ローカルスレッド内でのみ使用されることを意図しています。
pub struct LocalRepository<C> {
    impl_: InnerRepository<C>,

    /// This marker ensures that LocalRepository is !Send and !Sync
    _marker: PhantomData<std::rc::Rc<()>>,
}

impl<C: Channel> LocalRepository<C> {
    pub fn new() -> Self {
        LocalRepository {
            impl_: InnerRepository::<C>::new(),
            _marker: PhantomData,
        }
    }
}

impl<C> Repository<C> for LocalRepository<C>
where
    C: Channel,
{
    fn get(&self, id: crate::pcp::GnuId) -> Option<C> {
        self.impl_.get(id)
    }

    fn get_all(&self) -> Vec<C> {
        self.impl_.get_all()
    }

    fn create(&mut self, id: GnuId, config: Option<<C as Channel>::Config>) -> (C, bool) {
        self.impl_.create(id, config)
    }

    fn create_or_get(&mut self, id: crate::pcp::GnuId, config: Option<C::Config>) -> impl Future<Output = C> {
        let (mut ch, is_create) = self.impl_.create(id, config);
        async move {
            if is_create {
                ch.after_create().await;
            }
            ch
        }
    }

    fn delete_channel(&mut self, id: crate::pcp::GnuId) -> bool {
        self.impl_.delete_channel(id)
    }

    fn delete_all(&mut self) {
        self.impl_.delete_all();
    }

    fn filter_collect<F>(&self, mut f: F) -> Vec<C>
    where
        F: FnMut(&GnuId, &C) -> bool,
    {
        self.impl_.filter_collect(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::{dummy_channel::DummyChannel, inner_repository::test_repository};

    /// ```compile_fail
    /// コンパイルエラーになることを確認するテストコード
    /// use std::rc::Rc;
    ///
    /// struct NotSend(Rc<()>);
    /// fn assert_send<T: Send>() {}
    /// assert_send::<NotSend>();
    /// ```
    fn _doc() {}

    #[tokio::test]
    async fn test_local_repository() {
        let mut repo: LocalRepository<DummyChannel> = LocalRepository::new();
        test_repository(repo).await;
    }
}
