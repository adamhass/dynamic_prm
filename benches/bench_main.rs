use criterion::criterion_main;

mod benchmarks;

criterion_main! {
    // benchmarks::prm_benchmark::prm_benchmarks,
    // benchmarks::astar_benchmarks::astar_benchmarks,
    benchmarks::dprm_benchmarks::dprm_benchmarks,
}
