use geo::{Contains, EuclideanDistance, Intersects, Line, Point, Rect};
// use pathfinding::directed::astar::astar;
use plotters::prelude::*;
use rand::{prelude::*, seq::index};
use rand_chacha::ChaCha8Rng;
use std::sync::Arc;

const DIMENSIONS: usize = 2;
pub const GAMMA: f64 = 12.0 * 2.49;
pub fn gamma_prm(_: usize, _: usize, _: usize) -> f64 {
    3.664905117183084
    // let n_f64 = n as f64;
    // let w_f64 = w as f64;
    // let d_f64 = d as f64;
    // let log_n = n_f64.ln();
    // (log_n / n_f64).powf(1.0 / d_f64)*GAMMA
}

#[derive(Clone)]
pub struct PrmConfig {
    pub num_vertices: usize,
    pub width: usize,
    pub height: usize,
    pub seed: Arc<[u8; 32]>,
}

#[derive(Clone)]
pub struct Prm {
    pub vertices: Arc<Vec<Vertex>>,
    pub edges: Arc<Vec<Edge>>,
    pub obstacles: Arc<ObstacleSet>,
    pub cfg: PrmConfig,
}

impl Prm {
    pub fn new(cfg: PrmConfig, obstacles: Arc<ObstacleSet>) -> Prm {
        {
            Prm{
                vertices: Arc::new(Vec::new()),
                edges: Arc::new(Vec::new()),
                obstacles,
                cfg,
            }
        }
    }

    pub fn update_vertices_and_edges(&mut self, vertices: Vec<Vertex>, edges: Vec<Edge>) -> () {
        self.vertices = Arc::new(vertices);
        self.edges = Arc::new(edges);
    }

    /// Returns the set of vertices and edges to be removed
    pub async fn add_obstacle(&self, obstacle: Obstacle, num_threads: usize) -> Vec<Edge> {
        let chunk_size = self.edges.len() / num_threads;
        let mut handles = Vec::new();
        for i in 0..num_threads {
            let start = i * chunk_size;
            let end = if i == num_threads - 1 {
                self.edges.len()
            } else {
                (i + 1) * chunk_size
            };
            let clone = self.clone();
            let handle = tokio::spawn(async move { clone.remove_edges_worker(start, end, obstacle).await });

            handles.push(handle);
        }

        // Collect all results
        let mut remove_edges = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(edges) => {
                    remove_edges.extend(edges);
                }
                Err(e) => {
                    eprintln!("Error: {:?}", e);
                }
            }
        }
        remove_edges
    }

    // Returns subset of edges/vertices to be removed
    async fn remove_edges_worker(&self, start: usize, end: usize, obstacle: Obstacle) -> Vec<Edge> {
        let mut remove_edges = Vec::new();
        for i in start..end {
            if obstacle.intersects(&self.edges[i].line) {
                remove_edges.push(self.edges[i].clone());
            }
        }
        remove_edges
    }

    async fn prm_worker(&self, start: usize, end: usize) -> (Vec<Vertex>, Vec<Edge>) {
        let mut rng = ChaCha8Rng::from_seed(*self.cfg.seed);
        let vertices = generate_vertices(end, self.cfg.width, self.cfg.height, &mut rng);
        let mut vs = Vec::new();
        let gamma = gamma_prm(self.cfg.num_vertices, DIMENSIONS, self.cfg.width);
        let mut edges = Vec::new();
        for i in start..end {
            let p1 = vertices[i];
            if self.obstacles.contains(&p1) {
                continue;
            }
            vs.push(Vertex {
                point: p1,
                index: i,
            });
            for (j, p2) in vertices.iter().enumerate() {
                if self.obstacles.contains(&p1) {
                    continue;
                }
                let distance = p1.euclidean_distance(p2);
                if distance < gamma && p1 != *p2 {
                    let line = Line::new(p1, p2.clone());
                    if !self.obstacles.intersects(&line) {
                        edges.push(Edge {
                            line,
                            length: distance,
                            points: (i, j),
                        });
                    }
                }
            }
        }
        (vs, edges)
    }

    pub async fn run_prm(&self, num_threads: usize) -> (Vec<Vertex>, Vec<Edge>) {
        // Create parallel executors
        let chunk_size = self.cfg.num_vertices / num_threads;
        let mut handles = Vec::new();
        for i in 0..num_threads {
            let start = i * chunk_size;
            let end = if i == num_threads - 1 {
                self.cfg.num_vertices
            } else {
                (i + 1) * chunk_size
            };
            let clone = self.clone();
            let handle = tokio::spawn(async move { clone.prm_worker(start, end).await });

            handles.push(handle);
        }

        // Collect all results
        let mut all_vertices = Vec::new();
        let mut all_edges = Vec::new();
        for handle in handles {
            match handle.await {
                Ok((vertices, edges)) => {
                    all_vertices.extend(vertices);
                    all_edges.extend(edges);
                }
                Err(e) => {
                    eprintln!("Error: {:?}", e);
                }
            }
        }
        (all_vertices, all_edges)
    }

}

#[derive(Clone)]
pub struct Edge {
    pub line: Line<f64>,
    pub length: f64,
    pub points: (usize, usize),
}

#[derive(Clone)]
pub struct Vertex {
    pub point: Point<f64>,
    pub index: usize,
}

#[derive(Clone, Copy)]
pub struct Obstacle {
    pub rect: Rect<f64>,
}

impl Obstacle {
    pub fn new_random(rng: &mut ChaCha8Rng, w: usize, h: usize) -> Obstacle {
        let x_min = rng.gen_range(0.0..(w as f64 - 1.0)) as f64;
        let y_min = rng.gen_range(0.0..(h as f64 - 1.0)) as f64;
        let width = rng.gen_range(1.0..(w as f64 / 10.0)) as f64;
        let height = rng.gen_range(1.0..(h as f64 / 10.0)) as f64;
        let rect = Rect::new((x_min, y_min), (x_min + width, y_min + height));
        Obstacle { rect }
    }

    fn contains(&self, point: &Point<f64>) -> bool {
        self.rect.contains(point)
    }

    fn intersects(&self, edge: &Line<f64>) -> bool {
        self.rect.intersects(edge)
    }

    pub fn rectangle(&self) -> Rectangle<(f64, f64)> {
        Rectangle::new(
            [self.rect.min().x_y(), self.rect.max().x_y()],
            (&RED).filled(),
        )
    }
}

pub struct ObstacleSet {
    pub obstacles: Vec<Obstacle>,
}

impl ObstacleSet {
    pub fn new_random(n: usize, width: usize, height: usize, rng: &mut ChaCha8Rng) -> ObstacleSet {
        let mut obstacles = Vec::new();
        while obstacles.len() < n {
            obstacles.push(Obstacle::new_random(rng, width, height));
        }
        ObstacleSet { obstacles }
    }

    fn contains(&self, point: &Point<f64>) -> bool {
        self.obstacles.iter().any(|o| o.contains(point))
    }

    fn intersects(&self, edge: &Line<f64>) -> bool {
        self.obstacles.iter().any(|o| o.intersects(edge))
    }
}

fn generate_vertices(
    n: usize,
    width: usize,
    height: usize,
    rng: &mut ChaCha8Rng,
) -> Vec<Point<f64>> {
    let mut vertices = Vec::new();
    while vertices.len() < n {
        vertices.push(Point::new(
            rng.gen_range(0.0..width as f64),
            rng.gen_range(0.0..height as f64),
        ));
    }
    vertices
}