//! Upstream: doctrine/orm tests/Tests/ORM/Cache/DefaultEntityHydratorTest.php
//!
//! `assertEquals()` of the entry data compares each value: an
//! `AssociationCacheEntry` by its class and identifier.

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

use php_value::{Key, PhpArray, Value};
use php2rust_doctrine_orm::cache::{
    AssociationCacheEntry, DefaultEntityHydrator, EntityCacheEntry, EntityCacheKey, EntityHydrator,
};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::unit_of_work::STATE_MANAGED;

const COUNTRY: &str = "Doctrine\\Tests\\Models\\Cache\\Country";
const STATE: &str = "Doctrine\\Tests\\Models\\Cache\\State";

struct Fixture {
    em: Arc<EntityManager>,
    structure: DefaultEntityHydrator,
}

/// `setUp()`.
fn set_up() -> Fixture {
    let em = orm::entity_manager(orm::connection(), None);
    let structure = DefaultEntityHydrator::new(&em);
    Fixture { em, structure }
}

fn array(values: &[(&str, Value)]) -> PhpArray {
    values
        .iter()
        .map(|(key, value)| (*key, value.clone()))
        .collect()
}

fn id(id: i64) -> PhpArray {
    array(&[("id", Value::Int(id))])
}

fn country(name: &str) -> php_value::Object {
    let country = orm::new_object(COUNTRY);
    orm::set(&country, "name", Value::string(name));
    country
}

/// `new State($name, $country)`.
fn state(name: &str, country: &php_value::Object) -> php_value::Object {
    let state = orm::new_object(STATE);
    orm::set(&state, "name", Value::string(name));
    orm::set(&state, "country", Value::Object(country.clone()));
    state
}

/// `assertEquals(['id' => $id, 'name' => $name, 'country' => new
/// AssociationCacheEntry(Country::class, ['id' => 11])], $data)`.
fn assert_state_data(id: i64, name: &str, data: &PhpArray) {
    assert_eq!(
        vec![Key::from("id"), Key::from("name"), Key::from("country")],
        data.keys().cloned().collect::<Vec<_>>()
    );
    assert_eq!(Some(&Value::Int(id)), data.get(&Key::from("id")));
    assert_eq!(Some(&Value::string(name)), data.get(&Key::from("name")));
    assert_eq!(
        Some(&AssociationCacheEntry::new(COUNTRY, self::id(11))),
        data.get(&Key::from("country"))
            .and_then(AssociationCacheEntry::from_value)
    );
}

#[test]
fn testImplementsEntityEntryStructure() {
    let f = set_up();
    let _: &dyn EntityHydrator = &f.structure;
}

#[tokio::test]
async fn testCreateEntity() {
    let f = set_up();
    let metadata = f.em.get_class_metadata(COUNTRY).unwrap();
    let key = EntityCacheKey::new(COUNTRY, &id(1));
    let entry = EntityCacheEntry::new(
        COUNTRY,
        array(&[("id", Value::Int(1)), ("name", Value::string("Foo"))]),
    );
    let entity = f
        .structure
        .load_cache_entry(&metadata, &key, &entry, None)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(COUNTRY, entity.class());

    assert_eq!(Value::Int(1), orm::get(&entity, "id"));
    assert_eq!(Value::string("Foo"), orm::get(&entity, "name"));
    assert_eq!(
        STATE_MANAGED,
        f.em.get_unit_of_work()
            .get_entity_state(&entity)
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn testLoadProxy() {
    let f = set_up();
    let metadata = f.em.get_class_metadata(COUNTRY).unwrap();
    let key = EntityCacheKey::new(COUNTRY, &id(1));
    let entry = EntityCacheEntry::new(
        COUNTRY,
        array(&[("id", Value::Int(1)), ("name", Value::string("Foo"))]),
    );
    let proxy =
        f.em.get_reference(COUNTRY, Value::Array(key.identifier.clone()))
            .await
            .unwrap()
            .unwrap();
    let entity = f
        .structure
        .load_cache_entry(&metadata, &key, &entry, Some(&proxy))
        .await
        .unwrap()
        .unwrap();

    assert_eq!(COUNTRY, entity.class());
    assert!(Arc::ptr_eq(&proxy, &entity));

    assert_eq!(Value::Int(1), orm::get(&entity, "id"));
    assert_eq!(Value::string("Foo"), orm::get(&entity, "name"));
    assert_eq!(
        STATE_MANAGED,
        f.em.get_unit_of_work()
            .get_entity_state(&proxy)
            .await
            .unwrap()
    );
}

#[test]
fn testBuildCacheEntry() {
    let f = set_up();
    let entity = country("Foo");
    let uow = f.em.get_unit_of_work();
    let data = array(&[("id", Value::Int(1)), ("name", Value::string("Foo"))]);
    let metadata = f.em.get_class_metadata(COUNTRY).unwrap();
    let key = EntityCacheKey::new(COUNTRY, &id(1));

    orm::set(&entity, "id", 1_i64);
    uow.register_managed(&entity, key.identifier.clone(), data)
        .unwrap();

    let cache = f
        .structure
        .build_cache_entry(&metadata, &key, &entity)
        .unwrap();

    assert!(cache.data.contains_key(&Key::from("id")));
    assert!(cache.data.contains_key(&Key::from("name")));
    assert_eq!(
        array(&[("id", Value::Int(1)), ("name", Value::string("Foo"))]),
        cache.data
    );
}

#[test]
fn testBuildCacheEntryAssociation() {
    let f = set_up();
    let country = country("Foo");
    let state = state("Bat", &country);
    let uow = f.em.get_unit_of_work();
    let country_data = array(&[("id", Value::Int(11)), ("name", Value::string("Foo"))]);
    let state_data = array(&[
        ("id", Value::Int(12)),
        ("name", Value::string("Bar")),
        ("country", Value::Object(country.clone())),
    ]);
    let metadata = f.em.get_class_metadata(STATE).unwrap();
    let key = EntityCacheKey::new(STATE, &id(11));

    orm::set(&country, "id", 11_i64);
    orm::set(&state, "id", 12_i64);

    uow.register_managed(&country, id(11), country_data)
        .unwrap();
    uow.register_managed(&state, id(12), state_data).unwrap();

    let cache = f
        .structure
        .build_cache_entry(&metadata, &key, &state)
        .unwrap();

    assert_state_data(12, "Bar", &cache.data);
}

#[tokio::test]
async fn testBuildCacheEntryNonInitializedAssocProxy() {
    let f = set_up();
    let proxy =
        f.em.get_reference(COUNTRY, Value::Int(11))
            .await
            .unwrap()
            .unwrap();
    let entity = state("Bat", &proxy);
    let uow = f.em.get_unit_of_work();
    let entity_data = array(&[
        ("id", Value::Int(12)),
        ("name", Value::string("Bar")),
        ("country", Value::Object(proxy.clone())),
    ]);
    let metadata = f.em.get_class_metadata(STATE).unwrap();
    let key = EntityCacheKey::new(STATE, &id(11));

    orm::set(&entity, "id", 12_i64);

    uow.register_managed(&entity, id(12), entity_data).unwrap();

    let cache = f
        .structure
        .build_cache_entry(&metadata, &key, &entity)
        .unwrap();

    assert_state_data(12, "Bar", &cache.data);
}

#[tokio::test]
async fn testCacheEntryWithWrongIdentifierType() {
    let f = set_up();
    let proxy =
        f.em.get_reference(COUNTRY, Value::Int(11))
            .await
            .unwrap()
            .unwrap();
    let entity = state("Bat", &proxy);
    let uow = f.em.get_unit_of_work();
    let entity_data = array(&[
        ("id", Value::Int(12)),
        ("name", Value::string("Bar")),
        ("country", Value::Object(proxy.clone())),
    ]);
    let metadata = f.em.get_class_metadata(STATE).unwrap();
    let key = EntityCacheKey::new(STATE, &array(&[("id", Value::string("12"))]));

    orm::set(&entity, "id", 12_i64);

    uow.register_managed(&entity, id(12), entity_data).unwrap();

    let cache = f
        .structure
        .build_cache_entry(&metadata, &key, &entity)
        .unwrap();

    assert_eq!(
        Some(&orm::get(&entity, "id")),
        cache.data.get(&Key::from("id"))
    );
    assert_state_data(12, "Bar", &cache.data);
}
