//! Ticket/DDC1301Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the legacy models).
//! `assertQueryCount()` counts the statements in the query log. Each test
//! has its own entity manager, so `tearDown()` has no mappings to reset.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1301Test.php

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
#[path = "common/query_log.rs"]
mod query_log;

use std::sync::Arc;

use php_value::{Object, Value};
use php2rust_doctrine_orm::collection_object as coll;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::mapping::class_metadata::FETCH_EXTRA_LAZY;
use php2rust_doctrine_orm::persistent_collection::PersistentCollection;

use legacy::*;
use orm::*;
use query_log::QueryLog;

struct T {
    em: Arc<EntityManager>,
    log: Arc<QueryLog>,
    user_id: Value,
}

async fn set_up() -> T {
    let (em, log) = query_log::set_up_logged(&LEGACY_MODELS).await;
    let class = em.get_class_metadata(LEGACY_USER).unwrap();
    {
        let mut class = class.write();
        for field in ["articles", "references", "cars"] {
            class.association_mappings.get_mut(field).unwrap().fetch = Some(FETCH_EXTRA_LAZY);
        }
    }
    let user_id = load_fixture(&em).await;
    T { em, log, user_id }
}

async fn load_fixture(em: &EntityManager) -> Value {
    let user1 = legacy_user("beberlei", "Benjamin");
    let user2 = legacy_user("jwage", "Jonathan");
    let user3 = legacy_user("romanb", "Roman");

    em.persist(&user1).unwrap();
    em.persist(&user2).unwrap();
    em.persist(&user3).unwrap();

    let article1 = legacy_article("Test", "Test");
    // $article1->setAuthor($user1)
    set_obj(&article1, "user", &user1);

    let article2 = legacy_article("Test", "Test");
    set_obj(&article2, "user", &user1);

    em.persist(&article1).unwrap();
    em.persist(&article2).unwrap();

    let car1 = legacy_car("Test1");
    let car2 = legacy_car("Test2");
    let car3 = legacy_car("Test3");

    add_car(&user1, &car1);
    add_car(&user1, &car2);
    add_car(&user1, &car3);

    add_car(&user2, &car1);
    add_car(&user3, &car1);

    em.persist(&car1).unwrap();
    em.persist(&car2).unwrap();
    em.persist(&car3).unwrap();

    em.flush().await.unwrap();

    let detail1 = legacy_user_reference(&user1, &user2, "foo");
    let detail2 = legacy_user_reference(&user1, &user3, "bar");

    em.persist(&detail1).unwrap();
    em.persist(&detail2).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();

    get(&user1, "id")
}

fn is_initialized(object: &Object, name: &str) -> bool {
    PersistentCollection::from_value(&get(object, name))
        .unwrap()
        .is_initialized()
}

impl T {
    /// Counts `$user->$name` without initializing it, then iterates it.
    async fn assert_count_then_iterate(&self, name: &str, expected: usize) {
        let user = self
            .em
            .find(LEGACY_USER, self.user_id.clone())
            .await
            .unwrap()
            .unwrap();

        self.log.reset_and_enable();

        assert!(!is_initialized(&user, name));
        assert_eq!(expected, count_of(&user, name).await);
        assert!(!is_initialized(&user, name));

        // foreach ($user->$name as $element) {}
        coll::to_array(&get(&user, name)).await.unwrap();

        let queries = self.log.queries();
        assert_eq!(
            2,
            queries.len(),
            "Expecting two queries to be fired for count, then iteration. {queries:?}"
        );
    }
}

#[tokio::test]
async fn testCountNotInitializesLegacyCollection() {
    set_up()
        .await
        .assert_count_then_iterate("articles", 2)
        .await;
}

#[tokio::test]
async fn testCountNotInitializesLegacyCollectionWithForeignIdentifier() {
    set_up()
        .await
        .assert_count_then_iterate("references", 2)
        .await;
}

#[tokio::test]
async fn testCountNotInitializesLegacyManyToManyCollection() {
    set_up().await.assert_count_then_iterate("cars", 3).await;
}
