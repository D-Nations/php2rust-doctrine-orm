//! Ticket/DDC2759Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC2759Test.php

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

use php_value::Object;
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::*;
use ticket::*;

async fn set_up() -> Arc<EntityManager> {
    let em = ticket::set_up(&[
        "DDC2759Qualification",
        "DDC2759Category",
        "DDC2759QualificationMetadata",
        "DDC2759MetadataCategory",
    ])
    .await;

    let qualification = new("DDC2759Qualification");
    // new DDC2759QualificationMetadata($qualification)
    let qualification_metadata = new("DDC2759QualificationMetadata");
    set_obj(&qualification_metadata, "content", &qualification);

    let category1 = new("DDC2759Category");
    let category2 = new("DDC2759Category");

    // new DDC2759MetadataCategory($qualificationMetadata, $category)
    let metadata_category = |category: &Object| {
        let metadata_category = new("DDC2759MetadataCategory");
        set_obj(&metadata_category, "metadata", &qualification_metadata);
        set_obj(&metadata_category, "category", category);
        metadata_category
    };
    let metadata_category1 = metadata_category(&category1);
    let metadata_category2 = metadata_category(&category2);

    em.persist(&qualification).unwrap();
    em.persist(&qualification_metadata).unwrap();

    em.persist(&category1).unwrap();
    em.persist(&category2).unwrap();

    em.persist(&metadata_category1).unwrap();
    em.persist(&metadata_category2).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();
    em
}

#[tokio::test]
async fn testCorrectNumberOfAssociationsIsReturned() {
    let em = set_up().await;
    let repository = em.get_repository(&class("DDC2759Qualification")).unwrap();

    let mut builder = repository.create_query_builder("q", None).unwrap();
    builder
        .select(vec![s("q, qm, qmc")])
        .unwrap()
        .inner_join("q.metadata", "qm", None, None, None)
        .unwrap()
        .inner_join("qm.metadataCategories", "qmc", None, None, None)
        .unwrap();

    let result = builder
        .get_query()
        .unwrap()
        .get_array_result()
        .await
        .unwrap();

    assert_eq!(
        2,
        list(&at(&at(&at(&result, 0), "metadata"), "metadataCategories")).len()
    );
}
