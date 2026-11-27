//! Port of `DefaultValuesTest`, on `SQLite`.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/DefaultValuesTest.php

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

use php_value::Value;

use orm::{assert_loose_eq, get, get_object, new_object, s, set};

const USER: &str = "Doctrine\\Tests\\ORM\\Functional\\DefaultValueUser";
const ADDRESS: &str = "Doctrine\\Tests\\ORM\\Functional\\DefaultValueAddress";

#[tokio::test]
async fn testSimpleDetachMerge() {
    let em = orm::set_up(&[USER, ADDRESS]).await;

    let user = new_object(USER);
    set(&user, "name", s("romanb"));
    em.persist(&user).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let user_id = get(&user, "id"); // e.g. from $_REQUEST
    let user2 = em
        .get_reference(USER, user_id.clone())
        .await
        .unwrap()
        .unwrap();

    em.flush().await.unwrap();
    assert!(em.is_uninitialized_object(&Value::Object(user2.clone())));

    let a = new_object(ADDRESS);
    set(&a, "country", s("de"));
    set(&a, "zip", s("12345"));
    set(&a, "city", s("Berlin"));
    set(&a, "street", s("Sesamestreet"));

    set(&a, "user", Value::Object(user2.clone()));
    em.persist(&a).unwrap();
    em.flush().await.unwrap();

    assert!(em.is_uninitialized_object(&Value::Object(user2.clone())));
    em.clear().unwrap();

    let a2 = em.find(ADDRESS, get(&a, "id")).await.unwrap().unwrap();
    let a2_user = get_object(&a2, "user").expect("a DefaultValueUser");
    assert_eq!(a2_user.class(), USER);
    em.initialize_object(&a2_user).await.unwrap();
    assert_loose_eq(&user_id, &get(&a2_user, "id"));
    assert_loose_eq(&s("Poweruser"), &get(&a2_user, "type"));
}
