#![allow(unused)]
mod astar;
mod dprm;
mod prm;
pub mod prelude {
    use std::sync::Arc;
    use std::sync::Mutex;

    pub use crate::astar::*;
    pub use crate::dprm::*;

    use geo::{Contains, Intersects};
    use geo::{Line, Point, Rect};
    use plotters::prelude::*;
    use rand::{prelude::*, seq::index};
    use rand_chacha::ChaCha8Rng;

    pub type EdgeIndex = usize;
    pub type VertexIndex = usize;
    pub type ObstacleId = u128;

    #[derive(Clone)]
    pub struct Edge {
        pub line: Line<f64>,
        pub length: f64,
        pub points: (VertexIndex, VertexIndex),
    }

    impl PartialEq for Edge {
        fn eq(&self, other: &Self) -> bool {
            self.points == other.points
        }
    }

    #[derive(Clone)]
    pub struct Vertex {
        pub point: Point<f64>,
        pub index: VertexIndex,
    }

    #[derive(Clone, Copy)]
    pub struct Obstacle {
        pub rect: Rect<f64>,
        id: ObstacleId,
    }

    impl PartialEq for Obstacle {
        fn eq(&self, other: &Self) -> bool {
            self.rect.min() == other.rect.min() && self.rect.max() == other.rect.max()
        }
    }

    impl Obstacle {
        pub fn new_random(rng: &mut ChaCha8Rng, w: usize, h: usize) -> Obstacle {
            let x_min = rng.gen_range(0.0..(w as f64 - 1.0));
            let y_min = rng.gen_range(0.0..(h as f64 - 1.0));
            let width = rng.gen_range(1.0..(w as f64 / 10.0));
            let height = rng.gen_range(1.0..(h as f64 / 10.0));
            let rect = Rect::new((x_min, y_min), (x_min + width, y_min + height));
            Obstacle {
                rect,
                id: rng.gen_range(0..u128::MAX),
            }
        }

        pub fn new(c1: (f64, f64), c2: (f64, f64)) -> Obstacle {
            Obstacle {
                rect: Rect::new(c1, c2),
                id: 0,
            }
        }

        pub fn id(&self) -> ObstacleId {
            self.id
        }

        fn contains(&self, point: &Point<f64>) -> bool {
            self.rect.contains(point)
        }

        pub fn intersects(&self, edge: &Line<f64>) -> bool {
            self.rect.intersects(edge)
        }

        pub fn rectangle(&self) -> Rectangle<(f64, f64)> {
            Rectangle::new(
                [self.rect.min().x_y(), self.rect.max().x_y()],
                (MAGENTA).filled(),
            )
        }
    }

    #[derive(Clone)]
    pub struct ObstacleSet {
        pub obstacles: Vec<Obstacle>,
    }

    impl ObstacleSet {
        pub fn new_random(
            n: usize,
            width: usize,
            height: usize,
            rng: &mut ChaCha8Rng,
        ) -> ObstacleSet {
            let mut obstacles = Vec::new();
            while obstacles.len() < n {
                obstacles.push(Obstacle::new_random(rng, width, height));
            }
            ObstacleSet { obstacles }
        }

        pub fn contains(&self, point: &Point<f64>) -> bool {
            self.obstacles.iter().any(|o| o.contains(point))
        }

        pub fn intersects(&self, edge: &Line<f64>) -> bool {
            self.obstacles.iter().any(|o| o.intersects(edge))
        }

        pub fn remove(&mut self, obstacle: &Obstacle) {
            self.obstacles.retain(|o| o != obstacle);
        }
    }

    #[derive(Clone)]
    pub struct PrmConfig {
        pub num_vertices: usize,
        pub width: usize,
        pub height: usize,
        pub seed: Arc<Mutex<[u8; 32]>>,
        pub use_viable_edges: bool,
        pub use_blocked_per_obstacle: bool,
        pub threads: usize,
    }

    impl PrmConfig {
        pub fn new(
            num_vertices: usize,
            width: usize,
            height: usize,
            seed: [u8; 32],
            threads: usize,
        ) -> PrmConfig {
            PrmConfig {
                num_vertices,
                width,
                height,
                seed: Arc::new(Mutex::new(seed)),
                use_viable_edges: false,         // Default to false
                use_blocked_per_obstacle: false, // Default to false
                threads,
            }
        }
    }
}
