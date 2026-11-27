//! Upstream: doctrine/orm tests/Tests/ORM/Query/QueryTest.php
//!
//! The test entity manager is one on an empty `SQLite` database with the
//! schema of the models (upstream's mocked connection returns no rows).
//! `testFluentQueryInterface` chains the setters that are ported. The
//! result cache tests queue the mocked connection's results; here the
//! rows are in the database, and the test changes them between the
//! queries where the queued results differ.
//! The query cache's `createMock(CacheItemPoolInterface::class)` counts
//! its `getItem()` calls over an `ArrayAdapter`.

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
use php_value::{PhpArray, Thrown, Value};
use php2rust_doctrine_dbal::{ArrayParameterType, LockMode, ParameterType, WrapperParameterType};
use php2rust_doctrine_orm::cache::pool::{ArrayAdapter, CacheItem, CacheItemPool};
use php2rust_doctrine_orm::collection_object::ArrayCollectionObject;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::query::context::{HINT_LOCK_MODE, HYDRATE_OBJECT, Hint};
use php2rust_doctrine_orm::query::{Parameter, Query};
use php2rust_doctrine_persistence::mapping::driver::MappingDriverChain;
use php2rust_doctrine_persistence::php_reflection;

use orm::{CMS_USER, s};

const QUERY_EXCEPTION: &str = "Doctrine\\ORM\\Query\\QueryException";
const DATE_TIME_MODEL: &str = "Doctrine\\Tests\\Models\\Generic\\DateTimeModel";

async fn entity_manager() -> Arc<EntityManager> {
    let mut classes: Vec<&str> = orm::CMS_MODELS.to_vec();
    classes.push(DATE_TIME_MODEL);
    orm::set_up(&classes).await
}

/// `assertEquals($parameters, $query->getParameters())`: the names and
/// the values.
fn assert_parameters(expected: &[Parameter], actual: &[Parameter]) {
    assert_eq!(expected.len(), actual.len());
    for (expected, actual) in expected.iter().zip(actual) {
        assert_eq!(expected.get_name(), actual.get_name());
        assert!(
            php_value::compare::loose_equals(expected.get_value(), actual.get_value()),
            "{:?} != {:?}",
            expected.get_value(),
            actual.get_value()
        );
        assert_eq!(expected.get_type(), actual.get_type());
    }
}

fn hint_value(hint: &Hint) -> Value {
    match hint {
        Hint::Value(value) => value.clone(),
        other => panic!("{other:?} is not a value"),
    }
}

fn hints_array(hints: &[(String, Hint)]) -> PhpArray {
    let mut array = PhpArray::new();
    for (name, hint) in hints {
        array.insert(name.as_str(), hint_value(hint));
    }
    array
}

fn assert_query_exception(e: &Thrown) {
    assert_eq!(e.class, QUERY_EXCEPTION, "{e:?}");
}

/// `$enum::Case`.
fn case(enum_type: &str, name: &str) -> Value {
    models::register();
    let class = php_reflection::class_info(enum_type).expect("the enum is declared");
    let case = class
        .cases()
        .iter()
        .find(|case| {
            case.property(b"name")
                .is_some_and(|n| n == Value::string(name))
        })
        .cloned()
        .expect("the case is declared");
    Value::Object(case)
}

fn cities() -> PhpArray {
    let mut cities = PhpArray::new();
    cities.insert(0, s("Paris"));
    cities.insert(3, s("Cannes"));
    cities.insert(9, s("St Julien"));
    cities
}

#[tokio::test]
async fn testGetParameters() {
    let em = entity_manager().await;
    let query = em.create_query(
        "select u from Doctrine\\Tests\\Models\\CMS\\CmsUser u where u.username = ?1",
    );

    let parameters: Vec<Parameter> = Vec::new();

    assert_parameters(&parameters, &query.get_parameters());
}

#[tokio::test]
async fn testGetParametersHasSomeAlready() {
    let em = entity_manager().await;
    let query = em.create_query(
        "select u from Doctrine\\Tests\\Models\\CMS\\CmsUser u where u.username = ?1",
    );
    query.set_parameter(2, 84, None);

    let parameters = vec![Parameter::new(2, 84, None)];

    assert_parameters(&parameters, &query.get_parameters());
}

#[tokio::test]
async fn testSetParameters() {
    let em = entity_manager().await;
    let query = em.create_query(
        "select u from Doctrine\\Tests\\Models\\CMS\\CmsUser u where u.username = ?1",
    );

    let parameters = vec![
        Parameter::new(1, s("foo"), None),
        Parameter::new(2, s("bar"), None),
    ];

    query.set_parameters(parameters.clone());

    assert_parameters(&parameters, &query.get_parameters());
}

#[tokio::test]
async fn testSetLockMode() {
    // provideLockModes
    for lock_mode in [LockMode::PessimisticRead, LockMode::PessimisticWrite] {
        let em = entity_manager().await;
        let query = em
            .wrap_in_transaction(|em| async move {
                let query = em.create_query(
                    "select u from Doctrine\\Tests\\Models\\CMS\\CmsUser u where u.username = ?1",
                );
                query.set_lock_mode(lock_mode)?;
                Ok(query)
            })
            .await
            .unwrap();

        assert_eq!(Some(lock_mode), query.get_lock_mode());
        assert!(
            matches!(query.get_hint(HINT_LOCK_MODE), Hint::LockMode(mode) if mode == lock_mode)
        );
    }
}

#[tokio::test]
async fn testSetLockModeNoneDoesNotRequireTransaction() {
    let em = entity_manager().await;
    let query = em.create_query(
        "select u from Doctrine\\Tests\\Models\\CMS\\CmsUser u where u.username = ?1",
    );
    query.set_lock_mode(LockMode::None).unwrap();

    assert_eq!(Some(LockMode::None), query.get_lock_mode());
    assert!(matches!(
        query.get_hint(HINT_LOCK_MODE),
        Hint::LockMode(LockMode::None)
    ));
}

#[tokio::test]
async fn testFree() {
    let em = entity_manager().await;
    let query = em.create_query(
        "select u from Doctrine\\Tests\\Models\\CMS\\CmsUser u where u.username = ?1",
    );
    query.set_parameter(
        2,
        84,
        Some(WrapperParameterType::Parameter(ParameterType::Integer)),
    );

    query.free();

    assert_eq!(0, query.get_parameters().len());
}

#[tokio::test]
async fn testClone() {
    let em = entity_manager().await;
    let dql = "select u from Doctrine\\Tests\\Models\\CMS\\CmsUser u where u.username = ?1";

    let query = em.create_query(dql);
    query.set_parameter(
        2,
        84,
        Some(WrapperParameterType::Parameter(ParameterType::Integer)),
    );
    query.set_hint("foo", Hint::Value(s("bar")));

    let cloned = query.clone_query();

    assert_eq!(Some(dql.to_owned()), cloned.get_dql());
    assert_eq!(0, cloned.get_parameters().len());
    assert!(hint_value(&cloned.get_hint("foo")).identical(&Value::Bool(false)));
}

#[tokio::test]
async fn testFluentQueryInterface() {
    let em = entity_manager().await;
    let q = em.create_query("select a from Doctrine\\Tests\\Models\\CMS\\CmsArticle a");
    let q2 = q
        .set_hint("foo", Hint::Value(s("bar")))
        .set_hint("bar", Hint::Value(s("baz")))
        .set_parameter(1, s("bar"), None)
        .set_parameters(vec![Parameter::new(2, s("baz"), None)])
        .expire_result_cache(true)
        .set_result_cache(None)
        .unwrap()
        .set_result_cache_id(Some("foo"))
        .set_dql("foo")
        .set_first_result(10)
        .set_max_results(Some(10));

    assert!(std::ptr::eq(q2, &raw const q));
}

#[tokio::test]
async fn testHints() {
    let em = entity_manager().await;
    let q = em.create_query("select a from Doctrine\\Tests\\Models\\CMS\\CmsArticle a");
    q.set_hint("foo", Hint::Value(s("bar")))
        .set_hint("bar", Hint::Value(s("baz")));

    assert!(php_value::compare::loose_equals(
        &s("bar"),
        &hint_value(&q.get_hint("foo"))
    ));
    assert!(php_value::compare::loose_equals(
        &s("baz"),
        &hint_value(&q.get_hint("bar"))
    ));
    let mut expected = PhpArray::new();
    expected.insert("foo", s("bar"));
    expected.insert("bar", s("baz"));
    assert!(Value::Array(expected).identical(&Value::Array(hints_array(&q.get_hints()))));
    assert!(q.has_hint("foo"));
    assert!(!q.has_hint("barFooBaz"));
}

#[tokio::test]
async fn testToIterableWithNoDistinctAndWrongSelectClause() {
    let em = entity_manager().await;
    let q = em.create_query(
        "select u, a from Doctrine\\Tests\\Models\\CMS\\CmsUser u LEFT JOIN u.articles a",
    );
    let e = q.to_iterable(None, HYDRATE_OBJECT).await.unwrap_err();
    assert_query_exception(&e);
}

#[tokio::test]
async fn testToIterableWithNoDistinctAndWithValidSelectClause() {
    let em = entity_manager().await;
    let q = em.create_query(
        "select u from Doctrine\\Tests\\Models\\CMS\\CmsUser u LEFT JOIN u.articles a",
    );
    let e = q.to_iterable(None, HYDRATE_OBJECT).await.unwrap_err();
    assert_query_exception(&e);
}

#[tokio::test]
async fn testIterateWithDistinct() {
    let em = entity_manager().await;
    let q = em.create_query(
        "SELECT DISTINCT u from Doctrine\\Tests\\Models\\CMS\\CmsUser u LEFT JOIN u.articles a",
    );

    // assertInstanceOf(Generator::class, ...): the values it yields.
    q.to_iterable(None, HYDRATE_OBJECT).await.unwrap();
}

#[tokio::test]
async fn testIterateEmptyResult() {
    let em = entity_manager().await;
    let q = em.create_query("SELECT u from Doctrine\\Tests\\Models\\CMS\\CmsUser u");

    for _item in q.to_iterable(None, HYDRATE_OBJECT).await.unwrap() {}
}

#[tokio::test]
async fn testCollectionParameters() {
    let em = entity_manager().await;
    let cities = cities();

    let query = em.create_query(
        "SELECT a FROM Doctrine\\Tests\\Models\\CMS\\CmsAddress a WHERE a.city IN (:cities)",
    );
    query.set_parameter("cities", Value::Array(cities.clone()), None);

    let parameters = query.get_parameters();
    let parameter = parameters.first().unwrap();

    assert_eq!("cities", parameter.get_name());
    assert!(Value::Array(cities).identical(parameter.get_value()));
}

#[tokio::test]
async fn testExplicitCollectionParameters() {
    let em = entity_manager().await;
    let cities = cities();

    let query = em.create_query(
        "SELECT a FROM Doctrine\\Tests\\Models\\CMS\\CmsAddress a WHERE a.city IN (:cities)",
    );
    query.set_parameter(
        "cities",
        Value::Array(cities.clone()),
        Some(WrapperParameterType::Array(ArrayParameterType::String)),
    );

    let parameters = query.get_parameters();
    let parameter = parameters.first().unwrap();

    assert_eq!("cities", parameter.get_name());
    assert!(Value::Array(cities).identical(parameter.get_value()));
}

#[tokio::test]
async fn testProcessParameterValueIterable() {
    // provideProcessParameterValueIterable (a generator is not a value of
    // the port)
    let base_array = cities();
    let mut enums = PhpArray::new();
    for (key, city) in base_array.iter() {
        let name = match city {
            Value::String(name) if name.as_bytes() == b"St Julien" => "StJulien".to_owned(),
            other => php2rust_doctrine_orm::utility::to_text(other),
        };
        enums.insert(
            key.clone(),
            case("Doctrine\\Tests\\Models\\Enums\\City", &name),
        );
    }
    let cases = [
        Value::Array(base_array.clone()),
        ArrayCollectionObject::value(base_array.clone()),
        Value::Array(enums),
    ];
    for cities in cases {
        let em = entity_manager().await;
        let query = em.create_query(
            "SELECT a FROM Doctrine\\Tests\\Models\\CMS\\CmsAddress a WHERE a.city IN (:cities)",
        );
        assert!(php_value::compare::loose_equals(
            &Value::Array(base_array.clone()),
            &query.process_parameter_value(&cities).unwrap(),
        ));
    }
}

#[tokio::test]
async fn testProcessParameterValueWithIterableEntityShouldNotBeTreatedAsIterable() {
    let em = entity_manager().await;
    let group = orm::new_object(orm::CMS_GROUP);
    orm::set(&group, "id", 1);

    let query = em.create_query(
        "SELECT u FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u WHERE u.group IN (:group)",
    );
    assert!(php_value::compare::loose_equals(
        &Value::Int(1),
        &query
            .process_parameter_value(&Value::Object(group))
            .unwrap()
    ));
}

#[tokio::test]
async fn testProcessParameterValueClassMetadata() {
    let em = entity_manager().await;
    let query = em.create_query(
        "SELECT a FROM Doctrine\\Tests\\Models\\CMS\\CmsAddress a WHERE a.city IN (:cities)",
    );
    let metadata = em.get_class_metadata(orm::CMS_ADDRESS).unwrap();
    assert!(php_value::compare::loose_equals(
        &Value::from(orm::CMS_ADDRESS),
        &query
            .process_parameter_value(&Value::Object(metadata))
            .unwrap()
    ));
}

#[tokio::test]
async fn testProcessParameterValueObject() {
    let em = entity_manager().await;
    let query = em.create_query(
        "SELECT a FROM Doctrine\\Tests\\Models\\CMS\\CmsAddress a WHERE a.user = :user",
    );
    let user = orm::new_object(CMS_USER);
    orm::set(&user, "id", 12345);

    assert!(
        Value::Int(12345).identical(&query.process_parameter_value(&Value::Object(user)).unwrap())
    );
}

#[tokio::test]
async fn testProcessParameterValueValueObjectWithDriverChain() {
    let driver_chain = MappingDriverChain::new();
    driver_chain.add_driver(
        Arc::new(
            php2rust_doctrine_orm::mapping::driver::AttributeDriver::new(
                Vec::<String>::new(),
                true,
            )
            .unwrap(),
        ),
        "Foo",
    );
    let driver_chain = Arc::new(driver_chain);
    let em = orm::entity_manager_with(orm::connection(), move |config| {
        config.set_metadata_driver_impl(driver_chain);
    });

    let query = Query::new(&em);

    let vo = DateTime::object(DateTimeImmutable::from_timestamp(
        1_598_918_400,
        0,
        TimeZone::Utc,
    ));

    assert!(vo.identical(&query.process_parameter_value(&vo).unwrap()));
}

#[tokio::test]
async fn testProcessParameterValueNull() {
    let em = entity_manager().await;
    let query = em.create_query(
        "SELECT a FROM Doctrine\\Tests\\Models\\CMS\\CmsAddress a WHERE a.user = :user",
    );

    assert!(
        query
            .process_parameter_value(&Value::Null)
            .unwrap()
            .is_null()
    );
}

fn assert_backed_enum_values(query: &Query) {
    let active = Value::Array(PhpArray::list([case(
        "Doctrine\\Tests\\Models\\Enums\\UserStatus",
        "Active",
    )]));
    let user = Value::Array(PhpArray::list([case(
        "Doctrine\\Tests\\Models\\Enums\\AccessLevel",
        "User",
    )]));
    assert!(
        Value::Array(PhpArray::list([s("active")]))
            .identical(&query.process_parameter_value(&active).unwrap())
    );
    assert!(
        Value::Array(PhpArray::list([Value::Int(2)]))
            .identical(&query.process_parameter_value(&user).unwrap())
    );
}

#[tokio::test]
async fn testProcessParameterValueBackedEnum() {
    let em = entity_manager().await;
    let query = em.create_query(
        "SELECT u FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u WHERE u.status = :status",
    );

    assert_backed_enum_values(&query);
}

#[tokio::test]
async fn testProcessParameterValueBackedEnumArray() {
    let em = entity_manager().await;
    let query = em.create_query(
        "SELECT u FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u WHERE u.status IN (:status)",
    );

    assert_backed_enum_values(&query);
}

/// The configuration of the port is fixed once the entity manager is
/// created, so the default hints are set before (and the change of a
/// default hint after the creation is not tested).
#[tokio::test]
async fn testDefaultQueryHints() {
    let mut default_hints = PhpArray::new();
    default_hints.insert("hint_name_1", s("hint_value_1"));
    default_hints.insert("hint_name_2", s("hint_value_2"));
    default_hints.insert("hint_name_3", s("hint_value_3"));
    let hints = default_hints.clone();
    let em = orm::entity_manager_with(orm::connection(), move |config| {
        config.set_default_query_hints(hints);
    });
    let config = em.get_configuration();
    let query = Query::new(&em);
    assert!(
        Value::Array(config.get_default_query_hints().clone())
            .identical(&Value::Array(hints_array(&query.get_hints())))
    );
    query.set_hint("hint_name_1", Hint::Value(s("hint_another_value_1")));
    assert!(
        !Value::Array(config.get_default_query_hints().clone())
            .identical(&Value::Array(hints_array(&query.get_hints())))
    );
    let q2 = query.clone_query();
    assert!(
        Value::Array(config.get_default_query_hints().clone())
            .identical(&Value::Array(hints_array(&q2.get_hints())))
    );
}

#[tokio::test]
async fn testSelectJoinSubquery() {
    let em = entity_manager().await;
    let query =
        em.create_query("select u from Doctrine\\Tests\\Models\\CMS\\CmsUser u JOIN (SELECT )");

    let e = query.get_sql().unwrap_err();
    assert_query_exception(&e);
    assert!(orm::message(&e).contains("Subquery"), "{e:?}");
}

#[tokio::test]
async fn testSelectFromSubquery() {
    let em = entity_manager().await;
    let query =
        em.create_query("select u from (select Doctrine\\Tests\\Models\\CMS\\CmsUser c) as u");

    let e = query.get_sql().unwrap_err();
    assert_query_exception(&e);
    assert!(orm::message(&e).contains("Subquery"), "{e:?}");
}

#[tokio::test]
async fn testGetParameterTypeJuggling() {
    let em = entity_manager().await;
    let query = em.create_query(&format!("select u from {CMS_USER} u where u.id = ?0"));

    query.set_parameter(0, 0, None);

    assert_eq!(1, query.get_parameters().len());
    assert!(Value::Int(0).identical(query.get_parameter(0).unwrap().get_value()));
    assert!(Value::Int(0).identical(query.get_parameter("0").unwrap().get_value()));
}

#[tokio::test]
async fn testSetParameterWithNameZeroIsNotOverridden() {
    let em = entity_manager().await;
    let query = em.create_query(&format!(
        "select u from {CMS_USER} u where u.id != ?0 and u.username = :name"
    ));

    query.set_parameter(0, 0, None);
    query.set_parameter("name", s("Doctrine"), None);

    assert_eq!(2, query.get_parameters().len());
    assert!(Value::Int(0).identical(query.get_parameter("0").unwrap().get_value()));
    assert!(s("Doctrine").identical(query.get_parameter("name").unwrap().get_value()));
}

#[tokio::test]
async fn testSetParameterWithNameZeroDoesNotOverrideAnotherParameter() {
    let em = entity_manager().await;
    let query = em.create_query(&format!(
        "select u from {CMS_USER} u where u.id != ?0 and u.username = :name"
    ));

    query.set_parameter("name", s("Doctrine"), None);
    query.set_parameter(0, 0, None);

    assert_eq!(2, query.get_parameters().len());
    assert!(Value::Int(0).identical(query.get_parameter(0).unwrap().get_value()));
    assert!(s("Doctrine").identical(query.get_parameter("name").unwrap().get_value()));
}

#[tokio::test]
async fn testSetParameterWithTypeJugglingWorks() {
    let em = entity_manager().await;
    let query = em.create_query(&format!(
        "select u from {CMS_USER} u where u.id != ?0 and u.username = :name"
    ));

    query.set_parameter("0", 1, None);
    query.set_parameter("name", s("Doctrine"), None);
    query.set_parameter(0, 2, None);
    query.set_parameter("0", 3, None);

    assert_eq!(2, query.get_parameters().len());
    assert!(Value::Int(3).identical(query.get_parameter(0).unwrap().get_value()));
    assert!(Value::Int(3).identical(query.get_parameter("0").unwrap().get_value()));
    assert!(s("Doctrine").identical(query.get_parameter("name").unwrap().get_value()));
}

/// The unit of work is not a mock: the parameter of a specified type is
/// bound as given, and the query finds nothing.
#[tokio::test]
async fn testValuesAreNotBeingResolvedForSpecifiedParameterTypes() {
    let em = entity_manager().await;
    let query = em.create_query(&format!(
        "SELECT d FROM {DATE_TIME_MODEL} d WHERE d.datetime = :value"
    ));

    query.set_parameter(
        "value",
        DateTime::object(DateTimeImmutable::now(TimeZone::Utc)),
        Some(WrapperParameterType::Name("datetime".into())),
    );

    let result = query.get_result(HYDRATE_OBJECT).await.unwrap();
    assert!(!result.is_truthy());
}

#[tokio::test]
async fn testNonExistentExecutor() {
    let em = entity_manager().await;
    let e = em.create_query("0").execute(None, None).await.unwrap_err();
    assert_query_exception(&e);
    assert!(
        orm::message(&e).contains(
            "[Syntax Error] line 0, col -1: Error: Expected SELECT, UPDATE or DELETE, got end of string."
        ),
        "{e:?}"
    );
}

#[tokio::test]
async fn testGetParameterColonNormalize() {
    let em = entity_manager().await;
    let query = em.create_query(&format!("select u from {CMS_USER} u where u.name = :name"));

    query.set_parameter(":name", s("Benjamin"), None);
    query.set_parameter("name", s("Benjamin"), None);

    assert_eq!(1, query.get_parameters().len());
    assert!(s("Benjamin").identical(query.get_parameter(":name").unwrap().get_value()));
    assert!(s("Benjamin").identical(query.get_parameter("name").unwrap().get_value()));
}

/// `createMock(CacheItemPoolInterface::class)` whose `getItem()` is
/// counted.
#[derive(Debug, Default)]
struct CacheMock {
    inner: ArrayAdapter,
    get_item_calls: std::sync::atomic::AtomicUsize,
}

impl CacheItemPool for CacheMock {
    fn get_item(&self, key: &str) -> Result<CacheItem, Thrown> {
        self.get_item_calls
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.inner.get_item(key)
    }

    fn get_items(&self, keys: &[String]) -> Result<Vec<CacheItem>, Thrown> {
        self.inner.get_items(keys)
    }

    fn has_item(&self, key: &str) -> Result<bool, Thrown> {
        self.inner.has_item(key)
    }

    fn clear(&self, prefix: &str) -> bool {
        self.inner.clear(prefix)
    }

    fn delete_item(&self, key: &str) -> Result<bool, Thrown> {
        self.inner.delete_item(key)
    }

    fn save(&self, item: CacheItem) -> bool {
        self.inner.save(item)
    }
}

impl CacheMock {
    /// `expects(self::atLeastOnce())->method('getItem')`.
    fn assert_get_item_at_least_once(&self) {
        assert!(
            self.get_item_calls
                .load(std::sync::atomic::Ordering::Relaxed)
                >= 1
        );
    }
}

#[tokio::test]
async fn testGetQueryCacheDriverWithDefaults() {
    let em = entity_manager().await;
    let cache = Arc::new(CacheMock::default());

    em.get_configuration().set_query_cache(cache.clone());
    em.create_query(&format!("select u from {CMS_USER} u"))
        .get_sql()
        .unwrap();
    cache.assert_get_item_at_least_once();
}

#[tokio::test]
async fn testGetQueryCacheDriverWithCacheExplicitlySet() {
    let em = entity_manager().await;
    let cache = Arc::new(CacheMock::default());

    em.create_query(&format!("select u from {CMS_USER} u"))
        .set_query_cache(Some(cache.clone()))
        .get_sql()
        .unwrap();
    cache.assert_get_item_at_least_once();
}

/// `INSERT INTO cms_users` of a user with the given id.
async fn insert_user(em: &EntityManager, id: i64) {
    let mut params = PhpArray::new();
    params.insert("id", Value::Int(id));
    params.insert("status", s("dev"));
    params.insert("username", s(&format!("user{id}")));
    params.insert("name", s("name"));
    em.get_connection()
        .insert(
            "cms_users",
            &params,
            &php2rust_doctrine_dbal::ParameterTypes::new(),
        )
        .await
        .unwrap();
}

fn count(result: &Value) -> usize {
    match result {
        Value::Array(array) => array.len(),
        _ => panic!("not an array"),
    }
}

/// The first mocked result has one row and the second none: the row is
/// deleted between the queries.
#[tokio::test]
async fn testResultCacheCaching() {
    let entity_manager = entity_manager().await;
    insert_user(&entity_manager, 1).await;

    entity_manager
        .get_configuration()
        .set_result_cache(Arc::new(ArrayAdapter::new()));
    entity_manager
        .get_configuration()
        .set_query_cache(Arc::new(ArrayAdapter::new()));

    let res = entity_manager
        .create_query("select u from Doctrine\\Tests\\Models\\CMS\\CmsUser u")
        .use_query_cache(true)
        .enable_result_cache(Some(60), None)
        //let it cache
        .get_object_result()
        .await
        .unwrap();

    assert_eq!(count(&res), 1);

    entity_manager
        .get_connection()
        .execute_statement(
            "DELETE FROM cms_users",
            &PhpArray::new(),
            &php2rust_doctrine_dbal::ParameterTypes::new(),
        )
        .await
        .unwrap();
    entity_manager.clear().unwrap();

    let res = entity_manager
        .create_query("select u from Doctrine\\Tests\\Models\\CMS\\CmsUser u")
        .use_query_cache(true)
        .disable_result_cache()
        .get_object_result()
        .await
        .unwrap();
    assert_eq!(count(&res), 0);
}

/// DDC-3741
#[tokio::test]
async fn testSetHydrationCacheProfileNull() {
    let em = entity_manager().await;
    let query = em.create_query("");
    query.set_hydration_cache_profile(None);
    assert!(query.get_hydration_cache_profile().is_none());
}

/// 2947. The mocked results have one row, then two, then one: the
/// second user is inserted before the third query, and the third result
/// is never read (the last query reads the cache).
#[tokio::test]
async fn testResultCacheEviction() {
    let entity_manager = entity_manager().await;
    insert_user(&entity_manager, 1).await;

    entity_manager
        .get_configuration()
        .set_result_cache(Arc::new(ArrayAdapter::new()));

    let query =
        entity_manager.create_query("SELECT u FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u");
    query.enable_result_cache(None, None);

    // Performs the query and sets up the initial cache
    assert_eq!(count(&query.get_object_result().await.unwrap()), 1);

    insert_user(&entity_manager, 2).await;

    // Retrieves cached data since expire flag is false and we have a cached result set
    assert_eq!(count(&query.get_object_result().await.unwrap()), 1);

    // Performs the query and caches the result set since expire flag is true
    assert_eq!(
        count(
            &query
                .expire_result_cache(true)
                .get_object_result()
                .await
                .unwrap()
        ),
        2
    );

    entity_manager
        .get_connection()
        .execute_statement(
            "DELETE FROM cms_users WHERE id = 2",
            &PhpArray::new(),
            &php2rust_doctrine_dbal::ParameterTypes::new(),
        )
        .await
        .unwrap();

    // Retrieves cached data since expire flag is false and we have a cached result set
    assert_eq!(
        count(
            &query
                .expire_result_cache(false)
                .get_object_result()
                .await
                .unwrap()
        ),
        2
    );
}

/// 6748
#[tokio::test]
async fn testResultCacheProfileCanBeRemovedViaSetter() {
    let em = entity_manager().await;
    em.get_configuration()
        .set_result_cache(Arc::new(ArrayAdapter::new()));

    let query = em.create_query(&format!("SELECT u FROM {CMS_USER} u"));
    query.enable_result_cache(None, None);
    query.set_result_cache_profile(None);

    assert!(query.get_query_cache_profile().is_none());
}
