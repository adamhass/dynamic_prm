use geo::{EuclideanDistance, Line, Point};
// use pathfinding::directed::astar::astar;
use crate::prelude::*;
use plotters::prelude::*;
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap, f64::consts::PI, fs::File, hash::Hash, io::{BufReader, BufWriter}, sync::Arc
};
// use serde_json::{from_reader, to_writer_pretty};
use pathfinding::directed::astar::astar;

const DIMENSIONS: usize = 2;

// Prm stores all edges in viable edges
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DPrm {
    pub(crate) vertices: HashMap<VertexIndex, Vertex>,
    pub(crate) edges: Arc<HashMap<EdgeIndex, Edge>>,
    // viable_edges: Vec<Edge>,
    obstacles: ObstacleSet,
    blocked_per_obstacle: HashMap<ObstacleId, Vec<EdgeIndex>>,
    blockings_per_edge: HashMap<EdgeIndex, usize>,
    pub cfg: PrmConfig,
    neighbors: Neighbors,
}

impl DPrm {
    /*
     *** Initialization ***
     */

    /// Create a new DPrm with the given configuration and an initial ObstacleSet.
    /// Initializes viable edges and vertices.
    /// Finds all blocked edges per obstacle.
    pub async fn from_cfg(cfg: PrmConfig, obstacles: ObstacleSet) -> DPrm {
        let mut dprm = DPrm {
            vertices: HashMap::new(),
            edges: Arc::new(HashMap::new()),
            // viable_edges: Vec::new(),
            obstacles,
            blocked_per_obstacle: HashMap::new(),
            blockings_per_edge: HashMap::new(),
            cfg,
            neighbors: Neighbors::new(),
        };
        dprm.initialize_viable_edges_and_vertices().await;
        dprm.initialize_all_blocked().await;
        dprm.initialize_neighbors();
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
        let (vertices, edges) = self.generate_viable_edges_and_vertices().await;
        vertices.iter().enumerate().for_each(|(i, v)| {
            self.vertices.insert(i, v.clone());
        });
        let mut edge_map = HashMap::new();
        edges.iter().enumerate().for_each(|(i, e)| {
            edge_map.insert(i, e.clone());
        });
        self.edges = Arc::new(edge_map);
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
        let points = self.generate_vertices(end, self.cfg.width, self.cfg.height);
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
        let mut blocked_per_obstacle: HashMap<ObstacleId, Vec<EdgeIndex>> = HashMap::new();
        for o in self.obstacles.obstacles.iter() {
            let blockings = self.find_blocked_by_obstacle(*o).await;
            blocked_per_obstacle.insert(o.id(), blockings);
        }
        println!("Found all blocked edges");
        self.blocked_per_obstacle = blocked_per_obstacle;
        self.update_blockings();
    }

    fn update_blockings(&mut self) {
        for blocked in self.blocked_per_obstacle.values() {
            for edge in blocked {
                let count = self.blockings_per_edge.entry(*edge).or_insert(0);
                *count += 1;
            }
        }
    }

    fn get_all_free_edges(&self) -> Vec<EdgeIndex> {
        let mut free = Vec::new();
        for (i, _) in self.edges.iter() {
            if self.blockings_per_edge.get(i).unwrap_or(&0) == &0 {
                free.push(*i);
            }
        }
        free
    }

    fn get_all_blocked(&self) -> Vec<EdgeIndex> {
        let mut blocked = Vec::new();
        for edges in self.blocked_per_obstacle.values() {
            blocked.extend(edges);
        }
        blocked.sort();
        blocked
    }

    pub fn contains_obstacle(&self, oid: ObstacleId) -> bool {
        for o in &self.obstacles.obstacles {
            if o.id() == oid {
                return true;
            }
        }
        false
    }

    /*
     *** Dynamic Updates ***
     */
    /// Makes no changes to &self, only returns the edge id's blocked by the given obstacle
    pub async fn find_blocked_by_obstacle(&self, obstacle: Obstacle) -> Vec<EdgeIndex> {
        let threads = self.cfg.threads;
        let n = self.edges.len();
        let chunk_size = (n + threads - 1) / threads;
        let mut handles = Vec::new();
        for i in 0..threads {
            let start = i * chunk_size;
            let end = ((i + 1) * chunk_size).min(n);
            let clone = self.edges.clone();
            let handle =
                tokio::spawn(
                    async move { Self::find_blocked_by_obstacle_worker(clone, start, end, obstacle) },
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
        edges: Arc<HashMap<EdgeIndex, Edge>>,
        start: EdgeIndex,
        end: EdgeIndex,
        obstacle: Obstacle,
    ) -> Vec<EdgeIndex> {
        let mut blocked = Vec::new();
        for i in start..end {
            let edge = &edges[&i];
            if obstacle.intersects(&edge.line) {
                blocked.push(i);
            }
        }
        blocked
    }

    /// Inserts the given obstacle and updates the graph, returning the newly blocked edges.
    pub fn insert_blocked_by_obstacle(&mut self, obstacle: Obstacle, blockings: Vec<EdgeIndex>) {
        self.obstacles.add(obstacle);
        let mut newly_blocked_edges = Vec::new();
        for edge_index in blockings.iter() {
            let count = self.blockings_per_edge.entry(*edge_index).or_insert(0);
            *count += 1;
            if *count == 1 {
                newly_blocked_edges.push(*edge_index);
            }
        }
        self.blocked_per_obstacle.insert(obstacle.id(), blockings);
        
        // Update neighbors
        for e in newly_blocked_edges {
            self.neighbors.remove(&self.edges[&e]);
        }
    }

    /// Removes obstacle and updates the graph, and returns the newly unblocked edges.
    pub fn remove_obstacle(&mut self, oid: ObstacleId) {
        if let Some(unblocked) = self.blocked_per_obstacle.remove(&oid) {
            for edge_index in unblocked.iter() {
                let count = self.blockings_per_edge.entry(*edge_index).or_insert(0);
                *count -= 1;
                if *count == 0 {
                    let edge = self.edges[edge_index].clone();
                    // Update neighbors on the fly
                    self.neighbors.add(&edge);
                }
            }
        } else {
            println!("Obstacle {} not found", oid);
        }
        self.obstacles.remove_by_id(oid);
    }
    
    // /// Inserts new potential vertices and edges into the DPRM and updates the blockings and the graph.
    // /// Compares each edge in edges to each obstacle in self.obstacles.
    // pub fn add_potentials(&mut self, vertices: Vec<Vertex>, edges: Vec<(EdgeIndex, Edge)>) {
    //     // Insert vertices
    //     for v in vertices {self.vertices.insert(v.index, v);}
        
    //     // Insert edges and fix blockings on the fly
    //     for (idx, edge) in edges {
    //         let mut blockings = 0;
    //         for obstacle in self.obstacles.obstacles.iter() {
    //             if obstacle.intersects(&edge.line) {
    //                 blockings += 1;
    //                 self.blocked_per_obstacle.entry(obstacle.id()).or_insert(Vec::new()).push(idx);
    //             }
    //         }
    //         if blockings == 0 {
    //             self.neighbors.add(&edge);
    //         }
    //         self.blockings_per_edge.insert(idx, blockings);
    //         self.edges.insert(idx, edge);
    //     }
    // }

    /*
     *** Astar ***
     */

    /// Initializes the nearest neighbors for efficient Astar execution.
    /// Should be invoked once the graph is fully initialized for all the obstacle.
    /// Subsequent calls will overwrite the previous neighbors.
    /// The mutable insert_blocked_by_obstacle and remove_obstacle functions will update the neighbors.
    fn initialize_neighbors(&mut self) {
        for edge_index in self.get_all_free_edges() {
            self.neighbors.add(&self.edges[&edge_index]);
        }
    }

    /// Runs the A* algorithm on the optimized nearest neighbors structure.
    pub fn run_astar(&self, start: &VertexIndex, end: &VertexIndex) -> Option<DPrmPath> {
        if let Some((path, length)) = astar(
            start,
            |v| self.successors(&v),
            |v| self.heuristic(v, end),
            |v| *v == *end,
        ) {
            let mut ret = Vec::new();
            for i in path {
                ret.push(self.vertices[&i].clone());
            }
            return Some(DPrmPath {
                vertices: ret,
                length,
            });
        }
        None
    }

    fn successors(&self, start: &VertexIndex) -> Vec<(VertexIndex, Distance)> {
        // Get the successors
        self.neighbors.get(start).clone()
    }

    fn heuristic(&self, start: &VertexIndex, end: &VertexIndex) -> Distance {
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
    pub fn plot(&self, file_name: String, path: Option<DPrmPath>) {
        // let filename = format!("output/{}.png", file_name);
        // Create a drawing area
        let root = BitMapBackend::new(&file_name, (2000_u32, 2000_u32)).into_drawing_area();
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
                (self.vertices)
                    .clone()
                    .into_iter()
                    .map(|(_, v)| Circle::new(v.point.0.x_y(), 3, BLACK)),
            )
            .unwrap();

        // Draw edges
        chart
            .draw_series(self.get_all_free_edges().iter().map(|edge_index| {
                let Edge { line, .. } = &self.edges[edge_index];
                PathElement::new(vec![line.start.x_y(), line.end.x_y()], CYAN)
            }))
            .unwrap()
            .label("Edge")
            .legend(|(x, y)| PathElement::new([(x, y), (x + 20, y)], CYAN));

        // Draw blocked edges
        chart
            .draw_series(self.get_all_blocked().iter().map(|edge_index| {
                let edge = &self.edges[edge_index];
                PathElement::new(vec![edge.line.start.x_y(), edge.line.end.x_y()], YELLOW)
            }))
            .unwrap()
            .label("Edge")
            .legend(|(x, y)| PathElement::new([(x, y), (x + 20, y)], YELLOW));

        // Draw path
        if let Some(DPrmPath { vertices, .. }) = path {
            // Draw edges
            let mut pv = vertices[0].clone();
            chart
                .draw_series(vertices.iter().map(|v| {
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
        let neighbor_edges = self.neighbors.inner.iter().map(|(_, neighbors)| neighbors.len()).sum::<usize>();
        println!(
            "Vertices: {}, Edges: {}, Viable Edges: {}, Blocked Edges: {}, Obstacles: {}, Neighbors: {}, Total Neighbor Edges: {}",
            self.vertices.len(),
            self.get_all_free_edges().len(),
            self.edges.len(),
            self.get_all_blocked().len(),
            self.obstacles.obstacles.len(),
            self.neighbors.inner.len(),
            neighbor_edges
        );
    }

    /// Returns the nearest vertex to the given point.
    pub fn get_nearest(&self, point: Point<f64>) -> Vertex {
        let mut min_distance = f64::MAX;
        let mut nearest: VertexIndex = 0;
        for (vid, v) in self.vertices.iter() {
            let distance = v.point.euclidean_distance(&point);
            if distance < min_distance && self.is_free(&v.point) {
                min_distance = distance;
                nearest = *vid;
            }
        }
        self.vertices[&nearest].clone()
    }

    pub fn is_free(&self, point: &Point<f64>) -> bool {
        !self.obstacles.contains(point)
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct Neighbors {
    inner: HashMap<VertexIndex, Vec<(VertexIndex, Distance)>>,
}
const EMPTY: &'static Vec<(VertexIndex, Distance)> = &Vec::new();

impl Neighbors {
    fn new() -> Neighbors {
        Neighbors {
            inner: HashMap::new(),
        }
    }

    // Point to an empty vector
    fn get(&self, v: &VertexIndex) -> &Vec<(VertexIndex, Distance)> {
        if let Some(vec) = self.inner.get(v) {
            vec
        } else { EMPTY }
    }

    fn add(&mut self, e: &Edge) {
        self.inner.entry(e.points.0).or_insert(Vec::new()).push((e.points.1, e.length.round() as Distance));
        self.inner.entry(e.points.1).or_insert(Vec::new()).push((e.points.0, e.length.round() as Distance));
    }

    fn remove(&mut self, e: &Edge) {
        self.inner.entry(e.points.0).or_insert(Vec::new()).retain(|(v, _)| *v != e.points.1);
        self.inner.entry(e.points.1).or_insert(Vec::new()).retain(|(v, _)| *v != e.points.0);
    }
}