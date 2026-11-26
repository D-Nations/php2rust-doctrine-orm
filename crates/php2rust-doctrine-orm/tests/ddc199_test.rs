//! Ticket/DDC199Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC199Test.php

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
use php2rust_doctrine_orm::collection_object as coll;
use php2rust_doctrine_orm::persistent_collection::PersistentCollection;

use orm::*;
use ticket::*;

#[tokio::test]
async fn testPolymorphicLoading() {
    let em = ticket::set_up(&[
        "DDC199ParentClass",
        "DDC199ChildClass",
        "DDC199RelatedClass",
    ])
    .await;

    let child = new("DDC199ChildClass");
    set(&child, "parentData", s("parentData"));
    set(&child, "childData", s("childData"));
    em.persist(&child).unwrap();

    let related1 = new("DDC199RelatedClass");
    set(&related1, "relatedData", s("related1"));
    set_obj(&related1, "parent", &child);
    em.persist(&related1).unwrap();

    let related2 = new("DDC199RelatedClass");
    set(&related2, "relatedData", s("related2"));
    set_obj(&related2, "parent", &child);
    em.persist(&related2).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();

    let query = em.create_query(
        "select e,r from Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC199ParentClass e join e.relatedEntities r",
    );
    let result = list(&query.get_result(HYDRATE_OBJECT).await.unwrap());

    assert_eq!(1, result.len());
    assert_instance_of("DDC199ParentClass", &result[0]);
    let related = get(&obj(&result[0]), "relatedEntities");
    assert!(
        PersistentCollection::from_value(&related)
            .unwrap()
            .is_initialized()
    );
    assert_eq!(2, coll::count(&related).await.unwrap());
    assert_instance_of("DDC199RelatedClass", &coll::get(&related, 0).await.unwrap());
    assert_instance_of("DDC199RelatedClass", &coll::get(&related, 1).await.unwrap());
}
