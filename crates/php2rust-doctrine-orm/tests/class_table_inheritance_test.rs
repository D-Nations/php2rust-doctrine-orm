//! ClassTableInheritanceTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa.
//! Functional tests for the Class Table Inheritance mapping strategy: the
//! persons and the events of the `company` model set.
//! `Criteria::create(true)` is `Criteria::create()`.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/ClassTableInheritanceTest.php

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

use php_value::datetime::{DateTime, DateTimeImmutable, TimeZone};
use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_collections::Criteria;
use php2rust_doctrine_orm::abstract_query::HYDRATE_OBJECT;
use php2rust_doctrine_orm::collection_object as coll;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::persistent_collection::PersistentCollection;
use php2rust_doctrine_persistence::php_reflection;

use company::*;
use orm::*;

async fn set_up() -> Arc<EntityManager> {
    register_listeners();
    orm::set_up(&COMPANY_MODELS).await
}

/// `assertInstanceOf($class, $object)`.
fn assert_instance_of(class: &str, object: &Object) {
    assert!(
        php_reflection::is_a(object.class(), class),
        "{} is not a {class}",
        object.class()
    );
}

/// `assertIsNumeric($value)`.
fn assert_is_numeric(value: &Value) {
    assert!(matches!(value, Value::Int(_)), "{value:?} is not numeric");
}

fn objects(value: &Value) -> Vec<Object> {
    list(value).iter().map(obj).collect()
}

fn criteria_eq(field: &str, value: impl Into<Value>) -> Criteria {
    let mut criteria = Criteria::create();
    criteria.r#where(Criteria::expr().eq(field, value.into()));
    criteria
}

fn by(field: &str, value: Value) -> PhpArray {
    let mut criteria = PhpArray::new();
    criteria.insert(field, value);
    criteria
}

#[tokio::test]
async fn testCRUD() {
    let em = set_up().await;
    let person = company_person("Roman S. Borschel");

    em.persist(&person).unwrap();

    let employee = company_employee("Roman S. Borschel", 100_000, "IT");

    em.persist(&employee).unwrap();

    set(&employee, "name", s("Guilherme Blanco"));
    em.flush().await.unwrap();

    em.clear().unwrap();

    let query = em.create_query(&format!(
        "select p from {COMPANY_PERSON} p order by p.name desc"
    ));

    let entities = objects(&query.get_result(HYDRATE_OBJECT).await.unwrap());

    assert_eq!(entities.len(), 2);
    assert_instance_of(COMPANY_PERSON, &entities[0]);
    assert_instance_of(COMPANY_EMPLOYEE, &entities[1]);
    assert_is_numeric(&get(&entities[0], "id"));
    assert_is_numeric(&get(&entities[1], "id"));
    assert_loose_eq(&s("Roman S. Borschel"), &get(&entities[0], "name"));
    assert_loose_eq(&s("Guilherme Blanco"), &get(&entities[1], "name"));
    assert_loose_eq(&Value::Int(100_000), &get(&entities[1], "salary"));

    em.clear().unwrap();

    assert_results_are_the_same(&query).await;

    em.clear().unwrap();

    let query = em.create_query(&format!("select p from {COMPANY_EMPLOYEE} p"));

    let entities = objects(&query.get_result(HYDRATE_OBJECT).await.unwrap());

    assert_eq!(entities.len(), 1);
    assert_instance_of(COMPANY_EMPLOYEE, &entities[0]);
    assert_is_numeric(&get(&entities[0], "id"));
    assert_loose_eq(&s("Guilherme Blanco"), &get(&entities[0], "name"));
    assert_loose_eq(&Value::Int(100_000), &get(&entities[0], "salary"));

    em.clear().unwrap();

    assert_results_are_the_same(&query).await;

    em.clear().unwrap();

    let guilherme = em
        .get_repository(employee.class())
        .unwrap()
        .find_one_by(&by("name", s("Guilherme Blanco")), None)
        .await
        .unwrap()
        .unwrap();
    assert_instance_of(COMPANY_EMPLOYEE, &guilherme);
    assert_loose_eq(&s("Guilherme Blanco"), &get(&guilherme, "name"));

    em.clear().unwrap();

    let query = em.create_query(&format!(
        "update {COMPANY_EMPLOYEE} p set p.name = ?1, p.department = ?2 where p.name='Guilherme Blanco' and p.salary = ?3"
    ));
    query.set_parameter(1, s("NewName"), Some("string".into()));
    query.set_parameter(2, s("NewDepartment"), None);
    query.set_parameter(3, Value::Int(100_000), None);
    query.get_sql().unwrap();
    let num_updated = query.execute(None, None).await.unwrap();
    assert_loose_eq(&Value::Int(1), &num_updated);

    let query = em.create_query(&format!("delete from {COMPANY_PERSON} p"));
    let num_deleted = query.execute(None, None).await.unwrap();
    assert_loose_eq(&Value::Int(2), &num_deleted);
}

#[tokio::test]
async fn testMultiLevelUpdateAndFind() {
    let em = set_up().await;
    let manager = company_manager("Roman S. Borschel", 100_000, "IT", "CTO");
    em.persist(&manager).unwrap();
    em.flush().await.unwrap();

    set(&manager, "name", s("Roman B."));
    set(&manager, "salary", Value::Int(119_000));
    set(&manager, "title", s("CEO"));
    em.persist(&manager).unwrap();
    em.flush().await.unwrap();

    em.clear().unwrap();

    let manager = em
        .find(COMPANY_MANAGER, get(&manager, "id"))
        .await
        .unwrap()
        .unwrap();

    assert_instance_of(COMPANY_MANAGER, &manager);
    assert_loose_eq(&s("Roman B."), &get(&manager, "name"));
    assert_loose_eq(&Value::Int(119_000), &get(&manager, "salary"));
    assert_loose_eq(&s("CEO"), &get(&manager, "title"));
    assert_is_numeric(&get(&manager, "id"));
}

#[tokio::test]
async fn testFindOnBaseClass() {
    let em = set_up().await;
    let manager = company_manager("Roman S. Borschel", 100_000, "IT", "CTO");
    em.persist(&manager).unwrap();
    em.flush().await.unwrap();

    em.clear().unwrap();

    let person = em
        .find(COMPANY_PERSON, get(&manager, "id"))
        .await
        .unwrap()
        .unwrap();

    assert_instance_of(COMPANY_MANAGER, &person);
    assert_loose_eq(&s("Roman S. Borschel"), &get(&person, "name"));
    assert_loose_eq(&Value::Int(100_000), &get(&person, "salary"));
    assert_loose_eq(&s("CTO"), &get(&person, "title"));
    assert_is_numeric(&get(&person, "id"));
}

#[tokio::test]
async fn testSelfReferencingOneToOne() {
    let em = set_up().await;
    let manager = company_manager("John Smith", 100_000, "IT", "CTO");

    let wife = company_person("Mary Smith");
    set_spouse(&wife, &manager);

    assert!(same(&manager, &get_object(&wife, "spouse").unwrap()));
    assert!(same(&wife, &get_object(&manager, "spouse").unwrap()));

    em.persist(&manager).unwrap();
    em.persist(&wife).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let query = em.create_query(&format!(
        "select p, s from {COMPANY_PERSON} p join p.spouse s where p.name='Mary Smith'"
    ));

    let result = objects(&query.get_result(HYDRATE_OBJECT).await.unwrap());
    assert_eq!(result.len(), 1);
    assert_instance_of(COMPANY_PERSON, &result[0]);
    assert_loose_eq(&s("Mary Smith"), &get(&result[0], "name"));
    let spouse = get_object(&result[0], "spouse").unwrap();
    assert_instance_of(COMPANY_EMPLOYEE, &spouse);
    assert_loose_eq(&s("John Smith"), &get(&spouse, "name"));
    assert!(same(&result[0], &get_object(&spouse, "spouse").unwrap()));

    em.clear().unwrap();

    assert_results_are_the_same(&query).await;
}

#[tokio::test]
async fn testSelfReferencingManyToMany() {
    let em = set_up().await;
    let person1 = company_person("Roman");

    let person2 = company_person("Jonathan");

    add_friend(&person1, &person2).await;

    assert_eq!(count_of(&person1, "friends").await, 1);
    assert_eq!(count_of(&person2, "friends").await, 1);

    em.persist(&person1).unwrap();
    em.persist(&person2).unwrap();

    em.flush().await.unwrap();

    em.clear().unwrap();

    let query = em.create_query(&format!(
        "select p, f from {COMPANY_PERSON} p join p.friends f where p.name=?1"
    ));
    query.set_parameter(1, s("Roman"), None);

    let result = objects(&query.get_result(HYDRATE_OBJECT).await.unwrap());
    assert_eq!(result.len(), 1);
    assert_eq!(count_of(&result[0], "friends").await, 1);
    assert_loose_eq(&s("Roman"), &get(&result[0], "name"));

    let friends = elements(&result[0], "friends").await;
    assert_loose_eq(&s("Jonathan"), &get(&friends[0], "name"));
}

#[tokio::test]
async fn testLazyLoading1() {
    let em = set_up().await;
    let org = company_organization();
    let event1 = company_event(COMPANY_AUCTION, "auction");
    add_event(&org, &event1);
    let event2 = company_event(COMPANY_RAFFLE, "raffle");
    add_event(&org, &event2);

    em.persist(&org).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let org_id = get(&org, "id");

    let q = em.create_query(
        "select a from Doctrine\\Tests\\Models\\Company\\CompanyOrganization a where a.id = ?1",
    );
    q.set_parameter(1, org_id, None);

    let result = objects(&q.get_result(HYDRATE_OBJECT).await.unwrap());

    assert_eq!(result.len(), 1);
    assert_instance_of(COMPANY_ORGANIZATION, &result[0]);
    assert!(get(&result[0], "mainevent").is_null());

    let org_events = get(&result[0], "events");

    let collection =
        PersistentCollection::from_value(&org_events).expect("a persistent collection");
    assert!(!collection.is_initialized());

    assert_eq!(coll::count(&org_events).await.unwrap(), 2);
    let first = obj(&coll::get(&org_events, 0_i64).await.unwrap());
    let second = obj(&coll::get(&org_events, 1_i64).await.unwrap());
    if php_reflection::is_a(first.class(), COMPANY_AUCTION) {
        assert_instance_of(COMPANY_RAFFLE, &second);
    } else {
        assert_instance_of(COMPANY_RAFFLE, &first);
        assert_instance_of(COMPANY_AUCTION, &second);
    }

    em.clear().unwrap();

    assert_results_are_the_same(&q).await;
}

#[tokio::test]
async fn testLazyLoading2() {
    let em = set_up().await;
    let org = company_organization();
    let event1 = company_event(COMPANY_AUCTION, "auction");
    set_obj(&org, "mainevent", &event1);

    em.persist(&org).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let q = em.create_query(&format!("select a from {COMPANY_EVENT} a where a.id = ?1"));
    q.set_parameter(1, get(&event1, "id"), None);

    let result = objects(&q.get_result(HYDRATE_OBJECT).await.unwrap());
    assert_eq!(result.len(), 1);
    assert!(
        php_reflection::is_a(result[0].class(), COMPANY_AUCTION),
        "Is of class {}",
        result[0].class()
    );

    em.clear().unwrap();

    assert_results_are_the_same(&q).await;

    em.clear().unwrap();

    let q = em.create_query(&format!(
        "select a from {COMPANY_ORGANIZATION} a where a.id = ?1"
    ));
    q.set_parameter(1, get(&org, "id"), None);

    let result = objects(&q.get_result(HYDRATE_OBJECT).await.unwrap());

    assert_eq!(result.len(), 1);
    assert_instance_of(COMPANY_ORGANIZATION, &result[0]);

    let main_event = get(&result[0], "mainevent");
    // mainEvent should have been loaded because it can't be lazy
    assert_instance_of(COMPANY_AUCTION, &obj(&main_event));
    assert!(!em.is_uninitialized_object(&main_event));

    em.clear().unwrap();

    assert_results_are_the_same(&q).await;
}

#[tokio::test]
async fn testBulkUpdateIssueDDC368() {
    let em = set_up().await;
    em.create_query(&format!("UPDATE {COMPANY_EMPLOYEE} AS p SET p.salary = 1"))
        .execute(None, None)
        .await
        .unwrap();

    let query = em.create_query(&format!(
        "SELECT count(p.id) FROM {COMPANY_EMPLOYEE} p WHERE p.salary = 1"
    ));
    let result = query.get_result(HYDRATE_OBJECT).await.unwrap();

    // assertGreaterThan(0, $result): an array is greater than an int.
    assert!(matches!(result, Value::Array(_)));

    em.clear().unwrap();

    assert_results_are_the_same(&query).await;
}

#[tokio::test]
async fn testBulkUpdateNonScalarParameterDDC1341() {
    let em = set_up().await;
    let query = em.create_query(&format!(
        "UPDATE {COMPANY_EMPLOYEE} AS p SET p.startDate = ?0 WHERE p.department = ?1"
    ));
    query.set_parameter(
        0,
        DateTime::object(DateTimeImmutable::now(TimeZone::Utc)),
        None,
    );
    query.set_parameter(1, s("IT"), None);
    query.execute(None, None).await.unwrap();
}

#[tokio::test]
async fn testDeleteJoinTableRecords() {
    let em = set_up().await;
    let employee1 = company_employee("gblanco", 0, "IT");

    let employee2 = company_employee("jwage", 0, "IT");

    add_friend(&employee1, &employee2).await;

    em.persist(&employee1).unwrap();
    em.persist(&employee2).unwrap();
    em.flush().await.unwrap();

    let employee1_id = get(&employee1, "id");

    em.remove(&employee1).await.unwrap();
    em.flush().await.unwrap();

    assert!(
        em.find(employee1.class(), employee1_id)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn testQueryForInheritedSingleValuedAssociation() {
    let em = set_up().await;
    let manager = company_manager("gblanco", 1234, "IT", "Awesome!");

    let person = company_person("spouse");

    set_spouse(&manager, &person);

    em.persist(&manager).unwrap();
    em.persist(&person).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let query = em.create_query(&format!(
        "SELECT m FROM {COMPANY_MANAGER} m WHERE m.spouse = ?1"
    ));
    query.set_parameter(1, get(&person, "id"), None);
    let dql_manager = obj(&query.get_single_result(None).await.unwrap());

    assert_loose_eq(&get(&manager, "id"), &get(&dql_manager, "id"));
    assert_loose_eq(
        &get(&person, "id"),
        &get(&get_object(&dql_manager, "spouse").unwrap(), "id"),
    );
}

#[tokio::test]
async fn testFindByAssociation() {
    let em = set_up().await;
    let manager = company_manager("gblanco", 1234, "IT", "Awesome!");

    let person = company_person("spouse");

    set_spouse(&manager, &person);

    em.persist(&manager).unwrap();
    em.persist(&person).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let repos = em.get_repository(COMPANY_MANAGER).unwrap();
    let pmanager = repos
        .find_one_by(&by("spouse", get(&person, "id")), None)
        .await
        .unwrap()
        .unwrap();

    assert_loose_eq(&get(&manager, "id"), &get(&pmanager, "id"));

    let repos = em.get_repository(COMPANY_PERSON).unwrap();
    let pmanager = repos
        .find_one_by(&by("spouse", get(&person, "id")), None)
        .await
        .unwrap()
        .unwrap();

    assert_loose_eq(&get(&manager, "id"), &get(&pmanager, "id"));
}

#[tokio::test]
async fn testGetReferenceEntityWithSubclasses() {
    let em = set_up().await;
    let manager = company_manager("gblanco", 1234, "IT", "Awesome!");

    em.persist(&manager).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let reference = em
        .get_reference(COMPANY_PERSON, get(&manager, "id"))
        .await
        .unwrap()
        .unwrap();
    assert!(
        !em.is_uninitialized_object(&Value::Object(reference.clone())),
        "Cannot Request a proxy from a class that has subclasses."
    );
    assert_instance_of(COMPANY_PERSON, &reference);
    assert!(
        php_reflection::is_a(reference.class(), COMPANY_EMPLOYEE),
        "Direct fetch of the reference has to load the child class Employee directly."
    );
    em.clear().unwrap();

    let reference = em
        .get_reference(COMPANY_MANAGER, get(&manager, "id"))
        .await
        .unwrap()
        .unwrap();
    assert!(
        em.is_uninitialized_object(&Value::Object(reference)),
        "A proxy can be generated only if no subclasses exists for the requested reference."
    );
}

#[tokio::test]
async fn testGetSubClassManyToManyCollection() {
    let em = set_up().await;
    let manager = company_manager("gblanco", 1234, "IT", "Awesome!");

    let person = company_person("friend");

    add_friend(&manager, &person).await;

    em.persist(&manager).unwrap();
    em.persist(&person).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let manager = em
        .find(COMPANY_MANAGER, get(&manager, "id"))
        .await
        .unwrap()
        .unwrap();

    assert_eq!(count_of(&manager, "friends").await, 1);
}

#[tokio::test]
async fn testExistsSubclass() {
    let em = set_up().await;
    let manager = company_manager("gblanco", 1234, "IT", "Awesome!");

    let persister = em
        .get_unit_of_work()
        .get_entity_persister(manager.class())
        .unwrap();
    assert!(!persister.exists(&manager, None).await.unwrap());

    em.persist(&manager).unwrap();
    em.flush().await.unwrap();

    let persister = em
        .get_unit_of_work()
        .get_entity_persister(manager.class())
        .unwrap();
    assert!(persister.exists(&manager, None).await.unwrap());
}

#[tokio::test]
async fn testMatching() {
    let em = set_up().await;
    let manager = company_manager("gblanco", 1234, "IT", "Awesome!");

    em.persist(&manager).unwrap();
    em.flush().await.unwrap();

    let repository = em.get_repository(COMPANY_EMPLOYEE).unwrap();
    let users = repository
        .matching(&criteria_eq("department", s("IT")))
        .unwrap()
        .value();
    assert_eq!(coll::count(&users).await.unwrap(), 1);

    let repository = em.get_repository(COMPANY_MANAGER).unwrap();
    let users = repository
        .matching(&criteria_eq("department", s("IT")))
        .unwrap()
        .value();
    assert_eq!(coll::count(&users).await.unwrap(), 1);
}
