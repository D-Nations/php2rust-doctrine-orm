//! Upstream: doctrine/orm tests/Tests/ORM/Functional/CompositePrimaryKeyTest.php
//!
//! On `SQLite` (`tests/common/orm.rs`) with the navigation models; their
//! constructors and methods are the functions below.
//! `testCompositeCollectionMemberExpression` is skipped upstream
//! (`markTestSkipped('How to test this?')`): n/a.

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

use php_value::{Object, PhpArray, Value, php_array};
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::exception::{MISSING_IDENTIFIER_FIELD, UNRECOGNIZED_IDENTIFIER_FIELDS};

use orm::{get, new_object, s, set};

const NAV_COUNTRY: &str = "Doctrine\\Tests\\Models\\Navigation\\NavCountry";
const NAV_POINT_OF_INTEREST: &str = "Doctrine\\Tests\\Models\\Navigation\\NavPointOfInterest";
const NAV_TOUR: &str = "Doctrine\\Tests\\Models\\Navigation\\NavTour";
const NAV_USER: &str = "Doctrine\\Tests\\Models\\Navigation\\NavUser";
const NAV_PHOTOS: &str = "Doctrine\\Tests\\Models\\Navigation\\NavPhotos";
const QUERY_EXCEPTION: &str = "Doctrine\\ORM\\Query\\QueryException";

/// The `navigation` model set.
const NAVIGATION_MODELS: [&str; 5] = [
    NAV_COUNTRY,
    NAV_POINT_OF_INTEREST,
    NAV_TOUR,
    NAV_PHOTOS,
    NAV_USER,
];

/// `setUp()`.
async fn set_up() -> Arc<EntityManager> {
    orm::set_up(&NAVIGATION_MODELS).await
}

/// `new NavCountry($name)`.
fn nav_country(name: &str) -> Object {
    let country = new_object(NAV_COUNTRY);
    set(&country, "name", s(name));
    country
}

/// `new NavPointOfInterest($lat, $long, $name, $country)`.
fn nav_point_of_interest(lat: i64, long: i64, name: &str, country: &Object) -> Object {
    let poi = new_object(NAV_POINT_OF_INTEREST);
    set(&poi, "name", s(name));
    set(&poi, "country", Value::Object(country.clone()));
    set(&poi, "lat", Value::Int(lat));
    set(&poi, "long", Value::Int(long));
    set(
        &poi,
        "visitors",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    poi
}

/// `new NavTour($name)`.
fn nav_tour(name: &str) -> Object {
    let tour = new_object(NAV_TOUR);
    set(&tour, "name", s(name));
    set(&tour, "pois", ArrayCollectionObject::value(PhpArray::new()));
    tour
}

/// `new NavUser($name)`.
fn nav_user(name: &str) -> Object {
    let user = new_object(NAV_USER);
    set(&user, "name", s(name));
    user
}

/// `new NavPhotos($poi, $file)`.
fn nav_photos(poi: &Object, file: &str) -> Object {
    let photo = new_object(NAV_PHOTOS);
    set(&photo, "poi", Value::Object(poi.clone()));
    set(&photo, "file", s(file));
    photo
}

/// `['lat' => 100, 'long' => 200]`.
fn brandenburger_tor_id() -> Value {
    Value::Array(php_array!["lat" => 100, "long" => 200])
}

/// `putGermanysBrandenburderTor()`.
async fn put_germanys_brandenburder_tor(em: &Arc<EntityManager>) {
    let country = nav_country("Germany");
    em.persist(&country).unwrap();

    let poi = nav_point_of_interest(100, 200, "Brandenburger Tor", &country);
    em.persist(&poi).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();
}

/// `putTripAroundEurope()`.
async fn put_trip_around_europe(em: &Arc<EntityManager>) -> Object {
    let poi = em
        .find(NAV_POINT_OF_INTEREST, brandenburger_tor_id())
        .await
        .unwrap()
        .unwrap();

    let tour = nav_tour("Trip around Europe");
    // `$tour->addPointOfInterest($poi)`
    coll::add(&get(&tour, "pois"), Value::Object(poi)).unwrap();

    em.persist(&tour).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    tour
}

#[tokio::test]
async fn testPersistCompositePkEntity() {
    let em = set_up().await;
    put_germanys_brandenburder_tor(&em).await;

    let poi = em
        .find(NAV_POINT_OF_INTEREST, brandenburger_tor_id())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(NAV_POINT_OF_INTEREST, poi.class());
    assert_eq!(Value::Int(100), get(&poi, "lat"));
    assert_eq!(Value::Int(200), get(&poi, "long"));
    assert_eq!(s("Brandenburger Tor"), get(&poi, "name"));
}

#[tokio::test]
async fn testSetParameterCompositeKeyObject() {
    let em = set_up().await;
    put_germanys_brandenburder_tor(&em).await;

    let poi = em
        .find(NAV_POINT_OF_INTEREST, brandenburger_tor_id())
        .await
        .unwrap()
        .unwrap();
    let photo = nav_photos(&poi, "asdf");
    em.persist(&photo).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let dql = "SELECT t FROM Doctrine\\Tests\\Models\\Navigation\\NavPhotos t WHERE t.poi = ?1";

    let e = em.create_query(dql).get_sql().unwrap_err();
    assert_eq!(QUERY_EXCEPTION, e.class);
    assert!(
        orm::message(&e).contains(
            "A single-valued association path expression to an entity with a composite primary key is not supported."
        ),
        "{}",
        orm::message(&e)
    );
}

#[tokio::test]
async fn testIdentityFunctionWithCompositePrimaryKey() {
    let em = set_up().await;
    put_germanys_brandenburder_tor(&em).await;

    let poi = em
        .find(NAV_POINT_OF_INTEREST, brandenburger_tor_id())
        .await
        .unwrap()
        .unwrap();
    let photo = nav_photos(&poi, "asdf");
    em.persist(&photo).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let dql = "SELECT IDENTITY(p.poi, 'long') AS long, IDENTITY(p.poi, 'lat') AS lat FROM Doctrine\\Tests\\Models\\Navigation\\NavPhotos p";
    let query = em.create_query(dql);
    let result = query.get_object_result().await.unwrap();

    assert_eq!(1, orm::list(&result).len());
    orm::assert_loose_eq(&Value::Int(200), &orm::at(&orm::at(&result, 0), "long"));
    orm::assert_loose_eq(&Value::Int(100), &orm::at(&orm::at(&result, 0), "lat"));

    em.clear().unwrap();

    orm::assert_results_are_the_same(&query).await;
}

#[tokio::test]
async fn testManyToManyCompositeRelation() {
    let em = set_up().await;
    put_germanys_brandenburder_tor(&em).await;
    let tour = put_trip_around_europe(&em).await;

    let tour = em.find(NAV_TOUR, get(&tour, "id")).await.unwrap().unwrap();

    assert_eq!(1, orm::count_of(&tour, "pois").await);
}

#[tokio::test]
async fn testCompositeDqlEagerFetching() {
    let em = set_up().await;
    put_germanys_brandenburder_tor(&em).await;
    put_trip_around_europe(&em).await;

    let dql = "SELECT t, p, c FROM Doctrine\\Tests\\Models\\Navigation\\NavTour t \
               INNER JOIN t.pois p INNER JOIN p.country c";
    let _tours = em.create_query(dql).get_object_result().await.unwrap();

    let query = em.create_query(dql);
    let tours = query.get_object_result().await.unwrap();

    assert_eq!(1, orm::list(&tours).len());

    let pois = orm::elements(&orm::obj(&orm::at(&tours, 0)), "pois").await;

    assert_eq!(1, pois.len());
    assert_eq!(s("Brandenburger Tor"), get(&pois[0], "name"));
}

#[tokio::test]
async fn testSpecifyUnknownIdentifierPrimaryKeyFails() {
    let em = set_up().await;

    let e = em
        .find(
            NAV_POINT_OF_INTEREST,
            Value::Array(php_array!["key1" => 100]),
        )
        .await
        .unwrap_err();
    assert_eq!(MISSING_IDENTIFIER_FIELD, e.class);
    assert!(orm::message(&e).contains(
        "The identifier long is missing for a query of Doctrine\\Tests\\Models\\Navigation\\NavPointOfInterest"
    ));
}

#[tokio::test]
async fn testUnrecognizedIdentifierFieldsOnGetReference() {
    let em = set_up().await;

    let e = em
        .get_reference(
            NAV_POINT_OF_INTEREST,
            Value::Array(php_array!["lat" => 10, "long" => 20, "key1" => 100]),
        )
        .await
        .unwrap_err();
    assert_eq!(UNRECOGNIZED_IDENTIFIER_FIELDS, e.class);
    assert!(orm::message(&e).contains("Unrecognized identifier fields: \"key1\""));
}

#[tokio::test]
async fn testDeleteCompositePersistentCollection() {
    let em = set_up().await;
    put_germanys_brandenburder_tor(&em).await;

    let poi = em
        .find(NAV_POINT_OF_INTEREST, brandenburger_tor_id())
        .await
        .unwrap()
        .unwrap();
    // `$poi->addVisitor(new NavUser(...))`
    coll::add(&get(&poi, "visitors"), Value::Object(nav_user("test1"))).unwrap();
    coll::add(&get(&poi, "visitors"), Value::Object(nav_user("test2"))).unwrap();

    em.flush().await.unwrap();

    coll::clear(&get(&poi, "visitors")).await.unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();

    let poi = em
        .find(NAV_POINT_OF_INTEREST, brandenburger_tor_id())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(0, orm::count_of(&poi, "visitors").await);
}
