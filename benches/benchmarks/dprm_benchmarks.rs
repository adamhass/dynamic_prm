#![allow(unused)]
use criterion::BenchmarkId;
use criterion::{criterion_group, criterion_main, Criterion};
use dynamic_prm::prelude::*;
use pathfinding::matrix::directions::S;
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

const VERTICES: usize = 30000; // 10000;
const THREAD_LIST: [usize; 3] = [4, 8, 16];
const OBSTACLES: usize = 100;
const WIDTH: usize = 150;
const HEIGHT: usize = 150;
const SEED: [u8; 32] = [0u8; 32];
const OTHER_SEED: [u8; 32] = [1u8; 32];

fn init_dprm() -> DPrm {
    // Parameters common to all benchmarks:
    let cfg = PrmConfig::new(VERTICES, WIDTH, HEIGHT, SEED);
    DPrm::new(Prm::new(cfg, OBSTACLES))
}

fn benchmark_steps(c: &mut Criterion) {
    let mut group = c.benchmark_group(format!("DPrm computations, {} Vertices", VERTICES));
    let mut dprm = init_dprm();
    let mut rng = ChaCha8Rng::from_seed(OTHER_SEED);
    // Use a loop to create benchmarks for each number of threads
    for &num_threads in &THREAD_LIST {
        group.bench_with_input(
            BenchmarkId::new("Viable edges and vertices, Threads", num_threads),
            &num_threads,
            |b, &num_threads| {
                b.to_async(Runtime::new().unwrap()).iter_batched_ref(
                    || {},
                    |_| dprm.generate_viable_edges_and_vertices(num_threads),
                    criterion::BatchSize::SmallInput,
                );
            },
        );
        Runtime::new()
            .unwrap()
            .block_on(dprm.update_viable_edges_and_vertices(4));
        group.bench_with_input(
            BenchmarkId::new("Find all blocked, Threads", num_threads),
            &num_threads,
            |b, &num_threads| {
                b.to_async(Runtime::new().unwrap()).iter_batched(
                    || {},
                    |_| dprm.find_all_blocked(num_threads),
                    criterion::BatchSize::SmallInput,
                );
            },
        );
        Runtime::new().unwrap().block_on(dprm.update_all_blocked(4));
        group.bench_with_input(
            BenchmarkId::new("Find blocked edges by obstacle, Threads", num_threads),
            &num_threads,
            |b, &num_threads| {
                b.to_async(Runtime::new().unwrap()).iter_batched(
                    || {},
                    |_| {
                        dprm.find_blocked_by_obstacle(
                            Obstacle::new_random(&mut rng, WIDTH, HEIGHT),
                            num_threads,
                        )
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }
}

// Define the criterion group and criterion main functions
criterion_group!(dprm_benchmarks, benchmark_steps,); // benchmark_parallel_prm
criterion_main!(dprm_benchmarks);
