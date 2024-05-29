use criterion::{criterion_group, criterion_main, Criterion};
use dynamic_prm::prelude::*;
use rand_chacha::ChaCha8Rng;
use rand::prelude::*;
use std::sync::Arc;
use criterion::BenchmarkId;
use tokio::runtime::Runtime;



// Define a function to benchmark `parallel_prm`
fn benchmark_parallel_prm(c: &mut Criterion) {
    let seed = Arc::new([0u8; 32]);
    let width = 100;
    let height = 100;
    let num_vertices = 10000;
    let num_obstacles = 50;
    
    let mut rng = ChaCha8Rng::from_seed(*seed);
    let obstacles = Arc::new(ObstacleSet::new_random(num_obstacles, width, height, &mut rng));
    let cfg = PrmConfig {
        num_vertices,
        width,
        height,
        seed,
    };
    let prm = Prm::new(cfg, obstacles);
    
    // Define the number of threads to test
    let num_threads_list = vec![1, 2, 4, 8];
    let mut group = c.benchmark_group("Parallel PRM 10k Vertices");
    
    // Use a loop to create benchmarks for each number of threads
    for &num_threads in &num_threads_list {
        group.bench_with_input(BenchmarkId::new("Threads", num_threads), &num_threads, |b, &num_threads| {
            b.to_async(Runtime::new().unwrap()).iter(|| prm.run_prm(num_threads));
        });
    }

    group.finish()

}

// Define the criterion group and criterion main functions
criterion_group!(prm_benchmarks, benchmark_parallel_prm);
criterion_main!(prm_benchmarks);
