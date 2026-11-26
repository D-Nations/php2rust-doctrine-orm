//! ManyToManyTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa.
//! The entities all use a custom type that converts the value as
//! identifier(s) (`common/rot13.rs`). Each test has its own database, so
//! `tearDownAfterClass()` has nothing to drop.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/ValueConversionType/ManyToManyTest.php

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
#[path = "common/rot13.rs"]
mod rot13;
#[path = "common/vct.rs"]
mod vct;

use std::sync::Arc;

use php_value::Value;
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::s;
use vct::*;

fn inversed() -> String {
    class("InversedManyToManyEntity")
}
fn owning() -> String {
    class("OwningManyToManyEntity")
}

async fn set_up() -> Arc<EntityManager> {
    let em = vct::set_up(&[inversed(), owning()]).await;

    let inv = entity_with_collection(&inversed(), &[("id1", "abc")]);
    let own = entity_with_collection(&owning(), &[("id2", "def")]);

    add(&inv, &own);
    add(&own, &inv);

    persist_flush_clear(&em, &[&inv, &own]).await;
    em
}

#[tokio::test]
async fn testThatTheValueOfIdentifiersAreConvertedInTheDatabase() {
    let em = set_up().await;

    assert_fetch_one(
        &em,
        s("nop"),
        "SELECT id1 FROM vct_inversed_manytomany LIMIT 1",
    )
    .await;

    assert_fetch_one(
        &em,
        s("qrs"),
        "SELECT id2 FROM vct_owning_manytomany LIMIT 1",
    )
    .await;

    assert_fetch_one(
        &em,
        s("nop"),
        "SELECT inversed_id FROM vct_xref_manytomany LIMIT 1",
    )
    .await;
    assert_fetch_one(
        &em,
        s("qrs"),
        "SELECT owning_id FROM vct_xref_manytomany LIMIT 1",
    )
    .await;
}

#[tokio::test]
async fn testThatEntitiesAreFetchedFromTheDatabase() {
    let em = set_up().await;

    let inv = find(&em, &inversed(), s("abc")).await;
    let own = find(&em, &owning(), s("def")).await;

    assert_instance_of(&inversed(), &inv);
    assert_instance_of(&owning(), &own);
}

#[tokio::test]
async fn testThatTheValueOfIdentifiersAreConvertedBackAfterBeingFetchedFromTheDatabase() {
    let em = set_up().await;

    let inv = find(&em, &inversed(), s("abc")).await;
    let own = find(&em, &owning(), s("def")).await;

    assert_field("abc", &inv, "id1");
    assert_field("def", &own, "id2");
}

#[tokio::test]
async fn testThatTheCollectionFromOwningToInversedIsLoaded() {
    let em = set_up().await;
    let own = find(&em, &owning(), s("def")).await;

    assert_collection_count(1, &own).await;
}

#[tokio::test]
async fn testThatTheCollectionFromInversedToOwningIsLoaded() {
    let em = set_up().await;
    let inv = find(&em, &inversed(), s("abc")).await;

    assert_collection_count(1, &inv).await;
}

#[tokio::test]
async fn testThatTheJoinTableRowsAreRemovedWhenRemovingTheAssociation() {
    let em = set_up().await;

    // remove association

    let inv = find(&em, &inversed(), s("abc")).await;

    for own in orm::elements(&inv, "associatedEntities").await {
        orm::remove_element(&inv, "associatedEntities", &own).await;
        orm::remove_element(&own, "associatedEntities", &inv).await;
    }

    em.flush().await.unwrap();
    em.clear().unwrap();

    // test association is removed

    assert_fetch_one(
        &em,
        Value::Int(0),
        "SELECT COUNT(*) FROM vct_xref_manytomany",
    )
    .await;
}
