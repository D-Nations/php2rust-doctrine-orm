//! Port of `StronglyConnectedComponentsTest`.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Internal/StronglyConnectedComponentsTest.php

#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

use std::collections::HashMap;
use std::sync::Arc;

use php_value::object::GenericObject;
use php_value::{Object, php_array};
use php2rust_doctrine_orm::internal::{StronglyConnectedComponents, object_id};

#[derive(Default)]
struct Test {
    nodes: HashMap<String, Object>,
    scc: StronglyConnectedComponents,
}

impl Test {
    fn add_nodes(&mut self, names: &[&str]) {
        for n in names {
            let node: Object = Arc::new(GenericObject::new(
                "Doctrine\\Tests\\ORM\\Internal\\Node",
                php_array! {"name" => *n},
            ));
            self.nodes.insert((*n).to_owned(), node.clone());
            self.scc.add_node(&node);
        }
    }

    fn add_edge(&mut self, from: &str, to: &str) {
        let from = self.nodes[from].clone();
        let to = self.nodes[to].clone();
        self.scc.add_edge(&from, &to);
    }

    fn component(&self, name: &str) -> usize {
        object_id(
            &self
                .scc
                .get_node_representing_strongly_connected_component(&self.nodes[name])
                .unwrap(),
        )
    }

    fn assert_nodes_are_in_same_component(&self, first: &str, second: &str) {
        assert_eq!(self.component(first), self.component(second));
    }

    fn assert_nodes_are_not_in_same_component(&self, first: &str, second: &str) {
        assert_ne!(self.component(first), self.component(second));
    }
}

#[test]
fn testFindStronglyConnectedComponents() {
    // A -> B <-> C -> D <-> E
    let mut t = Test::default();
    t.add_nodes(&["A", "B", "C", "D", "E"]);
    t.add_edge("A", "B");
    t.add_edge("B", "C");
    t.add_edge("C", "B");
    t.add_edge("C", "D");
    t.add_edge("D", "E");
    t.add_edge("E", "D");
    t.scc.find_strongly_connected_components();
    t.assert_nodes_are_in_same_component("B", "C");
    t.assert_nodes_are_in_same_component("D", "E");
    t.assert_nodes_are_not_in_same_component("A", "B");
    t.assert_nodes_are_not_in_same_component("A", "D");
}

#[test]
fn testFindStronglyConnectedComponents2() {
    // A -> B -> C -> D -> B
    let mut t = Test::default();
    t.add_nodes(&["A", "B", "C", "D"]);
    t.add_edge("A", "B");
    t.add_edge("B", "C");
    t.add_edge("C", "D");
    t.add_edge("D", "B");
    t.scc.find_strongly_connected_components();
    t.assert_nodes_are_in_same_component("B", "C");
    t.assert_nodes_are_in_same_component("C", "D");
    t.assert_nodes_are_not_in_same_component("A", "B");
}

#[test]
fn testFindStronglyConnectedComponents3() {
    let mut t = Test::default();
    t.add_nodes(&["A", "B", "C", "D", "E"]);
    t.add_edge("A", "B");
    t.add_edge("B", "C");
    t.add_edge("C", "D");
    t.add_edge("D", "E");
    t.add_edge("E", "C");
    t.add_edge("D", "B");
    t.scc.find_strongly_connected_components();
    t.assert_nodes_are_in_same_component("B", "C");
    t.assert_nodes_are_in_same_component("C", "D");
    t.assert_nodes_are_in_same_component("D", "E");
    t.assert_nodes_are_in_same_component("E", "B");
    t.assert_nodes_are_not_in_same_component("A", "B");
}
