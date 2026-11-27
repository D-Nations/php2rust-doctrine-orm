//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/EagerSecondLevelCacheCollectionTest.php
//!
//! `enableSecondLevelCache()`: the second-level cache fixture of
//! `tests/common/slc_functional.rs` with the recorded models of the test
//! file.

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
use php2rust_doctrine_orm::collection_object::ArrayCollectionObject;

use orm::{append, count_of, get, s, set, set_obj};
use ticket::{class, new};

/// `new EagerSlcUser($name)`.
fn user(name: &str) -> Object {
    let user = new("EagerSlcUser");
    set(&user, "name", s(name));
    set(
        &user,
        "roles",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    user
}

/// `new EagerSlcRole($user, $name)`.
fn role(user: &Object, name: &str) -> Object {
    let role = new("EagerSlcRole");
    set_obj(&role, "user", user);
    set(&role, "name", s(name));
    role
}

#[tokio::test]
async fn testEagerCollectionIsNotDuplicatedAfterSecondLevelCacheHit() {
    let f = slc_functional::set_up_with(&[
        &class("EagerSlcUser"),
        &class("EagerSlcRole"),
        &class("EagerSlcOther"),
    ])
    .await;

    let user = user("alice");
    append(&user, "roles", Value::Object(role(&user, "ROLE_USER")));
    append(&user, "roles", Value::Object(role(&user, "ROLE_ADMIN")));
    let other = new("EagerSlcOther");
    set(&other, "name", s("marker"));

    f.em.persist(&user).unwrap();
    f.em.persist(&other).unwrap();
    f.em.flush().await.unwrap();

    let user_id = get(&user, "id");
    let other_id = get(&other, "id");

    f.em.clear().unwrap();

    // Warm entity + collection cache (first load misses, then puts).
    let warmed =
        f.em.find(&class("EagerSlcUser"), user_id.clone())
            .await
            .unwrap()
            .unwrap();
    assert_eq!(2, count_of(&warmed, "roles").await);
    f.em.clear().unwrap();

    // Load owner from the entity second-level cache. EAGER schedules deferred collection
    // loading, but DefaultEntityHydrator does not call triggerEagerLoads().
    let cached_user =
        f.em.find(&class("EagerSlcUser"), user_id)
            .await
            .unwrap()
            .unwrap();

    // Initialize the collection (collection-cache hit or DB) before deferred eager runs.
    assert_eq!(2, count_of(&cached_user, "roles").await);

    // A subsequent hydration flushes pending deferred eager collection loads.
    f.em.find(&class("EagerSlcOther"), other_id).await.unwrap();

    assert_eq!(
        2,
        count_of(&cached_user, "roles").await,
        "EAGER OneToMany must not be hydrated twice after a second-level cache hit"
    );
}
