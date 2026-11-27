//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH6217Test.php
//!
//! `enableSecondLevelCache()`: the second-level cache fixture of
//! `tests/common/slc_functional.rs` with the recorded models of the test
//! file. `uniqid(self::class, true)` is a counter after the class name.

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

use std::sync::atomic::{AtomicU64, Ordering};

use php_value::{Object, PhpArray};

use orm::{get, obj, s, set, set_obj};
use ticket::{class, new};

/// `new GH6217AssociatedEntity()`.
fn associated_entity() -> Object {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let entity = new("GH6217AssociatedEntity");
    let n = NEXT.fetch_add(1, Ordering::SeqCst);
    set(
        &entity,
        "id",
        s(&format!("{}{n:013x}.{n}", class("GH6217AssociatedEntity"))),
    );
    entity
}

#[tokio::test]
async fn testLoadingOfSecondLevelCacheOnEagerAssociations() {
    let f = slc_functional::set_up_with(&[
        &class("GH6217AssociatedEntity"),
        &class("GH6217FetchedEntity"),
    ])
    .await;

    let lazy = associated_entity();
    let eager = associated_entity();
    let fetched = new("GH6217FetchedEntity");
    set_obj(&fetched, "lazy", &lazy);
    set_obj(&fetched, "eager", &eager);

    f.em.persist(&eager).unwrap();
    f.em.persist(&lazy).unwrap();
    f.em.persist(&fetched).unwrap();
    f.em.flush().await.unwrap();
    f.em.clear().unwrap();

    let repository = f.em.get_repository(&class("GH6217FetchedEntity")).unwrap();
    let mut filters = PhpArray::new();
    filters.insert("eager", get(&eager, "id"));

    assert_eq!(
        1,
        repository
            .find_by(&filters, None, None, None)
            .await
            .unwrap()
            .len()
    );
    f.reset_query_log();

    let found = repository
        .find_by(&filters, None, None, None)
        .await
        .unwrap();

    assert_eq!(1, found.len());
    assert!(found[0].instance_of(&class("GH6217FetchedEntity")));
    assert!(get(&lazy, "id").identical(&get(&obj(&get(&found[0], "lazy")), "id")));
    assert!(get(&eager, "id").identical(&get(&obj(&get(&found[0], "eager")), "id")));
    // No queries were executed in `findBy`
    f.assert_query_count(0);
}
