//! TopologicalSort implements topological sorting, which is an ordering
//! algorithm for directed graphs (DG) using a depth-first searching (DFS)
//! to traverse the graph built in memory.
//! This algorithm has a linear running time based on nodes (V) and edges
//! between the nodes (E), resulting in a computational complexity of O(V + E).
//!
//! Nodes are objects (`php_value::Object`), told apart by identity
//! (`spl_object_id()`).
//!
//! Upstream: doctrine/orm src/Internal/TopologicalSort.php src/Internal/TopologicalSort/CycleDetectedException.php

use std::collections::HashMap;

use php_value::{Object, Thrown};

use super::object_id;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    NotVisited,
    InProgress,
    Visited,
}

/// `Doctrine\ORM\Internal\TopologicalSort`.
#[derive(Debug, Default)]
pub struct TopologicalSort {
    /// All nodes, in the order they were added.
    order: Vec<usize>,
    nodes: HashMap<usize, Object>,
    /// DFS state for the different nodes, indexed by node object id.
    states: HashMap<usize, State>,
    /// Edges between the nodes: outgoing node → (destination, optional),
    /// in the order the edges were first added.
    edges: HashMap<usize, Vec<(usize, bool)>>,
    /// Builds up the result during the DFS.
    sort_result: Vec<Object>,
}

/// `Doctrine\ORM\Internal\TopologicalSort\CycleDetectedException`.
#[derive(Debug, Clone)]
pub struct CycleDetectedException {
    start_node: Object,
    cycle: Vec<Object>,
    /// Do we have the complete cycle collected?
    cycle_collected: bool,
}

impl CycleDetectedException {
    pub const CLASS: &'static str = "Doctrine\\ORM\\Internal\\TopologicalSort\\CycleDetectedException";
    pub const MESSAGE: &'static str = "A cycle has been detected, so a topological sort is not possible. The getCycle() method provides the list of nodes that form the cycle.";

    pub fn new(start_node: Object) -> Self {
        Self {
            cycle: vec![start_node.clone()],
            start_node,
            cycle_collected: false,
        }
    }

    pub fn get_cycle(&self) -> &[Object] {
        &self.cycle
    }

    pub fn add_to_cycle(&mut self, node: Object) {
        let is_start = object_id(&node) == object_id(&self.start_node);
        self.cycle.insert(0, node);
        if is_start {
            self.cycle_collected = true;
        }
    }

    pub fn is_cycle_collected(&self) -> bool {
        self.cycle_collected
    }

    /// The exception as a PHP `RuntimeException` subclass.
    pub fn to_thrown(&self) -> Thrown {
        Thrown::new(Self::CLASS, Self::MESSAGE, 0)
    }
}

impl TopologicalSort {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: &Object) {
        let id = object_id(node);
        if !self.nodes.contains_key(&id) {
            self.order.push(id);
        }
        self.nodes.insert(id, node.clone());
        self.states.insert(id, State::NotVisited);
        self.edges.insert(id, Vec::new());
    }

    pub fn has_node(&self, node: &Object) -> bool {
        self.nodes.contains_key(&object_id(node))
    }

    /// Adds a new edge between two nodes to the graph. `optional` indicates
    /// whether the edge may be ignored during the topological sort if it
    /// is necessary to break cycles.
    pub fn add_edge(&mut self, from: &Object, to: &Object, optional: bool) {
        let from_id = object_id(from);
        let to_id = object_id(to);
        let edges = self.edges.entry(from_id).or_default();
        match edges.iter_mut().find(|(id, _)| *id == to_id) {
            // we already know about this dependency, and it is not optional
            Some((_, false)) => {}
            Some((_, known)) => *known = optional,
            None => edges.push((to_id, optional)),
        }
    }

    /// Returns a topological sort of all nodes. When we have an edge A->B
    /// between two nodes A and B, then B will be listed before A in the
    /// result. Visually speaking, when ordering the nodes in the result
    /// order from left to right, all edges point to the left.
    pub fn sort(&mut self) -> Result<Vec<Object>, CycleDetectedException> {
        for oid in self.order.clone() {
            if self.states.get(&oid) == Some(&State::NotVisited) {
                self.visit(oid)?;
            }
        }
        Ok(self.sort_result.clone())
    }

    fn node(&self, oid: usize) -> Option<Object> {
        self.nodes.get(&oid).cloned()
    }

    fn visit(&mut self, oid: usize) -> Result<(), CycleDetectedException> {
        let Some(node) = self.node(oid) else {
            return Ok(());
        };
        match self.states.get(&oid) {
            // This node is already on the current DFS stack. We've found a cycle!
            Some(State::InProgress) => return Err(CycleDetectedException::new(node)),
            // We've reached a node that we've already seen, including all
            // other nodes that are reachable from here. We're done here, return.
            Some(State::Visited) => return Ok(()),
            _ => {}
        }
        self.states.insert(oid, State::InProgress);

        // Continue the DFS downwards the edge list
        let edges = self.edges.get(&oid).cloned().unwrap_or_default();
        for (adjacent_id, optional) in edges {
            if let Err(mut exception) = self.visit(adjacent_id) {
                if exception.is_cycle_collected() {
                    // There is a complete cycle downstream of the current node. We cannot
                    // do anything about that anymore.
                    return Err(exception);
                }
                if optional {
                    // The current edge is part of a cycle, but it is optional and the closest
                    // such edge while backtracking. Break the cycle here by skipping the edge
                    // and continuing with the next one.
                    continue;
                }
                // We have found a cycle and cannot break it at $edge. Best we can do
                // is to backtrack from the current vertex, hoping that somewhere up the
                // stack this can be salvaged.
                self.states.insert(oid, State::NotVisited);
                exception.add_to_cycle(node);
                return Err(exception);
            }
        }

        // We have traversed all edges and visited all other nodes reachable from here.
        // So we're done with this vertex as well.
        self.states.insert(oid, State::Visited);
        self.sort_result.push(node);
        Ok(())
    }
}
