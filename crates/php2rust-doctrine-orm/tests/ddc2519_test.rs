//! Ticket/DDC2519Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the legacy models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC2519Test.php

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/legacy.rs"]
mod legacy;
#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;

use php_value::Value;
use php2rust_doctrine_orm::abstract_query::HYDRATE_OBJECT;

use legacy::*;
use orm::*;

#[tokio::test]
async fn testIssue() {
    let em = orm::set_up(&LEGACY_MODELS).await;
    load_fixture(&em).await;

    let dql = "SELECT PARTIAL l.{_source, _target} FROM Doctrine\\Tests\\Models\\Legacy\\LegacyUserReference l";
    let result = list(
        &em.create_query(dql)
            .get_result(HYDRATE_OBJECT)
            .await
            .unwrap(),
    );

    assert_eq!(2, result.len());
    let result: Vec<_> = result.iter().map(obj).collect();
    for reference in &result {
        assert_eq!(LEGACY_USER_REFERENCE, reference.class());
    }

    for reference in &result {
        // $reference->source(), $reference->target()
        for user in [get(reference, "_source"), get(reference, "_target")] {
            assert_eq!(LEGACY_USER, obj(&user).class());
            assert!(em.is_uninitialized_object(&user));
            assert_ne!(Value::Null, get(&obj(&user), "id"));
        }
    }
}

async fn load_fixture(em: &php2rust_doctrine_orm::entity_manager::EntityManager) {
    let user1 = legacy_user("FabioBatSilva", "Fabio B. Silva");
    let user2 = legacy_user("doctrinebot", "Doctrine Bot");
    let user3 = legacy_user("test", "Tester");

    em.persist(&user1).unwrap();
    em.persist(&user2).unwrap();
    em.persist(&user3).unwrap();

    em.flush().await.unwrap();

    em.persist(&legacy_user_reference(&user1, &user2, "foo"))
        .unwrap();
    em.persist(&legacy_user_reference(&user1, &user3, "bar"))
        .unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();
}
