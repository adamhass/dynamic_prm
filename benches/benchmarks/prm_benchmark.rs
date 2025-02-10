/*
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
const THREAD_LIST: [usize; 7] = [1, 2, 4, 8, 16, 32, 64];
const OBSTACLES: usize = 50;
const WIDTH: usize = 100;
const HEIGHT: usize = 100;
const SEED: [u8; 32] = [0u8; 32];
const OTHER_SEED: [u8; 32] = [1u8; 32];

fn init_prm() -> Prm {
    // Parameters common to all benchmarks:
    let cfg = PrmConfig::new(VERTICES, WIDTH, HEIGHT, SEED);
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
    let mut group = c.benchmark_group(format!("Parallel PRM {} Vertices", VERTICES));
    // Use a loop to create benchmarks for each number of threads
    for &num_threads in &THREAD_LIST {
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
    for &num_threads in &THREAD_LIST {
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
    let mut group = c.benchmark_group(format!(
        "Parallel Obstacle Insertion, {} Vertices",
        VERTICES
    ));

    // Use a loop to create benchmarks for each number of threads
    let mut rng = ChaCha8Rng::from_seed(OTHER_SEED);
    for &num_threads in &THREAD_LIST {
        let prm = precompute_prm(false);
        group.bench_with_input(
            BenchmarkId::new("Basic, Threads", num_threads),
            &num_threads,
            |b, &num_threads| {
                b.to_async(Runtime::new().unwrap()).iter(|| {
                    prm.remove_edges(Obstacle::new_random(&mut rng, WIDTH, HEIGHT), num_threads)
                });
            },
        );
    }
    let mut rng = ChaCha8Rng::from_seed(OTHER_SEED);
    for &num_threads in &THREAD_LIST {
        let prm = precompute_prm(true);
        group.bench_with_input(
            BenchmarkId::new("Basic, Threads", num_threads),
            &num_threads,
            |b, &num_threads| {
                b.to_async(Runtime::new().unwrap()).iter(|| {
                    prm.remove_edges(Obstacle::new_random(&mut rng, WIDTH, HEIGHT), num_threads)
                });
            },
        );
    }
    group.finish()
}

fn benchmark_remove_obstacle(c: &mut Criterion) {
    let mut group = c.benchmark_group(format!("Parallel Obstacle Removal, {} Vertices", VERTICES));
    let mut rng = ChaCha8Rng::from_seed(OTHER_SEED);
    let obstacle = Obstacle::new_random(&mut rng, WIDTH, HEIGHT);
    // Create a new PRM with viable edges
    let mut prm = precompute_prm(true);
    let original_obstacles = prm.obstacles.clone();

    // Insert the obstacle properly.
    Runtime::new()
        .unwrap()
        .block_on(prm.add_obstacle(obstacle.clone(), 4));

    // Re-remove the obstacle from the obstacle set so that its removal can be processed properly
    prm.obstacles = original_obstacles;

    for &num_threads in &THREAD_LIST {
        group.bench_with_input(
            BenchmarkId::new("Viable edges, Threads", num_threads),
            &num_threads,
            |b, &num_threads| {
                b.to_async(Runtime::new().unwrap()).iter_batched(
                    || {},
                    |_| prm.find_new_edges(obstacle.clone(), num_threads),
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }
    group.finish()
}

// Define the criterion group and criterion main functions
criterion_group!(
    prm_benchmarks,
    benchmark_remove_obstacle,
    benchmark_add_obstacle
); // benchmark_parallel_prm
criterion_main!(prm_benchmarks);

*/