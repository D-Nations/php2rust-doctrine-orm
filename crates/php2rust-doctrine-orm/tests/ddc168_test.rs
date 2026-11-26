//! Ticket/DDC168Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the company models). `tearDown()` puts the old metadata
//! back; each test here has its own entity manager, so it is not needed.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC168Test.php

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/company.rs"]
mod company;
#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;

use std::sync::Arc;

use php_value::Value;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::mapping::ClassMetadata;
use php2rust_doctrine_orm::mapping::ordered_map::OrderedMap;
use php2rust_doctrine_persistence::mapping::AbstractClassMetadataFactory;

use company::{COMPANY_EMPLOYEE, COMPANY_MODELS, company_employee, set_spouse};
use orm::{assert_loose_eq, get, obj, s};

async fn set_up() -> Arc<EntityManager> {
    company::register_listeners();
    let em = orm::set_up(&COMPANY_MODELS).await;

    let old_metadata = em.get_class_metadata(COMPANY_EMPLOYEE).unwrap();

    let mut metadata = old_metadata.read().clone();
    let mut accessors: Vec<_> = metadata
        .property_accessors
        .iter()
        .map(|(name, accessor)| (name.to_owned(), Arc::clone(accessor)))
        .collect();
    accessors.sort_by(|a, b| a.0.cmp(&b.0));
    metadata.property_accessors = accessors.into_iter().collect::<OrderedMap<_>>();
    em.get_metadata_factory().set_metadata_for(
        COMPANY_EMPLOYEE,
        Arc::new(ClassMetadata::from_info(metadata)),
    );

    em
}

#[tokio::test]
async fn testJoinedSubclassPersisterRequiresSpecificOrderOfMetadataReflFieldsArray() {
    let em = set_up().await;
    let spouse = company_employee("Blub", 500, "Accounting");

    let employee = company_employee("Foo", 1000, "bar");
    set_spouse(&employee, &spouse);

    em.persist(&spouse).unwrap();
    em.persist(&employee).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();

    let q = em.create_query(
        "SELECT e FROM Doctrine\\Tests\\Models\\Company\\CompanyEmployee e WHERE e.name = ?1",
    );
    q.set_parameter(1, s("Foo"), None);
    let the_employee = obj(&q.get_single_result(None).await.unwrap());

    assert_loose_eq(&s("bar"), &get(&the_employee, "department"));
    assert_loose_eq(&s("Foo"), &get(&the_employee, "name"));
    assert_loose_eq(&Value::Int(1000), &get(&the_employee, "salary"));
    assert!(the_employee.instance_of(COMPANY_EMPLOYEE));
    let spouse = get(&the_employee, "spouse");
    assert!(obj(&spouse).instance_of(COMPANY_EMPLOYEE), "{spouse:?}");
}
