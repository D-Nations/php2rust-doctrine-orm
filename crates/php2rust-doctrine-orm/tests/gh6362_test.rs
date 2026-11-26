//! Ticket/GH6362Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models). The faked result set is
//! `tests/common/hydration.rs`'s rows (upstream's `ArrayResultFactory`).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH6362Test.php

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/hydration.rs"]
mod hydration;
#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;
#[path = "common/ticket.rs"]
mod ticket;

use php2rust_doctrine_orm::abstract_query::HYDRATE_OBJECT;
use php2rust_doctrine_orm::query::ResultSetMapping;

use orm::*;
use ticket::*;

/// SELECT a as base, b, c, d
/// FROM Start a
/// LEFT JOIN a.bases b
/// LEFT JOIN Child c ON b.id = c.id
/// LEFT JOIN c.joins d
#[tokio::test]
async fn testInheritanceJoinAlias() {
    let em = ticket::set_up(&["GH6362Start", "GH6362Base", "GH6362Child", "GH6362Join"]).await;

    let mut rsm = ResultSetMapping::new();
    rsm.add_entity_result(&class("GH6362Start"), "a", Some("base"));
    rsm.add_joined_entity_result(&class("GH6362Base"), "b", "a", "bases", false);
    rsm.add_entity_result(&class("GH6362Child"), "c", None);
    rsm.add_joined_entity_result(&class("GH6362Join"), "d", "c", "joins", false);

    rsm.add_field_result("a", "id_0", "id", None);
    rsm.add_field_result("b", "id_1", "id", None);
    rsm.add_field_result("c", "id_2", "id", None);
    rsm.add_field_result("d", "id_3", "id", None);

    rsm.add_meta_result("a", "bases_id_4", "bases_id", false, Some("integer"));
    rsm.add_meta_result("b", "type_5", "type", false, None);
    rsm.add_meta_result("c", "type_6", "type", false, None);
    rsm.add_meta_result("d", "child_id_7", "child_id", false, Some("integer"));

    rsm.set_discriminator_column("b", "type_5");
    rsm.set_discriminator_column("c", "type_6");

    let result_set = hydration::rows(&[&[
        ("id_0", s("1")),
        ("id_1", s("1")),
        ("id_2", s("1")),
        ("id_3", s("1")),
        ("bases_id_4", s("1")),
        ("type_5", s("child")),
        ("type_6", s("child")),
        ("child_id_7", s("1")),
    ]]);

    let result = hydration::hydrate_all(&em, HYDRATE_OBJECT, result_set, &rsm)
        .await
        .unwrap();

    assert_instance_of("GH6362Start", &at(&at(&result, 0), "base"));
    assert_instance_of("GH6362Child", &at(&at(&result, 1), 0));
}
