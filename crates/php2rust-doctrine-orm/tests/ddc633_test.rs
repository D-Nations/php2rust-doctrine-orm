//! Ticket/DDC633Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC633Test.php

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

use php2rust_doctrine_orm::abstract_query::HYDRATE_OBJECT;

use orm::*;
use ticket::*;

async fn set_up() -> std::sync::Arc<php2rust_doctrine_orm::entity_manager::EntityManager> {
    ticket::set_up(&["DDC633Patient", "DDC633Appointment"]).await
}

#[tokio::test]
async fn testOneToOneEager() {
    let em = set_up().await;

    let app = new("DDC633Appointment");
    let pat = new("DDC633Patient");
    set_obj(&app, "patient", &pat);
    set_obj(&pat, "appointment", &app);

    em.persist(&app).unwrap();
    em.persist(&pat).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let eager_appointment = find(&em, "DDC633Appointment", get(&app, "id"))
        .await
        .unwrap();

    // Eager loading of one to one leads to fetch-join
    assert!(!is_uninitialized(&em, &get(&eager_appointment, "patient")));
    assert!(
        em.contains(&obj(&get(&eager_appointment, "patient")))
            .unwrap()
    );
}

#[tokio::test]
async fn testDQLDeferredEagerLoad() {
    let em = set_up().await;

    for _ in 0..10 {
        let app = new("DDC633Appointment");
        let pat = new("DDC633Patient");
        set_obj(&app, "patient", &pat);
        set_obj(&pat, "appointment", &app);

        em.persist(&app).unwrap();
        em.persist(&pat).unwrap();
    }

    em.flush().await.unwrap();
    em.clear().unwrap();

    let appointments = list(
        &em.create_query(&format!("SELECT a FROM {NS}DDC633Appointment a"))
            .get_result(HYDRATE_OBJECT)
            .await
            .unwrap(),
    );

    for eager_appointment in appointments {
        assert!(
            !is_uninitialized(&em, &get(&obj(&eager_appointment), "patient")),
            "Proxy should already be initialized due to eager loading!"
        );
    }
}
