use geo::{Contains, Coord, EuclideanDistance, Intersects, Line, Point, Rect};
// use pathfinding::directed::astar::astar;
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use std::{env, time::Instant};
use plotters::prelude::*;
use plotters::style::colors::*;
use std::error::Error;
use plotters::prelude::*;

const DIMENSIONS: usize = 2;
struct Edge {
    line: Line<f64>,
    length: f64,
    points: (usize, usize),
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

const GAMMA: f64 = 10.0*2.49;
fn gamma_prm(n: usize, d: usize) -> f64 {
    let n_f64 = n as f64;
    let d_f64 = d as f64;
    let log_n = n_f64.ln();
    (log_n / n_f64).powf(1.0 / d_f64)*GAMMA
}

fn prm(obstacles: &ObstacleSet, n: usize, width: usize, height: usize, rng: &mut ChaCha8Rng) -> (Vec<Point>, Vec<Edge>) {
    // Sample free_vertices:
    let vertices = generate_vertices(n, width, height, rng);
    let free_vertices: Vec<Point> = vertices.into_iter().filter(|v| !obstacles.contains(v)).collect();
    let gamma = gamma_prm(n, DIMENSIONS);
    print!("Gamma: {}", gamma);
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

fn parse_env_var(name: &str) -> usize {
    env::var(name)
        .expect(&format!("Environment variable {} not set", name))
        .parse()
        .expect(&format!("Failed to parse environment variable {}", name))
}

fn main() {
    // Experiment params:
    let iterations: usize = parse_env_var("ITERATIONS");
    let start_num_obstacles: usize = parse_env_var("NUM_OBSTACLES");
    let start_num_vertices: usize = parse_env_var("NUM_VERTICES");
    let start_width: usize = parse_env_var("WIDTH");
    let start_height: usize = parse_env_var("HEIGHT");
    
    for i in 1..iterations+1 {
        // Print the parameters
        let width = start_width*((1+i)/2);
        let height = start_height*((1+i)/2);
        let num_vertices = start_num_vertices*((1+i)/2);
        let num_obstacles = start_num_obstacles*((1+i)/2);
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

        // Start timer
        let start_time = Instant::now();
        // Do PRM
        let (vertices, edges) = prm(&obstacles, num_vertices, width, height, &mut rng);
        // End timer, convert to ms
        let duration = start_time.elapsed().as_millis() as f64;
        let max_edge_length = edges.iter().map(|e| e.length).reduce(f64::max);
        if let Some(max) = max_edge_length {
            println!("Max edge length: {}", max);
        }
        println!("Vertices: {}, Edges: {}", vertices.len(), edges.len());
        println!("Duration (ms): {}", duration);
        plot(i, edges, vertices, obstacles.obstacles, width, height);
    }
}

fn plot(iteration: usize, edges: Vec<Edge>, vertices: Vec<Point<f64>>, obstacles: Vec<Obstacle>, width: usize, height: usize) -> () {
    let filename = format!("output/{}.png", iteration);
    // Create a drawing area
    let root = BitMapBackend::new(&filename, (1000_u32, 1000_u32)).into_drawing_area();
    root.fill(&WHITE).unwrap();

    // Define the chart
    let mut chart = ChartBuilder::on(&root)
        .caption("Edges and Obstacles", ("sans-serif", 50))
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(0.0..(width as f64), 0.0..(height as f64)).unwrap();

    chart.configure_mesh().draw().unwrap(); 

    // Draw vertices
    chart.draw_series(vertices.into_iter().map(|v| Circle::new(v.0.x_y(), 3, &BLACK))).unwrap();

    // Draw edges
    chart
    .draw_series(edges.iter().map(|edge| PathElement::new(
        vec![edge.line.start.x_y(), edge.line.end.x_y()],
        &BLUE,
    ))).unwrap()
    .label("Edge")
    .legend(|(x, y)| PathElement::new([(x, y), (x + 20, y)], &BLUE));

    // Draw obstacles
    chart.draw_series(obstacles.into_iter().map(|o| o.rectangle())).unwrap();
    root.present().unwrap();
}