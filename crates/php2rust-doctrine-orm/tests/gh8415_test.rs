//! Ticket/GH8415Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH8415Test.php

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
#[path = "common/ticket.rs"]
mod ticket;

use orm::*;
use ticket::*;

#[tokio::test]
async fn testAssociationIsBasedOnBaseClass() {
    let em = ticket::set_up(&[
        "GH8415BaseClass",
        "GH8415MiddleMappedSuperclass",
        "GH8415LeafClass",
        "GH8415AssociationTarget",
    ])
    .await;
    let target = new("GH8415AssociationTarget");
    let leaf = new("GH8415LeafClass");
    set(&leaf, "baseField", s("base"));
    set(&leaf, "middleField", s("middle"));
    set(&leaf, "leafField", s("leaf"));
    set_obj(&leaf, "target", &target);

    em.persist(&target).unwrap();
    em.persist(&leaf).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let query = em.create_query(
        "SELECT leaf FROM Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH8415LeafClass leaf JOIN leaf.target t",
    );
    let result = query.get_one_or_null_result(None).await.unwrap();

    assert_instance_of("GH8415LeafClass", &result);
    let result = obj(&result);
    assert!(s("base").identical(&get(&result, "baseField")));
    assert!(s("middle").identical(&get(&result, "middleField")));
    assert!(s("leaf").identical(&get(&result, "leafField")));
}
