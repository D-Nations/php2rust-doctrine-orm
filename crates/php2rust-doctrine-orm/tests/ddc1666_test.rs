//! Ticket/DDC1666Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the cms models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1666Test.php

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

use orm::*;

#[tokio::test]
async fn testGivenOrphanRemovalOneToOneWhenReplacingThenNoUniqueConstraintError() {
    let em = orm::set_up(&CMS_MODELS).await;

    let user = cms_user_with("Benjamin", "beberlei", "something");
    let email = cms_email("kontakt@beberlei.de");
    set_email(&user, &email);

    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    assert!(em.contains(&email).unwrap());

    let new_email = cms_email("benjamin.eberlei@googlemail.com");
    set_email(&user, &new_email);

    em.flush().await.unwrap();

    assert!(!em.contains(&email).unwrap());
}
