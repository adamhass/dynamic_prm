use std::sync::Arc;

use super::prelude::*;
use geo::EuclideanDistance;
use pathfinding::directed::astar::astar;

pub type AstarPath = (Vec<Vertex>, usize);
pub type Distance = usize;
pub struct Astar {
    pub prm: DPrm,
    pub optimized: bool,
    pub neighbours: Vec<Vec<(VertexIndex, Distance)>>,
}

impl Astar {
    pub fn new(prm: DPrm) -> Self {
        Astar {
            prm,
            optimized: false,
            neighbours: Vec::new(),
        }
    }

    pub fn init_neighbours(&mut self) {
        self.neighbours = (0..self.prm.vertices.len())
            .map(|i| (self.basic_successors(&i)))
            .collect();
    }

    pub fn run_astar(&self, start: Vertex, end: Vertex) -> Option<AstarPath> {
        // Run the A* algorithm
        // If optimized, use the optimized A* algorithm
        if self.optimized {
            self.run_optimized_astar(start, end)
        } else {
            self.run_basic_astar(start.index, end.index)
        }
    }

    pub fn run_basic_astar(&self, start: VertexIndex, end: VertexIndex) -> Option<AstarPath> {
        // Run the basic A* algorithm
        if let Some((path, length)) = astar(
            &start,
            |v| self.basic_successors(v),
            |v| self.heuristic(*v, end),
            |v| *v == end,
        ) {
            let mut ret = Vec::new();
            for i in path {
                ret.push(self.prm.vertices[i].clone());
            }
            return Some((ret, length));
        }
        None
    }

    pub fn basic_successors(&self, start: &VertexIndex) -> Vec<(VertexIndex, Distance)> {
        // Get the basic successors
        let mut successors = Vec::new();
        for e in self.prm.edges.iter() {
            if e.points.0 == *start {
                successors.push((e.points.1, e.length.round() as Distance));
            } else if e.points.1 == *start {
                successors.push((e.points.0, e.length.round() as Distance));
            }
        }
        successors
    }

    pub fn heuristic(&self, start: VertexIndex, end: VertexIndex) -> Distance {
        // Get the heuristic
        self.prm.vertices[start]
            .point
            .euclidean_distance(&self.prm.vertices[end].point)
            .round() as Distance
    }

    fn successors(&self, start: &VertexIndex) -> Vec<(VertexIndex, Distance)> {
        // Get the successors
        self.neighbours[*start].clone()
    }

    pub fn run_optimized_astar(&self, start: Vertex, end: Vertex) -> Option<AstarPath> {
        // Run the basic A* algorithm
        if let Some((path, length)) = astar(
            &start.index,
            |v| self.successors(v),
            |v| self.heuristic(*v, end.index),
            |v| *v == end.index,
        ) {
            let mut ret = Vec::new();
            for i in path {
                ret.push(self.prm.vertices[i].clone());
            }
            return Some((ret, length));
        }
        None
    }
}
