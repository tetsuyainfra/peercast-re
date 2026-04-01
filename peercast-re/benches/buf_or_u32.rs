use std::hint::black_box;

use bytes::Buf;
use criterion::{Criterion, criterion_group};

static BUF: [u8; 8] = [0, 0, 0, 0, 1, 2, 3, 0xFF];

fn create_from_buf() -> u32 {
    (&BUF[4..8]).get_u32_le()
}

fn create_from_u32() -> u32 {
    u32::from_le_bytes(BUF[4..8].try_into().unwrap())
}

// 構造体Aのクローン操作ベンチマーク
fn benchmark_from_buf(c: &mut Criterion) {
    c.bench_function("Create From Buf", |b| {
        b.iter(|| {
            let _a_clone = black_box(create_from_buf());
        });
    });
}

// 構造体Bのクローン操作ベンチマーク
fn benchmark_from_u32(c: &mut Criterion) {
    c.bench_function("Create From U32", |b| {
        b.iter(|| {
            let _b_clone = black_box(create_from_u32());
        });
    });
}

criterion_group!(benches, benchmark_from_buf, benchmark_from_u32,);
