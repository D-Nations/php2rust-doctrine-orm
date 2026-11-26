//! ManyToManyCriteriaMatchingTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa.
//! The `vct_manytomany` model set uses `common/rot13.rs`. The data provider's
//! cases run in one test, each on its own database.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/ValueConversionType/ManyToManyCriteriaMatchingTest.php

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
#[path = "common/rot13.rs"]
mod rot13;

use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_collections::Criteria;
use php2rust_doctrine_collections::expr::Comparison;
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};
use php2rust_doctrine_orm::persistent_collection::PersistentCollection;

use orm::*;

const INVERSED: &str = "Doctrine\\Tests\\Models\\ValueConversionType\\InversedManyToManyEntity";
const OWNING: &str = "Doctrine\\Tests\\Models\\ValueConversionType\\OwningManyToManyEntity";

fn provide_matching_expressions() -> Vec<Comparison> {
    vec![
        Criteria::expr().eq("field", s("match this")), // should convert to 'zngpu guvf'
        Criteria::expr().r#in("field", PhpArray::list([s("match this")])), // should convert to ['zngpu guvf']
    ]
}

fn entity(class: &str, id_field: &str, id: &str) -> Object {
    let entity = new_object(class);
    set(
        &entity,
        "associatedEntities",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    set(&entity, id_field, s(id));
    entity
}

#[tokio::test]
async fn testCriteriaMatchingOnFieldInManyToManyTarget() {
    for comparison in provide_matching_expressions() {
        rot13::register_rot13_type();
        let (em, log) = query_log::set_up_logged(&[INVERSED, OWNING]).await;

        let associated = entity(INVERSED, "id1", "associated");

        let matching = entity(OWNING, "id2", "first");
        set(&matching, "field", s("match this")); // stored as 'zngpu guvf'
        coll::add(
            &get(&matching, "associatedEntities"),
            Value::Object(associated.clone()),
        )
        .unwrap();

        let non_matching = entity(OWNING, "id2", "second");
        set(&non_matching, "field", s("this is no match"));
        coll::add(
            &get(&non_matching, "associatedEntities"),
            Value::Object(associated.clone()),
        )
        .unwrap();

        em.persist(&associated).unwrap();
        em.persist(&matching).unwrap();
        em.persist(&non_matching).unwrap();
        em.flush().await.unwrap();
        em.clear().unwrap();

        log.reset_and_enable();

        let associated = em.find(INVERSED, s("associated")).await.unwrap().unwrap();
        assert!(
            !PersistentCollection::from_value(&get(&associated, "associatedEntities"))
                .unwrap()
                .is_initialized(),
            "Pre-condition: lazy collection"
        );

        let mut criteria = Criteria::create();
        criteria.r#where(comparison);
        let result = coll::matching(&get(&associated, "associatedEntities"), &criteria)
            .await
            .unwrap();

        assert_eq!(coll::count(&result).await.unwrap(), 1);
        let Value::Object(first) = coll::get(&result, 0).await.unwrap() else {
            panic!("an entity");
        };
        assert!(get(&first, "id2").identical(&s("first")));
    }
}
