//! Upstream: doctrine/orm tests/Tests/ORM/Functional/SecondLevelCacheCompositePrimaryKeyWithAssociationsTest.php
//! Upstream: doctrine/orm tests/Tests/Models/GeoNames/Country.php tests/Tests/Models/GeoNames/Admin1.php
//! Upstream: doctrine/orm tests/Tests/Models/GeoNames/Admin1AlternateName.php
//!
//! The `geonames` model set's `City` is not in the recorded classes; the
//! test does not use it, so the schema has the other three classes.

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
#[path = "common/slc.rs"]
mod slc;
#[path = "common/slc_functional.rs"]
mod slc_functional;

use php_value::{Object, Value};

use slc_functional::Fixture;

const COUNTRY: &str = "Doctrine\\Tests\\Models\\GeoNames\\Country";
const ADMIN1: &str = "Doctrine\\Tests\\Models\\GeoNames\\Admin1";
const ADMIN1_ALTERNATE_NAME: &str = "Doctrine\\Tests\\Models\\GeoNames\\Admin1AlternateName";

/// `new Country($id, $name)`.
fn country(id: &str, name: &str) -> Object {
    let country = orm::new_object(COUNTRY);
    orm::set(&country, "id", Value::string(id));
    orm::set(&country, "name", Value::string(name));
    country
}

/// `new Admin1($id, $name, $country)`.
fn admin1(id: i64, name: &str, country: &Object) -> Object {
    let admin1 = orm::new_object(ADMIN1);
    orm::set(&admin1, "names", Value::Array(php_value::PhpArray::new()));
    orm::set(&admin1, "id", Value::Int(id));
    orm::set(&admin1, "name", Value::string(name));
    orm::set_obj(&admin1, "country", country);
    admin1
}

/// `new Admin1AlternateName($id, $name, $admin1)`.
fn admin1_alternate_name(id: i64, name: &str, admin1: &Object) -> Object {
    let alternate = orm::new_object(ADMIN1_ALTERNATE_NAME);
    orm::set(&alternate, "name", Value::string(name));
    orm::set_obj(&alternate, "admin1", admin1);
    orm::set(&alternate, "id", Value::string(id.to_string()));
    alternate
}

/// `setUp()`.
async fn set_up() -> Fixture {
    let f = slc_functional::set_up_with(&[COUNTRY, ADMIN1, ADMIN1_ALTERNATE_NAME]).await;

    let it = country("IT", "Italy");

    f.em.persist(&it).unwrap();
    f.em.flush().await.unwrap();

    let admin1 = admin1(1, "Rome", &it);

    f.em.persist(&admin1).unwrap();
    f.em.flush().await.unwrap();

    let name1 = admin1_alternate_name(1, "Roma", &admin1);
    let name2 = admin1_alternate_name(2, "Rome", &admin1);

    slc::add_to(&admin1, "names", &name1);
    slc::add_to(&admin1, "names", &name2);

    f.em.persist(&admin1).unwrap();
    f.em.persist(&name1).unwrap();
    f.em.persist(&name2).unwrap();

    f.em.flush().await.unwrap();
    f.em.clear().unwrap();
    f.evict_regions();
    f
}

#[tokio::test]
async fn testFindByReturnsCachedEntity() {
    let f = set_up().await;
    let admin1_repo = f.em.get_repository(ADMIN1).unwrap();

    f.reset_query_log();

    let criteria = slc::array(&[("country", Value::string("IT")), ("id", Value::Int(1))]);
    let admin1_rome = admin1_repo
        .find_one_by(&criteria, None)
        .await
        .unwrap()
        .unwrap();

    let country = f.load_one(&admin1_rome, "country").await;
    assert_eq!(Value::string("Italy"), orm::get(&country, "name"));
    assert_eq!(2, slc_functional::count(&admin1_rome, "names").await);
    f.assert_query_count(3);

    f.em.clear().unwrap();

    f.reset_query_log();

    let admin1_rome = admin1_repo
        .find_one_by(&criteria, None)
        .await
        .unwrap()
        .unwrap();

    let country = f.load_one(&admin1_rome, "country").await;
    assert_eq!(Value::string("Italy"), orm::get(&country, "name"));
    assert_eq!(2, slc_functional::count(&admin1_rome, "names").await);
    f.assert_query_count(0);
}
