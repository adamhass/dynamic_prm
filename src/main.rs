use geo::{Contains, Coord, EuclideanDistance, Intersects, Line, Point, Rect};
// use pathfinding::directed::astar::astar;
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use std::{task, thread};
use std::{env, time::Instant};
use plotters::prelude::*;
use std::sync::Arc;
use tokio::*;

const DIMENSIONS: usize = 2;
struct Edge {
    line: Line<f64>,
    length: f64,
    points: (usize, usize),
}

struct Vertex {
    point: Point<f64>,
    index: usize,
}

struct Obstacle {
    rect: Rect<f64>,
}

impl Obstacle {
    fn new_random(rng: &mut ChaCha8Rng, w: usize, h: usize) -> Obstacle {
        let x_min = rng.gen_range(0.0 ..(w as f64 - 1.0)) as f64;
        let y_min = rng.gen_range(0.0..(h as f64 - 1.0)) as f64;
        let width = rng.gen_range(1.0..(w as f64 / 10.0)) as f64;
        let height = rng.gen_range(1.0..(h as f64 / 10.0)) as f64;
        let rect = Rect::new(
            (x_min, y_min),
            (x_min + width, y_min + height),
        );
        Obstacle { rect }
    }

    fn contains(&self, point: &Point<f64>) -> bool {
        self.rect.contains(point)
    }

    fn intersects(&self, edge: &Line<f64>) -> bool {
        self.rect.intersects(edge)
    }

    fn rectangle(&self) -> Rectangle<(f64, f64)> {
        Rectangle::new([self.rect.min().x_y(), self.rect.max().x_y()], (&RED).filled())
    }
}

struct ObstacleSet {
    obstacles: Vec<Obstacle>,
}

impl ObstacleSet {
    fn new_random(n: usize, width: usize, height: usize, rng: &mut ChaCha8Rng) -> ObstacleSet {
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

fn generate_vertices(n: usize, width: usize, height: usize, rng: &mut ChaCha8Rng) -> Vec<Point<f64>> {
    let mut vertices = Vec::new();
    while vertices.len() < n {
        vertices.push(Point::new(
            rng.gen_range(0.0..width as f64),
            rng.gen_range(0.0..height as f64),
        ));
    }
    vertices
}

// fn heuristic(p: &Point<f64>, goal: &Point<f64>) -> f64 {
//    p.euclidean_distance(goal)
// }

const GAMMA: f64 = 12.0*2.49;
fn gamma_prm(n: usize, d: usize, w: usize) -> f64 {
    0.664905117183084
    // let n_f64 = n as f64;
    // let w_f64 = w as f64;
    // let d_f64 = d as f64;
    // let log_n = n_f64.ln();
    // (log_n / n_f64).powf(1.0 / d_f64)*GAMMA
}

fn prm(obstacles: &ObstacleSet, n: usize, width: usize, height: usize, rng: &mut ChaCha8Rng) -> (Vec<Point>, Vec<Edge>) {
    // Sample free_vertices:
    let vertices = generate_vertices(n, width, height, rng);
    let free_vertices: Vec<Point> = vertices.into_iter().filter(|v| !obstacles.contains(v)).collect();
    let gamma = gamma_prm(n, DIMENSIONS, width);
    let mut edges = Vec::new();
    for i in 0..free_vertices.len() {
        let p1 = free_vertices[i];
        for j in (i+1)..free_vertices.len() {
            let p2 = free_vertices[j];
            let distance = p1.euclidean_distance(&p2);
            if distance < gamma {
                let line = Line::new(p1, p2);
                if !obstacles.intersects(&line) {
                    edges.push(Edge {
                        line,
                        length: distance,
                        points: (i, j),
                    });
                }
            }
        }
    }
    (free_vertices, edges)
}

async fn parallel_prm(start: usize, end: usize, obstacles: &ObstacleSet, n: usize, width: usize, height: usize, seed: [u8; 32]) -> (Vec<Vertex>, Vec<Edge>) {
    let mut rng = ChaCha8Rng::from_seed(seed);
    let vertices = generate_vertices(end, width, height, &mut rng);
    let mut vs = Vec::new();
    let gamma = gamma_prm(n, DIMENSIONS, width);
    let mut edges = Vec::new();
    for i in start..end {
        let p1 = vertices[i];
        if obstacles.contains(&p1) {
            continue;
        }
        vs.push(Vertex { point: p1, index: i });
        for (j, p2) in vertices.iter().enumerate() {
            if obstacles.contains(&p1) {
                continue;
            }
            let distance = p1.euclidean_distance(p2);
            if distance < gamma && p1 != *p2 {
                let line = Line::new(p1, p2.clone());
                if !obstacles.intersects(&line) {
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

fn parse_env_var(name: &str) -> usize {
    env::var(name)
        .expect(&format!("Environment variable {} not set", name))
        .parse()
        .expect(&format!("Failed to parse environment variable {}", name))
}

async fn run_paralell(num_threads: usize, num_vertices: usize, obstacles: Arc<ObstacleSet>, seed: Arc<[u8; 32]>, width: usize, height: usize) -> (Vec<Vertex>, Vec<Edge>) {
    let chunk_size = num_vertices / num_threads;
    let seed = Arc::new(seed); // Use Arc to share seed between threads
    
    let mut handles = Vec::new();
    
    for i in 0..num_threads {
        let start = i * chunk_size;
        let end = if i == num_threads - 1 {
            num_vertices
        } else {
            (i + 1) * chunk_size
        };
        
        let obstacles = Arc::clone(&obstacles);
        let seed = Arc::clone(&seed);
        
        let handle = tokio::spawn(async move {
            parallel_prm(start, end, &obstacles, num_vertices, width, height, **seed).await
        });
        
        handles.push(handle);
    }
    
    let mut all_vertices = Vec::new();
    let mut all_edges = Vec::new();
    
    for handle in handles {
        match handle.await {
            Ok((vertices, edges)) => {
                all_vertices.extend(vertices);
                all_edges.extend(edges);
            },
            Err(e) => {
                eprintln!("Error: {:?}", e);
            }
        }
    }
    (all_vertices, all_edges)
}

#[tokio::main]
async fn main() {
    // Experiment params:
    let iterations: usize = parse_env_var("ITERATIONS");
    let start_num_obstacles: usize = parse_env_var("NUM_OBSTACLES");
    let start_num_vertices: usize = parse_env_var("NUM_VERTICES");
    let start_width: usize = parse_env_var("WIDTH");
    let start_height: usize = parse_env_var("HEIGHT");
    
    for i in 1..iterations+1 {
        // Print the parameters
        let width = start_width;
        let height = start_height;
        let num_vertices = start_num_vertices;
        let num_obstacles = start_num_obstacles;
        println!("* * * RUNNING NEW ITERATION {} * * *", i);
        println!("Parsed Parameters:");
        println!("ITERATIONS: {}", iterations);
        println!("NUM_OBSTACLES: {}", num_obstacles);
        println!("NUM_VERTICES: {}", num_vertices);
        println!("WIDTH: {}", width);
        println!("HEIGHT: {}", height);
        // Iteration set-up
        let seed = [i as u8; 32];
        let mut rng = ChaCha8Rng::from_seed(seed);
        let obstacles = ObstacleSet::new_random(num_obstacles, width, height, &mut rng);
        let obstacles = Arc::new(obstacles); // Use Arc to share obstacles between threads

        // Start timer
        let start_time = Instant::now();
        /* 
            // Do PRM
            let (vertices, edges) = prm(&obstacles, num_vertices, width, height, &mut rng);
        */
        // Do parallel PRM
        let (vertices, edges) = run_paralell(4, num_vertices, obstacles.clone(), Arc::new(seed), width, height).await;
        
        // End timer, convert to ms
        let duration = start_time.elapsed().as_millis() as f64;
        
        let max_edge_length = edges.iter().map(|e| e.length).reduce(f64::max);
        if let Some(max) = max_edge_length {
            println!("Max edge length: {}", max);
        }
        println!("Vertices: {}, Edges: {}", vertices.len(), edges.len());
        println!("Duration (ms): {}", duration);
        plot(i, edges, vertices, obstacles, width, height);
    }
}

fn plot(iteration: usize, edges: Vec<Edge>, vertices: Vec<Vertex>, obstacles: Arc<ObstacleSet>, width: usize, height: usize) -> () {
    let filename = format!("output/{}.png", iteration);
    // Create a drawing area
    let root = BitMapBackend::new(&filename, (2000_u32, 2000_u32)).into_drawing_area();
    root.fill(&WHITE).unwrap();

    // Define the chart
    let mut chart = ChartBuilder::on(&root)
        .caption("Edges and Obstacles", ("sans-serif", 50))
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(0.0..(width as f64), 0.0..(height as f64)).unwrap();

    chart.configure_mesh().draw().unwrap(); 

    // Draw vertices
    chart.draw_series(vertices.into_iter().map(|v| Circle::new(v.point.0.x_y(), 2, &BLACK))).unwrap();

    // Draw edges
    chart
    .draw_series(edges.iter().map(|edge| PathElement::new(
        vec![edge.line.start.x_y(), edge.line.end.x_y()],
        &BLUE,
    ))).unwrap()
    .label("Edge")
    .legend(|(x, y)| PathElement::new([(x, y), (x + 20, y)], &BLUE));

    // Draw obstacles
    chart.draw_series(obstacles.obstacles.iter().map(|o| o.rectangle())).unwrap();
    root.present().unwrap();
}