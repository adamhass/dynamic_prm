use pathfinding::directed::astar::astar;
use rand::rngs::ThreadRng;
use rand::Rng;
use std::collections::HashSet;
use std::time::Instant;
use geo::{Point, Line, Rect};

const WIDTH: usize = 100;
const HEIGHT: usize = 100;
const NUM_VERTICES: usize = 1000;
const NUM_OBSTACLES: usize = 10;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct Point {
    x: usize,
    y: usize,
}

struct Edge {
    p1: Point,
    p2: Point,
    length: f32,
}

struct Obstacle {
    x_min: usize,
    x_max: usize,
    y_min: usize,
    y_max: usize,
}

impl Obstacle {
    fn new_random(rng: &mut ThreadRng) -> Obstacle {
        
        let x_min: rng.gen_range(0..(WIDTH-1));
        let y_min: rng.gen_range(0..(HEIGHT-1));
        Obstacle {
            x_min,
            x_max: x_min + rng.gen_range(1..(WIDTH/10)),
            y_min,
            y_max: y_min + rng.gen_range(1..(HEIGHT/10)),
        }
    }

    fn contains(&self, point: &Point) -> bool {
        todo!()
    }

    fn intersects(&self, e: &Edge) -> bool {
        let p1 = &e.p1;
        let p2 = &e.p2;
        self.contains(p1) || self.contains(p2) ||
        todo!()
    }
}

struct ObstacleSet {
    obstacles: Vec<Obstacle>,
}

impl ObstacleSet {
    fn new(n: usize, rng: &mut ThreadRng) -> ObstacleSet {
        let mut obstacles = Vec::new();
        while obstacles.len() < n {
            obstacles.push(Obstacle::new_random(rng));
        }
        obstacles
        ObstacleSet { obstacles }
    }

    fn contains(&self, point: &Point) -> bool {
        self.obstacles.iter().any(|o| o.contains(point))
    }

    fn intersects(&self, e: &Edge) -> bool {
        self.obstacles.iter().any(|o| o.intersects(e))
    }
    
}

fn generate_vertices(obstacles: &ObstacleSet) -> Vec<Point> {
    let mut vertices = Vec::new();
    let mut rng = rand::thread_rng();
    while vertices.len() < NUM_VERTICES {
        let vertex = Point {
            x: rng.gen_range(0..WIDTH),
            y: rng.gen_range(0..HEIGHT),
        };
        if !obstacles.contains(&vertex) {
            vertices.push(vertex);
        }
    }
    vertices
}

fn generate_edges(vertices: &[Point], obstacles: &HashSet<Point>) -> Vec<(Point, Point)> {
    let mut edges = Vec::new();
    for i in 0..vertices.len() {
        for j in (i+1)..vertices.len() {
            let p1 = &vertices[i];
            let p2 = &vertices[j];
            if !line_intersects_obstacles(p1, p2, obstacles) {
                edges.push((p1.clone(), p2.clone()));
            }
        }
    }
    edges
}

fn main() {
    // Initialize graph structure and space X
    let obstacles = generate_obstacles();
    let vertices = generate_vertices(&obstacles);
    let edges = generate_edges(&vertices, &obstacles);

    // Choose start and goal vertices
    let start = vertices[0].clone();
    let goal = vertices[1].clone();

    // Define a closure for getting neighbors
    let neighbors = |p: &Point| -> Vec<(Point, usize)> {
        edges.iter()
            .filter_map(|(p1, p2)| {
                if p1 == p { Some((p2.clone(), 1)) } 
                else if p2 == p { Some((p1.clone(), 1)) } 
                else { None }
            })
            .collect()
    };

    // Run A* algorithm and time it repeatedly
    let mut total_duration = 0;
    let iterations = 100;
    for _ in 0..iterations {
        let start_time = Instant::now();
        let result = astar(&start, |p| neighbors(p), |p| heuristic(p, &goal), |p| *p == goal);
        let duration = start_time.elapsed();
        total_duration += duration.as_micros();
        if let Some((path, cost)) = result {
            println!("Found path of length {} with cost {}.", path.len(), cost);
        } else {
            println!("No path found.");
        }
    }
    println!("Average duration: {} microseconds", total_duration / iterations);
}
