//! ManyToManyCompositeIdTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa.
//! The entities all use a custom type that converts the value as
//! identifier(s) (`common/rot13.rs`). Each test has its own database, so
//! `tearDownAfterClass()` has nothing to drop.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/ValueConversionType/ManyToManyCompositeIdTest.php

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
    class("InversedManyToManyCompositeIdEntity")
}
fn owning() -> String {
    class("OwningManyToManyCompositeIdEntity")
}

async fn set_up() -> Arc<EntityManager> {
    let em = vct::set_up(&[inversed(), owning()]).await;

    let inv = entity_with_collection(&inversed(), &[("id1", "abc"), ("id2", "def")]);
    let own = entity_with_collection(&owning(), &[("id3", "ghi")]);

    add(&inv, &own);
    add(&own, &inv);

    persist_flush_clear(&em, &[&inv, &own]).await;
    em
}

fn inversed_id() -> Value {
    ids(&[("id1", s("abc")), ("id2", s("def"))])
}

#[tokio::test]
async fn testThatTheValueOfIdentifiersAreConvertedInTheDatabase() {
    let em = set_up().await;

    assert_fetch_one(
        &em,
        s("nop"),
        "SELECT id1 FROM vct_inversed_manytomany_compositeid LIMIT 1",
    )
    .await;
    assert_fetch_one(
        &em,
        s("qrs"),
        "SELECT id2 FROM vct_inversed_manytomany_compositeid LIMIT 1",
    )
    .await;

    assert_fetch_one(
        &em,
        s("tuv"),
        "SELECT id3 FROM vct_owning_manytomany_compositeid LIMIT 1",
    )
    .await;

    assert_fetch_one(
        &em,
        s("nop"),
        "SELECT inversed_id1 FROM vct_xref_manytomany_compositeid LIMIT 1",
    )
    .await;
    assert_fetch_one(
        &em,
        s("qrs"),
        "SELECT inversed_id2 FROM vct_xref_manytomany_compositeid LIMIT 1",
    )
    .await;
    assert_fetch_one(
        &em,
        s("tuv"),
        "SELECT owning_id FROM vct_xref_manytomany_compositeid LIMIT 1",
    )
    .await;
}

#[tokio::test]
async fn testThatEntitiesAreFetchedFromTheDatabase() {
    let em = set_up().await;

    let inv = find(&em, &inversed(), inversed_id()).await;
    let own = find(&em, &owning(), s("ghi")).await;

    assert_instance_of(&inversed(), &inv);
    assert_instance_of(&owning(), &own);
}

#[tokio::test]
async fn testThatTheValueOfIdentifiersAreConvertedBackAfterBeingFetchedFromTheDatabase() {
    let em = set_up().await;

    let inv = find(&em, &inversed(), inversed_id()).await;
    let own = find(&em, &owning(), s("ghi")).await;

    assert_field("abc", &inv, "id1");
    assert_field("def", &inv, "id2");
    assert_field("ghi", &own, "id3");
}

#[tokio::test]
async fn testThatTheCollectionFromOwningToInversedIsLoaded() {
    let em = set_up().await;
    let own = find(&em, &owning(), s("ghi")).await;

    assert_collection_count(1, &own).await;
}

#[tokio::test]
async fn testThatTheCollectionFromInversedToOwningIsLoaded() {
    let em = set_up().await;
    let inv = find(&em, &inversed(), inversed_id()).await;

    assert_collection_count(1, &inv).await;
}

#[tokio::test]
async fn testThatTheJoinTableRowsAreRemovedWhenRemovingTheAssociation() {
    let em = set_up().await;

    // remove association

    let inv = find(&em, &inversed(), inversed_id()).await;

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
        "SELECT COUNT(*) FROM vct_xref_manytomany_compositeid",
    )
    .await;
}
