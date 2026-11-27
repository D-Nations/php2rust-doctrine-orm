//! Port of `CompositePrimaryKeyWithAssociationsTest`, on `SQLite` with the
//! `geonames` model set. The models' constructors are written here, on
//! their recorded classes.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/CompositePrimaryKeyWithAssociationsTest.php

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::similar_names,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;

use std::sync::Arc;

use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::{append, assert_loose_eq, get, new_object, s, set};

const NS: &str = "Doctrine\\Tests\\Models\\GeoNames\\";

fn class(short: &str) -> String {
    format!("{NS}{short}")
}

/// `new Country($id, $name)`.
fn country(id: &str, name: &str) -> Object {
    let country = new_object(&class("Country"));
    set(&country, "id", s(id));
    set(&country, "name", s(name));
    country
}

/// `new Admin1($id, $name, $country)`.
fn admin1(id: i64, name: &str, country: &Object) -> Object {
    let admin1 = new_object(&class("Admin1"));
    set(&admin1, "id", Value::Int(id));
    set(&admin1, "name", s(name));
    set(&admin1, "country", Value::Object(country.clone()));
    admin1
}

/// `new Admin1AlternateName($id, $name, $admin1)`.
fn admin1_alternate_name(id: i64, name: &str, admin1: &Object) -> Object {
    let alternate = new_object(&class("Admin1AlternateName"));
    set(&alternate, "name", s(name));
    set(&alternate, "admin1", Value::Object(admin1.clone()));
    set(&alternate, "id", s(&id.to_string()));
    alternate
}

/// `setUp()`.
async fn set_up() -> Arc<EntityManager> {
    let em = orm::set_up(&[
        &class("Country"),
        &class("Admin1"),
        &class("Admin1AlternateName"),
        &class("City"),
    ])
    .await;

    let it = country("IT", "Italy");

    em.persist(&it).unwrap();
    em.flush().await.unwrap();

    let admin1 = admin1(1, "Rome", &it);

    em.persist(&admin1).unwrap();
    em.flush().await.unwrap();

    let name1 = admin1_alternate_name(1, "Roma", &admin1);
    let name2 = admin1_alternate_name(2, "Rome", &admin1);

    append(&admin1, "names", Value::Object(name1.clone()));
    append(&admin1, "names", Value::Object(name2.clone()));

    em.persist(&admin1).unwrap();
    em.persist(&name1).unwrap();
    em.persist(&name2).unwrap();

    em.flush().await.unwrap();

    em.clear().unwrap();
    em
}

fn criteria(entries: Vec<(&str, Value)>) -> PhpArray {
    let mut criteria = PhpArray::new();
    for (key, value) in entries {
        criteria.insert(key, value);
    }
    criteria
}

#[tokio::test]
async fn testFindByAbleToGetCompositeEntitiesWithMixedTypeIdentifiers() {
    let em = set_up().await;
    let admin1_repo = em.get_repository(&class("Admin1")).unwrap();
    let admin1_names_repo = em.get_repository(&class("Admin1AlternateName")).unwrap();

    let admin1_rome = admin1_repo
        .find_one_by(
            &criteria(vec![("country", s("IT")), ("id", Value::Int(1))]),
            None,
        )
        .await
        .unwrap()
        .unwrap();

    let names = admin1_names_repo
        .find_by(
            &criteria(vec![("admin1", Value::Object(admin1_rome.clone()))]),
            None,
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(names.len(), 2);

    let name1 = admin1_names_repo
        .find_one_by(
            &criteria(vec![
                ("admin1", Value::Object(admin1_rome.clone())),
                ("id", Value::Int(1)),
            ]),
            None,
        )
        .await
        .unwrap()
        .unwrap();
    let name2 = admin1_names_repo
        .find_one_by(
            &criteria(vec![
                ("admin1", Value::Object(admin1_rome.clone())),
                ("id", Value::Int(2)),
            ]),
            None,
        )
        .await
        .unwrap()
        .unwrap();

    assert_loose_eq(&Value::Int(1), &get(&name1, "id"));
    assert_loose_eq(&s("Roma"), &get(&name1, "name"));

    assert_loose_eq(&Value::Int(2), &get(&name2, "id"));
    assert_loose_eq(&s("Rome"), &get(&name2, "name"));
}
