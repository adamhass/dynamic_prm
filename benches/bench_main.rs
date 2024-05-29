use criterion::criterion_main;

mod benchmarks;

criterion_main! {
    benchmarks::prm_benchmark::prm_benchmarks,
}
