//! OrderedJoinedTableInheritanceCollectionTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa.
//! `$poofy->children[$i]` is the collection's `get($i)`.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/OrderedJoinedTableInheritanceCollectionTest.php

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

use php_value::{Object, Value};
use php2rust_doctrine_orm::abstract_query::HYDRATE_OBJECT;
use php2rust_doctrine_orm::collection_object as coll;
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::*;

const OJTIC_PET: &str = "Doctrine\\Tests\\ORM\\Functional\\OJTICPet";
const OJTIC_CAT: &str = "Doctrine\\Tests\\ORM\\Functional\\OJTICCat";
const OJTIC_DOG: &str = "Doctrine\\Tests\\ORM\\Functional\\OJTICDog";

fn dog(name: &str) -> Object {
    let dog = new_object(OJTIC_DOG);
    set(&dog, "name", s(name));
    dog
}

async fn set_up() -> Arc<EntityManager> {
    let em = orm::set_up(&[OJTIC_PET, OJTIC_CAT, OJTIC_DOG]).await;

    let dog0 = dog("Poofy");

    let dog1 = dog("Zampa");
    let dog2 = dog("Aari");

    set_obj(&dog1, "mother", &dog0);
    set_obj(&dog2, "mother", &dog0);

    append(&dog0, "children", Value::Object(dog1.clone()));
    append(&dog0, "children", Value::Object(dog2.clone()));

    em.persist(&dog0).unwrap();
    em.persist(&dog1).unwrap();
    em.persist(&dog2).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();
    em
}

async fn child_name(pet: &Object, index: i64) -> Value {
    let child = coll::get(&get(pet, "children"), index).await.unwrap();
    get(&obj(&child), "name")
}

#[tokio::test]
async fn testOrderdOneToManyCollection() {
    let em = set_up().await;
    let poofy = obj(&em
        .create_query(
            "SELECT p FROM Doctrine\\Tests\\ORM\\Functional\\OJTICPet p WHERE p.name = 'Poofy'",
        )
        .get_single_result(None)
        .await
        .unwrap());

    assert_loose_eq(&s("Aari"), &child_name(&poofy, 0).await);
    assert_loose_eq(&s("Zampa"), &child_name(&poofy, 1).await);

    em.clear().unwrap();

    let result = em
        .create_query(
            "SELECT p, c FROM Doctrine\\Tests\\ORM\\Functional\\OJTICPet p JOIN p.children c WHERE p.name = 'Poofy'",
        )
        .get_result(HYDRATE_OBJECT)
        .await
        .unwrap();

    let result = list(&result);
    assert_eq!(result.len(), 1);
    let poofy = obj(&result[0]);

    assert_loose_eq(&s("Aari"), &child_name(&poofy, 0).await);
    assert_loose_eq(&s("Zampa"), &child_name(&poofy, 1).await);
}
