//! Ticket/GH9467/GH9467Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models). `#[Depends]` runs the insert test first and
//! hands over its id, with the entity manager cleared. The insert and
//! update tests of the child columns share their bodies.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH9467/GH9467Test.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH9467/JoinedInheritanceChild.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH9467/JoinedInheritanceNonInsertableColumn.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH9467/JoinedInheritanceNonUpdatableColumn.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH9467/JoinedInheritanceNonWritableColumn.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH9467/JoinedInheritanceRoot.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH9467/JoinedInheritanceWritableColumn.php

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

use php_value::Value;
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::*;

const NS: &str = "Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH9467\\";

fn class(short: &str) -> String {
    format!("{NS}{short}")
}

async fn set_up() -> Arc<EntityManager> {
    orm::set_up(&[
        &class("JoinedInheritanceRoot"),
        &class("JoinedInheritanceChild"),
        &class("JoinedInheritanceWritableColumn"),
        &class("JoinedInheritanceNonWritableColumn"),
        &class("JoinedInheritanceNonInsertableColumn"),
        &class("JoinedInheritanceNonUpdatableColumn"),
    ])
    .await
}

/// `$this->_em->find($class, $id)`, asserted to be an instance of it.
async fn find_of(em: &EntityManager, short: &str, id: Value) -> php_value::Object {
    let entity = em.find(&class(short), id).await.unwrap();
    let entity = entity.unwrap_or_else(|| panic!("a {short}"));
    assert!(entity.instance_of(&class(short)));
    entity
}

fn assert_field(expected: &str, entity: &php_value::Object, field: &str) {
    assert_loose_eq(&s(expected), &get(entity, field));
}

async fn root_columns_insert(em: &EntityManager) -> Value {
    let entity = new_object(&class("JoinedInheritanceChild"));
    set(&entity, "rootWritableContent", s("foo"));
    set(&entity, "rootNonWritableContent", s("foo"));
    set(&entity, "rootNonInsertableContent", s("foo"));
    set(&entity, "rootNonUpdatableContent", s("foo"));

    em.persist(&entity).unwrap();
    em.flush().await.unwrap();

    // check INSERT query cause set database values into non-insertable entity properties
    assert_field("foo", &entity, "rootWritableContent");
    assert_field("dbDefault", &entity, "rootNonWritableContent");
    assert_field("dbDefault", &entity, "rootNonInsertableContent");
    assert_field("foo", &entity, "rootNonUpdatableContent");

    // check other process get same state
    em.clear().unwrap();
    let entity = find_of(em, "JoinedInheritanceChild", get(&entity, "id")).await;
    assert_field("foo", &entity, "rootWritableContent");
    assert_field("dbDefault", &entity, "rootNonWritableContent");
    assert_field("dbDefault", &entity, "rootNonInsertableContent");
    assert_field("foo", &entity, "rootNonUpdatableContent");

    get(&entity, "id")
}

#[tokio::test]
async fn testRootColumnsInsert() {
    let em = set_up().await;
    root_columns_insert(&em).await;
}

#[tokio::test]
async fn testRootColumnsUpdate() {
    let em = set_up().await;
    let entity_id = root_columns_insert(&em).await;
    em.clear().unwrap();

    let entity = find_of(&em, "JoinedInheritanceChild", entity_id).await;

    // update exist entity
    set(&entity, "rootWritableContent", s("bar"));
    set(&entity, "rootNonInsertableContent", s("bar"));
    set(&entity, "rootNonWritableContent", s("bar"));
    set(&entity, "rootNonUpdatableContent", s("bar"));

    em.persist(&entity).unwrap();
    em.flush().await.unwrap();

    // check UPDATE query cause set database values into non-insertable entity properties
    assert_field("bar", &entity, "rootWritableContent");
    assert_field("dbDefault", &entity, "rootNonWritableContent");
    assert_field("bar", &entity, "rootNonInsertableContent");
    assert_field("foo", &entity, "rootNonUpdatableContent");

    // check other process get same state
    em.clear().unwrap();
    let entity = find_of(&em, "JoinedInheritanceChild", get(&entity, "id")).await;
    assert_field("bar", &entity, "rootWritableContent");
    assert_field("dbDefault", &entity, "rootNonWritableContent");
    assert_field("bar", &entity, "rootNonInsertableContent");
    assert_field("foo", &entity, "rootNonUpdatableContent");
}

/// The `testChild*ColumnInsert()` tests: `$expected` is the content after
/// the INSERT of `'foo'`.
async fn child_column_insert(
    em: &EntityManager,
    short: &str,
    field: &str,
    expected: &str,
) -> Value {
    let entity = new_object(&class(short));
    set(&entity, field, s("foo"));

    em.persist(&entity).unwrap();
    em.flush().await.unwrap();

    assert_field(expected, &entity, field);

    // check other process get same state
    em.clear().unwrap();
    let entity = find_of(em, short, get(&entity, "id")).await;
    assert_field(expected, &entity, field);

    get(&entity, "id")
}

/// The `testChild*ColumnUpdate()` tests: `$expected` is the content after
/// the UPDATE to `'bar'`; `$touch_root` also changes `rootField` to ensure
/// an UPDATE query is done.
async fn child_column_update(
    em: &EntityManager,
    entity_id: Value,
    short: &str,
    field: &str,
    expected: &str,
    touch_root: bool,
) {
    let entity = find_of(em, short, entity_id).await;

    // update exist entity
    set(&entity, field, s("bar"));
    if touch_root {
        // change some property to ensure UPDATE query will be done
        assert!(!php_value::compare::loose_equals(
            &s("bar"),
            &get(&entity, "rootField")
        ));
        set(&entity, "rootField", s("bar"));
    }

    em.persist(&entity).unwrap();
    em.flush().await.unwrap();

    assert_field(expected, &entity, field);

    // check other process get same state
    em.clear().unwrap();
    let entity = find_of(em, short, get(&entity, "id")).await;
    if touch_root {
        assert_field("bar", &entity, "rootField"); // check that UPDATE query done
    }
    assert_field(expected, &entity, field);
}

#[tokio::test]
async fn testChildWritableColumnInsert() {
    let em = set_up().await;
    // check INSERT query doesn't change insertable entity property
    child_column_insert(
        &em,
        "JoinedInheritanceWritableColumn",
        "writableContent",
        "foo",
    )
    .await;
}

#[tokio::test]
async fn testChildWritableColumnUpdate() {
    let em = set_up().await;
    let entity_id = child_column_insert(
        &em,
        "JoinedInheritanceWritableColumn",
        "writableContent",
        "foo",
    )
    .await;
    em.clear().unwrap();
    // check UPDATE query doesn't change updatable entity property
    child_column_update(
        &em,
        entity_id,
        "JoinedInheritanceWritableColumn",
        "writableContent",
        "bar",
        false,
    )
    .await;
}

#[tokio::test]
async fn testChildNonWritableColumnInsert() {
    let em = set_up().await;
    // check INSERT query cause set database value into non-insertable entity property
    child_column_insert(
        &em,
        "JoinedInheritanceNonWritableColumn",
        "nonWritableContent",
        "dbDefault",
    )
    .await;
}

#[tokio::test]
async fn testChildNonWritableColumnUpdate() {
    let em = set_up().await;
    let entity_id = child_column_insert(
        &em,
        "JoinedInheritanceNonWritableColumn",
        "nonWritableContent",
        "dbDefault",
    )
    .await;
    em.clear().unwrap();
    // check UPDATE query cause set database value into non-updatable entity property
    child_column_update(
        &em,
        entity_id,
        "JoinedInheritanceNonWritableColumn",
        "nonWritableContent",
        "dbDefault",
        true,
    )
    .await;
}

#[tokio::test]
async fn testChildNonInsertableColumnInsert() {
    let em = set_up().await;
    // check INSERT query cause set database value into non-insertable entity property
    child_column_insert(
        &em,
        "JoinedInheritanceNonInsertableColumn",
        "nonInsertableContent",
        "dbDefault",
    )
    .await;
}

#[tokio::test]
async fn testChildNonInsertableColumnUpdate() {
    let em = set_up().await;
    let entity_id = child_column_insert(
        &em,
        "JoinedInheritanceNonInsertableColumn",
        "nonInsertableContent",
        "dbDefault",
    )
    .await;
    em.clear().unwrap();
    // check UPDATE query doesn't change updatable entity property
    child_column_update(
        &em,
        entity_id,
        "JoinedInheritanceNonInsertableColumn",
        "nonInsertableContent",
        "bar",
        false,
    )
    .await;
}

#[tokio::test]
async fn testChildNonUpdatableColumnInsert() {
    let em = set_up().await;
    // check INSERT query doesn't change insertable entity property
    child_column_insert(
        &em,
        "JoinedInheritanceNonUpdatableColumn",
        "nonUpdatableContent",
        "foo",
    )
    .await;
}

#[tokio::test]
async fn testChildNonUpdatableColumnUpdate() {
    let em = set_up().await;
    let entity_id = child_column_insert(
        &em,
        "JoinedInheritanceNonUpdatableColumn",
        "nonUpdatableContent",
        "foo",
    )
    .await;
    em.clear().unwrap();
    let entity = find_of(
        &em,
        "JoinedInheritanceNonUpdatableColumn",
        entity_id.clone(),
    )
    .await;
    assert_field("foo", &entity, "nonUpdatableContent");
    // check UPDATE query cause set database value into non-updatable entity property
    child_column_update(
        &em,
        entity_id,
        "JoinedInheritanceNonUpdatableColumn",
        "nonUpdatableContent",
        "foo",
        true,
    )
    .await;
}
