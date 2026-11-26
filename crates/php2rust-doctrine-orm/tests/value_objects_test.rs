//! ValueObjectsTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite`: embeddables. The test models declared next to the test
//! are recorded in `tests/data/models.json`; their constructors are the
//! functions below. `assertEquals()` on entities from the identity map
//! is `same()`. The data provider's rows run in one test, labelled.
//! `testPartialDqlOnEmbeddedObjectsField` is pending: a partial object is
//! not a lazy ghost of its selected fields.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/ValueObjectsTest.php

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

use std::sync::Arc;

use php_value::datetime::{DateTime, DateTimeImmutable, TimeZone};
use php_value::{Object, Value};
use php2rust_doctrine_orm::abstract_query::HYDRATE_OBJECT;
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::*;

const NS: &str = "Doctrine\\Tests\\ORM\\Functional";
const DDC93_PERSON: &str = "Doctrine\\Tests\\ORM\\Functional\\DDC93Person";
const DDC93_ADDRESS: &str = "Doctrine\\Tests\\ORM\\Functional\\DDC93Address";
const DDC93_VEHICLE: &str = "Doctrine\\Tests\\ORM\\Functional\\DDC93Vehicle";
const DDC93_CAR: &str = "Doctrine\\Tests\\ORM\\Functional\\DDC93Car";
const DDC93_COUNTRY: &str = "Doctrine\\Tests\\ORM\\Functional\\DDC93Country";
const DDC93_TIMESTAMPS: &str = "Doctrine\\Tests\\ORM\\Functional\\DDC93Timestamps";
const DDC3027_ANIMAL: &str = "Doctrine\\Tests\\ORM\\Functional\\DDC3027Animal";
const DDC3027_DOG: &str = "Doctrine\\Tests\\ORM\\Functional\\DDC3027Dog";
const DDC3028_PERSON_WITH_PREFIX: &str =
    "Doctrine\\Tests\\ORM\\Functional\\DDC3028PersonWithPrefix";
const DDC3028_PERSON_EMPTY_PREFIX: &str =
    "Doctrine\\Tests\\ORM\\Functional\\DDC3028PersonEmptyPrefix";
const DDC3028_PERSON_PREFIX_FALSE: &str =
    "Doctrine\\Tests\\ORM\\Functional\\DDC3028PersonPrefixFalse";

async fn set_up() -> Arc<EntityManager> {
    orm::set_up(&[
        DDC93_PERSON,
        DDC93_ADDRESS,
        DDC93_VEHICLE,
        DDC93_CAR,
        DDC3027_ANIMAL,
        DDC3027_DOG,
    ])
    .await
}

/// `new DDC93Country($name)`.
fn country(name: &str) -> Object {
    let country = new_object(DDC93_COUNTRY);
    set(&country, "name", s(name));
    country
}

/// `new DDC93Address($street, $zip, $city, $country)`.
fn address(street: &str, zip: &str, city: &str, country: Option<Object>) -> Object {
    let address = new_object(DDC93_ADDRESS);
    set(&address, "street", s(street));
    set(&address, "zip", s(zip));
    set(&address, "city", s(city));
    set(
        &address,
        "country",
        country.map_or(Value::Null, Value::Object),
    );
    address
}

/// `new DDC93Person($name, $address)`.
fn person(name: Option<&str>, address: Option<Object>) -> Object {
    let person = new_object(DDC93_PERSON);
    set(&person, "name", name.map_or(Value::Null, s));
    set(
        &person,
        "address",
        address.map_or(Value::Null, Value::Object),
    );
    // new DDC93Timestamps(new DateTime())
    let timestamps = new_object(DDC93_TIMESTAMPS);
    set(
        &timestamps,
        "createdAt",
        DateTime::object(DateTimeImmutable::now(TimeZone::Utc)),
    );
    set(&person, "timestamps", Value::Object(timestamps));
    person
}

/// `$person->address->$field`.
fn address_of(person: &Object) -> Object {
    get_object(person, "address").unwrap()
}

fn country_name(person: &Object) -> Value {
    get(&get_object(&address_of(person), "country").unwrap(), "name")
}

#[tokio::test]
async fn testCRUD() {
    let em = set_up().await;
    let person = person(None, None);
    set(&person, "name", s("Tara"));
    let new_address = new_object(DDC93_ADDRESS);
    set(&person, "address", Value::Object(new_address.clone()));
    set(&new_address, "street", s("United States of Tara Street"));
    set(&new_address, "zip", s("12345"));
    set(&new_address, "city", s("funkytown"));
    set(&new_address, "country", Value::Object(country("Germany")));

    // 1. check saving value objects works
    em.persist(&person).unwrap();
    em.flush().await.unwrap();

    em.clear().unwrap();

    // 2. check loading value objects works
    let person = em
        .find(DDC93_PERSON, get(&person, "id"))
        .await
        .unwrap()
        .unwrap();

    let address = address_of(&person);
    assert_eq!(DDC93_ADDRESS, address.class());
    assert_eq!(s("United States of Tara Street"), get(&address, "street"));
    assert_eq!(s("12345"), get(&address, "zip"));
    assert_eq!(s("funkytown"), get(&address, "city"));
    assert_eq!(
        DDC93_COUNTRY,
        get_object(&address, "country").unwrap().class()
    );
    assert_eq!(s("Germany"), country_name(&person));

    // 3. check changing value objects works
    set(&address, "street", s("Street"));
    set(&address, "zip", s("54321"));
    set(&address, "city", s("another town"));
    set(
        &get_object(&address, "country").unwrap(),
        "name",
        s("United States of America"),
    );
    em.flush().await.unwrap();

    em.clear().unwrap();

    let person = em
        .find(DDC93_PERSON, get(&person, "id"))
        .await
        .unwrap()
        .unwrap();

    let address = address_of(&person);
    assert_eq!(s("Street"), get(&address, "street"));
    assert_eq!(s("54321"), get(&address, "zip"));
    assert_eq!(s("another town"), get(&address, "city"));
    assert_eq!(s("United States of America"), country_name(&person));

    // 4. check deleting works
    let person_id = get(&person, "id");

    em.remove(&person).await.unwrap();
    em.flush().await.unwrap();

    assert!(em.find(DDC93_PERSON, person_id).await.unwrap().is_none());
}

#[tokio::test]
async fn testLoadDql() {
    let em = set_up().await;
    for i in 0..3 {
        let person = person(None, None);
        set(&person, "name", s(&format!("Donkey Kong{i}")));
        let address = new_object(DDC93_ADDRESS);
        set(&person, "address", Value::Object(address.clone()));
        set(&address, "street", s("Tree"));
        set(&address, "zip", s("12345"));
        set(&address, "city", s("funkytown"));
        set(
            &address,
            "country",
            Value::Object(country("United States of America")),
        );

        em.persist(&person).unwrap();
    }

    em.flush().await.unwrap();
    em.clear().unwrap();

    let dql = format!("SELECT p FROM {NS}\\DDC93Person p");
    let persons = list(
        &em.create_query(&dql)
            .get_result(HYDRATE_OBJECT)
            .await
            .unwrap(),
    );

    assert_eq!(3, persons.len());
    for person in &persons {
        let person = obj(person);
        let address = address_of(&person);
        assert_eq!(DDC93_ADDRESS, address.class());
        assert_eq!(s("Tree"), get(&address, "street"));
        assert_eq!(s("12345"), get(&address, "zip"));
        assert_eq!(s("funkytown"), get(&address, "city"));
        assert_eq!(
            DDC93_COUNTRY,
            get_object(&address, "country").unwrap().class()
        );
        assert_eq!(s("United States of America"), country_name(&person));
    }

    let persons = list(&em.create_query(&dql).get_array_result().await.unwrap());

    for person in &persons {
        assert_eq!(s("Tree"), at(person, "address.street"));
        assert_eq!(s("12345"), at(person, "address.zip"));
        assert_eq!(s("funkytown"), at(person, "address.city"));
        assert_eq!(
            s("United States of America"),
            at(person, "address.country.name")
        );
    }
}

#[tokio::test]
async fn testDqlOnEmbeddedObjectsField() {
    let em = set_up().await;
    let person = person(
        Some("Johannes"),
        Some(address(
            "Moo",
            "12345",
            "Karlsruhe",
            Some(country("Germany")),
        )),
    );
    em.persist(&person).unwrap();
    em.flush().await.unwrap();

    // SELECT
    let select_dql = format!(
        "SELECT p FROM {NS}\\DDC93Person p WHERE p.address.city = :city AND p.address.country.name = :country"
    );
    let query = em.create_query(&select_dql);
    query.set_parameter("city", s("Karlsruhe"), None);
    query.set_parameter("country", s("Germany"), None);
    let loaded_person = obj(&query.get_single_result(None).await.unwrap());
    assert!(same(&person, &loaded_person));

    let query = em.create_query(&select_dql);
    query.set_parameter("city", s("asdf"), None);
    query.set_parameter("country", s("Germany"), None);
    assert!(query.get_one_or_null_result(None).await.unwrap().is_null());

    // UPDATE
    let update_dql = format!(
        "UPDATE {NS}\\DDC93Person p SET p.address.street = :street, p.address.country.name = :country WHERE p.address.city = :city"
    );
    let query = em.create_query(&update_dql);
    query.set_parameter("street", s("Boo"), None);
    query.set_parameter("country", s("DE"), None);
    query.set_parameter("city", s("Karlsruhe"), None);
    query.execute(None, None).await.unwrap();

    em.refresh(&person).await.unwrap();
    assert_eq!(s("Boo"), get(&address_of(&person), "street"));
    assert_eq!(s("DE"), country_name(&person));

    // DELETE
    let query = em.create_query(&format!(
        "DELETE {NS}\\DDC93Person p WHERE p.address.city = :city AND p.address.country.name = :country"
    ));
    query.set_parameter("city", s("Karlsruhe"), None);
    query.set_parameter("country", s("DE"), None);
    query.execute(None, None).await.unwrap();

    em.clear().unwrap();
    assert!(
        em.find(DDC93_PERSON, get(&person, "id"))
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn testDqlWithNonExistentEmbeddableField() {
    let em = set_up().await;
    let e = em
        .create_query(&format!(
            "SELECT p FROM {NS}\\DDC93Person p WHERE p.address.asdfasdf IS NULL"
        ))
        .execute(None, None)
        .await
        .unwrap_err();

    assert_eq!("Doctrine\\ORM\\Query\\QueryException", e.class);
    assert!(
        e.message
            .to_string()
            .contains("no field or association named address.asdfasdf"),
        "{}",
        e.message
    );
}

#[tokio::test]
async fn testPartialDqlWithNonExistentEmbeddableField() {
    let em = set_up().await;
    let e = em
        .create_query(&format!(
            "SELECT PARTIAL p.{{id,address.asdfasdf}} FROM {NS}\\DDC93Person p"
        ))
        .execute(None, None)
        .await
        .unwrap_err();

    assert_eq!("Doctrine\\ORM\\Query\\QueryException", e.class);
    assert!(
        e.message
            .to_string()
            .contains("no mapped field named 'address.asdfasdf'"),
        "{}",
        e.message
    );
}

#[tokio::test]
async fn testEmbeddableWithInheritance() {
    let em = set_up().await;
    let car = new_object(DDC93_CAR);
    set(
        &car,
        "address",
        Value::Object(address("Foo", "12345", "Asdf", None)),
    );
    em.persist(&car).unwrap();
    em.flush().await.unwrap();

    let reloaded_car = em.find(DDC93_CAR, get(&car, "id")).await.unwrap().unwrap();
    assert!(same(&car, &reloaded_car));
}

#[tokio::test]
async fn testInlineEmbeddableWithPrefix() {
    let em = set_up().await;
    let metadata = em.get_class_metadata(DDC3028_PERSON_WITH_PREFIX).unwrap();
    let metadata = metadata.read();

    assert_eq!("foobar_id", metadata.get_column_name("id.id"));
    assert_eq!(
        "bloo_foo_id",
        metadata.get_column_name("nested.nestedWithPrefix.id")
    );
    assert_eq!(
        "bloo_nestedWithEmptyPrefix_id",
        metadata.get_column_name("nested.nestedWithEmptyPrefix.id")
    );
    assert_eq!(
        "bloo_id",
        metadata.get_column_name("nested.nestedWithPrefixFalse.id")
    );
}

#[tokio::test]
async fn testInlineEmbeddableEmptyPrefix() {
    let em = set_up().await;
    let metadata = em.get_class_metadata(DDC3028_PERSON_EMPTY_PREFIX).unwrap();
    let metadata = metadata.read();

    assert_eq!("id_id", metadata.get_column_name("id.id"));
    assert_eq!(
        "nested_foo_id",
        metadata.get_column_name("nested.nestedWithPrefix.id")
    );
    assert_eq!(
        "nested_nestedWithEmptyPrefix_id",
        metadata.get_column_name("nested.nestedWithEmptyPrefix.id")
    );
    assert_eq!(
        "nested_id",
        metadata.get_column_name("nested.nestedWithPrefixFalse.id")
    );
}

#[tokio::test]
async fn testInlineEmbeddablePrefixFalse() {
    let em = set_up().await;
    let expected_column_name = "id";

    let actual_column_name = em
        .get_class_metadata(DDC3028_PERSON_PREFIX_FALSE)
        .unwrap()
        .read()
        .get_column_name("id.id");

    assert_eq!(expected_column_name, actual_column_name);
}

#[tokio::test]
async fn testInlineEmbeddableInMappedSuperClass() {
    let em = set_up().await;
    let is_field_mapped = em
        .get_class_metadata(DDC3027_DOG)
        .unwrap()
        .read()
        .has_field("address.street");

    assert!(is_field_mapped);
}

#[tokio::test]
async fn testThrowsExceptionOnInfiniteEmbeddableNesting() {
    for (embeddable_class_name, declared_embeddable_class_name) in [
        (
            "DDCInfiniteNestingEmbeddable",
            "DDCInfiniteNestingEmbeddable",
        ),
        ("DDCNestingEmbeddable1", "DDCNestingEmbeddable4"),
    ] {
        let em = set_up().await;
        // createSchemaForModels(): the metadata of the class.
        let e = em
            .get_class_metadata(&format!("{NS}\\{embeddable_class_name}"))
            .unwrap_err();

        assert_eq!(
            "Doctrine\\ORM\\Mapping\\MappingException", e.class,
            "{embeddable_class_name}"
        );
        assert_eq!(
            format!(
                "Infinite nesting detected for embedded property {NS}\\{declared_embeddable_class_name}::nested. You cannot embed an embeddable from the same type inside an embeddable."
            ),
            e.message.to_string(),
            "{embeddable_class_name}"
        );
    }
}
