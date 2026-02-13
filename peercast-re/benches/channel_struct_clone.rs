#![allow(unused)]
use criterion::{Criterion, criterion_group, criterion_main};
use libpeercast_re::pcp::{ChannelInfo, TrackInfo};
use std::{
    hint::black_box,
    sync::{Arc, Mutex, RwLock},
};
use tokio::sync::mpsc;

// 構造体Aの定義
#[derive(Clone)]
struct A {
    tracker_info: TrackInfo,
    channel_info: ChannelInfo,
}

// 構造体Bの定義
#[derive(Clone)]
struct B {
    tracker_info: Arc<RwLock<TrackInfo>>,
    channel_info: Arc<RwLock<ChannelInfo>>,
}

// 構造体Cの定義 -> これが最速っぽい
#[derive(Clone)]
struct CRwLock {
    impl_: Arc<RwLock<CImpl>>,
}

// 構造体Cの定義 -> これが最速っぽい
#[derive(Clone)]
struct CMutex {
    impl_: Arc<Mutex<CImpl>>,
}
struct CImpl {
    tracker_info: TrackInfo,
    channel_info: ChannelInfo,
}

// 構造体D定義
#[derive(Clone)]
struct D {
    sender: mpsc::UnboundedSender<ChanneStatus>,
}

#[derive(Debug, Clone, Default)]
struct ChanneStatus {
    tracker_info: TrackInfo,
    channel_info: ChannelInfo,
}

fn create_struct_a() -> A {
    A {
        tracker_info: TrackInfo::default(),
        channel_info: ChannelInfo::default(),
    }
}

fn create_struct_b() -> B {
    B {
        tracker_info: Arc::new(RwLock::new(TrackInfo::default())),
        channel_info: Arc::new(RwLock::new(ChannelInfo::default())),
    }
}

fn create_struct_crwlock() -> CRwLock {
    CRwLock {
        impl_: Arc::new(RwLock::new(CImpl {
            tracker_info: TrackInfo::default(),
            channel_info: ChannelInfo::default(),
        })),
    }
}
fn create_struct_cmutex() -> CMutex {
    CMutex {
        impl_: Arc::new(Mutex::new(CImpl {
            tracker_info: TrackInfo::default(),
            channel_info: ChannelInfo::default(),
        })),
    }
}

fn create_struct_d() -> D {
    let (tx, _rx) = mpsc::unbounded_channel();
    D {
        sender: tx,
    }
}

// 構造体Aのクローン操作ベンチマーク
fn benchmark_clone_a(c: &mut Criterion) {
    let sa = create_struct_a(); // ベンチマーク用に事前生成

    c.bench_function("Clone Struct A", |b| {
        b.iter(|| {
            let _a_clone = black_box(sa.clone()); // black_boxを使用して最適化を防ぐ
        });
    });
}

// 構造体Bのクローン操作ベンチマーク
fn benchmark_clone_b(c: &mut Criterion) {
    let sb = create_struct_b(); // ベンチマーク用に事前生成

    c.bench_function("Clone Struct B", |b| {
        b.iter(|| {
            let _b_clone = black_box(sb.clone()); // black_boxを使用して最適化を防ぐ
        });
    });
}

// 構造体Cのクローン操作ベンチマーク
fn benchmark_clone_crwlock(c: &mut Criterion) {
    let sc = create_struct_crwlock(); // ベンチマーク用に事前生成

    c.bench_function("Clone Struct CRwLock", |b| {
        b.iter(|| {
            let _c_clone = black_box(sc.clone()); // black_boxを使用して最適化を防ぐ
        });
    });
}
fn benchmark_clone_cmutex(c: &mut Criterion) {
    let sc = create_struct_cmutex(); // ベンチマーク用に事前生成

    c.bench_function("Clone Struct CMutex", |b| {
        b.iter(|| {
            let _c_clone = black_box(sc.clone()); // black_boxを使用して最適化を防ぐ
        });
    });
}

// 構造体Dのクローン操作ベンチマーク
fn benchmark_clone_d(c: &mut Criterion) {
    let sd = create_struct_d(); // ベンチマーク用に事前生成

    c.bench_function("Clone Struct D", |b| {
        b.iter(|| {
            let _d_clone = black_box(sd.clone()); // black_boxを使用して最適化を防ぐ
        });
    });
}

criterion_group!(
    benches,
    benchmark_clone_a,
    benchmark_clone_b,
    benchmark_clone_crwlock,
    benchmark_clone_cmutex,
    benchmark_clone_d
);
criterion_main!(benches);
