#![allow(unused)]
use criterion::BenchmarkId;
use criterion::{criterion_group, criterion_main, Criterion};
use dynamic_prm::prelude::*;
use pathfinding::matrix::directions::S;
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

const VERTICES: usize = 10000;
const OBSTACLES: usize = 50;
const WIDTH: usize = 100;
const HEIGHT: usize = 100;
const SEED: [u8; 32] = [0u8; 32];

fn init_prm() -> Prm {
    // Parameters common to all benchmarks:
    let cfg = PrmConfig::new(
        VERTICES,
        WIDTH,
        HEIGHT,
        SEED,
    );
    Prm::new(cfg, OBSTACLES)
}

fn precompute_prm(viable_edges: bool) -> Prm {
    let mut prm = init_prm();
    prm.cfg.use_viable_edges = viable_edges;
    Runtime::new().unwrap().block_on(prm.compute(4));
    prm
}

// Define a function to benchmark `parallel_prm`
fn benchmark_parallel_prm(c: &mut Criterion) {
    // Define the number of threads to test
    let num_threads_list = vec![1, 2, 4, 8];
    let mut group = c.benchmark_group(format!("Parallel PRM {} Vertices", VERTICES));
    // Use a loop to create benchmarks for each number of threads
    for &num_threads in &num_threads_list {
        let prm = init_prm();
        group.bench_with_input(
            BenchmarkId::new("Basic, Threads", num_threads),
            &num_threads,
            |b, &num_threads| {
                b.to_async(Runtime::new().unwrap()).iter(|| {
                    prm.increment_seed();
                    // Run the PRM algorithm, immutable borrow
                    prm.run_prm(num_threads)
                });
            },
        );
    }
    for &num_threads in &num_threads_list {
        let mut prm = init_prm();
        prm.cfg.use_viable_edges = true;
        group.bench_with_input(
            BenchmarkId::new("Viable edges, Threads", num_threads),
            &num_threads,
            |b, &num_threads| {
                b.to_async(Runtime::new().unwrap()).iter(|| {
                    prm.increment_seed();
                    // Run the PRM algorithm, immutable borrow
                    prm.run_prm(num_threads)
                });
            },
        );
    }
    group.finish()
}

fn benchmark_add_obstacle(c: &mut Criterion) {
    // Define the number of threads to test
    let num_threads_list = vec![1, 2, 4, 8];
    let mut group = c.benchmark_group(format!(
        "Parallel Obstacle Insert and Remove, {} Vertices",
        VERTICES
    ));
    let mut rng = ChaCha8Rng::from_seed(SEED);

    // Use a loop to create benchmarks for each number of threads
    let mut prm_original = Arc::new(Mutex::new(precompute_prm(false)));
    for &num_threads in &num_threads_list {
        group.bench_with_input(
            BenchmarkId::new("Basic, Threads", num_threads),
            &num_threads,
            |b, &num_threads| {
                b.to_async(Runtime::new().unwrap()).iter_batched(
                    || {
                        (Obstacle::new_random(&mut rng, WIDTH, HEIGHT))
                    },
                    |(obstacle)| {
                        let prm = Arc::clone(&prm_original);
                        async move {
                            let mut prm = prm.lock().unwrap();
                            prm.add_obstacle(obstacle, num_threads).await;
                            prm.remove_obstacle(obstacle, num_threads).await;
                        }
                    },
                    criterion::BatchSize::LargeInput,
                );
            },
        );
    }
    // Use a loop to create benchmarks for each number of threads
    let mut prm_original = Arc::new(Mutex::new(precompute_prm(true)));
    for &num_threads in &num_threads_list {
        group.bench_with_input(
            BenchmarkId::new("Viable edges, Threads", num_threads),
            &num_threads,
            |b, &num_threads| {
                b.to_async(Runtime::new().unwrap()).iter_batched(
                    || {
                        (Obstacle::new_random(&mut rng, WIDTH, HEIGHT))
                    },
                    |(obstacle)| {
                        let prm = Arc::clone(&prm_original);
                        async move {
                            let mut prm = prm.lock().unwrap();
                            prm.add_obstacle(obstacle, num_threads).await;
                            prm.remove_obstacle(obstacle, num_threads).await;
                        }
                    },
                    criterion::BatchSize::LargeInput,
                );
            },
        );
    }
    group.finish()
}

// Define the criterion group and criterion main functions
criterion_group!(prm_benchmarks, benchmark_parallel_prm, benchmark_add_obstacle);
criterion_main!(prm_benchmarks);
