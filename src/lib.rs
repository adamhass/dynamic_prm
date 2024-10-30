mod dprm;
pub mod prelude {
    use serde::{Deserialize, Serialize};
    pub use crate::dprm::*;

    use geo::{Contains, Intersects};
    use geo::{Line, Point, Rect};
    use plotters::prelude::*;
    use rand::{prelude::*};
    use rand_chacha::ChaCha8Rng;

    pub type EdgeIndex = usize;
    pub type VertexIndex = usize;
    pub type ObstacleId = u128;

    #[derive(Clone, Serialize, Deserialize, Debug)]
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

    #[derive(Clone, Serialize, Deserialize, Debug)]
    pub struct Vertex {
        pub point: Point<f64>,
        pub index: VertexIndex,
    }

    #[derive(Clone, Copy, Serialize, Deserialize, Debug)]
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
        pub fn new_random(rng: &mut ChaCha8Rng, obstacle_max_size: f64, obstacle_min_size: f64, x_min: f64, y_min: f64, x_max: f64, y_max: f64) -> Obstacle {
            let x_pos = rng.gen_range(x_min..(x_max));
            let y_pos = rng.gen_range(y_min..(y_max));
            let width = rng.gen_range(obstacle_min_size..(obstacle_max_size));
            let height = rng.gen_range(obstacle_min_size..(obstacle_max_size));
            let rect = Rect::new((x_pos, y_pos), (x_pos + width, y_pos + height));
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

    #[derive(Clone, Serialize, Deserialize, Debug)]
    pub struct ObstacleSet {
        pub obstacles: Vec<Obstacle>,
    }

    impl ObstacleSet {
        pub fn new_random(
            n: usize,
            obstacle_max_size: f64, obstacle_min_size: f64, x_min: f64, y_min: f64, x_max: f64, y_max: f64,
            rng: &mut ChaCha8Rng,
        ) -> ObstacleSet {
            let mut obstacles = Vec::new();
            while obstacles.len() < n {
                obstacles.push(Obstacle::new_random(rng, obstacle_max_size, obstacle_min_size, x_min, y_min, x_max, y_max));
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

        pub fn add(&mut self, obstacle: Obstacle) {
            self.obstacles.push(obstacle);
        }
    }

    #[derive(Clone, Serialize, Deserialize, Debug)]
    pub struct PrmConfig {
        pub num_vertices: usize,
        pub width: usize,
        pub height: usize,
        pub seed: [u8; 32],
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
                seed,
                use_viable_edges: false,         // Default to false
                use_blocked_per_obstacle: false, // Default to false
                threads,
            }
        }
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct DPrmPath {
        pub vertices: Vec<Vertex>,
        pub length: Distance,
    }

    pub type Distance = usize;
}
