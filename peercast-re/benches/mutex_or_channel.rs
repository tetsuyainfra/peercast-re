#![allow(unused)]
use std::{
    hint::black_box,
    sync::{Arc, Mutex},
};

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use libpeercast_re::pcp::{ChannelInfo, TrackInfo};
use tokio::sync::watch;

// 構造体Aの定義
#[derive(Clone)]
struct A {
    tracker_info: TrackInfo,
    channel_info: ChannelInfo,
}

fn create_mutex() -> Arc<Mutex<A>> {
    Arc::new(Mutex::new(A {
        tracker_info: TrackInfo::default(),
        channel_info: ChannelInfo::default(),
    }))
}

fn lock_and_get_clone(mutex: &Arc<Mutex<A>>) -> A {
    let guard = mutex.lock().unwrap();
    guard.clone()
}

fn benchmark_mutex(c: &mut Criterion) {
    c.bench_function("Mutex Clone", |b| {
        let mutex_struct = create_mutex();
        b.iter(|| {
            black_box(lock_and_get_clone(&mutex_struct));
        });
    });
}

fn create_channel_watch() -> (watch::Sender<A>, watch::Receiver<A>) {
    let (tx, rx) = tokio::sync::watch::channel(A {
        tracker_info: TrackInfo::default(),
        channel_info: ChannelInfo::default(),
    });
    (tx, rx)
}

fn get_clone(rx: &watch::Receiver<A>) -> A {
    let borrowed = rx.borrow();
    borrowed.clone()
}

fn benchmark_channel(c: &mut Criterion) {
    use std::sync::atomic::Ordering;
    let rt = tokio::runtime::Runtime::new().unwrap();

    let stop_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let stop_flag_clone = Arc::clone(&stop_flag);

    let (tx, rx) = create_channel_watch();
    // 別スレッドで並行タスクを実行
    let _handle = std::thread::spawn(move || {
        rt.block_on(async move {
            while !stop_flag_clone.load(Ordering::Relaxed) {
                // 別スレッドの非同期タスク (例: ログ出力や監視)
            }
        });
    });

    c.bench_function("Channel Clone", |b| {
        // b.iter_batched_ref(|| data.clone(), |mut data| sort(&mut data), BatchSize::SmallInput)
        b.iter_batched(|| rx.clone(), |mut rx| black_box(get_clone(&rx)), BatchSize::SmallInput);
    });

    // ベンチマーク終了後にフラグをセットして別スレッドを停止
    stop_flag.store(true, Ordering::Relaxed);
}

criterion_group!(benches, benchmark_mutex, benchmark_channel);
criterion_main!(benches);
