/*
#![allow(unused)]
use criterion::BenchmarkId;
use criterion::{criterion_group, criterion_main, Criterion};
use dynamic_prm::prelude::*;
use geo::Point;
use pathfinding::matrix::directions::S;
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

const VERTICES: usize = 30000;
const THREAD_LIST: [usize; 7] = [1, 2, 4, 8, 16, 32, 64];
const OBSTACLES: usize = 100;
const WIDTH: usize = 150;
const HEIGHT: usize = 150;
const SEED: [u8; 32] = [0u8; 32];
const OTHER_SEED: [u8; 32] = [1u8; 32];

fn init_dprm() -> DPrm {
    // Parameters common to all benchmarks:
    let cfg = PrmConfig::new(VERTICES, WIDTH, HEIGHT, SEED);
    DPrm::from_cfg(Prm::new(cfg, OBSTACLES))
}

fn precompute_prm(viable_edges: bool) -> DPrm {
    let mut dprm = init_dprm();
    dprm.cfg.use_viable_edges = viable_edges;
    // Runtime::new().unwrap().block_on(dprm.compute(4));
    Runtime::new()
        .unwrap()
        .block_on(dprm.update_viable_edges_and_vertices(4));
    Runtime::new().unwrap().block_on(dprm.update_all_blocked(4));
    dprm
}

fn get_random_points(rng: &mut ChaCha8Rng) -> (Point, Point) {
    let x1 = rng.gen_range(0.0..WIDTH as f64);
    let y1 = rng.gen_range(0.0..HEIGHT as f64);
    let x2 = rng.gen_range(0.0..WIDTH as f64);
    let y2 = rng.gen_range(0.0..HEIGHT as f64);
    (Point::new(x1, y1), Point::new(x2, y2))
}

// Define a function to benchmark `parallel_prm`
fn benchmark_astar(c: &mut Criterion) {
    let mut group = c.benchmark_group(format!("Astar {} Vertices", VERTICES));
    let prm = precompute_prm(false);
    let mut astar = Astar::new(prm.clone());
    astar.init_neighbours();
    astar.optimized = true;
    let mut rng = ChaCha8Rng::from_seed(OTHER_SEED);
    group.bench_function(BenchmarkId::new("Optimized", 0), |b| {
        b.iter(|| {
            let (start, end) = get_random_points(&mut rng);
            let start = prm.get_nearest(start);
            let end = prm.get_nearest(end);
            astar.run_astar(start, end)
        });
    });

    astar.optimized = false;
    let mut rng = ChaCha8Rng::from_seed(OTHER_SEED);
    group.bench_function(BenchmarkId::new("Basic", 0), |b| {
        b.iter(|| {
            let (start, end) = get_random_points(&mut rng);
            let start = prm.get_nearest(start);
            let end = prm.get_nearest(end);
            astar.run_astar(start, end)
        });
    });
}

// Define a function to benchmark `parallel_prm`
fn benchmark_astar_updates(c: &mut Criterion) {
    let mut group = c.benchmark_group(format!("Astar {} Vertices", VERTICES));
    let prm = precompute_prm(false);
    let mut astar = Astar::new(prm.clone());
    astar.init_neighbours();
    astar.optimized = true;
    let mut rng = ChaCha8Rng::from_seed(OTHER_SEED);
    group.bench_function(BenchmarkId::new("Optimized", 0), |b| {
        b.iter(|| {
            let (start, end) = get_random_points(&mut rng);
            let start = prm.get_nearest(start);
            let end = prm.get_nearest(end);
            astar.run_astar(start, end)
        });
    });
}

criterion_group!(astar_benchmarks, benchmark_astar,);
criterion_main!(astar_benchmarks);

*/