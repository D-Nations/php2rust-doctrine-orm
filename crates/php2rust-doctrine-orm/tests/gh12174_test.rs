//! Port of `GH12174Test`, on `SQLite`: the listener's
//! `addEventSubscriber()` is `ResolveTargetEntityListener::subscribe()`.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH12174Test.php

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

use std::sync::Arc;

use php_value::PhpArray;
use php2rust_doctrine_orm::deprecations;
use php2rust_doctrine_orm::tools::ResolveTargetEntityListener;

/// `https://github.com/doctrine/orm/pull/10431`.
const DEPRECATION: &str = "https://github.com/doctrine/orm/pull/10431";

#[tokio::test]
async fn testMappedSuperclassNameCanBeUsedToResolveTargetEntityClass() {
    // setUp()
    models::register();
    deprecations::enable_tracking_deprecations();
    let before = deprecations::count(DEPRECATION);
    let em = orm::entity_manager(orm::connection(), None);

    let resolve_target_entity = Arc::new(ResolveTargetEntityListener::new());

    resolve_target_entity.add_resolve_target_entity(
        &ticket::class("GH12174Smurf"),
        &ticket::class("GH12174BlueSmurf"),
        PhpArray::new(),
    );

    resolve_target_entity.subscribe(&em).unwrap();

    orm::create_schema(
        &em,
        &[
            &ticket::class("GH12174Smurf"),
            &ticket::class("GH12174BlueSmurf"),
            &ticket::class("GH12174PapaSmurf"),
        ],
    )
    .await;

    let smurf = em
        .get_class_metadata(&ticket::class("GH12174Smurf"))
        .unwrap();
    let smurf = smurf.read();
    assert!(smurf.is_mapped_superclass);
    assert_eq!(ticket::class("GH12174Smurf"), smurf.name);
    assert_eq!(
        ticket::class("GH12174BlueSmurf"),
        smurf
            .get_association_mapping("children")
            .unwrap()
            .target_entity
    );

    let blue = em
        .get_class_metadata(&ticket::class("GH12174BlueSmurf"))
        .unwrap();
    let blue = blue.read();
    assert!(!blue.is_mapped_superclass);
    assert_eq!(ticket::class("GH12174BlueSmurf"), blue.name);

    let papa = em
        .get_class_metadata(&ticket::class("GH12174PapaSmurf"))
        .unwrap();
    let papa = papa.read();
    assert!(!papa.is_mapped_superclass);
    assert_eq!(ticket::class("GH12174PapaSmurf"), papa.name);

    // expectNoDeprecationWithIdentifier()
    assert_eq!(before, deprecations::count(DEPRECATION));
}
