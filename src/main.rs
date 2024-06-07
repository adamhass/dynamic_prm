#![allow(unused)]
// use pathfinding::directed::astar::astar;
use bytes::{Buf, Bytes, BytesMut};
use hyper::body::Incoming;
use hyper::client::conn::http1::SendRequest;
use hyper::{Method, Request, Response, StatusCode, Uri};
use http_body_util::{BodyExt, Full};
use hyper_util::rt::{TokioExecutor, TokioIo};
use tokio::net::TcpListener;
use dynamic_prm::prelude::*;
use plotters::prelude::*;
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use std::io::{stdin, Stdin};
use std::sync::Arc;
use std::{env, time::Instant};
use geo::{Contains, Intersects};
use geo::{Line, Point, Rect};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

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
    let width = 100;
    let height = 100;
    let num_vertices = 10000;
    let num_obstacles = 100;
    // Print the parameters
    println!("* Parsed Parameters:");
    println!("* NUM_OBSTACLES: {}", num_obstacles);
    println!("* NUM_VERTICES: {}", num_vertices);
    println!("* WIDTH: {}", width);
    println!("* HEIGHT: {}", height);
    println!("* THREADS: {}", threads);
    // Iteration set-up
    let seed = [0_u8; 32];
    let mut cfg = PrmConfig::new(num_vertices, width, height, seed);
    let mut prm = Prm::new(cfg, num_obstacles);
    prm.print();
    // Initialize the dprm
    println!("Initializing DPrm...");
    let mut dprm = DPrm::new(prm);
    dprm.update_viable_edges_and_vertices(threads).await;
    dprm.update_all_blocked(threads).await;
    let addr = "127.0.0.1:8080".to_string();
    run_server(addr, dprm).await;
}

async fn test_dprm(mut dprm: DPrm, threads: usize, width: usize, height: usize) {
    // Do parallel PRM
    let start_time = Instant::now();
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
    dprm.plot(format!("dprm_with_path"), path);
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

fn plot(name: String, prm: &Prm, path: Option<Vec<Vertex>>) -> () {
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
                .map(|v| Circle::new(v.point.0.x_y(), 2, &BLACK)),
        )
        .unwrap();

    // Draw edges
    chart
        .draw_series(
            prm.edges.iter().map(|edge| {
                PathElement::new(vec![edge.line.start.x_y(), edge.line.end.x_y()], &BLUE)
            }),
        )
        .unwrap()
        .label("Edge")
        .legend(|(x, y)| PathElement::new([(x, y), (x + 20, y)], &BLUE));

    // Draw viable edges
    chart
        .draw_series(
            prm.viable_edges.iter().map(|edge| {
                PathElement::new(vec![edge.line.start.x_y(), edge.line.end.x_y()], &RED)
            }),
        )
        .unwrap()
        .label("Edge")
        .legend(|(x, y)| PathElement::new([(x, y), (x + 20, y)], &BLUE));

    // Draw path 
    if let Some(path) = path {
        let style = GREEN;
        style.stroke_width(25);
        // Draw edges
        let mut pv = path[0].clone();
        chart
            .draw_series(
                path.iter().map(|v| {
                    let e = PathElement::new(vec![pv.point.x_y(), v.point.x_y()], style.clone());
                    pv = v.clone();
                    e
                }),
            )
            .unwrap()
            .label("Edge")
            .legend(|(x, y)| PathElement::new([(x, y), (x + 20, y)], &GREEN));
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
