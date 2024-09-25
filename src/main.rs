#![allow(unused)]
// use pathfinding::directed::astar::astar;
use dynamic_prm::prelude::*;
use geo::{Contains, Intersects};
use geo::{Line, Point, Rect};
use plotters::prelude::*;
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use std::io::{stdin, Stdin};
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
    let start_num_obstacles = 100;
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
        let seed = [i as u8; 32];
        let mut cfg = PrmConfig::new(num_vertices, width, height, seed);
        if i == 2 {
            cfg.use_viable_edges = true;
        }
        let mut prm = Prm::new(cfg, num_obstacles);
        prm.print();
        // Run the experiment
        let mut dprm = DPrm::new(prm);
        // Start timer
        let start_time = Instant::now();
        // Do parallel PRM
        dprm.update_viable_edges_and_vertices(threads).await;
        dprm.update_all_blocked(threads).await;
        // End timer, convert to ms
        let duration = start_time.elapsed().as_millis() as f64;
        println!("Duration (ms): {}", duration);
        dprm.print();
        let astar = Astar::new(dprm.clone());
        let start = dprm.get_nearest(Point::new(0.0, height as f64));
        let end = dprm.get_nearest(Point::new(width as f64, 0.0));
        let path = astar.run_astar(start, end);
        println!("{}", path.is_some());
        dprm.plot(format!("{}_dprm", i), path);
        println!("Waiting for stdin");
        /*

        // Start timer
        let start_time = Instant::now();
        // Do parallel PRM
        prm.compute(threads).await;
        // End timer, convert to ms
        let duration = start_time.elapsed().as_millis() as f64;

        let max_edge_length = prm.edges.iter().map(|e| e.length).reduce(f64::max);
        prm.print();
        println!("Duration (ms): {}", duration);

        let mut astar = Astar::new(prm.clone());
        let start_time = Instant::now();
        let start = prm.get_nearest((0.0, 0.0).into());
        let end = prm.get_nearest((width as f64, height as f64).into());
        let path = astar.run_basic_astar(start.index, end.index);
        let duration = start_time.elapsed().as_millis() as f64;
        if let Some((path, length)) = path {
            println!("Found a basic path of length {} in (ms): {}", length, duration);

            plot(format!("{}_with_path", i), &prm, Some(path));
        } else {
            println!("Found NO path in (ms): {}", duration);
            plot(format!("{}_with_no_path", i), &prm, None);
        }
        astar.optimized = true;
        let start_time = Instant::now();
        let start = prm.get_nearest((0.0, 0.0).into());
        let end = prm.get_nearest((width as f64, height as f64).into());
        let path = astar.run_basic_astar(start.index, end.index);
        let duration = start_time.elapsed().as_millis() as f64;
        if let Some((path, length)) = path {
            println!("Found a basic path of length {} in (ms): {}", length, duration);

            plot(format!("{}_with_path_optimized", i), &prm, Some(path));
        } else {
            println!("Found NO path in (ms): {}", duration);
            plot(format!("{}_with_no_path", i), &prm, None);
        }
         */
    }
}

async fn add_remove(mut prm: Prm, i: usize, threads: usize) {
    let new_obstacle: Obstacle = Obstacle::new((40.0, 40.0), (60.0, 60.0));
    plot(format!("{}_new", i), &prm, None);
    // Add new obstacle
    let start_time = Instant::now();
    prm.add_obstacle(new_obstacle, threads).await;
    let duration = start_time.elapsed().as_millis() as f64;
    prm.print();
    plot(format!("{}_added_obstacle", i), &prm, None);

    // Remove obstacle
    // let remove_obstacle = prm.obstacles.obstacles.get(0).unwrap().clone();
    let start_time = Instant::now();
    prm.remove_obstacle(new_obstacle, threads).await;
    let duration = start_time.elapsed().as_millis() as f64;
    prm.print();
    println!("Duration (ms): {}", duration);
    plot(format!("{}_removed_obstacle", i), &prm, None);
}
/*
    HELPER FUNCTIONS
*/
fn parse_env_var(name: &str) -> usize {
    env::var(name)
        .unwrap_or_else(|_| panic!("Environment variable {} not set", name))
        .parse()
        .unwrap_or_else(|_| panic!("Failed to parse environment variable {}", name))
}

fn plot(name: String, prm: &Prm, path: Option<Vec<Vertex>>) {
    let filename = format!("output/{}.png", name);
    // Create a drawing area
    let root = BitMapBackend::new(&filename, (2000_u32, 2000_u32)).into_drawing_area();
    root.fill(&WHITE).unwrap();

    // Define the chart
    let mut chart = ChartBuilder::on(&root)
        .caption("Edges and Obstacles", ("sans-serif", 50))
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(0.0..(prm.cfg.width as f64), 0.0..(prm.cfg.height as f64))
        .unwrap();

    chart.configure_mesh().draw().unwrap();

    // Draw obstacles
    chart
        .draw_series(prm.obstacles.obstacles.iter().map(|o| o.rectangle()))
        .unwrap();
    root.present().unwrap();

    // Draw vertices
    chart
        .draw_series(
            (*prm.vertices)
                .clone()
                .into_iter()
                .map(|v| Circle::new(v.point.0.x_y(), 2, BLACK)),
        )
        .unwrap();

    // Draw edges
    chart
        .draw_series(
            prm.edges.iter().map(|edge| {
                PathElement::new(vec![edge.line.start.x_y(), edge.line.end.x_y()], BLUE)
            }),
        )
        .unwrap()
        .label("Edge")
        .legend(|(x, y)| PathElement::new([(x, y), (x + 20, y)], BLUE));

    // Draw viable edges
    chart
        .draw_series(
            prm.viable_edges.iter().map(|edge| {
                PathElement::new(vec![edge.line.start.x_y(), edge.line.end.x_y()], RED)
            }),
        )
        .unwrap()
        .label("Edge")
        .legend(|(x, y)| PathElement::new([(x, y), (x + 20, y)], BLUE));

    // Draw path
    if let Some(path) = path {
        let style = GREEN;
        style.stroke_width(25);
        // Draw edges
        let mut pv = path[0].clone();
        chart
            .draw_series(path.iter().map(|v| {
                let e = PathElement::new(vec![pv.point.x_y(), v.point.x_y()], style);
                pv = v.clone();
                e
            }))
            .unwrap()
            .label("Edge")
            .legend(|(x, y)| PathElement::new([(x, y), (x + 20, y)], GREEN));
    }
}

/*
export ITERATIONS=5
export NUM_OBSTACLES=120
export NUM_VERTICES=1000
export WIDTH=100
export HEIGHT=100
export THREADS=1
*/
