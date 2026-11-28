//! Ticket/GH10348Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH10348Test.php

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

use php_value::{PhpArray, Value};
use php2rust_doctrine_orm::abstract_query::HYDRATE_OBJECT;
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};

use orm::*;
use ticket::*;

#[tokio::test]
async fn testTheORMRemovesReferencedEmployeeBeforeReferencingEmployee() {
    let em = ticket::set_up(&["GH10348Person", "GH10348Company"]).await;

    let person1 = new("GH10348Person");
    let person2 = new("GH10348Person");
    set_obj(&person2, "mentor", &person1);

    let company = new("GH10348Company");
    set(
        &company,
        "employees",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    for person in [&person1, &person2] {
        // GH10348Company::addEmployee()
        set_obj(person, "employer", &company);
        coll::add(&get(&company, "employees"), Value::Object(person.clone())).unwrap();
    }

    em.persist(&company).unwrap();
    em.flush().await.unwrap();

    let company = find(&em, "GH10348Company", get(&company, "id"))
        .await
        .unwrap();
    em.remove(&company).await.unwrap();
    em.flush().await.unwrap();

    for short in ["GH10348Company", "GH10348Person"] {
        let alias = if short == "GH10348Company" { "c" } else { "p" };
        let result = em
            .create_query(&format!("SELECT {alias} FROM {} {alias}", class(short)))
            .get_result(HYDRATE_OBJECT)
            .await
            .unwrap();
        assert_eq!(list(&result).len(), 0);
    }
}
