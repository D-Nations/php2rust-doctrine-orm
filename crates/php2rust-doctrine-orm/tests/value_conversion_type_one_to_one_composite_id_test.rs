//! OneToOneCompositeIdTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa.
//! The entities all use a custom type that converts the value as
//! identifier(s) (`common/rot13.rs`). Each test has its own database, so
//! `tearDownAfterClass()` has nothing to drop.
//! The inverse side's one-to-one has two join columns: it is loaded
//! through the persister's LEFT JOIN of the inverse one-to-one. A lazy
//! proxy does not load on a property read here, so the proxy test calls
//! `initialize_object()` where PHP's property read would.
//! `#[Depends]` is a fresh fixture.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/ValueConversionType/OneToOneCompositeIdTest.php

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

use php2rust_doctrine_orm::entity_manager::EntityManager;

use php_value::Value;

use orm::s;
use vct::*;

fn inversed() -> String {
    class("InversedOneToOneCompositeIdEntity")
}
fn owning() -> String {
    class("OwningOneToOneCompositeIdEntity")
}

async fn set_up() -> Arc<EntityManager> {
    let em = vct::set_up(&[inversed(), owning()]).await;

    let inv = entity(
        &inversed(),
        &[
            ("id1", "abc"),
            ("id2", "def"),
            ("someProperty", "some value to be loaded"),
        ],
    );
    let own = entity(&owning(), &[("id3", "ghi")]);

    orm::set_obj(&inv, "associatedEntity", &own);
    orm::set_obj(&own, "associatedEntity", &inv);

    persist_flush_clear(&em, &[&inv, &own]).await;
    em
}

#[tokio::test]
async fn testThatTheValueOfIdentifiersAreConvertedInTheDatabase() {
    let em = set_up().await;

    assert_fetch_one(
        &em,
        s("nop"),
        "SELECT id1 FROM vct_inversed_onetoone_compositeid LIMIT 1",
    )
    .await;
    assert_fetch_one(
        &em,
        s("qrs"),
        "SELECT id2 FROM vct_inversed_onetoone_compositeid LIMIT 1",
    )
    .await;

    assert_fetch_one(
        &em,
        s("tuv"),
        "SELECT id3 FROM vct_owning_onetoone_compositeid LIMIT 1",
    )
    .await;
    assert_fetch_one(
        &em,
        s("nop"),
        "SELECT associated_id1 FROM vct_owning_onetoone_compositeid LIMIT 1",
    )
    .await;
    assert_fetch_one(
        &em,
        s("qrs"),
        "SELECT associated_id2 FROM vct_owning_onetoone_compositeid LIMIT 1",
    )
    .await;
}

/// `['id1' => 'abc', 'id2' => 'def']`.
fn inversed_id() -> Value {
    ids(&[("id1", s("abc")), ("id2", s("def"))])
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
async fn testThatTheProxyFromOwningToInversedIsLoaded() {
    let em = set_up().await;
    let own = find(&em, &owning(), s("ghi")).await;

    let inversed_proxy = orm::get_object(&own, "associatedEntity").unwrap();

    em.initialize_object(&inversed_proxy).await.unwrap();
    assert_field("some value to be loaded", &inversed_proxy, "someProperty");
}

#[tokio::test]
async fn testThatTheEntityFromInversedToOwningIsEagerLoaded() {
    let em = set_up().await;
    let inv = find(&em, &inversed(), inversed_id()).await;

    assert_instance_of(
        &owning(),
        &orm::get_object(&inv, "associatedEntity").unwrap(),
    );
}
