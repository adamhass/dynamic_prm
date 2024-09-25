#![allow(unused)]
use geo::{Contains, EuclideanDistance, Intersects, Line, Point, Rect};
// use pathfinding::directed::astar::astar;
use plotters::prelude::*;
use rand::{prelude::*, seq::index};
use rand_chacha::ChaCha8Rng;
use std::{collections::HashMap, f64::consts::PI, sync::{Arc, Mutex, RwLock}};
use crate::prelude::*;

const DIMENSIONS: usize = 2;

// Prm stores all edges in viable edges
#[derive(Clone)]
pub struct DPrm {
    pub vertices: Arc<Vec<Vertex>>,
    pub edges: Arc<Vec<Edge>>,
    pub viable_edges: Arc<Vec<Edge>>,
    pub obstacles: Arc<ObstacleSet>,
    pub blocked_per_obstacle: HashMap<ObstacleId, Vec<usize>>,
    pub cfg: PrmConfig,
}

impl DPrm {
    pub fn new(prm: Prm) -> DPrm {
        DPrm {
            vertices: prm.vertices,
            edges: prm.edges,
            viable_edges: prm.viable_edges,
            obstacles: prm.obstacles,
            blocked_per_obstacle: HashMap::new(),
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

    pub fn print(&self) {
        println!(
            "Vertices: {}, Edges: {}, Viable Edges: {}, Blocked Edges: {}, Obstacles: {}",
            self.vertices.len(),
            self.edges.len(),
            self.viable_edges.len(),
            self.get_all_blocked().len(),
            self.obstacles.obstacles.len()
        );
    }

    fn max_radius(&self) -> f64 {
        let d = DIMENSIONS as f64;
        let id = 1.0/d;
        let n = self.cfg.num_vertices as f64;
        let area = self.cfg.width as f64 * self.cfg.height as f64;
        let mu_free = area*0.5; // Free space
        let zeta = PI; // Area of the unit circle
        let a = 2.0 * (1.0 + 1.0/d);
        let b = mu_free/zeta;
        let gamma = a.powf(id)*b.powf(id);
        return gamma * (n.log(d)/n).powf(id)
    }

    pub fn get_nearest(&self, point: Point<f64>) -> Vertex {
        let mut min_distance = f64::MAX;
        let mut nearest = self.vertices[0].clone();
        for v in self.vertices.iter() {
            let distance = v.point.euclidean_distance(&point);
            if distance < min_distance &&
                !self.obstacles.contains(&v.point) {
                min_distance = distance;
                nearest = v.clone();
            }
        }
        nearest
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

    pub fn update_edges(&mut self) {
        // Indices of blocked edges in viable_edges
        let blocked = self.get_all_blocked();
        // Return viable_edges without blocked edges
        self.edges = Arc::new(self.viable_edges.iter().enumerate().filter(|(i, _)| !blocked.contains(i)).map(|(_, e)| e.clone()).collect())
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
        let radius = self.max_radius();
        println!("start {} end {}", start, end);
        for i in start..end {
            let p1 = points[i];
            vertices.push(Vertex {
                point: p1,
                index: i,
            });
            for j in 0..points.len() {
                let length = p1.euclidean_distance(&points[j]);
                if length < radius {
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
        self.blocked_per_obstacle = blocked_per_obstacle;
        self.update_edges();
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

    async fn remove_obstacle(&mut self, oid: ObstacleId) -> Vec<EdgeIndex> {
        let mut unblocked = self.blocked_per_obstacle.remove(&oid).unwrap();
        let all_blocked = self.get_all_blocked();
        unblocked.retain( |i| {
            all_blocked.contains(i)
        });
        unblocked
    }

    fn get_all_blocked(&self) -> Vec<EdgeIndex> {
        let mut blocked = Vec::new();
        for edges in self.blocked_per_obstacle.values() {
            blocked.extend(edges);
        }
        blocked.sort();
        blocked.dedup();
        blocked
    }

    pub fn plot(&self, file_name: String, path: Option<AstarPath>) -> () {
        let filename = format!("output/{}.png", file_name);
        // Create a drawing area
        let root = BitMapBackend::new(&filename, (2000_u32, 2000_u32)).into_drawing_area();
        root.fill(&WHITE).unwrap();

        // Define the chart
        let mut chart = ChartBuilder::on(&root)
            .caption("Edges and Obstacles", ("sans-serif", 50))
            .x_label_area_size(30)
            .y_label_area_size(30)
            .build_cartesian_2d(0.0..(self.cfg.width as f64), 0.0..(self.cfg.height as f64))
            .unwrap();

        chart.configure_mesh().draw().unwrap();

        // Draw obstacles
        chart
            .draw_series(self.obstacles.obstacles.iter().map(|o| o.rectangle()))
            .unwrap();
        root.present().unwrap();

        // Draw vertices
        chart
            .draw_series(
                (*self.vertices)
                    .clone()
                    .into_iter()
                    .map(|v| Circle::new(v.point.0.x_y(), 3, BLACK)),
            )
            .unwrap();

        // Draw edges
        chart
            .draw_series(
                self.edges.iter().map(|edge| {
                    PathElement::new(vec![edge.line.start.x_y(), edge.line.end.x_y()], CYAN)
                }),
            )
            .unwrap()
            .label("Edge")
            .legend(|(x, y)| PathElement::new([(x, y), (x + 20, y)], CYAN));
            
        // Draw blocked edges
        chart
            .draw_series(
                self.get_all_blocked().iter().map(|edge_index| {
                    let edge = &self.viable_edges[*edge_index];
                    PathElement::new(vec![edge.line.start.x_y(), edge.line.end.x_y()], YELLOW)
                }),
            )
            .unwrap()
            .label("Edge")
            .legend(|(x, y)| PathElement::new([(x, y), (x + 20, y)], YELLOW));
 
        // Draw path 
        if let Some((path, _)) = path {
            // Draw edges
            let mut pv = path[0].clone();
            chart
                .draw_series(
                    path.iter().map(|v| {
                        let e = PathElement::new(vec![pv.point.x_y(), v.point.x_y()], BLACK);
                        pv = v.clone();
                        e
                    }),
                )
                .unwrap()
                .label("Edge")
                .legend(|(x, y)| PathElement::new([(x, y), (x + 20, y)], BLACK));
        }
    }
}