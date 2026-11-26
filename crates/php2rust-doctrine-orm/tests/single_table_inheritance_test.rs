//! SingleTableInheritanceTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa.
//! The contracts of the `company` model set (single table inheritance).
//! `Criteria::create(true)` is `Criteria::create()`.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/SingleTableInheritanceTest.php

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

use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_collections::Criteria;
use php2rust_doctrine_orm::abstract_query::HYDRATE_OBJECT;
use php2rust_doctrine_orm::collection_object as coll;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::mapping::class_metadata::FETCH_EAGER;
use php2rust_doctrine_persistence::php_reflection;

use company::*;
use orm::*;

struct Fixture {
    em: Arc<EntityManager>,
    sales_person: Object,
    engineers: Vec<Object>,
    fix: Option<Object>,
    flex: Option<Object>,
    ultra: Option<Object>,
}

impl Fixture {
    fn fix(&self) -> &Object {
        self.fix.as_ref().unwrap()
    }

    fn flex(&self) -> &Object {
        self.flex.as_ref().unwrap()
    }

    fn ultra(&self) -> &Object {
        self.ultra.as_ref().unwrap()
    }
}

async fn set_up() -> Arc<EntityManager> {
    register_listeners();
    orm::set_up(&COMPANY_MODELS).await
}

/// `persistRelatedEmployees()`.
async fn persist_related_employees() -> Fixture {
    let em = set_up().await;
    let sales_person = company_employee("Poor Sales Guy", 100, "Sales");

    let engineers = vec![
        company_employee("Roman B.", 100, "IT"),
        company_employee("Jonathan W.", 100, "IT"),
        company_employee("Benjamin E.", 100, "IT"),
        company_employee("Guilherme B.", 100, "IT"),
    ];

    em.persist(&sales_person).unwrap();
    for engineer in &engineers {
        em.persist(engineer).unwrap();
    }
    Fixture {
        em,
        sales_person,
        engineers,
        fix: None,
        flex: None,
        ultra: None,
    }
}

/// `loadFullFixture()`.
async fn load_full_fixture() -> Fixture {
    let mut f = persist_related_employees().await;

    let fix = company_contract(COMPANY_FIX_CONTRACT);
    set(&fix, "fixPrice", Value::Int(1000));
    set_obj(&fix, "salesPerson", &f.sales_person);
    add_engineer(&fix, &f.engineers[0]);
    add_engineer(&fix, &f.engineers[1]);
    mark_completed(&fix);

    let flex = company_contract(COMPANY_FLEX_CONTRACT);
    set_obj(&flex, "salesPerson", &f.sales_person);
    set(&flex, "hoursWorked", Value::Int(100));
    set(&flex, "pricePerHour", Value::Int(100));
    add_engineer(&flex, &f.engineers[2]);
    add_engineer(&flex, &f.engineers[1]);
    add_engineer(&flex, &f.engineers[3]);
    mark_completed(&flex);

    let ultra = company_contract(COMPANY_FLEX_ULTRA_CONTRACT);
    set_obj(&ultra, "salesPerson", &f.sales_person);
    set(&ultra, "hoursWorked", Value::Int(150));
    set(&ultra, "pricePerHour", Value::Int(150));
    set(&ultra, "maxPrice", Value::Int(7000));
    add_engineer(&ultra, &f.engineers[3]);
    add_engineer(&ultra, &f.engineers[0]);

    f.em.persist(&fix).unwrap();
    f.em.persist(&flex).unwrap();
    f.em.persist(&ultra).unwrap();
    f.em.flush().await.unwrap();
    f.em.clear().unwrap();

    f.fix = Some(fix);
    f.flex = Some(flex);
    f.ultra = Some(ultra);
    f
}

async fn find(em: &EntityManager, class: &str, id: Value) -> Option<Object> {
    em.find(class, id).await.unwrap()
}

/// `assertInstanceOf($class, $object)`.
fn assert_instance_of(class: &str, object: &Object) {
    assert!(
        php_reflection::is_a(object.class(), class),
        "{} is not a {class}",
        object.class()
    );
}

/// `assertContainsOnly($class, $array)`.
fn assert_contains_only(class: &str, values: &[Object]) {
    for value in values {
        assert_instance_of(class, value);
    }
}

fn objects(value: &Value) -> Vec<Object> {
    list(value).iter().map(obj).collect()
}

async fn query_result(em: &Arc<EntityManager>, dql: &str) -> Vec<Object> {
    objects(
        &em.create_query(dql)
            .get_result(HYDRATE_OBJECT)
            .await
            .unwrap(),
    )
}

async fn execute(em: &Arc<EntityManager>, dql: &str) -> Value {
    em.create_query(dql).execute(None, None).await.unwrap()
}

fn criteria_eq(field: &str, value: impl Into<Value>) -> Criteria {
    let mut criteria = Criteria::create();
    criteria.r#where(Criteria::expr().eq(field, value.into()));
    criteria
}

#[tokio::test]
async fn testPersistChildOfBaseClass() {
    let f = persist_related_employees().await;

    let fix_contract = company_contract(COMPANY_FIX_CONTRACT);
    set(&fix_contract, "fixPrice", Value::Int(1000));
    set_obj(&fix_contract, "salesPerson", &f.sales_person);

    f.em.persist(&fix_contract).unwrap();
    f.em.flush().await.unwrap();
    f.em.clear().unwrap();

    let contract = find(&f.em, COMPANY_FIX_CONTRACT, get(&fix_contract, "id"))
        .await
        .unwrap();

    assert_instance_of(COMPANY_FIX_CONTRACT, &contract);
    assert_loose_eq(&Value::Int(1000), &get(&contract, "fixPrice"));
    assert_loose_eq(
        &get(&f.sales_person, "id"),
        &get(&get_object(&contract, "salesPerson").unwrap(), "id"),
    );
}

#[tokio::test]
async fn testPersistDeepChildOfBaseClass() {
    let f = persist_related_employees().await;

    let ultra_contract = company_contract(COMPANY_FLEX_ULTRA_CONTRACT);
    set_obj(&ultra_contract, "salesPerson", &f.sales_person);
    set(&ultra_contract, "hoursWorked", Value::Int(100));
    set(&ultra_contract, "pricePerHour", Value::Int(50));
    set(&ultra_contract, "maxPrice", Value::Int(7000));

    f.em.persist(&ultra_contract).unwrap();
    f.em.flush().await.unwrap();
    f.em.clear().unwrap();

    let contract = find(
        &f.em,
        COMPANY_FLEX_ULTRA_CONTRACT,
        get(&ultra_contract, "id"),
    )
    .await
    .unwrap();

    assert_instance_of(COMPANY_FLEX_ULTRA_CONTRACT, &contract);
    assert_loose_eq(&Value::Int(7000), &get(&contract, "maxPrice"));
    assert_loose_eq(&Value::Int(100), &get(&contract, "hoursWorked"));
    assert_loose_eq(&Value::Int(50), &get(&contract, "pricePerHour"));
}

#[tokio::test]
async fn testChildClassLifecycleUpdate() {
    let f = load_full_fixture().await;

    let fix = find(&f.em, COMPANY_CONTRACT, get(f.fix(), "id"))
        .await
        .unwrap();
    set(&fix, "fixPrice", Value::Int(2500));

    f.em.flush().await.unwrap();
    f.em.clear().unwrap();

    let new_fix = find(&f.em, COMPANY_CONTRACT, get(f.fix(), "id"))
        .await
        .unwrap();
    assert_loose_eq(&Value::Int(2500), &get(&new_fix, "fixPrice"));
}

#[tokio::test]
async fn testChildClassLifecycleRemove() {
    let f = load_full_fixture().await;

    let fix = find(&f.em, COMPANY_CONTRACT, get(f.fix(), "id"))
        .await
        .unwrap();
    f.em.remove(&fix).await.unwrap();
    f.em.flush().await.unwrap();

    assert!(
        find(&f.em, COMPANY_CONTRACT, get(f.fix(), "id"))
            .await
            .is_none()
    );
}

#[tokio::test]
async fn testFindAllForAbstractBaseClass() {
    let f = load_full_fixture().await;
    let contracts =
        f.em.get_repository(COMPANY_CONTRACT)
            .unwrap()
            .find_all()
            .await
            .unwrap();

    assert_eq!(contracts.len(), 3);
    assert_contains_only(COMPANY_CONTRACT, &contracts);
}

#[tokio::test]
async fn testFindAllForChildClass() {
    let f = load_full_fixture().await;

    for (class, count) in [
        (COMPANY_FIX_CONTRACT, 1),
        (COMPANY_FLEX_CONTRACT, 2),
        (COMPANY_FLEX_ULTRA_CONTRACT, 1),
    ] {
        let all =
            f.em.get_repository(class)
                .unwrap()
                .find_all()
                .await
                .unwrap();
        assert_eq!(all.len(), count, "{class}");
    }
}

#[tokio::test]
async fn testFindForAbstractBaseClass() {
    let f = load_full_fixture().await;

    let contract = find(&f.em, COMPANY_CONTRACT, get(f.fix(), "id"))
        .await
        .unwrap();

    assert_instance_of(COMPANY_FIX_CONTRACT, &contract);
    assert_loose_eq(&Value::Int(1000), &get(&contract, "fixPrice"));
}

#[tokio::test]
async fn testQueryForAbstractBaseClass() {
    let f = load_full_fixture().await;

    let contracts = query_result(
        &f.em,
        "SELECT c FROM Doctrine\\Tests\\Models\\Company\\CompanyContract c",
    )
    .await;

    assert_eq!(contracts.len(), 3);
    assert_contains_only(COMPANY_CONTRACT, &contracts);
}

#[tokio::test]
async fn testQueryForChildClass() {
    let f = load_full_fixture().await;

    assert_eq!(
        query_result(
            &f.em,
            "SELECT c FROM Doctrine\\Tests\\Models\\Company\\CompanyFixContract c"
        )
        .await
        .len(),
        1
    );
    assert_eq!(
        query_result(
            &f.em,
            "SELECT c FROM Doctrine\\Tests\\Models\\Company\\CompanyFlexContract c"
        )
        .await
        .len(),
        2
    );
    assert_eq!(
        query_result(
            &f.em,
            "SELECT c FROM Doctrine\\Tests\\Models\\Company\\CompanyFlexUltraContract c"
        )
        .await
        .len(),
        1
    );
}

#[tokio::test]
async fn testQueryBaseClassWithJoin() {
    let f = load_full_fixture().await;

    let contracts = query_result(
        &f.em,
        "SELECT c, p FROM Doctrine\\Tests\\Models\\Company\\CompanyContract c JOIN c.salesPerson p",
    )
    .await;
    assert_eq!(contracts.len(), 3);
    assert_contains_only(COMPANY_CONTRACT, &contracts);
}

#[tokio::test]
async fn testQueryScalarWithDiscriminatorValue() {
    let f = load_full_fixture().await;

    let contracts =
        f.em.create_query(
            "SELECT c FROM Doctrine\\Tests\\Models\\Company\\CompanyContract c ORDER BY c.id",
        )
        .get_scalar_result()
        .await
        .unwrap();

    let mut discr_values: Vec<String> = list(&contracts)
        .iter()
        .map(|a| match at(a, "c_discr") {
            Value::String(text) => String::from_utf8_lossy(text.as_bytes()).into_owned(),
            other => panic!("not a string: {other:?}"),
        })
        .collect();

    discr_values.sort();

    assert_eq!(discr_values, ["fix", "flexible", "flexultra"]);
}

#[tokio::test]
async fn testQueryChildClassWithCondition() {
    let f = load_full_fixture().await;

    let dql = "SELECT c FROM Doctrine\\Tests\\Models\\Company\\CompanyFixContract c WHERE c.fixPrice = ?1";
    let query = f.em.create_query(dql);
    query.set_parameter(1, Value::Int(1000), None);
    let contract = obj(&query.get_single_result(None).await.unwrap());

    assert_instance_of(COMPANY_FIX_CONTRACT, &contract);
    assert_loose_eq(&Value::Int(1000), &get(&contract, "fixPrice"));
}

#[tokio::test]
async fn testUpdateChildClassWithCondition() {
    let f = load_full_fixture().await;

    let dql = "UPDATE Doctrine\\Tests\\Models\\Company\\CompanyFlexContract c SET c.hoursWorked = c.hoursWorked * 2 WHERE c.hoursWorked = 150";
    let affected = execute(&f.em, dql).await;

    assert_loose_eq(&Value::Int(1), &affected);

    let flex_contract = find(&f.em, COMPANY_CONTRACT, get(f.flex(), "id"))
        .await
        .unwrap();
    let ultra_contract = find(&f.em, COMPANY_CONTRACT, get(f.ultra(), "id"))
        .await
        .unwrap();

    assert_loose_eq(&Value::Int(300), &get(&ultra_contract, "hoursWorked"));
    assert_loose_eq(&Value::Int(100), &get(&flex_contract, "hoursWorked"));
}

#[tokio::test]
async fn testUpdateBaseClassWithCondition() {
    let f = load_full_fixture().await;

    let dql = "UPDATE Doctrine\\Tests\\Models\\Company\\CompanyContract c SET c.completed = true WHERE c.completed = false";
    let affected = execute(&f.em, dql).await;

    assert_loose_eq(&Value::Int(1), &affected);

    let dql = "UPDATE Doctrine\\Tests\\Models\\Company\\CompanyContract c SET c.completed = false";
    let affected = execute(&f.em, dql).await;

    assert_loose_eq(&Value::Int(3), &affected);
}

#[tokio::test]
async fn testDeleteByChildClassCondition() {
    let f = load_full_fixture().await;

    let dql = "DELETE Doctrine\\Tests\\Models\\Company\\CompanyFlexContract c";
    let affected = execute(&f.em, dql).await;

    assert_loose_eq(&Value::Int(2), &affected);
}

#[tokio::test]
async fn testDeleteByBaseClassCondition() {
    let f = load_full_fixture().await;

    let dql = "DELETE Doctrine\\Tests\\Models\\Company\\CompanyContract c WHERE c.completed = true";
    let affected = execute(&f.em, dql).await;

    assert_loose_eq(&Value::Int(2), &affected);

    let contracts = query_result(
        &f.em,
        "SELECT c FROM Doctrine\\Tests\\Models\\Company\\CompanyContract c",
    )
    .await;
    assert_eq!(contracts.len(), 1);

    assert!(
        !get(&contracts[0], "completed").is_truthy(),
        "Only non completed contracts should be left."
    );
}

#[tokio::test]
async fn testDeleteJoinTableRecords() {
    let f = load_full_fixture().await;

    // remove managed copy of the fix contract
    let class = f.fix().class().to_owned();
    let fix = find(&f.em, &class, get(f.fix(), "id")).await.unwrap();
    f.em.remove(&fix).await.unwrap();
    f.em.flush().await.unwrap();

    assert!(
        find(&f.em, &class, get(f.fix(), "id")).await.is_none(),
        "Contract should not be present in the database anymore."
    );
}

#[tokio::test]
async fn testFindByAssociation() {
    let f = load_full_fixture().await;
    let sales_person_id = get(&f.sales_person, "id");

    for (class, count) in [
        (COMPANY_CONTRACT, 3),
        (COMPANY_FIX_CONTRACT, 1),
        (COMPANY_FLEX_CONTRACT, 2),
        (COMPANY_FLEX_ULTRA_CONTRACT, 1),
    ] {
        let repos = f.em.get_repository(class).unwrap();
        let mut criteria = PhpArray::new();
        criteria.insert("salesPerson", sales_person_id.clone());
        let contracts = repos.find_by(&criteria, None, None, None).await.unwrap();
        assert_eq!(
            contracts.len(),
            count,
            "There should be {count} entities related to {} for '{class}'",
            sales_person_id.to_php_string().unwrap().to_string_lossy()
        );
    }
}

#[tokio::test]
async fn testInheritanceMatching() {
    let f = load_full_fixture().await;

    let repository = f.em.get_repository(COMPANY_CONTRACT).unwrap();
    let contracts = repository
        .matching(&criteria_eq(
            "salesPerson",
            Value::Object(f.sales_person.clone()),
        ))
        .unwrap()
        .value();
    assert_eq!(coll::count(&contracts).await.unwrap(), 3);

    let repository = f.em.get_repository(COMPANY_FIX_CONTRACT).unwrap();
    let contracts = repository
        .matching(&criteria_eq(
            "salesPerson",
            Value::Object(f.sales_person.clone()),
        ))
        .unwrap()
        .value();
    assert_eq!(coll::count(&contracts).await.unwrap(), 1);
}

#[tokio::test]
async fn testMatchingNonObjectOnAssocationThrowsException() {
    let f = load_full_fixture().await;

    let repository = f.em.get_repository(COMPANY_CONTRACT).unwrap();

    let contracts = repository
        .matching(&criteria_eq("salesPerson", get(&f.sales_person, "id")))
        .unwrap()
        .value();

    // Load the association because it's wrapped in a lazy collection
    let e = coll::to_array(&contracts).await.unwrap_err();
    assert_eq!(
        e.class,
        "Doctrine\\ORM\\Persisters\\MatchingAssociationFieldRequiresObject"
    );
    assert!(message(&e).contains(
        "annot match on Doctrine\\Tests\\Models\\Company\\CompanyContract::salesPerson with a non-object value."
    ));
}

#[tokio::test]
async fn testGetReferenceEntityWithSubclasses() {
    let f = load_full_fixture().await;

    let reference =
        f.em.get_reference(COMPANY_CONTRACT, get(f.fix(), "id"))
            .await
            .unwrap()
            .unwrap();
    assert!(
        !f.em
            .is_uninitialized_object(&Value::Object(reference.clone())),
        "Cannot Request a proxy from a class that has subclasses."
    );
    assert_instance_of(COMPANY_CONTRACT, &reference);
    assert!(
        php_reflection::is_a(reference.class(), COMPANY_FIX_CONTRACT),
        "Direct fetch of the reference has to load the child class Employee directly."
    );
    f.em.clear().unwrap();

    let reference =
        f.em.get_reference(COMPANY_FIX_CONTRACT, get(f.fix(), "id"))
            .await
            .unwrap()
            .unwrap();

    assert!(
        f.em.is_uninitialized_object(&Value::Object(reference)),
        "A proxy can be generated only if no subclasses exists for the requested reference."
    );
}

#[tokio::test]
async fn testEagerLoadInheritanceHierarchy() {
    let f = load_full_fixture().await;

    let dql =
        "SELECT f FROM Doctrine\\Tests\\Models\\Company\\CompanyFixContract f WHERE f.id = ?1";
    let query = f.em.create_query(dql);
    query.set_fetch_mode(COMPANY_FIX_CONTRACT, "salesPerson", FETCH_EAGER);
    query.set_parameter(1, get(f.fix(), "id"), None);
    let contract = obj(&query.get_single_result(None).await.unwrap());

    assert!(!f.em.is_uninitialized_object(&get(&contract, "salesPerson")));
}
