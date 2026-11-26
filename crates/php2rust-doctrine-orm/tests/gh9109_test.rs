//! Ticket/GH9109Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH9109Test.php

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
use php2rust_doctrine_collections::Criteria;
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};

use orm::*;
use ticket::*;

#[tokio::test]
async fn testIssue() {
    let em = ticket::set_up(&["GH9109User", "GH9109Product"]).await;
    let user_first_name = "GH9109Test";
    let user_last_name = "UserGH9109";
    let product_title = "Test product";

    let user_repository = em.get_repository(&class("GH9109User")).unwrap();

    let user = new("GH9109User");
    set(&user, "firstName", s(user_first_name));
    set(&user, "lastName", s(user_last_name));

    let product = new("GH9109Product");
    set(
        &product,
        "buyers",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    set(&product, "title", s(product_title));

    em.persist(&user).unwrap();
    em.persist(&product).unwrap();
    em.flush().await.unwrap();

    append(&product, "buyers", Value::Object(user.clone()));

    em.persist(&product).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let persisted_product = find(&em, "GH9109Product", get(&product, "id"))
        .await
        .unwrap();

    // assert Product was persisted
    assert_instance_of("GH9109Product", &Value::Object(persisted_product.clone()));
    assert_loose_eq(&s(product_title), &get(&persisted_product, "title"));

    // assert Product has a Buyer
    let count = coll::count(&get(&persisted_product, "buyers"))
        .await
        .unwrap();
    assert_eq!(1, count);

    let find_one_by = async |field: &str, value: &str| {
        let mut criteria = PhpArray::new();
        criteria.insert(field, s(value));
        user_repository
            .find_one_by(&criteria, None)
            .await
            .unwrap()
            .expect("a user")
    };
    let matching = async |field: &str, value: &str| {
        // `Criteria::ASC` is not defined in collections 3: `Criteria::create()`.
        let mut criteria = Criteria::create();
        criteria.r#where(Criteria::expr().eq(field, s(value)));
        let matched = coll::matching(&get(&persisted_product, "buyers"), &criteria)
            .await
            .unwrap();
        coll::first(&matched).await.unwrap()
    };

    // assert NOT QUOTED will WORK with findOneBy
    let user = find_one_by("lastName", user_last_name).await;
    assert_instance_of("GH9109User", &Value::Object(user.clone()));
    assert_loose_eq(&s(user_last_name), &get(&user, "lastName"));

    // assert NOT QUOTED will WORK with Criteria
    let user = matching("lastName", user_last_name).await;
    assert_instance_of("GH9109User", &user);
    assert_loose_eq(&s(user_last_name), &get(&obj(&user), "lastName"));

    // assert QUOTED will WORK with findOneBy
    let user = find_one_by("firstName", user_first_name).await;
    assert_instance_of("GH9109User", &Value::Object(user.clone()));
    assert_loose_eq(&s(user_first_name), &get(&user, "firstName"));

    // assert QUOTED will WORK with Criteria
    let user = matching("firstName", user_first_name).await;
    assert_instance_of("GH9109User", &user);
    assert_loose_eq(&s(user_first_name), &get(&obj(&user), "firstName"));
}
