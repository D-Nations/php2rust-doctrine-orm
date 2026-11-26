//! Ticket/DDC353Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC353Test.php

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

use php_value::Value;
use php2rust_doctrine_orm::unit_of_work::STATE_MANAGED;

use orm::*;
use ticket::*;

#[tokio::test]
async fn testWorkingCase() {
    let em = ticket::set_up(&["DDC353File", "DDC353Picture"]).await;

    let file = new("DDC353File");

    let picture = new("DDC353Picture");
    set_obj(&picture, "file", &file);

    em.persist(&picture).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let file_id = get(&file, "fileId");
    assert!(matches!(file_id, Value::Int(id) if id > 0));

    let file = em
        .get_reference(&class("DDC353File"), file_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        STATE_MANAGED,
        em.get_unit_of_work().get_entity_state(&file).await.unwrap(),
        "Reference Proxy should be marked MANAGED."
    );

    let picture = find(&em, "DDC353Picture", get(&picture, "pictureId"))
        .await
        .unwrap();
    assert_eq!(
        STATE_MANAGED,
        em.get_unit_of_work()
            .get_entity_state(&obj(&get(&picture, "file")))
            .await
            .unwrap(),
        "Lazy Proxy should be marked MANAGED."
    );

    em.remove(&picture).await.unwrap();
    em.flush().await.unwrap();
}

#[tokio::test]
async fn testFailingCase() {
    let em = ticket::set_up(&["DDC353File", "DDC353Picture"]).await;

    let file = new("DDC353File");

    let picture = new("DDC353Picture");
    set_obj(&picture, "file", &file);

    em.persist(&picture).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let file_id = get(&file, "fileId");
    let picture_id = get(&picture, "pictureId");
    assert!(matches!(file_id, Value::Int(id) if id > 0));

    let picture = find(&em, "DDC353Picture", picture_id).await.unwrap();
    assert_eq!(
        STATE_MANAGED,
        em.get_unit_of_work()
            .get_entity_state(&obj(&get(&picture, "file")))
            .await
            .unwrap(),
        "Lazy Proxy should be marked MANAGED."
    );

    em.remove(&picture).await.unwrap();
    em.flush().await.unwrap();
}
