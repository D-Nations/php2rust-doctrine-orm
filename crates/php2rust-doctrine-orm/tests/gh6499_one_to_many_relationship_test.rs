//! Ticket/GH6499OneToManyRelationshipTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models), with the foreign keys enforced as
//! upstream's `EnableForeignKeys` middleware does.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH6499OneToManyRelationshipTest.php

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

use php_value::{Object, PhpArray};
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};

use orm::*;
use ticket::*;

/// `new $short()` with its `applicationPeople` collection.
fn with_application_people(short: &str) -> Object {
    let entity = new(short);
    set(
        &entity,
        "applicationPeople",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    entity
}

/// Test for the bug described in issue #6499.
#[tokio::test]
async fn testIssue() {
    let em = set_up_with_foreign_keys(&["Application", "Person", "ApplicationPerson"]).await;

    let person = with_application_people("Person");
    em.persist(&person).unwrap();

    let application = with_application_people("Application");
    em.persist(&application).unwrap();

    // new ApplicationPerson($person, $application)
    let application_person = new("ApplicationPerson");
    set_obj(&application_person, "person", &person);
    set_obj(&application_person, "application", &application);

    em.persist(&application_person).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let person_from_database = find(&em, "Person", get(&person, "id")).await.unwrap();
    let application_from_database = find(&em, "Application", get(&application, "id"))
        .await
        .unwrap();

    assert_loose_eq(&get(&person_from_database, "id"), &get(&person, "id"));
    assert!(
        !coll::is_empty(&get(&person_from_database, "applicationPeople"))
            .await
            .unwrap()
    );

    assert_loose_eq(
        &get(&application_from_database, "id"),
        &get(&application, "id"),
    );
    assert!(
        !coll::is_empty(&get(&application_from_database, "applicationPeople"))
            .await
            .unwrap()
    );
}
