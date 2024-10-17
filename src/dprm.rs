#![allow(unused)]
use geo::{Contains, EuclideanDistance, Intersects, Line, Point, Rect};
// use pathfinding::directed::astar::astar;
use crate::prelude::*;
use plotters::prelude::*;
use rand::{prelude::*, seq::index};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    f64::consts::PI,
    fs::File,
    io::{BufReader, BufWriter},
    sync::{Arc, Mutex, RwLock},
};
// use serde_json::{from_reader, to_writer_pretty};
use pathfinding::directed::astar::astar;

const DIMENSIONS: usize = 2;

// Prm stores all edges in viable edges
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DPrm {
    pub(crate) vertices: Arc<Vec<Vertex>>,
    pub(crate) edges: HashMap<EdgeIndex, Edge>,
    viable_edges: Arc<Vec<Edge>>,
    obstacles: Arc<ObstacleSet>,
    blocked_per_obstacle: HashMap<ObstacleId, Vec<EdgeIndex>>,
    blockings_per_edge: HashMap<EdgeIndex, usize>,
    cfg: PrmConfig,
    neighbours: Vec<Vec<(VertexIndex, Distance)>>,
}

impl DPrm {
    /*
     *** Initialization ***
     */

    /// Create a new DPrm with the given configuration and an initial ObstacleSet.
    /// Initializes viable edges and vertices.
    /// Finds all blocked edges per obstacle.
    pub async fn from_cfg(cfg: PrmConfig, obstacles: Arc<ObstacleSet>) -> DPrm {
        let mut dprm = DPrm {
            vertices: Arc::new(Vec::new()),
            edges: HashMap::new(),
            viable_edges: Arc::new(Vec::new()),
            obstacles,
            blocked_per_obstacle: HashMap::new(),
            blockings_per_edge: HashMap::new(),
            cfg,
            neighbours: Vec::new(),
        };
        dprm.initialize_viable_edges_and_vertices().await;
        dprm.initialize_all_blocked().await;
        dprm.initialize_neighbours();
        dprm
    }

    // Writes the DPrm to a binary file at the given path using Bincode.
    pub fn to_file(&self, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Create or truncate the file at the specified path
        let file = File::create(file_path)?;
        let writer = BufWriter::new(file);

        // Serialize `self` using Bincode and write to the file
        bincode::serialize_into(writer, self)?;

        println!("DPrm successfully serialized to {}", file_path);
        Ok(())
    }

    // Reads the DPrm from a binary file at the given path using Bincode.
    pub fn from_file(file_path: &str) -> Result<DPrm, Box<dyn std::error::Error>> {
        // Open the file in read-only mode
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);

        // Deserialize the binary data into a `DPrm` instance using Bincode
        let dprm = bincode::deserialize_from(reader)?;

        println!("DPrm successfully deserialized from {}", file_path);
        Ok(dprm)
    }

    /// Generates all the vertices and finds viable edges between them.
    async fn initialize_viable_edges_and_vertices(&mut self) {
        let (vertices, viable_edges) = self.generate_viable_edges_and_vertices().await;
        self.vertices = Arc::new(vertices);
        self.viable_edges = Arc::new(viable_edges);
    }

    async fn generate_viable_edges_and_vertices(&self) -> (Vec<Vertex>, Vec<Edge>) {
        let threads = self.cfg.threads;
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
        println!("Found {} viable edges", all_viable_edges.len());
        (all_vertices, all_viable_edges)
    }

    // Generates vertices randomly within the given width and height.
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

    async fn viable_edges_worker(&self, start: usize, end: usize) -> (Vec<Vertex>, Vec<Edge>) {
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
            for (j, point) in points.iter().enumerate() {
                let length = p1.euclidean_distance(point);
                if length < radius {
                    let line = Line::new(p1, *point);
                    edges.push(Edge {
                        line,
                        length,
                        points: (i, j),
                    });
                }
            }
        }
        (vertices, edges)
    }

    /// Updates self to be an accurate representation of all current obstacles.
    async fn initialize_all_blocked(&mut self) {
        println!("Finding blocked per obstacle...");
        let mut handles = Vec::new();
        for o in &self.obstacles.obstacles {
            let clone = self.clone();
            let obstacle = *o;
            let handle =
                tokio::spawn(async move { clone.find_blocked_by_obstacle(obstacle).await });
            handles.push((handle, o.id()));
        }
        //        let mut all_blocked: Vec<EdgeIndex> = Vec::new();
        let mut blocked_per_obstacle: HashMap<ObstacleId, Vec<EdgeIndex>> = HashMap::new();
        for (handle, id) in handles {
            match handle.await {
                Ok(e_index) => {
                    //                    all_blocked.extend(&e_index);
                    blocked_per_obstacle.insert(id, e_index);
                }
                Err(e) => {
                    eprintln!("Error: {:?}", e);
                }
            }
        }
        println!("Found all blocked edges");
        self.blocked_per_obstacle = blocked_per_obstacle;
        self.update_blockings();
        self.update_edges();
    }

    fn update_blockings(&mut self) {
        for blocked in self.blocked_per_obstacle.values() {
            for edge in blocked {
                let count = self.blockings_per_edge.entry(*edge).or_insert(0);
                *count += 1;
            }
        }
    }

    fn update_edges(&mut self) {
        for (i, edge) in self.viable_edges.iter().enumerate() {
            if self.blockings_per_edge.get(&i).unwrap_or(&0) == &0 {
                self.edges.insert(i, edge.clone());
            }
        }
    }

    fn get_all_blocked(&self) -> Vec<EdgeIndex> {
        let mut blocked = Vec::new();
        for edges in self.blocked_per_obstacle.values() {
            blocked.extend(edges);
        }
        blocked.sort();
        blocked
    }

    /*
     *** Dynamic Updates ***
     */
    /// Makes no changes to &self, only returns the edge id's blocked by the given obstacle
    pub async fn find_blocked_by_obstacle(&self, obstacle: Obstacle) -> Vec<EdgeIndex> {
        let threads = self.cfg.threads;
        let n = self.viable_edges.len();
        let chunk_size = (n + threads - 1) / threads;
        let mut handles = Vec::new();
        for i in 0..threads {
            let start = i * chunk_size;
            let end = ((i + 1) * chunk_size).min(n);
            let clone = self.clone();
            let handle =
                tokio::spawn(
                    async move { clone.find_blocked_by_obstacle_worker(start, end, obstacle) },
                );

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

    fn find_blocked_by_obstacle_worker(
        &self,
        start: EdgeIndex,
        end: EdgeIndex,
        obstacle: Obstacle,
    ) -> Vec<EdgeIndex> {
        let mut blocked = Vec::new();
        for i in start..end {
            let edge = &self.viable_edges[i];
            if obstacle.intersects(&edge.line) {
                blocked.push(i);
            }
        }
        blocked
    }

    /// Inserts the given obstacle and updates the graph, returning the newly blocked edges.
    pub async fn insert_blocked_by_obstacle(
        &mut self,
        oid: ObstacleId,
        blockings: Vec<EdgeIndex>,
    ) -> Vec<EdgeIndex> {
        let mut blocked_edges = Vec::new();
        for edge_index in &blockings {
            let count = self.blockings_per_edge.entry(*edge_index).or_insert(0);
            *count += 1;
            if *count == 1 {
                let _ = self.edges.remove(edge_index);
                blocked_edges.push(*edge_index);
            }
        }
        self.blocked_per_obstacle.insert(oid, blockings);
        blocked_edges
    }

    /// Removes obstacle and updates the graph, and returns the newly unblocked edges.
    pub async fn remove_obstacle(&mut self, oid: ObstacleId) -> Vec<EdgeIndex> {
        let mut unblocked_edges = Vec::new();
        let mut unblocked = self.blocked_per_obstacle.remove(&oid).unwrap();
        for edge_index in &unblocked {
            let count = self.blockings_per_edge.entry(*edge_index).or_insert(0);
            *count -= 1;
            if *count == 0 {
                let edge = self.viable_edges[*edge_index].clone();
                self.edges.insert(*edge_index, edge);
                unblocked_edges.push(*edge_index);
            }
        }
        unblocked_edges
    }

    /*
     *** Astar ***
     */

    /// Initializes the nearest neighbours for efficient Astar execution.
    /// Should be invoked once the graph is fully initialized for all the obstacle.
    /// Subsequent calls will overwrite the previous neighbours.
    /// The mutable insert_blocked_by_obstacle and remove_obstacle functions will update the neighbours.
    fn initialize_neighbours(&mut self) {
        for i in 0..self.vertices.len() {
            self.neighbours.push(Vec::new());
        }
        for e in self.edges.values() {
            self.neighbours[e.points.0].push((e.points.1, e.length.round() as Distance));
            self.neighbours[e.points.1].push((e.points.0, e.length.round() as Distance));
        }
    }

    /// Runs the A* algorithm on the optimized nearest neighbors structure.
    pub fn run_astar(&self, start: &VertexIndex, end: &VertexIndex) -> Option<AstarPath> {
        if let Some((path, length)) = astar(
            start,
            |v| self.successors(v),
            |v| self.heuristic(*v, *end),
            |v| *v == *end,
        ) {
            let mut ret = Vec::new();
            for i in path {
                ret.push(self.vertices[i].clone());
            }
            return Some((ret, length));
        }
        None
    }

    fn successors(&self, start: &VertexIndex) -> Vec<(VertexIndex, Distance)> {
        // Get the successors
        self.neighbours[*start].clone()
    }

    fn heuristic(&self, start: VertexIndex, end: VertexIndex) -> Distance {
        // Get the heuristic
        self.vertices[start]
            .point
            .euclidean_distance(&self.vertices[end].point)
            .round() as Distance
    }

    /*
     *** Utilities ***
     */

    /// Plots the current state of the graph, including vertices, edges, and obstacles.
    /// If a path is provided, it will also be plotted.
    /// Saves the plot to a file with the given name.
    pub fn plot(&self, file_name: String, path: Option<AstarPath>) {
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
            .draw_series(self.edges.values().map(|edge| {
                PathElement::new(vec![edge.line.start.x_y(), edge.line.end.x_y()], CYAN)
            }))
            .unwrap()
            .label("Edge")
            .legend(|(x, y)| PathElement::new([(x, y), (x + 20, y)], CYAN));

        // Draw blocked edges
        chart
            .draw_series(self.get_all_blocked().iter().map(|edge_index| {
                let edge = &self.viable_edges[*edge_index];
                PathElement::new(vec![edge.line.start.x_y(), edge.line.end.x_y()], YELLOW)
            }))
            .unwrap()
            .label("Edge")
            .legend(|(x, y)| PathElement::new([(x, y), (x + 20, y)], YELLOW));

        // Draw path
        if let Some((path, _)) = path {
            // Draw edges
            let mut pv = path[0].clone();
            chart
                .draw_series(path.iter().map(|v| {
                    let e = PathElement::new(vec![pv.point.x_y(), v.point.x_y()], BLACK);
                    pv = v.clone();
                    e
                }))
                .unwrap()
                .label("Edge")
                .legend(|(x, y)| PathElement::new([(x, y), (x + 20, y)], BLACK));
        }
    }

    /// Max length of edges in the graph.
    fn max_radius(&self) -> f64 {
        let d = DIMENSIONS as f64;
        let id = 1.0 / d;
        let n = self.cfg.num_vertices as f64;
        let area = self.cfg.width as f64 * self.cfg.height as f64;
        let mu_free = area * 0.5; // Free space
        let zeta = PI; // Area of the unit circle
        let a = 2.0 * (1.0 + 1.0 / d);
        let b = mu_free / zeta;
        let gamma = a.powf(id) * b.powf(id);
        gamma * (n.log(d) / n).powf(id)
    }

    /// Returns a random number generator with the seed from the configuration.
    fn get_rng(&self) -> ChaCha8Rng {
        ChaCha8Rng::from_seed(self.cfg.seed)
    }

    /// Displays the current state of the graph.
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

    /// Returns the nearest vertex to the given point.
    pub fn get_nearest(&self, point: Point<f64>) -> Vertex {
        let mut min_distance = f64::MAX;
        let mut nearest = self.vertices[0].clone();
        for v in self.vertices.iter() {
            let distance = v.point.euclidean_distance(&point);
            if distance < min_distance && !self.obstacles.contains(&v.point) {
                min_distance = distance;
                nearest = v.clone();
            }
        }
        nearest
    }
}
