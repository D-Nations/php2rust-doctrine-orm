//! Ticket/Issue5989Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the `issue5989` model set).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/Issue5989Test.php

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

use php_value::{PhpArray, Value};

use orm::*;

const ISSUE5989_PERSON: &str = "Doctrine\\Tests\\Models\\Issue5989\\Issue5989Person";
const ISSUE5989_EMPLOYEE: &str = "Doctrine\\Tests\\Models\\Issue5989\\Issue5989Employee";
const ISSUE5989_MANAGER: &str = "Doctrine\\Tests\\Models\\Issue5989\\Issue5989Manager";

fn tags(tags: &[&str]) -> Value {
    Value::Array(PhpArray::list(tags.iter().map(|tag| s(tag))))
}

#[tokio::test]
async fn testSimpleArrayTypeHydratedCorrectlyInJoinedInheritance() {
    let em = orm::set_up(&[ISSUE5989_PERSON, ISSUE5989_EMPLOYEE, ISSUE5989_MANAGER]).await;
    let manager = new_object(ISSUE5989_MANAGER);

    let manager_tags = tags(&["tag1", "tag2"]);
    set(&manager, "tags", manager_tags.clone());
    em.persist(&manager).unwrap();

    let employee = new_object(ISSUE5989_EMPLOYEE);

    let employee_tags = tags(&["tag2", "tag3"]);
    set(&employee, "tags", employee_tags.clone());
    em.persist(&employee).unwrap();

    em.flush().await.unwrap();

    let manager_id = get(&manager, "id");
    let employee_id = get(&employee, "id");

    // clear entity manager so that $repository->find actually fetches them and uses the hydrator
    // instead of just returning the existing managed entities
    em.clear().unwrap();

    let repository = em.get_repository(ISSUE5989_PERSON).unwrap();

    let manager = repository.find(manager_id).await.unwrap().unwrap();
    let employee = repository.find(employee_id).await.unwrap().unwrap();

    assert_loose_eq(&manager_tags, &get(&manager, "tags"));
    assert_loose_eq(&employee_tags, &get(&employee, "tags"));
}
