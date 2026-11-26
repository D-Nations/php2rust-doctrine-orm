//! `StronglyConnectedComponents` implements Tarjan's algorithm to find
//! strongly connected components (SCC) in a directed graph. This algorithm
//! has a linear running time based on nodes (V) and edges between the
//! nodes (E), resulting in a computational complexity of O(V + E).
//!
//! See <https://en.wikipedia.org/wiki/Tarjan%27s_strongly_connected_components_algorithm>
//! for an explanation and the meaning of the DFS and lowlink numbers.
//!
//! Upstream: doctrine/orm src/Internal/StronglyConnectedComponents.php

use std::collections::HashMap;

use php_value::{Object, Thrown};

use super::object_id;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    NotVisited,
    InProgress,
    Visited,
}

/// `Doctrine\ORM\Internal\StronglyConnectedComponents`.
#[derive(Debug, Default)]
pub struct StronglyConnectedComponents {
    order: Vec<usize>,
    nodes: HashMap<usize, Object>,
    states: HashMap<usize, State>,
    edges: HashMap<usize, Vec<usize>>,
    dfs: HashMap<usize, usize>,
    lowlink: HashMap<usize, usize>,
    maxdfs: usize,
    /// Nodes representing the SCC another node is in, indexed by
    /// lookup-node object ID.
    representing_nodes: HashMap<usize, Object>,
    /// Stack with OIDs of nodes visited in the current state of the DFS.
    stack: Vec<usize>,
}

impl StronglyConnectedComponents {
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

    /// Adds a new edge between two nodes to the graph.
    pub fn add_edge(&mut self, from: &Object, to: &Object) {
        let to_id = object_id(to);
        let edges = self.edges.entry(object_id(from)).or_default();
        if !edges.contains(&to_id) {
            edges.push(to_id);
        }
    }

    pub fn find_strongly_connected_components(&mut self) {
        for oid in self.order.clone() {
            if self.states.get(&oid) == Some(&State::NotVisited) {
                self.tarjan(oid);
            }
        }
    }

    fn tarjan(&mut self, oid: usize) {
        let number = self.maxdfs;
        self.maxdfs += 1;
        self.dfs.insert(oid, number);
        self.lowlink.insert(oid, number);
        self.states.insert(oid, State::InProgress);
        self.stack.push(oid);

        for adjacent_id in self.edges.get(&oid).cloned().unwrap_or_default() {
            match self.states.get(&adjacent_id) {
                Some(State::NotVisited) => {
                    self.tarjan(adjacent_id);
                    let low = self.lowlink[&oid].min(self.lowlink[&adjacent_id]);
                    self.lowlink.insert(oid, low);
                }
                Some(State::InProgress) => {
                    let low = self.lowlink[&oid].min(self.dfs[&adjacent_id]);
                    self.lowlink.insert(oid, low);
                }
                _ => {}
            }
        }

        if self.lowlink[&oid] == self.dfs[&oid] {
            let mut representing_node: Option<Object> = None;
            while let Some(unwind_oid) = self.stack.pop() {
                if representing_node.is_none() {
                    representing_node = self.nodes.get(&unwind_oid).cloned();
                }
                if let Some(node) = &representing_node {
                    self.representing_nodes.insert(unwind_oid, node.clone());
                }
                self.states.insert(unwind_oid, State::Visited);
                if unwind_oid == oid {
                    break;
                }
            }
        }
    }

    /// The node representing the strongly connected component `node` is
    /// in; `InvalidArgumentException` "unknown node" for another node.
    pub fn get_node_representing_strongly_connected_component(
        &self,
        node: &Object,
    ) -> Result<Object, Thrown> {
        self.representing_nodes
            .get(&object_id(node))
            .cloned()
            .ok_or_else(|| Thrown::new("InvalidArgumentException", "unknown node", 0))
    }
}
