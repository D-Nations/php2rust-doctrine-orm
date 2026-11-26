//! Ticket/GH10808Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH10808Test.php

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
use php2rust_doctrine_orm::query::context::Hint;
use php2rust_doctrine_orm::unit_of_work::HINT_DEFEREAGERLOAD;

use orm::*;
use ticket::*;

#[tokio::test]
async fn testDQLDeferredEagerLoad() {
    let em = ticket::set_up(&["GH10808Appointment", "GH10808AppointmentChild"]).await;

    // new GH10808Appointment()
    let appointment = new("GH10808Appointment");
    set_obj(&appointment, "child", &new("GH10808AppointmentChild"));
    em.persist(&appointment).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let query = em.create_query(
        "SELECT appointment from Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH10808Appointment appointment
               JOIN appointment.child appointment_child",
    );

    // By default, UnitOfWork::HINT_DEFEREAGERLOAD is set to 'true'
    let deferred_load_result = obj(&query.get_single_result(None).await.unwrap());

    // Clear the EM to prevent the recovery of the loaded instance, which would otherwise result in a proxy.
    em.clear().unwrap();

    query.set_hint(HINT_DEFEREAGERLOAD, Hint::Value(Value::Bool(false)));
    let _eager_load_result = query.get_single_result(None).await.unwrap();

    assert!(
        !is_uninitialized(&em, &get(&deferred_load_result, "child")),
        "$deferredLoadResult->child should be a native lazy object"
    );
    assert!(
        !is_uninitialized(&em, &get(&deferred_load_result, "child")),
        "$eagerLoadResult->child should not be a proxy"
    );
}
