//! Ticket/DDC881Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC881Test.php

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

use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_orm::abstract_query::HYDRATE_OBJECT;
use php2rust_doctrine_orm::collection_object::ArrayCollectionObject;
use php2rust_doctrine_orm::persistent_collection::PersistentCollection;

use orm::*;
use ticket::*;

/// `new DDC881PhoneNumber()`, `setUser($user)`, `setId($id)` and
/// `setPhoneNumber($number)`.
fn phone_number(user: &Object, id: i64, number: &str) -> Object {
    let phone = new("DDC881PhoneNumber");
    set(
        &phone,
        "calls",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    set_obj(&phone, "user", user);
    set(&phone, "id", Value::Int(id));
    set(&phone, "phonenumber", s(number));
    phone
}

#[tokio::test]
async fn testIssue() {
    let em = ticket::set_up(&["DDC881User", "DDC881PhoneNumber", "DDC881PhoneCall"]).await;

    /* Create two test users: albert and alfons */
    let albert = new("DDC881User");
    set(&albert, "name", s("albert"));
    em.persist(&albert).unwrap();

    let alfons = new("DDC881User");
    set(&alfons, "name", s("alfons"));
    em.persist(&alfons).unwrap();

    em.flush().await.unwrap();

    /* Assign two phone numbers to each user */
    let phone_albert1 = phone_number(&albert, 1, "albert home: 012345");
    em.persist(&phone_albert1).unwrap();

    let phone_albert2 = phone_number(&albert, 2, "albert mobile: 67890");
    em.persist(&phone_albert2).unwrap();

    let phone_alfons1 = phone_number(&alfons, 1, "alfons home: 012345");
    em.persist(&phone_alfons1).unwrap();

    let phone_alfons2 = phone_number(&alfons, 2, "alfons mobile: 67890");
    em.persist(&phone_alfons2).unwrap();

    /* We call alfons and albert once on their mobile numbers */
    let call1 = new("DDC881PhoneCall");
    set_obj(&call1, "phonenumber", &phone_alfons2);
    em.persist(&call1).unwrap();

    let call2 = new("DDC881PhoneCall");
    set_obj(&call2, "phonenumber", &phone_albert2);
    em.persist(&call2).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();

    // fetch-join that foreign-key/primary-key entity association
    let dql = format!(
        "SELECT c, p FROM {} c JOIN c.phonenumber p",
        class("DDC881PhoneCall")
    );
    let phone_calls = list(
        &em.create_query(&dql)
            .get_result(HYDRATE_OBJECT)
            .await
            .unwrap(),
    );

    assert_eq!(2, phone_calls.len());
    assert!(!is_uninitialized(
        &em,
        &get(&obj(&phone_calls[0]), "phonenumber")
    ));
    assert!(!is_uninitialized(
        &em,
        &get(&obj(&phone_calls[1]), "phonenumber")
    ));

    let dql = format!(
        "SELECT p, c FROM {} p JOIN p.calls c",
        class("DDC881PhoneNumber")
    );
    let numbers = list(
        &em.create_query(&dql)
            .get_result(HYDRATE_OBJECT)
            .await
            .unwrap(),
    );

    assert_eq!(2, numbers.len());
    let number_calls = PersistentCollection::from_value(&get(&obj(&numbers[0]), "calls"))
        .expect("a PersistentCollection");
    assert!(number_calls.is_initialized());
}
