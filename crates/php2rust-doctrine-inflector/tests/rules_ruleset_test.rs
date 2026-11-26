//! RulesetTest.php at doctrine/inflector 6d6c96277ea252fc1304627204c3d5e6e15faa3b.
//! Names and assertions are PHPUnit's; `assertSame()` of the mocks is `Arc::ptr_eq()`.
//!
//! Upstream: doctrine/inflector tests/Rules/RulesetTest.php

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

mod support;

use std::sync::Arc;

use php2rust_doctrine_inflector::WordInflector;
use php2rust_doctrine_inflector::rules::{Ruleset, WordMatcher};
use support::{MockWordInflector, MockWordMatcher};

struct TestCase {
    regular: Arc<dyn WordInflector>,
    uninflected: Arc<dyn WordMatcher>,
    irregular: Arc<dyn WordInflector>,
    ruleset: Ruleset,
}

fn set_up() -> TestCase {
    let regular: Arc<dyn WordInflector> = Arc::new(MockWordInflector::default());
    let uninflected: Arc<dyn WordMatcher> = Arc::new(MockWordMatcher::default());
    let irregular: Arc<dyn WordInflector> = Arc::new(MockWordInflector::default());
    let ruleset = Ruleset::from_parts(regular.clone(), uninflected.clone(), irregular.clone());
    TestCase {
        regular,
        uninflected,
        irregular,
        ruleset,
    }
}

#[test]
fn testGetRegular() {
    let t = set_up();
    assert!(Arc::ptr_eq(&t.regular, t.ruleset.get_regular()));
}

#[test]
fn testGetUninflected() {
    let t = set_up();
    assert!(Arc::ptr_eq(&t.uninflected, t.ruleset.get_uninflected()));
}

#[test]
fn testGetIrregular() {
    let t = set_up();
    assert!(Arc::ptr_eq(&t.irregular, t.ruleset.get_irregular()));
}
