// use criterion::{black_box, criterion_group, criterion_main, Criterion};

// fn fibonacci(n: u64) -> u64 {
//     match n {
//         0 => 1,
//         1 => 1,
//         n => fibonacci(n - 1) + fibonacci(n - 2),
//     }
// }

// fn criterion_benchmark(c: &mut Criterion) {
//     c.bench_function("fib 20", |b| b.iter(|| fibonacci(black_box(20))));
// }

// criterion_group!(benches, criterion_benchmark);

// criterion_main!(benches);

use criterion::criterion_main;

mod benchmarks {
    pub mod compare_channels;
    pub mod compare_fibonacci;
}

criterion_main! {
    // benchmarks::compare_fibonacci::fibonaccis,
    benchmarks::compare_channels::channels,
    // benchmarks::external_process::benches,
    // benchmarks::iter_with_large_drop::benches,
    // benchmarks::iter_with_large_setup::benches,
    // benchmarks::iter_with_setup::benches,
    // benchmarks::with_inputs::benches,
    // benchmarks::special_characters::benches,
    // benchmarks::measurement_overhead::benches,
    // benchmarks::custom_measurement::benches,
    // benchmarks::sampling_mode::benches,
    // benchmarks::async_measurement_overhead::benches,
}
