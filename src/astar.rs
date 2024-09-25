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

    /// Initializes the neighbours of the vertices, to speed up A*
    pub fn init_neighbours(&mut self) {
        for i in 0..self.prm.vertices.len() {
            self.neighbours.push(Vec::new());
        }
        for e in self.prm.edges.values() {
            self.neighbours[e.points.0].push((e.points.1, e.length.round() as Distance));
            self.neighbours[e.points.1].push((e.points.0, e.length.round() as Distance));
        }
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
        for e in self.prm.edges.values() {
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

    pub fn remove_edges(&mut self, edges: Vec<EdgeIndex>) {
        if self.optimized {
            for e in edges {
                let edge = self.prm.edges.get(&e).unwrap();
                self.neighbours[edge.points.0].retain(|(v, _)| *v != edge.points.1);
                self.neighbours[edge.points.1].retain(|(v, _)| *v != edge.points.0);
            }
        }
    }

    pub fn insert_edges(&mut self, edges: Vec<EdgeIndex>) {
        if self.optimized {
            for e in edges {
                let edge = self.prm.edges.get(&e).unwrap();
                self.neighbours[edge.points.0]
                    .push((edge.points.1, edge.length.round() as Distance));
                self.neighbours[edge.points.1]
                    .push((edge.points.0, edge.length.round() as Distance));
            }
        }
    }
}
