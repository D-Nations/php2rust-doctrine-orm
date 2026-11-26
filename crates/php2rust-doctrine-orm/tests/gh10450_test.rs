//! Ticket/GH10450Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! A data provider's rows run in one test.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH10450Test.php

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

use ticket::*;

/// `classesThatOverrideFieldNames()`.
const CLASSES_THAT_OVERRIDE_FIELD_NAMES: [(&str, &str); 4] = [
    (
        "Entity class that redeclares a private field inherited from a base entity",
        "GH10450EntityChildPrivate",
    ),
    (
        "Entity class that redeclares a private field inherited from a mapped superclass",
        "GH10450MappedSuperclassChildPrivate",
    ),
    (
        "Entity class that redeclares a protected field inherited from a base entity",
        "GH10450EntityChildProtected",
    ),
    (
        "Entity class that redeclares a protected field inherited from a mapped superclass",
        "GH10450MappedSuperclassChildProtected",
    ),
];

#[tokio::test]
async fn testDuplicatePrivateFieldsShallBeRejected() {
    for (name, short) in CLASSES_THAT_OVERRIDE_FIELD_NAMES {
        let em = orm::entity_manager(orm::connection(), None);
        let error = em.get_class_metadata(&class(short)).expect_err(name);
        assert_eq!(
            "Doctrine\\ORM\\Mapping\\MappingException", error.class,
            "{name}"
        );
    }
}
