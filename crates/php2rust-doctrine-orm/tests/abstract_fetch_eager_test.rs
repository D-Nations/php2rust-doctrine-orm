//! Port of `AbstractFetchEagerTest`, on `SQLite`. The models' constructors
//! are written here, on their recorded classes.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/AbstractFetchEagerTest.php

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

use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_orm::collection_object::ArrayCollectionObject;

use orm::{assert_loose_eq, get, get_object, new_object, s, set};

const NS: &str = "Doctrine\\Tests\\Models\\AbstractFetchEager\\";

fn class(short: &str) -> String {
    format!("{NS}{short}")
}

/// `new MobileRemoteControl($name)`.
fn mobile_remote_control(name: &str) -> Object {
    let control = new_object(&class("MobileRemoteControl"));
    set(&control, "name", s(name));
    set(
        &control,
        "users",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    control
}

/// `new User($control)`.
fn user(control: &Object) -> Object {
    let user = new_object(&class("User"));
    set(&user, "remoteControl", Value::Object(control.clone()));
    user
}

#[tokio::test]
async fn testWithAbstractFetchEager() {
    let em = orm::set_up(&[&class("AbstractRemoteControl"), &class("User")]).await;

    let control = mobile_remote_control("smart");
    let user = user(&control);

    em.persist(&control).unwrap();
    em.persist(&user).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let user = em
        .find(&class("User"), get(&user, "id"))
        .await
        .unwrap()
        .expect("the user");

    let remote_control = get_object(&user, "remoteControl").unwrap();
    em.initialize_object(&remote_control).await.unwrap();
    assert_loose_eq(&s("smart"), &get(&remote_control, "name"));
    assert!(orm::contains(&remote_control, "users", &user).await);
}
