//! Port of `TopologicalSortTest`; `Node` is a generic object with a
//! public `name`.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Internal/TopologicalSortTest.php tests/Tests/ORM/Internal/Node.php

#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

use std::collections::HashMap;
use std::sync::Arc;

use php_value::object::GenericObject;
use php_value::{Object, Value, php_array};
use php2rust_doctrine_orm::internal::{CycleDetectedException, TopologicalSort, object_id};

#[derive(Default)]
struct Test {
    nodes: HashMap<String, Object>,
    topological_sort: TopologicalSort,
}

fn name(node: &Object) -> String {
    match node.properties().get(&"name".into()) {
        Some(Value::String(s)) => s.to_string_lossy().into_owned(),
        _ => String::new(),
    }
}

impl Test {
    fn add_nodes(&mut self, names: &[&str]) {
        for n in names {
            let node: Object = Arc::new(GenericObject::new(
                "Doctrine\\Tests\\ORM\\Internal\\Node",
                php_array! {"name" => *n},
            ));
            self.nodes.insert((*n).to_owned(), node.clone());
            self.topological_sort.add_node(&node);
        }
    }

    fn add_edge(&mut self, from: &str, to: &str, optional: bool) {
        let from = self.nodes[from].clone();
        let to = self.nodes[to].clone();
        self.topological_sort.add_edge(&from, &to, optional);
    }

    fn compute_result(&mut self) -> Result<Vec<String>, CycleDetectedException> {
        Ok(self.topological_sort.sort()?.iter().map(name).collect())
    }
}

#[test]
fn testSimpleOrdering() {
    let mut t = Test::default();
    t.add_nodes(&["C", "B", "A", "E"]);
    t.add_edge("A", "B", false);
    t.add_edge("B", "C", false);
    t.add_edge("E", "A", false);
    // There is only 1 valid ordering for this constellation
    assert_eq!(vec!["C", "B", "A", "E"], t.compute_result().unwrap());
}

#[test]
fn testSkipOptionalEdgeToBreakCycle() {
    let mut t = Test::default();
    t.add_nodes(&["A", "B"]);
    t.add_edge("A", "B", true);
    t.add_edge("B", "A", false);
    assert_eq!(vec!["A", "B"], t.compute_result().unwrap());
}

#[test]
fn testBreakCycleByBacktracking() {
    let mut t = Test::default();
    t.add_nodes(&["A", "B", "C", "D"]);
    t.add_edge("A", "B", false);
    t.add_edge("B", "C", true);
    t.add_edge("C", "D", false);
    t.add_edge("D", "A", false); // closes the cycle
    // We can only break B -> C, so the result must be C -> D -> A -> B
    assert_eq!(vec!["B", "A", "D", "C"], t.compute_result().unwrap());
}

#[test]
fn testCycleRemovedByEliminatingLastOptionalEdge() {
    let mut t = Test::default();
    t.add_nodes(&["A", "B", "C", "D"]);
    t.add_edge("A", "B", true);
    t.add_edge("B", "C", true);
    t.add_edge("C", "A", false);
    t.add_edge("B", "D", true);
    t.add_edge("D", "A", false);
    assert_eq!(vec!["B", "A", "C", "D"], t.compute_result().unwrap());
}

#[test]
fn testGH7180Example() {
    let mut t = Test::default();
    t.add_nodes(&["E", "F", "D", "G"]);
    t.add_edge("D", "G", false);
    t.add_edge("D", "F", true);
    t.add_edge("F", "E", false);
    t.add_edge("E", "D", false);
    assert_eq!(vec!["G", "D", "E", "F"], t.compute_result().unwrap());
}

#[test]
fn testCommitOrderingFromGH7259Test() {
    let mut t = Test::default();
    t.add_nodes(&["A", "B", "C", "D"]);
    t.add_edge("D", "A", false);
    t.add_edge("A", "B", false);
    t.add_edge("D", "C", false);
    t.add_edge("A", "D", true);
    let correct_orders = [
        vec!["C", "B", "A", "D"],
        vec!["B", "C", "A", "D"],
        vec!["B", "A", "C", "D"],
    ];
    let result = t.compute_result().unwrap();
    assert!(correct_orders.iter().any(|order| *order == result));
}

#[test]
fn testCommitOrderingFromGH8349Case1Test() {
    let mut t = Test::default();
    t.add_nodes(&["A", "B", "C", "D"]);
    t.add_edge("D", "A", false);
    t.add_edge("A", "B", true);
    t.add_edge("B", "D", true);
    t.add_edge("B", "C", true);
    t.add_edge("C", "D", true);
    // Many orderings are possible here, but the bottom line is A must be before D (it's the only hard requirement).
    let result = t.compute_result().unwrap();
    let index_a = result.iter().position(|n| n == "A").unwrap();
    let index_d = result.iter().position(|n| n == "D").unwrap();
    assert!(index_d > index_a);
}

#[test]
fn testCommitOrderingFromGH8349Case2Test() {
    let mut t = Test::default();
    t.add_nodes(&["A", "B"]);
    t.add_edge("B", "A", false);
    t.add_edge("B", "A", true); // interesting: We have two edges in that direction
    t.add_edge("A", "B", true);
    // The B -> A requirement determines the result here
    assert_eq!(vec!["A", "B"], t.compute_result().unwrap());
}

#[test]
fn testNodesMaintainOrderWhenNoDepencency() {
    let mut t = Test::default();
    t.add_nodes(&["A", "B", "C"]);
    assert_eq!(vec!["A", "B", "C"], t.compute_result().unwrap());
}

#[test]
fn testNodesReturnedInDepthFirstOrder() {
    let mut t = Test::default();
    t.add_nodes(&["A", "B", "C"]);
    t.add_edge("A", "B", false);
    t.add_edge("A", "C", false);
    assert_eq!(vec!["B", "C", "A"], t.compute_result().unwrap());
}

#[test]
fn testNodesReturnedInDepthFirstOrderWithEdgesInDifferentOrderThanNodes() {
    let mut t = Test::default();
    t.add_nodes(&["A", "B", "C"]);
    t.add_edge("A", "C", false);
    t.add_edge("A", "B", false);
    assert_eq!(vec!["C", "B", "A"], t.compute_result().unwrap());
}

#[test]
fn testNodesReturnedInDepthFirstOrderWithDependingNodeLast() {
    let mut t = Test::default();
    t.add_nodes(&["B", "C", "A"]);
    t.add_edge("A", "B", false);
    t.add_edge("A", "C", false);
    assert_eq!(vec!["B", "C", "A"], t.compute_result().unwrap());
}

#[test]
fn testNodesReturnedInDepthFirstOrderWithDependingNodeLastAndEdgeOrderInversed() {
    let mut t = Test::default();
    t.add_nodes(&["B", "C", "A"]);
    t.add_edge("A", "C", false);
    t.add_edge("A", "B", false);
    assert_eq!(vec!["B", "C", "A"], t.compute_result().unwrap());
}

#[test]
fn testDetectSmallCycle() {
    let mut t = Test::default();
    t.add_nodes(&["A", "B"]);
    t.add_edge("A", "B", false);
    t.add_edge("B", "A", false);
    let exception = t.compute_result().unwrap_err();
    assert_eq!(CycleDetectedException::CLASS, exception.to_thrown().class);
}

#[test]
fn testMultipleEdges() {
    let mut t = Test::default();
    t.add_nodes(&["A", "B"]);
    t.add_edge("A", "B", true); // optional comes first
    t.add_edge("A", "B", false);
    t.add_edge("B", "A", false);
    t.add_edge("B", "A", true); // optional comes last
    assert!(t.compute_result().is_err());
}

#[test]
fn testDetectLargerCycleNotIncludingStartNode() {
    let mut t = Test::default();
    t.add_nodes(&["A", "B", "C", "D"]);
    t.add_edge("A", "B", false);
    t.add_edge("B", "C", false);
    t.add_edge("C", "D", false);
    t.add_edge("D", "B", false);
    if let Err(exception) = t.compute_result() {
        let ids: Vec<usize> = exception.get_cycle().iter().map(object_id).collect();
        let expected: Vec<usize> = ["B", "C", "D", "B"]
            .iter()
            .map(|n| object_id(&t.nodes[*n]))
            .collect();
        assert_eq!(expected, ids);
    }
}
