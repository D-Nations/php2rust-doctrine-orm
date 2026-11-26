//! Ticket/GH10387Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! A data provider's rows run in one test.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH10387Test.php

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

use php2rust_doctrine_orm::tools::schema_tool::SchemaTool;

use ticket::*;

/// `classHierachies()`.
const CLASS_HIERARCHIES: [(&str, [&str; 3]); 3] = [
    (
        "hierarchy with Entity classes only",
        [
            "GH10387EntitiesOnlyRoot",
            "GH10387EntitiesOnlyMiddle",
            "GH10387EntitiesOnlyLeaf",
        ],
    ),
    (
        "MappedSuperclass in the middle of the hierarchy",
        [
            "GH10387MappedSuperclassRoot",
            "GH10387MappedSuperclassMiddle",
            "GH10387MappedSuperclassLeaf",
        ],
    ),
    (
        "abstract entity at the root and in the middle of the hierarchy",
        [
            "GH10387AbstractEntitiesRoot",
            "GH10387AbstractEntitiesMiddle",
            "GH10387AbstractEntitiesLeaf",
        ],
    ),
];

#[tokio::test]
async fn testSchemaToolCreatesColumnForFieldInTheMiddleClass() {
    for (name, classes) in CLASS_HIERARCHIES {
        let em = orm::entity_manager(orm::connection(), None);
        let schema_tool = SchemaTool::new(&em);
        let metadata: Vec<_> = classes
            .iter()
            .map(|short| em.get_class_metadata(&class(short)).unwrap())
            .collect();
        let schema = schema_tool
            .get_schema_from_metadata(&metadata[..1])
            .unwrap();
        let table = schema.get_table("root").unwrap();
        assert!(table.get_column("middle_class_field").is_ok(), "{name}");
        assert!(table.get_column("leaf_class_field").is_ok(), "{name}");
    }
}
