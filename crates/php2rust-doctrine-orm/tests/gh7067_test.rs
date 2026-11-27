//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH7067Test.php
//!
//! `enableSecondLevelCache()`: the second-level cache fixture of
//! `tests/common/slc_functional.rs` with the recorded `GH7067Entity` model.

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

use php_value::datetime::{DateTime, DateTimeImmutable, TimeZone};

use orm::{get, set};
use ticket::{class, new};

/// GH-7067
#[tokio::test]
async fn testSLCWithVersion() {
    let f = slc_functional::set_up_with(&[&class("GH7067Entity")]).await;
    let entity = new("GH7067Entity");
    set(
        &entity,
        "lastUpdate",
        DateTime::object(DateTimeImmutable::now(TimeZone::Utc)),
    );

    f.em.persist(&entity).unwrap();
    f.em.flush().await.unwrap();
    f.em.clear().unwrap();

    let not_cached =
        f.em.find(&class("GH7067Entity"), get(&entity, "id"))
            .await
            .unwrap()
            .unwrap();

    assert!(
        !get(&not_cached, "version").is_null(),
        "Version already cached by persister above, it must be not null"
    );

    // new DateTime('+1 seconds')
    let now = DateTimeImmutable::now(TimeZone::Utc);
    set(
        &not_cached,
        "lastUpdate",
        DateTime::object(DateTimeImmutable::from_timestamp(
            now.timestamp() + 1,
            now.microsecond(),
            TimeZone::Utc,
        )),
    );

    f.em.flush().await.unwrap();
    f.em.clear().unwrap();
}
