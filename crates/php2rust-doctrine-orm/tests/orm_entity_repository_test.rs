//! Port of `ORM\EntityRepositoryTest` (the functional `EntityRepositoryTest`
//! is `entity_repository_test.rs`). The entity manager is a real one, not
//! a stub; `Expr` has no state, so `assertSame()` compares the two
//! builders' values.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/EntityRepositoryTest.php

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;

use std::sync::Arc;

use php2rust_doctrine_orm::entity_repository::EntityRepository;
use php2rust_doctrine_orm::mapping::ClassMetadata;

#[test]
fn testExpr() {
    let em = orm::entity_manager(orm::connection(), None);

    let metadata = Arc::new(ClassMetadata::new("foo", None, None));

    let repository = EntityRepository::new(&em, metadata);

    assert_eq!(
        format!("{:?}", em.get_expression_builder()),
        format!("{:?}", repository.expr().unwrap())
    );
}
