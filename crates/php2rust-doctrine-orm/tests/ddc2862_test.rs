//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC2862Test.php
//!
//! `enableSecondLevelCache()`: the second-level cache fixture of
//! `tests/common/slc_functional.rs` with the recorded models of the test
//! file. Reading a property of a lazy proxy does not load it: the tests
//! call `initialize_object()` where PHP's getter on the proxy would.

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
#[path = "common/query_log.rs"]
mod query_log;
#[path = "common/slc.rs"]
mod slc;
#[path = "common/slc_functional.rs"]
mod slc_functional;
#[path = "common/ticket.rs"]
mod ticket;

use php_value::{Object, PhpArray, Value};

use orm::{assert_loose_eq, get, obj, s, set, set_obj};
use slc_functional::Fixture;
use ticket::{class, new};

async fn set_up() -> Fixture {
    slc_functional::set_up_with(&[&class("DDC2862User"), &class("DDC2862Driver")]).await
}

/// `new DDC2862User($name)`.
fn user(name: &str) -> Object {
    let user = new("DDC2862User");
    set(&user, "name", s(name));
    user
}

/// `new DDC2862Driver($name, $profile)`.
fn driver(name: &str, profile: &Object) -> Object {
    let driver = new("DDC2862Driver");
    set(&driver, "name", s(name));
    set_obj(&driver, "userProfile", profile);
    driver
}

/// `$this->_em->getCache()->containsEntity($class, ['id' => $entity->getId()])`.
fn contains(f: &Fixture, short: &str, entity: &Object) -> bool {
    let mut id = PhpArray::new();
    id.insert("id", get(entity, "id"));
    f.em.get_cache()
        .unwrap()
        .contains_entity(&class(short), &Value::Array(id))
        .unwrap()
}

async fn find_driver(f: &Fixture, driver: &Object) -> Object {
    f.em.find(&class("DDC2862Driver"), get(driver, "id"))
        .await
        .unwrap()
        .unwrap()
}

/// `$driver->getUserProfile()`, which is a `DDC2862User`.
fn user_profile(driver: &Object) -> Object {
    let profile = obj(&get(driver, "userProfile"));
    assert!(profile.instance_of(&class("DDC2862User")));
    profile
}

#[tokio::test]
async fn testIssue() {
    let f = set_up().await;
    let user1 = user("Foo");
    let driver1 = driver("Bar", &user1);

    f.em.persist(&user1).unwrap();
    f.em.persist(&driver1).unwrap();
    f.em.flush().await.unwrap();
    f.em.clear().unwrap();

    assert!(contains(&f, "DDC2862User", &user1));
    assert!(contains(&f, "DDC2862Driver", &driver1));

    f.reset_query_log();
    let driver2 = find_driver(&f, &driver1).await;

    f.assert_query_count(0);
    assert!(driver2.instance_of(&class("DDC2862Driver")));
    user_profile(&driver2);

    set(&driver2, "name", s("Franta"));

    f.em.flush().await.unwrap();
    f.em.clear().unwrap();

    assert!(contains(&f, "DDC2862User", &user1));
    assert!(contains(&f, "DDC2862Driver", &driver1));

    f.reset_query_log();
    let driver3 = find_driver(&f, &driver1).await;

    f.assert_query_count(0);
    assert!(driver3.instance_of(&class("DDC2862Driver")));
    let profile = user_profile(&driver3);
    assert_loose_eq(&s("Franta"), &get(&driver3, "name"));
    f.em.initialize_object(&profile).await.unwrap();
    assert_loose_eq(&s("Foo"), &get(&profile, "name"));
}

#[tokio::test]
async fn testIssueReopened() {
    let f = set_up().await;
    let user1 = user("Foo");
    let driver1 = driver("Bar", &user1);

    f.em.persist(&user1).unwrap();
    f.em.persist(&driver1).unwrap();
    f.em.flush().await.unwrap();
    f.em.clear().unwrap();

    let cache = f.em.get_cache().unwrap();
    cache.evict_entity_region(&class("DDC2862User")).unwrap();
    cache.evict_entity_region(&class("DDC2862Driver")).unwrap();

    assert!(!contains(&f, "DDC2862User", &user1));
    assert!(!contains(&f, "DDC2862Driver", &driver1));

    f.reset_query_log();
    let driver2 = find_driver(&f, &driver1).await;

    assert!(driver2.instance_of(&class("DDC2862Driver")));
    user_profile(&driver2);
    f.assert_query_count(1);

    f.em.clear().unwrap();

    assert!(!contains(&f, "DDC2862User", &user1));
    assert!(contains(&f, "DDC2862Driver", &driver1));

    f.reset_query_log();
    let driver3 = find_driver(&f, &driver1).await;

    assert!(driver3.instance_of(&class("DDC2862Driver")));
    let profile = user_profile(&driver3);
    f.assert_query_count(0);
    // $driver3->getUserProfile()->getName()
    f.em.initialize_object(&profile).await.unwrap();
    assert_loose_eq(&s("Foo"), &get(&profile, "name"));
    f.assert_query_count(1);

    f.reset_query_log();
    let driver4 = find_driver(&f, &driver1).await;

    assert!(driver4.instance_of(&class("DDC2862Driver")));
    let profile = user_profile(&driver4);
    f.assert_query_count(0);
    f.em.initialize_object(&profile).await.unwrap();
    assert_loose_eq(&s("Foo"), &get(&profile, "name"));
    f.assert_query_count(0);
}
