//! Ticket/GH10880Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH10880Test.php

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
#[path = "common/ticket.rs"]
mod ticket;

use orm::*;
use ticket::*;

#[tokio::test]
async fn testProcessShouldBeUpdated() {
    let classes = [
        class("GH10880BaseProcess"),
        class("GH10880Process"),
        class("GH10880ProcessOwner"),
    ];
    let classes: Vec<&str> = classes.iter().map(String::as_str).collect();
    let (em, log) = query_log::set_up_logged(&classes).await;

    let process = new("GH10880Process");
    set(&process, "description", s("first value"));

    let owner = new("GH10880ProcessOwner");
    set_obj(&owner, "process", &process);

    em.persist(&process).unwrap();
    em.persist(&owner).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let owner_loaded = em
        .get_repository(&class("GH10880ProcessOwner"))
        .unwrap()
        .find(get(&owner, "id"))
        .await
        .unwrap()
        .unwrap();
    set(
        &obj(&get(&owner_loaded, "process")),
        "description",
        s("other description"),
    );

    log.reset_and_enable();
    em.flush().await.unwrap();
    // removeTransactionCommandsFromQueryLog()
    let queries: Vec<String> = log
        .queries()
        .into_iter()
        .filter(|sql| sql != "\"START TRANSACTION\"" && sql != "\"COMMIT\"")
        .collect();

    assert_eq!(1, queries.len(), "{queries:?}");
    assert_eq!(
        "UPDATE GH10880BaseProcess SET description = ? WHERE id = ?",
        queries[0]
    );
}
