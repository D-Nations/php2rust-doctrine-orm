//! Ticket/GH11017/GH11017Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH11017/GH11017Test.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH11017/GH11017Entity.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH11017/GH11017Enum.php

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

use php2rust_doctrine_orm::abstract_query::HYDRATE_ARRAY;
use php2rust_doctrine_orm::mapping::ordered_map::OrderedMap;
use php2rust_doctrine_orm::query::ResultSetMappingBuilder;

use orm::*;

const GH11017_ENTITY: &str = "Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH11017\\GH11017Entity";
const GH11017_ENUM: &str = "Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH11017\\GH11017Enum";

#[tokio::test]
async fn testPostPersistListenerUpdatingObjectFieldWhileOtherInsertPending() {
    let em = orm::set_up(&[GH11017_ENTITY]).await;
    let entity1 = new_object(GH11017_ENTITY);
    set(&entity1, "field", ticket::case(GH11017_ENUM, "FIRST"));

    em.persist(&entity1).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let mut rsm = ResultSetMappingBuilder::new(
        &em,
        Some(ResultSetMappingBuilder::COLUMN_RENAMING_INCREMENT),
    );
    rsm.add_root_entity_from_class_metadata(GH11017_ENTITY, "e", &OrderedMap::new(), None)
        .unwrap();
    let table_name = em
        .get_class_metadata(GH11017_ENTITY)
        .unwrap()
        .read()
        .get_table_name()
        .unwrap();
    let sql = format!(
        "SELECT {} FROM {} e WHERE id = :id",
        rsm.generate_select_clause(&php_value::PhpArray::new())
            .unwrap(),
        table_name
    );

    let query = em
        .create_native_query(&sql, rsm.into_result_set_mapping())
        .unwrap();
    query.set_parameter("id", get(&entity1, "id"), None);

    let entity1_reloaded = query.get_single_result(Some(HYDRATE_ARRAY)).await.unwrap();
    assert!(!entity1_reloaded.is_null());
    assert!(get(&entity1, "field").identical(&at(&entity1_reloaded, "field")));
}
