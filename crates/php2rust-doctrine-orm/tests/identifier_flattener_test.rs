//! Port of tests/Tests/ORM/Utility/IdentifierFlattenerTest.php, on
//! `SQLite` with the recorded models. `new IdentifierFlattener($uow,
//! $metadataFactory)->flattenIdentifier()` is `flatten_identifier()`
//! over the entity manager.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Utility/IdentifierFlattenerTest.php

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

use php_value::datetime::{DateTimeImmutable, TimeZone};
use php_value::{Key, Value, php_array};
use php2rust_doctrine_orm::collection_object::ArrayCollectionObject;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::utility::flatten_identifier;

use orm::{get, new_object, s, same_value, set, set_obj};

const FIRST_RELATED_ENTITY: &str = "Doctrine\\Tests\\Models\\VersionedOneToOne\\FirstRelatedEntity";
const SECOND_RELATED_ENTITY: &str =
    "Doctrine\\Tests\\Models\\VersionedOneToOne\\SecondRelatedEntity";
const FLIGHT: &str = "Doctrine\\Tests\\Models\\Cache\\Flight";
const CITY: &str = "Doctrine\\Tests\\Models\\Cache\\City";

async fn set_up() -> Arc<EntityManager> {
    orm::set_up(&[FIRST_RELATED_ENTITY, SECOND_RELATED_ENTITY, FLIGHT, CITY]).await
}

/// `new City($name)`.
fn city(name: &str) -> php_value::Object {
    let city = new_object(CITY);
    set(&city, "name", s(name));
    set(&city, "state", Value::Null);
    set(
        &city,
        "travels",
        ArrayCollectionObject::value(php_value::PhpArray::new()),
    );
    set(
        &city,
        "attractions",
        ArrayCollectionObject::value(php_value::PhpArray::new()),
    );
    city
}

fn key(name: &str) -> Key {
    Key::from(name)
}

#[tokio::test]
async fn testFlattenIdentifierWithOneToOneId() {
    let em = set_up().await;
    let second_related_entity = new_object(SECOND_RELATED_ENTITY);
    set(&second_related_entity, "name", s("Bob"));

    em.persist(&second_related_entity).unwrap();
    em.flush().await.unwrap();

    let first_related_entity = new_object(FIRST_RELATED_ENTITY);
    set(&first_related_entity, "name", s("Fred"));
    set_obj(
        &first_related_entity,
        "secondEntity",
        &second_related_entity,
    );

    em.persist(&first_related_entity).unwrap();
    em.flush().await.unwrap();

    let first_entity = em
        .get_repository(FIRST_RELATED_ENTITY)
        .unwrap()
        .find_one_by(&php_array!["name" => "Fred"], None)
        .await
        .unwrap()
        .unwrap();

    let class = em.get_class_metadata(FIRST_RELATED_ENTITY).unwrap();

    let id = class.read().get_identifier_values(&first_entity).unwrap();

    assert_eq!(id.len(), 1, "We should have 1 identifier");

    assert!(
        id.contains_key(&key("secondEntity")),
        "It should be called secondEntity"
    );

    let second = id.get(&key("secondEntity")).unwrap().clone();
    assert!(
        matches!(&second, Value::Object(o) if o.instance_of(SECOND_RELATED_ENTITY)),
        "The entity should be an instance of SecondRelatedEntity"
    );

    let flat_ids = flatten_identifier(&em, &class.read(), &id).unwrap();

    assert_eq!(flat_ids.len(), 1, "We should have 1 flattened id");

    assert!(
        flat_ids.contains_key(&key("secondEntity")),
        "It should be called secondEntity"
    );

    let Value::Object(second) = second else {
        unreachable!()
    };
    assert!(php_value::compare::loose_equals(
        &get(&second, "id"),
        flat_ids.get(&key("secondEntity")).unwrap()
    ));
}

#[tokio::test]
async fn testFlattenIdentifierWithMutlipleIds() {
    let em = set_up().await;
    let leeds = city("Leeds");
    let london = city("London");

    em.persist(&leeds).unwrap();
    em.persist(&london).unwrap();
    em.flush().await.unwrap();

    // `new Flight($leeds, $london)`.
    let flight = new_object(FLIGHT);
    set_obj(&flight, "leavingFrom", &leeds);
    set_obj(&flight, "goingTo", &london);
    set(
        &flight,
        "departure",
        php_value::datetime::DateTime::object(DateTimeImmutable::now(TimeZone::Utc)),
    );

    em.persist(&flight).unwrap();
    em.flush().await.unwrap();

    let class = em.get_class_metadata(FLIGHT).unwrap();
    let id = class.read().get_identifier_values(&flight).unwrap();

    assert_eq!(id.len(), 2);

    assert!(id.contains_key(&key("leavingFrom")));
    assert!(id.contains_key(&key("goingTo")));

    assert!(same_value(id.get(&key("leavingFrom")).unwrap(), &leeds));
    assert!(same_value(id.get(&key("goingTo")).unwrap(), &london));

    let flat_ids = flatten_identifier(&em, &class.read(), &id).unwrap();

    assert_eq!(flat_ids.len(), 2);

    assert!(flat_ids.contains_key(&key("leavingFrom")));
    assert!(flat_ids.contains_key(&key("goingTo")));

    assert!(php_value::compare::loose_equals(
        &get(&leeds, "id"),
        flat_ids.get(&key("leavingFrom")).unwrap()
    ));
    assert!(php_value::compare::loose_equals(
        &get(&london, "id"),
        flat_ids.get(&key("goingTo")).unwrap()
    ));
}
