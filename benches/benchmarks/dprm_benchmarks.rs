#![allow(unused)]
use criterion::BenchmarkId;
use criterion::{criterion_group, criterion_main, Criterion};
use dynamic_prm::prelude::*;
use geo::Rect;
use pathfinding::matrix::directions::S;
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

// const VERTICES: usize = 30000; // 10000;
const THREADS: usize = 8;
const OBSTACLES: usize = 100;
const WIDTH: usize = 150;
const HEIGHT: usize = 150;
const SEED: [u8; 32] = [0u8; 32];
const OTHER_SEED: [u8; 32] = [1u8; 32];
const VERTICES_LIST: [usize; 4] = [10000, 20000, 40000, 80000];
// const VERTICES_LIST: [usize; 4] = [100, 200, 400, 800];

fn cfg(vertices: usize) -> PrmConfig {
    PrmConfig::new(vertices, WIDTH, HEIGHT, SEED, 4)
}

fn obstacles() -> ObstacleSet {
    ObstacleSet::new_random(
        OBSTACLES,
        10.0,
        1.0,
        0.0,
        0.0,
        (WIDTH as f64) - 1.0,
        (HEIGHT as f64) - 1.0,
        &mut ChaCha8Rng::from_seed(OTHER_SEED),
    )
}

async fn make_dprm(vertices: usize) -> DPrm {
    // Parameters common to all benchmarks:
    DPrm::from_cfg(cfg(vertices), obstacles()).await
}

async fn prm(vertices: usize, obstacles: ObstacleSet) -> Prm {
    Prm::from_cfg(cfg(vertices), obstacles).await
} 

fn benchmark_steps(c: &mut Criterion) {
    let mut group = c.benchmark_group(format!("DPrm vs. Prm, {} threads", THREADS));
    
    // Use a loop to create benchmarks for each number of threads
    for &vertices in &VERTICES_LIST {
        // Precompute dprm
        let dprm = Runtime::new().unwrap().block_on(make_dprm(vertices));
        // Benchmark the obstacle insertion
        let extra_obstacle: Obstacle = Obstacle{rect: Rect::new((70.0, 70.0), (80.0, 80.0)), id: (OBSTACLES+1) as u128};
        group.bench_with_input(
            BenchmarkId::new("DPrm Obstacle Insertion", vertices),
            &vertices,
            |b, &vertices| {
                b.to_async(Runtime::new().unwrap()).iter_batched_ref(
                    || {},
                    |_| dprm.find_blocked_by_obstacle(extra_obstacle.clone()),
                    criterion::BatchSize::SmallInput,
                );
            },
        );

        // Generate ObstacleSet
        let mut obstacles = obstacles();
        group.bench_with_input(
            BenchmarkId::new("Full Prm Compute", vertices),
            &vertices,
            |b, &vertices| {
                b.to_async(Runtime::new().unwrap()).iter_batched_ref(
                    || {},
                    |_| prm(vertices, obstacles.clone()),
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }
}

// Define the criterion group and criterion main functions
criterion_group!{
    name = dprm_benchmarks;
    config = Criterion::default().sample_size(10);
    targets = benchmark_steps
} // benchmark_parallel_prm
criterion_main!(dprm_benchmarks);
