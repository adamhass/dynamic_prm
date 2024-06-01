#![allow(unused)]
use geo::{Contains, EuclideanDistance, Intersects, Line, Point, Rect};
// use pathfinding::directed::astar::astar;
use plotters::prelude::*;
use rand::{prelude::*, seq::index};
use rand_chacha::ChaCha8Rng;
use std::{collections::HashMap, sync::{Arc, Mutex, RwLock}};
use crate::prelude::*;

const DIMENSIONS: usize = 2;

// Prm stores all edges in viable edges
#[derive(Clone)]
pub struct DPrm {
    pub vertices: Arc<Vec<Vertex>>,
    pub edges: Arc<Vec<Edge>>,
    pub viable_edges: Arc<Vec<Edge>>,
    pub obstacles: Arc<ObstacleSet>,
    pub blocked_per_obstacle: Arc<HashMap<ObstacleId, Vec<usize>>>,
    pub cfg: PrmConfig,
}

impl DPrm {
    pub fn new(prm: Prm) -> DPrm {
        DPrm {
            vertices: prm.vertices,
            edges: prm.edges,
            viable_edges: prm.viable_edges,
            obstacles: prm.obstacles,
            blocked_per_obstacle: Arc::new(HashMap::new()),
            cfg: prm.cfg,
        }
    }

    pub fn get_prm(&self) -> Prm {
        Prm {
            vertices: self.vertices.clone(),
            edges: self.edges.clone(),
            viable_edges: self.viable_edges.clone(),
            obstacles: self.obstacles.clone(),
            cfg: self.cfg.clone(),
        }
    }

    pub fn increment_seed(&self, increment: u8) -> () {
        let mut seed = self.cfg.seed.lock().unwrap(); // Borrow a mutable reference
        for i in 0..seed.len() {
            seed[i] = seed[i].wrapping_add(increment);
        }
    }

    pub fn get_rng(&self) -> ChaCha8Rng {
        ChaCha8Rng::from_seed((self.cfg.seed.lock().unwrap()).clone())
    }

    fn generate_vertices(&self, n: usize, width: usize, height: usize) -> Vec<Point<f64>> {
        let mut rng = self.get_rng();
        let mut vertices = Vec::new();
        while vertices.len() < n {
            vertices.push(Point::new(
                rng.gen_range(0.0..width as f64),
                rng.gen_range(0.0..height as f64),
            ));
        }
        vertices
    }

    pub async fn update_viable_edges_and_vertices(&mut self, threads: usize) {
        let (vertices, viable_edges) = self.generate_viable_edges_and_vertices(threads).await;
        self.vertices = Arc::new(vertices);
        self.viable_edges = Arc::new(viable_edges);
    }

    pub async fn generate_viable_edges_and_vertices(&self, threads: usize) -> (Vec<Vertex>, Vec<Edge>) {
        // Create parallel executors
        let chunk_size = self.cfg.num_vertices / threads;
        let mut handles = Vec::new();
        for i in 0..threads {
            let start = i * chunk_size;
            let end = if i == threads - 1 {
                self.cfg.num_vertices
            } else {
                (i + 1) * chunk_size
            };
            let clone = self.clone();
            let handle = tokio::spawn(async move { clone.viable_edges_worker(start, end).await });

            handles.push(handle);
        }
        // Collect all results
        let mut all_vertices = Vec::new();
        let mut all_viable_edges = Vec::new();
        for handle in handles {
            match handle.await {
                Ok((vertices, viable_edges)) => {
                    all_vertices.extend(vertices);
                    all_viable_edges.extend(viable_edges);
                }
                Err(e) => {
                    eprintln!("Error: {:?}", e);
                }
            }
        }
        (all_vertices, all_viable_edges)
    }

    pub async fn viable_edges_worker(&self, start: usize, end: usize) -> (Vec<Vertex>, Vec<Edge>) {
        let mut points = self.generate_vertices(end, self.cfg.width, self.cfg.height);
        let mut vertices = Vec::new();
        let mut edges = Vec::new();
        let gamma = gamma_prm(self.cfg.num_vertices, DIMENSIONS, self.cfg.width);
        for i in start..end {
            let p1 = points[i];
            vertices.push(Vertex {
                point: p1,
                index: i,
            });
            for j in i + 1..vertices.len() {
                let length = p1.euclidean_distance(&points[j]);
                if length < gamma {
                    let line = Line::new(p1, points[j].clone());
                    edges.push(Edge{line, length, points: (i, j)});
                }
            }
        }
        (vertices, edges)
    }

    /// Updates self to be an accurate representation of all current obstacles.
    pub async fn find_all_blocked(&self, threads: usize) -> (HashMap<ObstacleId, Vec<EdgeIndex>>, Vec<Edge>){
        let mut handles = Vec::new();
        for o in &self.obstacles.obstacles {
            let clone = self.clone();
            let obstacle = o.clone();
            let handle = tokio::spawn(async move { clone.find_blocked_by_obstacle(obstacle, threads).await });

            handles.push((handle, o.id()));
        }
        let mut all_blocked: Vec<EdgeIndex> = Vec::new();
        let mut blocked_per_obstacle: HashMap<ObstacleId, Vec<EdgeIndex>> = HashMap::new();
        for (handle, id) in handles {
            match handle.await {
                Ok(e_index) => {
                    all_blocked.extend(&e_index);
                    blocked_per_obstacle.insert(id, e_index);
                }
                Err(e) => {
                    eprintln!("Error: {:?}", e);
                }
            }
        }
        let mut edges: Vec<Edge> = Vec::new();
        all_blocked.sort();
        all_blocked.dedup();
        for i in all_blocked {
            edges.push(self.viable_edges[i].clone());
        }
        (blocked_per_obstacle, edges)
    }

    pub async fn update_all_blocked(&mut self, threads: usize) {
        let (blocked_per_obstacle, edges) = self.find_all_blocked(threads).await;
        self.edges = Arc::new(edges);
        self.blocked_per_obstacle = Arc::new(blocked_per_obstacle);
    }

    // Returns a sorted deduplicated Vec of blocked EdgeIndices
    pub async fn recompute_all_blocked(&self, threads: usize) -> Vec<EdgeIndex> {
        let mut v: Vec<EdgeIndex> = self.blocked_per_obstacle.values()
            .flatten()
            .map(|i| *i)
            .collect();
        v.sort();
        v.dedup();
        v
    }

    pub async fn find_blocked_by_obstacle(&self, obstacle: Obstacle, threads: usize) -> Vec<usize> {
        let n = self.viable_edges.len();
        let chunk_size = (n + threads - 1) / threads;
        let mut handles = Vec::new();
        for i in 0..threads {
            let start = i * chunk_size;
            let end = ((i + 1) * chunk_size).min(n);
            let clone = self.clone();
            let handle = tokio::spawn(async move { clone.find_blocked_by_obstacle_worker(start, end, obstacle.clone()).await });

            handles.push(handle);
        }
        // Collect all results
        let mut blocked_edges = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(edges) => {
                    blocked_edges.extend(edges);
                }
                Err(e) => {
                    eprintln!("Error: {:?}", e);
                }
            }
        }
        blocked_edges
    }

    async fn find_blocked_by_obstacle_worker(&self, start: EdgeIndex, end: EdgeIndex, obstacle: Obstacle) -> Vec<EdgeIndex> {
        let mut blocked = Vec::new();
        for i in start..end {
            let edge = &self.viable_edges[i];
            if obstacle.intersects(&edge.line) {
                blocked.push(i);
            }
        }
        blocked
    }

    async fn find_unblocked_by_obstacle(&self, oid: ObstacleId) -> Vec<EdgeIndex> {
        todo!();
        // let mut unblocked = Vec::new();
    }


}