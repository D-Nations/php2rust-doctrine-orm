//! Ticket/DDC1113Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1113Test.php

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

const NS: &str = "Doctrine\\Tests\\ORM\\Functional\\Ticket\\";

fn class(short: &str) -> String {
    format!("{NS}{short}")
}

#[tokio::test]
async fn testIssue() {
    let classes = [
        class("DDC1113Engine"),
        class("DDC1113Vehicle"),
        class("DDC1113Car"),
        class("DDC1113Bus"),
    ];
    let em = orm::set_up(&classes.iter().map(String::as_str).collect::<Vec<_>>()).await;

    let car = new_object(&class("DDC1113Car"));
    set_obj(&car, "engine", &new_object(&class("DDC1113Engine")));

    let bus = new_object(&class("DDC1113Bus"));
    set_obj(&bus, "engine", &new_object(&class("DDC1113Engine")));

    em.persist(&car).unwrap();
    em.flush().await.unwrap();

    em.persist(&bus).unwrap();
    em.flush().await.unwrap();

    em.remove(&bus).await.unwrap();
    em.remove(&car).await.unwrap();
    em.flush().await.unwrap();

    for short in ["DDC1113Car", "DDC1113Bus", "DDC1113Engine"] {
        let all = em
            .get_repository(&class(short))
            .unwrap()
            .find_all()
            .await
            .unwrap();
        assert!(all.is_empty(), "{short}");
    }
}
