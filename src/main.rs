// use pathfinding::directed::astar::astar;
use dynamic_prm::prelude::*;
use plotters::prelude::*;
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use std::sync::Arc;
use std::{env, time::Instant};

#[tokio::main]
async fn main() {
    /*
    // Experiment params:
    let iterations: usize = parse_env_var("ITERATIONS");
    let start_num_obstacles: usize = parse_env_var("NUM_OBSTACLES");
    let start_num_vertices: usize = parse_env_var("NUM_VERTICES");
    let start_width: usize = parse_env_var("WIDTH");
    let start_height: usize = parse_env_var("HEIGHT");
    */
    let threads: usize = 4;
    let seed = Arc::new([0u8; 32]);
    let start_width = 100;
    let start_height = 100;
    let start_num_vertices = 10000;
    let start_num_obstacles = 50;
    let iterations = 1;
    for i in 1..iterations + 1 {
        // Print the parameters
        let width = start_width;
        let height = start_height;
        let num_vertices = start_num_vertices;
        let num_obstacles = start_num_obstacles;
        println!("* * * RUNNING NEW ITERATION {} * * *", i);
        println!("* Parsed Parameters:");
        println!("* ITERATIONS: {}", iterations);
        println!("* NUM_OBSTACLES: {}", num_obstacles);
        println!("* NUM_VERTICES: {}", num_vertices);
        println!("* WIDTH: {}", width);
        println!("* HEIGHT: {}", height);
        println!("* THREADS: {}", threads);
        // Iteration set-up
        let seed = Arc::new([i as u8; 32]);
        let mut rng = ChaCha8Rng::from_seed(*seed);
        let obstacles = Arc::new(ObstacleSet::new_random(
            num_obstacles,
            width,
            height,
            &mut rng,
        ));
        let cfg = PrmConfig {
            num_vertices,
            width,
            height,
            seed,
        };
        let mut prm = Prm::new(cfg, obstacles);

        // Start timer
        let start_time = Instant::now();
        // Do parallel PRM
        let (vertices, edges) = prm.run_prm(threads).await;

        // End timer, convert to ms
        let duration = start_time.elapsed().as_millis() as f64;

        let max_edge_length = edges.iter().map(|e| e.length).reduce(f64::max);
        if let Some(max) = max_edge_length {
            println!("Max edge length: {}", max);
        }
        println!("Vertices: {}, Edges: {}", vertices.len(), edges.len());
        println!("Duration (ms): {}", duration);
        plot(i, &edges, &vertices, &prm.obstacles, width, height);
        prm.update_vertices_and_edges(vertices, edges);

        // Add obstacle
        let start_time = Instant::now();
        let new_obstacle = Obstacle::new_random(&mut rng, width, height);
        let edges =  prm.add_obstacle(new_obstacle, threads).await;
        let duration = start_time.elapsed().as_millis() as f64;
        println!("Edges to be removed: {}", edges.len());
        println!("Duration (ms): {}", duration);
        plot(i, &edges, &prm.vertices, &prm.obstacles, width, height);
    }
}

/*
    HELPER FUNCTIONS
*/
fn parse_env_var(name: &str) -> usize {
    env::var(name)
        .expect(&format!("Environment variable {} not set", name))
        .parse()
        .expect(&format!("Failed to parse environment variable {}", name))
}

fn plot(
    iteration: usize,
    edges: &Vec<Edge>,
    vertices: &Vec<Vertex>,
    obstacles: &Arc<ObstacleSet>,
    width: usize,
    height: usize,
) -> () {
    let filename = format!("output/{}.png", iteration);
    // Create a drawing area
    let root = BitMapBackend::new(&filename, (2000_u32, 2000_u32)).into_drawing_area();
    root.fill(&WHITE).unwrap();

    // Define the chart
    let mut chart = ChartBuilder::on(&root)
        .caption("Edges and Obstacles", ("sans-serif", 50))
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(0.0..(width as f64), 0.0..(height as f64))
        .unwrap();

    chart.configure_mesh().draw().unwrap();

    // Draw vertices
    chart
        .draw_series(
            vertices
                .into_iter()
                .map(|v| Circle::new(v.point.0.x_y(), 2, &BLACK)),
        )
        .unwrap();

    // Draw edges
    chart
        .draw_series(
            edges.iter().map(|edge| {
                PathElement::new(vec![edge.line.start.x_y(), edge.line.end.x_y()], &BLUE)
            }),
        )
        .unwrap()
        .label("Edge")
        .legend(|(x, y)| PathElement::new([(x, y), (x + 20, y)], &BLUE));

    // Draw obstacles
    chart
        .draw_series(obstacles.obstacles.iter().map(|o| o.rectangle()))
        .unwrap();
    root.present().unwrap();
}

/*
export ITERATIONS=5
export NUM_OBSTACLES=120
export NUM_VERTICES=1000
export WIDTH=100
export HEIGHT=100
export THREADS=1
*/
