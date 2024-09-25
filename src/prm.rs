// #![allow(unused)]
// use geo::{Contains, EuclideanDistance, Intersects, Line, Point, Rect};
// // use pathfinding::directed::astar::astar;
// use crate::prelude::*;
// use plotters::prelude::*;
// use rand::{prelude::*, seq::index};
// use rand_chacha::ChaCha8Rng;
// use std::{
//     collections::HashMap,
//     sync::{Arc, Mutex, RwLock},
// };

// const DIMENSIONS: usize = 2;
// pub const GAMMA: f64 = 12.0 * 2.49;
// pub fn gamma_prm(_: usize, _: usize, _: usize) -> f64 {
//     3.664905117183084
//     // let n_f64 = n as f64;
//     // let w_f64 = w as f64;
//     // let d_f64 = d as f64;
//     // let log_n = n_f64.ln();
//     // (log_n / n_f64).powf(1.0 / d_f64)*GAMMA
// }

// // Prm Shuffles edges from 'viable' to 'edges' back and forth
// #[derive(Clone)]
// pub struct Prm {
//     pub vertices: Arc<Vec<Vertex>>,
//     pub edges: Arc<Vec<Edge>>,
//     pub viable_edges: Arc<Vec<Edge>>,
//     pub obstacles: Arc<ObstacleSet>,
//     pub cfg: PrmConfig,
// }

// impl Prm {
//     pub fn new(cfg: PrmConfig, n_obstacles: usize) -> Prm {
//         let mut rng = ChaCha8Rng::from_seed(*cfg.seed.lock().unwrap());
//         let obstacles = Arc::new(ObstacleSet::new_random(
//             n_obstacles,
//             cfg.width,
//             cfg.height,
//             &mut rng,
//         ));
//         {
//             Prm {
//                 vertices: Arc::new(Vec::new()),
//                 edges: Arc::new(Vec::new()),
//                 viable_edges: Arc::new(Vec::new()),
//                 obstacles,
//                 cfg,
//             }
//         }
//     }

//     pub fn get_nearest(&self, point: Point<f64>) -> Vertex {
//         let mut min_distance = f64::MAX;
//         let mut nearest = self.vertices[0].clone();
//         for v in self.vertices.iter() {
//             let distance = v.point.euclidean_distance(&point);
//             if distance < min_distance && !self.obstacles.contains(&v.point) {
//                 min_distance = distance;
//                 nearest = v.clone();
//             }
//         }
//         nearest
//     }

//     pub fn print(&self) {
//         println!(
//             "Vertices: {}, Edges: {}, Viable Edges: {}, Obstacles: {}",
//             self.vertices.len(),
//             self.edges.len(),
//             self.viable_edges.len(),
//             self.obstacles.obstacles.len()
//         );
//     }

//     pub fn increment_seed(&self) {
//         let mut seed = self.cfg.seed.lock().unwrap(); // Borrow a mutable reference
//         for i in 0..seed.len() {
//             seed[i] = seed[i].wrapping_add(1);
//         }
//     }

//     pub fn get_rng(&self) -> ChaCha8Rng {
//         ChaCha8Rng::from_seed(*(self.cfg.seed.lock().unwrap()))
//     }

//     fn generate_vertices(&self, n: usize, width: usize, height: usize) -> Vec<Point<f64>> {
//         let mut rng = self.get_rng();
//         let mut vertices = Vec::new();
//         while vertices.len() < n {
//             vertices.push(Point::new(
//                 rng.gen_range(0.0..width as f64),
//                 rng.gen_range(0.0..height as f64),
//             ));
//         }
//         vertices
//     }

//     pub async fn compute(&mut self, num_threads: usize) {
//         let (v, e, viable_edges) = self.run_prm(num_threads).await;
//         self.update_vertices_and_edges(v, e, viable_edges);
//     }

//     pub fn update_vertices_and_edges(
//         &mut self,
//         vertices: Vec<Vertex>,
//         edges: Vec<Edge>,
//         viable_edges: Vec<Edge>,
//     ) {
//         self.vertices = Arc::new(vertices);
//         self.edges = Arc::new(edges);
//         self.viable_edges = Arc::new(viable_edges);
//     }

//     /// Returns the set of vertices and edges to be removed
//     pub async fn remove_edges(&self, obstacle: Obstacle, num_threads: usize) -> Vec<Edge> {
//         let chunk_size = self.edges.len() / num_threads;
//         let mut handles = Vec::new();
//         for i in 0..num_threads {
//             let start = i * chunk_size;
//             let end = if i == num_threads - 1 {
//                 self.edges.len()
//             } else {
//                 (i + 1) * chunk_size
//             };
//             let clone = self.clone();
//             let handle =
//                 tokio::spawn(async move { clone.remove_edges_worker(start, end, obstacle).await });

//             handles.push(handle);
//         }

//         // Collect all results
//         let mut remove_edges = Vec::new();
//         for handle in handles {
//             match handle.await {
//                 Ok(edges) => {
//                     remove_edges.extend(edges);
//                 }
//                 Err(e) => {
//                     eprintln!("Error: {:?}", e);
//                 }
//             }
//         }
//         remove_edges
//     }

//     pub async fn add_obstacle(&mut self, obstacle: Obstacle, num_threads: usize) {
//         let blocked_edges = self.remove_edges(obstacle, num_threads).await;
//         // println!("Removing {} edges", remove_edges.len());
//         let mut edges = (*self.edges).clone();
//         edges.retain(|e| !&blocked_edges.contains(e));
//         if self.cfg.use_viable_edges {
//             let mut viable_edges = (*self.viable_edges).clone();
//             viable_edges.extend(blocked_edges);
//             self.viable_edges = Arc::new(viable_edges);
//         }
//         self.edges = Arc::new(edges);
//         let mut obstacles = (*self.obstacles).clone();
//         obstacles.obstacles.push(obstacle);
//         self.obstacles = Arc::new(obstacles);
//     }

//     // Returns subset of edges/vertices to be removed
//     async fn remove_edges_worker(&self, start: usize, end: usize, obstacle: Obstacle) -> Vec<Edge> {
//         let mut remove_edges = Vec::new();
//         // println!("Worker comparing {} edges", end-start);
//         for i in start..end {
//             if obstacle.intersects(&self.edges[i].line) {
//                 remove_edges.push(self.edges[i].clone());
//             }
//         }
//         // println!("Worker found {} edges to remove", remove_edges.len());
//         remove_edges
//     }

//     // Returns subset of viable_edges indicies that can be added to edges
//     async fn create_edges_worker(
//         &self,
//         worker_index: usize,
//         num_workers: usize,
//         obstacle: Obstacle,
//     ) -> (Vec<usize>) {
//         let mut new_edges = Vec::new();
//         let n = self.viable_edges.len();
//         let chunk_size = (n + num_workers - 1) / num_workers;
//         let start = worker_index * chunk_size;
//         let end = ((worker_index + 1) * chunk_size).min(n); // Ensure end does not exceed n

//         for i in start..end {
//             let e = self.viable_edges[i].clone();
//             if obstacle.intersects(&e.line) && !self.obstacles.intersects(&e.line) {
//                 new_edges.push(i)
//             }
//         }
//         // println!("Worker found {} new edges", new_edges.len());
//         new_edges
//     }

//     /// Removes an obstacle from the PRM and computes the new set of obstacles
//     pub async fn remove_obstacle(&mut self, obstacle: Obstacle, num_threads: usize) {
//         let mut obstacles = (*self.obstacles).clone();
//         obstacles.remove(&obstacle);
//         self.obstacles = Arc::new(obstacles);

//         if !self.cfg.use_viable_edges {
//             // Rerun PRM* and return
//             return self.compute(num_threads).await;
//         }

//         let new_edges = self.find_new_edges(obstacle, num_threads).await;
//         // println!("Adding {} edges", new_edges.len());
//         // Move the edges from viable to edges:
//         let mut edges = (*self.edges).clone();
//         let mut viable_edges = (*self.viable_edges).clone();
//         for (i, index) in new_edges.iter().enumerate() {
//             edges.push(viable_edges.remove(index - i));
//         }
//         self.edges = Arc::new(edges);
//         self.viable_edges = Arc::new(viable_edges);
//     }

//     /// obstacle must already be removed the self.obstacles
//     pub async fn find_new_edges(&self, obstacle: Obstacle, num_threads: usize) -> Vec<usize> {
//         assert!(!self.obstacles.obstacles.contains(&obstacle));

//         let mut handles = Vec::new();
//         for i in 0..num_threads {
//             let clone = self.clone();
//             let handle =
//                 tokio::spawn(
//                     async move { clone.create_edges_worker(i, num_threads, obstacle).await },
//                 );
//             handles.push(handle);
//         }

//         // Collect all results
//         let mut new_edges = Vec::new();
//         for handle in handles {
//             match handle.await {
//                 Ok(edges) => {
//                     new_edges.extend(edges);
//                 }
//                 Err(e) => {
//                     eprintln!("Error: {:?}", e);
//                 }
//             }
//         }
//         new_edges
//     }

//     async fn prm_worker(&self, start: usize, end: usize) -> (Vec<Vertex>, Vec<Edge>, Vec<Edge>) {
//         let vertices = self.generate_vertices(end, self.cfg.width, self.cfg.height);
//         let mut vs = Vec::new();
//         let gamma = gamma_prm(self.cfg.num_vertices, DIMENSIONS, self.cfg.width);
//         let mut edges = Vec::new();
//         let mut viable_edges = Vec::new();
//         for i in start..end {
//             let p1 = vertices[i];
//             // if self.obstacles.contains(&p1) && !self.cfg.use_viable_edges {
//             //    continue;
//             //}
//             vs.push(Vertex {
//                 point: p1,
//                 index: i,
//             });
//             for (j, p2) in vertices.iter().enumerate() {
//                 if self.obstacles.contains(&p1) && !self.cfg.use_viable_edges {
//                     continue;
//                 }
//                 let distance = p1.euclidean_distance(p2);
//                 if distance < gamma && p1 != *p2 {
//                     let line = Line::new(p1, *p2);
//                     if !self.obstacles.intersects(&line) {
//                         edges.push(Edge {
//                             line,
//                             length: distance,
//                             points: (i, j),
//                         });
//                     } else if self.cfg.use_viable_edges {
//                         viable_edges.push(Edge {
//                             line,
//                             length: distance,
//                             points: (i, j),
//                         });
//                     }
//                 }
//             }
//         }
//         (vs, edges, viable_edges)
//     }

//     pub async fn run_prm(&self, num_threads: usize) -> (Vec<Vertex>, Vec<Edge>, Vec<Edge>) {
//         // Create parallel executors
//         let chunk_size = self.cfg.num_vertices / num_threads;
//         let mut handles = Vec::new();
//         for i in 0..num_threads {
//             let start = i * chunk_size;
//             let end = if i == num_threads - 1 {
//                 self.cfg.num_vertices
//             } else {
//                 (i + 1) * chunk_size
//             };
//             let clone = self.clone();
//             let handle = tokio::spawn(async move { clone.prm_worker(start, end).await });

//             handles.push(handle);
//         }

//         // Collect all results
//         let mut all_vertices = Vec::new();
//         let mut all_edges = Vec::new();
//         let mut all_viable_edges = Vec::new();
//         for handle in handles {
//             match handle.await {
//                 Ok((vertices, edges, viable_edges)) => {
//                     all_vertices.extend(vertices);
//                     all_edges.extend(edges);
//                     all_viable_edges.extend(viable_edges);
//                 }
//                 Err(e) => {
//                     eprintln!("Error: {:?}", e);
//                 }
//             }
//         }
//         (all_vertices, all_edges, all_viable_edges)
//     }
// }
