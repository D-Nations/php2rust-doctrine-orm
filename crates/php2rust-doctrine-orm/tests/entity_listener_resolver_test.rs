//! Upstream: doctrine/orm tests/Tests/ORM/Mapping/EntityListenerResolverTest.php
//!
//! `assertSame()` compares the objects' `Arc`s; `new $className()` is an
//! `EntityObject` of the recorded class.

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

use php2rust_doctrine_orm::mapping::entity_listener_resolver::{
    DefaultEntityListenerResolver, EntityListenerResolver,
};

use orm::same;

const COMPANY_CONTRACT_LISTENER: &str = "Doctrine\\Tests\\Models\\Company\\CompanyContractListener";
const COMPANY_FLEX_ULTRA_CONTRACT_LISTENER: &str =
    "Doctrine\\Tests\\Models\\Company\\CompanyFlexUltraContractListener";

fn set_up() -> DefaultEntityListenerResolver {
    models::register();
    DefaultEntityListenerResolver::new()
}

#[test]
fn testResolve() {
    let resolver = set_up();
    let class_name = COMPANY_CONTRACT_LISTENER;
    let object = resolver.resolve(class_name).unwrap();

    assert!(object.instance_of(class_name));
    assert!(same(&object, &resolver.resolve(class_name).unwrap()));
}

#[test]
fn testRegisterAndResolve() {
    let resolver = set_up();
    let class_name = COMPANY_CONTRACT_LISTENER;
    let object = orm::new_object(class_name);

    resolver.register(object.clone());

    assert!(same(&object, &resolver.resolve(class_name).unwrap()));
}

#[test]
fn testClearOne() {
    let resolver = set_up();
    let class_name1 = COMPANY_CONTRACT_LISTENER;
    let class_name2 = COMPANY_FLEX_ULTRA_CONTRACT_LISTENER;

    let obj1 = resolver.resolve(class_name1).unwrap();
    let obj2 = resolver.resolve(class_name2).unwrap();

    assert!(obj1.instance_of(class_name1));
    assert!(obj2.instance_of(class_name2));

    assert!(same(&obj1, &resolver.resolve(class_name1).unwrap()));
    assert!(same(&obj2, &resolver.resolve(class_name2).unwrap()));

    resolver.clear(Some(class_name1));

    assert!(resolver.resolve(class_name1).unwrap().instance_of(class_name1));
    assert!(resolver.resolve(class_name2).unwrap().instance_of(class_name2));

    assert!(!same(&obj1, &resolver.resolve(class_name1).unwrap()));
    assert!(same(&obj2, &resolver.resolve(class_name2).unwrap()));
}

#[test]
fn testClearAll() {
    let resolver = set_up();
    let class_name1 = COMPANY_CONTRACT_LISTENER;
    let class_name2 = COMPANY_FLEX_ULTRA_CONTRACT_LISTENER;

    let obj1 = resolver.resolve(class_name1).unwrap();
    let obj2 = resolver.resolve(class_name2).unwrap();

    assert!(obj1.instance_of(class_name1));
    assert!(obj2.instance_of(class_name2));

    assert!(same(&obj1, &resolver.resolve(class_name1).unwrap()));
    assert!(same(&obj2, &resolver.resolve(class_name2).unwrap()));

    resolver.clear(None);

    assert!(resolver.resolve(class_name1).unwrap().instance_of(class_name1));
    assert!(resolver.resolve(class_name2).unwrap().instance_of(class_name2));

    assert!(!same(&obj1, &resolver.resolve(class_name1).unwrap()));
    assert!(!same(&obj2, &resolver.resolve(class_name2).unwrap()));
}
