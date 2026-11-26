//! OneToManyCompositeIdForeignKeyTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa.
//! The entities all use a custom type that converts the value as
//! identifier(s) (`common/rot13.rs`). Each test has its own database, so
//! `tearDownAfterClass()` has nothing to drop.
//! A lazy proxy does not load on a property read here, so the proxy tests
//! call `initialize_object()` where PHP's property read would.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/ValueConversionType/OneToManyCompositeIdForeignKeyTest.php

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

fn auxiliary() -> String {
    class("AuxiliaryEntity")
}
fn inversed() -> String {
    class("InversedOneToManyCompositeIdForeignKeyEntity")
}
fn owning() -> String {
    class("OwningManyToOneCompositeIdForeignKeyEntity")
}

async fn set_up() -> Arc<EntityManager> {
    let em = vct::set_up(&[auxiliary(), inversed(), owning()]).await;

    let aux = entity(&auxiliary(), &[("id4", "abc")]);

    let inv = entity_with_collection(
        &inversed(),
        &[("id1", "def"), ("someProperty", "some value to be loaded")],
    );
    orm::set_obj(&inv, "foreignEntity", &aux);

    let own = entity(&owning(), &[("id2", "ghi")]);

    add(&inv, &own);
    orm::set_obj(&own, "associatedEntity", &inv);

    persist_flush_clear(&em, &[&aux, &inv, &own]).await;
    em
}

fn inversed_id() -> Value {
    ids(&[("id1", s("def")), ("foreignEntity", s("abc"))])
}

#[tokio::test]
async fn testThatTheValueOfIdentifiersAreConvertedInTheDatabase() {
    let em = set_up().await;

    assert_fetch_one(&em, s("nop"), "SELECT id4 FROM vct_auxiliary LIMIT 1").await;

    assert_fetch_one(
        &em,
        s("qrs"),
        "SELECT id1 FROM vct_inversed_onetomany_compositeid_foreignkey LIMIT 1",
    )
    .await;
    assert_fetch_one(
        &em,
        s("nop"),
        "SELECT foreign_id FROM vct_inversed_onetomany_compositeid_foreignkey LIMIT 1",
    )
    .await;

    assert_fetch_one(
        &em,
        s("tuv"),
        "SELECT id2 FROM vct_owning_manytoone_compositeid_foreignkey LIMIT 1",
    )
    .await;
    assert_fetch_one(
        &em,
        s("qrs"),
        "SELECT associated_id FROM vct_owning_manytoone_compositeid_foreignkey LIMIT 1",
    )
    .await;
    assert_fetch_one(
        &em,
        s("nop"),
        "SELECT associated_foreign_id FROM vct_owning_manytoone_compositeid_foreignkey LIMIT 1",
    )
    .await;
}

#[tokio::test]
async fn testThatEntitiesAreFetchedFromTheDatabase() {
    let em = set_up().await;

    let aux = find(&em, &auxiliary(), s("abc")).await;
    let inv = find(&em, &inversed(), inversed_id()).await;
    let own = find(&em, &owning(), s("ghi")).await;

    assert_instance_of(&auxiliary(), &aux);
    assert_instance_of(&inversed(), &inv);
    assert_instance_of(&owning(), &own);
}

#[tokio::test]
async fn testThatTheValueOfIdentifiersAreConvertedBackAfterBeingFetchedFromTheDatabase() {
    let em = set_up().await;

    let aux = find(&em, &auxiliary(), s("abc")).await;
    let inv = find(&em, &inversed(), inversed_id()).await;
    let own = find(&em, &owning(), s("ghi")).await;

    assert_field("abc", &aux, "id4");
    assert_field("def", &inv, "id1");
    assert_field(
        "abc",
        &orm::get_object(&inv, "foreignEntity").unwrap(),
        "id4",
    );
    assert_field("ghi", &own, "id2");
}

#[tokio::test]
async fn testThatInversedEntityIsFetchedFromTheDatabaseUsingAuxiliaryEntityAsId() {
    let em = set_up().await;

    let aux = find(&em, &auxiliary(), s("abc")).await;
    let inv = find(
        &em,
        &inversed(),
        ids(&[("id1", s("def")), ("foreignEntity", Value::Object(aux))]),
    )
    .await;

    assert_instance_of(&inversed(), &inv);
}

#[tokio::test]
async fn testThatTheProxyFromOwningToInversedIsLoaded() {
    let em = set_up().await;
    let own = find(&em, &owning(), s("ghi")).await;

    let inversed_proxy = orm::get_object(&own, "associatedEntity").unwrap();

    assert_eq!(
        orm::get(&inversed_proxy, "id1"),
        s("def"),
        "Proxy identifier is converted"
    );

    em.initialize_object(&inversed_proxy).await.unwrap();
    assert_field("some value to be loaded", &inversed_proxy, "someProperty");
}

#[tokio::test]
async fn testThatTheCollectionFromInversedToOwningIsLoaded() {
    let em = set_up().await;
    let inv = find(&em, &inversed(), inversed_id()).await;

    assert_collection_count(1, &inv).await;
}
