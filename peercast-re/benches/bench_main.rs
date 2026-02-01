use criterion::criterion_main;

mod buf_or_u32;

criterion_main! {
    buf_or_u32::benches
}
