pub trait Spawner {
    type Output;

    fn spawn(&self, fut: impl Future<Output = ()> + Send + 'static) -> Self::Output;
}

/// Tokioのタスクを実装したSpawner
#[derive(Debug)]
pub struct TokioSpawner;

impl Spawner for TokioSpawner {
    type Output = tokio::task::JoinHandle<()>;

    fn spawn(&self, fut: impl Future<Output = ()> + Send + 'static) -> Self::Output {
        tokio::task::spawn(fut)
    }
}

/// テスト用のSpawner。実際には何もしない。
#[derive(Debug)]
pub struct TestSpawner;

impl Spawner for TestSpawner {
    type Output = ();

    fn spawn(&self, _fut: impl Future<Output = ()> + Send + 'static) -> Self::Output {
        // 何もしない
    }
}
