use std::{any, io::Write, time::Duration};

use chrono_tz::Asia::Kabul;
use futures_util::{future::BoxFuture, FutureExt};
use hyper_util::server::graceful;
use tokio_util::sync::CancellationToken;
use tracing::info;

//
/// HowToUse1:
/// ```
/// let (mut shutdown, graceful, force) = shutdown::craete_thread();
/// tokio::select!{
///     _ = something_await_fn(graceful, force) => {},
///     _ = shutdown => {
///     }
/// }
/// before something...
/// ```
// HowToUse2
/// ```
/// let (mut shutdown, graceful, force) = shutdown::craete_thread();
/// tokio::spawn!(shutdown);
/// before something...
/// ```

static MAX_COUNT: usize = 2;

/// システム外部からのシャットダウン操作をキャプチャするスレッドを作成する
/// 内部で使用されるCancellationTokenが最上位にあたる
pub(crate) fn create_task() -> (BoxFuture<'static, ()>, CancellationToken, CancellationToken) {
    let graceful_shutdown = tokio_util::sync::CancellationToken::new();
    let force_shutdown = tokio_util::sync::CancellationToken::new();

    let child_graceful_shutdown = graceful_shutdown.child_token();
    let child_force_shutdown = force_shutdown.child_token();

    let handler = _create(graceful_shutdown, force_shutdown);
    (handler, child_graceful_shutdown, child_force_shutdown)
}
pub(crate) fn create_task_anyhow() -> (
    impl std::future::Future<Output = anyhow::Result<()>>,
    CancellationToken,
    CancellationToken,
) {
    let (handler, graceful, force) = create_task();

    let new_handler = async move {
        handler.await;
        anyhow::Ok(())
    };

    (new_handler, graceful, force)
}

fn _create(
    graceful_shutdown: CancellationToken,
    force_shutdown: CancellationToken,
) -> BoxFuture<'static, ()> {
    use tokio::signal::ctrl_c;

    fn terminate() -> BoxFuture<'static, ()> {
        #[cfg(unix)]
        let terminate = async {
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("failed to install signal handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        terminate.boxed()
    }

    async move {
        tokio::select! {
            _ = ctrl_c() => {
                println!("\nCtrl-C received, initiating graceful shutdown...");
            }
            _ = terminate() => {}
        }
        graceful_shutdown.cancel();
        info!("GRACEFUL SHUTDOWN START");
    }
    .boxed()
}

/*
// create_handle()内では必ずgraceful_shutdownが呼ばれてからforce_shutdownが呼ばれることを保障する
// TODO: Signaleキャプチャした時どうする？
async fn _create(graceful_shutdown: CancellationToken, force_shutdown: CancellationToken) -> () {
    use futures_util::{future::BoxFuture, FutureExt};
    #[derive(Debug, PartialEq)]
    enum ShutdownState {
        GarcefullShutdown { count: usize, max_count: usize },
        Shutdown,
    }

    async fn wait_and_count(count: usize, max_count: usize) -> ShutdownState {
        let _ = tokio::signal::ctrl_c().await;
        let next_count = count + 1;
        info!("catch ctrl_c() {}/{}", next_count, max_count);

        if next_count < max_count {
            ShutdownState::GarcefullShutdown {
                count: next_count,
                max_count: max_count,
            }
        } else {
            ShutdownState::Shutdown
        }
    }
    async fn wait_for() -> ShutdownState {
        let _res = tokio::time::timeout(Duration::from_secs(5), async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            loop {
                interval.tick().await;
                {
                    let mut lock = std::io::stdout().lock();
                    let _ = lock.write_all(b".");
                    let _ = lock.flush();
                }
            }
        })
        .await;
        ShutdownState::Shutdown
    }

    let mut futures: futures_util::future::SelectAll<BoxFuture<ShutdownState>> =
        futures_util::future::select_all(vec![wait_and_count(0, MAX_COUNT).boxed()]);

    'wait_loop: loop {
        let (state, _idx, remain_future) = futures.await;
        let mut new_futures = remain_future;

        // dbg!(&state);
        match state {
            ShutdownState::GarcefullShutdown { count, max_count } => {
                graceful_shutdown.cancel();
                if count == 1 {
                    new_futures.push(wait_for().boxed())
                }
                new_futures.push(wait_and_count(count, max_count).boxed());
            }
            ShutdownState::Shutdown => break 'wait_loop,
        }

        futures = futures_util::future::select_all(new_futures);
    }

    force_shutdown.cancel();

    info!("SHUTDOWN")
}
 */
