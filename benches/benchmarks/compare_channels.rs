use std::time::Instant;

use criterion::{async_executor::FuturesExecutor, criterion_group, BenchmarkId, Criterion};
use futures_util::FutureExt;
use tokio::sync::{broadcast, mpsc, watch};

async fn mpsc_recv(rx: &mut mpsc::UnboundedReceiver<()>) {
    let r = rx.recv().now_or_never();
    assert!(r.is_none());
}
async fn watch_recv(rx: &mut watch::Receiver<()>) {
    let r = rx.changed().now_or_never();
    assert!(r.is_none());
}
async fn broadcast_recv(rx: &mut broadcast::Receiver<()>) {
    let r = rx.recv().now_or_never();
    assert!(r.is_none());
}

fn compare_channels(c: &mut Criterion) {
    let size = 1024;

    let mut group = c.benchmark_group("Channels");
    for i in [size].iter() {
        group.bench_with_input(BenchmarkId::new("mpsc", i), i, |b, i| {
            b.to_async(FuturesExecutor).iter_custom(|iters| async move {
                let (tx, mut rx) = mpsc::unbounded_channel();
                let start = Instant::now();
                for _i in 0..iters {
                    mpsc_recv(&mut rx).await;
                }
                start.elapsed()
            })
        });
        group.bench_with_input(BenchmarkId::new("watch", i), i, |b, i| {
            b.to_async(FuturesExecutor).iter_custom(|iters| async move {
                let (tx, mut rx) = watch::channel(());
                let start = Instant::now();
                for _i in 0..iters {
                    watch_recv(&mut rx).await;
                }
                start.elapsed()
            })
        });
        group.bench_with_input(BenchmarkId::new("broadcast", i), i, |b, i| {
            b.to_async(FuturesExecutor).iter_custom(|iters| async move {
                let (tx, mut rx) = broadcast::channel::<()>(1);
                let start = Instant::now();
                for _i in 0..iters {
                    broadcast_recv(&mut rx).await;
                }
                start.elapsed()
            })
        });
    }
}

// criterion_group!(benches, criterion_benchmark);
// criterion_group!(channels, compare_fibonaccis, compare_fibonaccis_group,);

criterion_group!(channels, compare_channels);
