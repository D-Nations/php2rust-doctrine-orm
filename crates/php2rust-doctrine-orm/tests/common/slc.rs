//! Helpers shared by the second-level cache tests that need an entity
//! manager: `OrmTestCase::enableSecondLevelCache()` with
//! `getTestEntityManager()`, the `Region`, `ConcurrentRegion`,
//! `EntityPersister` and `CollectionPersister` mocks (`createMock()`: each records its calls and
//! returns what the test sets, or its type's default: `''`, `false`,
//! `null`, `[]`, `0`), and the `Doctrine\Tests\Models\Cache` models'
//! constructors.
//!
//! Upstream: doctrine/orm tests/Tests/OrmTestCase.php
//! Upstream: doctrine/orm tests/Tests/Models/Cache/Country.php
//! Upstream: doctrine/orm tests/Tests/Models/Cache/State.php
//! Upstream: doctrine/orm tests/Tests/Models/Cache/City.php
//! Upstream: doctrine/orm tests/Tests/Models/Cache/Restaurant.php

#![allow(dead_code, reason = "each test file uses a subset")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use php_value::{Key, Object, PhpArray, Thrown, Value};
use php2rust_doctrine_collections::Criteria;
use php2rust_doctrine_dbal::LockMode;
use php2rust_doctrine_orm::cache::pool::ArrayAdapter;
use php2rust_doctrine_orm::cache::region::MultipleEntries;
use php2rust_doctrine_orm::cache::{
    CacheConfiguration, CacheEntry, CacheKey, CollectionCacheEntry, ConcurrentRegion,
    DefaultCacheFactory, Lock, Region,
};
use php2rust_doctrine_orm::collection_object::ArrayCollectionObject;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::mapping::driver::AttributeDriver;
use php2rust_doctrine_orm::mapping::driver::attribute_driver::Paths;
use php2rust_doctrine_orm::mapping::{AssociationMapping, ClassMetadata};
use php2rust_doctrine_orm::persistent_collection::PersistentCollection;
use php2rust_doctrine_orm::persisters::collection::CollectionPersister;
use php2rust_doctrine_orm::persisters::{
    CommitPersister, EntityPersister, EntityPersisterInterface, Params, PersisterCriteria,
};
use php2rust_doctrine_orm::query::ResultSetMapping;
use php2rust_doctrine_orm::unit_of_work::{Fut, Hints};
use php2rust_doctrine_persistence::mapping::driver::ClassLocator;
use php2rust_doctrine_persistence::php_reflection::{self, ClassKind};

use crate::orm;

pub const NS: &str = "Doctrine\\Tests\\Models\\Cache\\";
pub const COUNTRY: &str = "Doctrine\\Tests\\Models\\Cache\\Country";
pub const STATE: &str = "Doctrine\\Tests\\Models\\Cache\\State";
pub const CITY: &str = "Doctrine\\Tests\\Models\\Cache\\City";
pub const RESTAURANT: &str = "Doctrine\\Tests\\Models\\Cache\\Restaurant";

/// The test entity manager with the second level cache, its pool and its
/// cache factory.
pub struct Slc {
    pub em: Arc<EntityManager>,
    pub pool: Arc<ArrayAdapter>,
    pub factory: Arc<DefaultCacheFactory>,
    pub cache_config: Arc<CacheConfiguration>,
}

/// The classes of `tests/Tests/Models/Cache`: the recorded classes and
/// enums of its namespace.
#[derive(Debug)]
pub struct CacheModelsLocator;

impl ClassLocator for CacheModelsLocator {
    fn get_class_names(&self) -> Result<Vec<String>, Thrown> {
        Ok(php_reflection::declared()
            .iter()
            .filter(|class| matches!(class.kind(), ClassKind::Class | ClassKind::Enum))
            .map(|class| class.name().to_owned())
            .filter(|name| name.starts_with(NS))
            .collect())
    }
}

/// `enableSecondLevelCache()` and `getTestEntityManager()` (the attribute
/// driver over `Models/Cache`): a `DefaultCacheFactory` over a new
/// `ArrayAdapter`.
pub fn slc_entity_manager() -> Slc {
    let pool = Arc::new(ArrayAdapter::new());
    let cache_config = Arc::new(CacheConfiguration::new());
    let factory = Arc::new(DefaultCacheFactory::new(
        cache_config.get_regions_configuration(),
        pool.clone(),
    ));
    cache_config.set_cache_factory(factory.clone());
    let config = Arc::clone(&cache_config);
    let em = orm::entity_manager_with(orm::connection(), move |c| {
        let locator: Arc<dyn ClassLocator> = Arc::new(CacheModelsLocator);
        c.set_metadata_driver_impl(Arc::new(
            AttributeDriver::new(Paths::Locator(locator), true).unwrap(),
        ));
        c.set_second_level_cache_enabled(true);
        c.set_second_level_cache_configuration(config);
    });
    Slc {
        em,
        pool,
        factory,
        cache_config,
    }
}

/// `new Country($name)`.
pub fn country(name: &str) -> Object {
    let country = orm::new_object(COUNTRY);
    orm::set(&country, "name", Value::string(name));
    country
}

/// `new State($name)`.
pub fn state(name: &str) -> Object {
    state_in(name, None)
}

/// `new State($name, $country)`.
pub fn state_in(name: &str, country: Option<&Object>) -> Object {
    let state = orm::new_object(STATE);
    orm::set(&state, "name", Value::string(name));
    orm::set(
        &state,
        "country",
        country.map_or(Value::Null, |c| Value::Object(c.clone())),
    );
    orm::set(
        &state,
        "cities",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    state
}

/// `new City($name, $state)`.
pub fn city(name: &str, state: Option<&Object>) -> Object {
    let city = orm::new_object(CITY);
    orm::set(&city, "name", Value::string(name));
    orm::set(
        &city,
        "state",
        state.map_or(Value::Null, |s| Value::Object(s.clone())),
    );
    orm::set(
        &city,
        "travels",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    orm::set(
        &city,
        "attractions",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    city
}

/// `new Restaurant($name, $city)`.
pub fn restaurant(name: &str, city: &Object) -> Object {
    let restaurant = orm::new_object(RESTAURANT);
    orm::set(&restaurant, "name", Value::string(name));
    orm::set(&restaurant, "city", Value::Object(city.clone()));
    orm::set(
        &restaurant,
        "infos",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    restaurant
}

/// `$this->$field[] = $element` on a collection property.
pub fn add_to(entity: &Object, field: &str, element: &Object) {
    let collection = orm::get(entity, field);
    if let Some(collection) = ArrayCollectionObject::from_value(&collection) {
        collection
            .lock()
            .add(Value::Object(element.clone()))
            .unwrap();
    } else if let Some(collection) = PersistentCollection::from_value(&collection) {
        collection.add(Value::Object(element.clone())).unwrap();
    } else {
        panic!("{field} is not a collection");
    }
}

/// `$state->addCity($city)`.
pub fn add_city(state: &Object, city: &Object) {
    add_to(state, "cities", city);
}

/// `$city->addAttraction($attraction)`.
pub fn add_attraction(city: &Object, attraction: &Object) {
    add_to(city, "attractions", attraction);
}

/// A PHP array of string keys.
pub fn array(values: &[(&str, Value)]) -> PhpArray {
    values
        .iter()
        .map(|(key, value)| (*key, value.clone()))
        .collect()
}

/// `['id' => $id]`.
pub fn id(id: i64) -> PhpArray {
    array(&[("id", Value::Int(id))])
}

/// A recorded argument: a PHP value (objects compare by identity), or a
/// text for the Rust values (a mapping's field name, a lock mode, a
/// criteria).
#[derive(Debug, Clone, PartialEq)]
pub enum Arg {
    Value(Value),
    Text(String),
}

impl Arg {
    pub fn object(object: &Object) -> Self {
        Self::Value(Value::Object(object.clone()))
    }

    pub fn array(array: &PhpArray) -> Self {
        Self::Value(Value::Array(array.clone()))
    }

    pub fn opt_array(array: Option<&PhpArray>) -> Self {
        Self::Value(array.map_or(Value::Null, |a| Value::Array(a.clone())))
    }

    pub fn opt_int(value: Option<i64>) -> Self {
        Self::Value(value.map_or(Value::Null, Value::Int))
    }

    pub fn opt_object(object: Option<&Object>) -> Self {
        Self::Value(object.map_or(Value::Null, |o| Value::Object(o.clone())))
    }

    pub fn mapping(assoc: Option<&AssociationMapping>) -> Self {
        Self::Text(assoc.map_or_else(String::new, |a| {
            format!("{}::{}->{}", a.source_entity, a.field_name, a.target_entity)
        }))
    }

    pub fn lock_mode(lock_mode: Option<LockMode>) -> Self {
        Self::Text(format!("{lock_mode:?}"))
    }

    pub fn criteria(criteria: &Criteria) -> Self {
        Self::Text(format!("{criteria:?}"))
    }
}

/// A value a mock returns.
#[derive(Debug, Clone)]
pub enum Ret {
    Bool(bool),
    Text(String),
    Int(i64),
    Object(Option<Object>),
    Objects(Vec<Object>),
    Array(PhpArray),
    Params(Params),
    Rsm(Box<ResultSetMapping>),
    Entry(Option<CacheEntry>),
    Lock(Option<Lock>),
}

type Calls = Vec<(&'static str, Vec<Arg>)>;

/// The calls and returns of a mock.
#[derive(Debug, Default)]
pub struct Recorder {
    calls: Mutex<Calls>,
    returns: Mutex<HashMap<&'static str, VecDeque<Ret>>>,
    /// `method(...)->willReturn(...)` without `expects()`: every call.
    always: Mutex<HashMap<&'static str, Ret>>,
}

impl Recorder {
    fn record(&self, method: &'static str, args: Vec<Arg>) -> Option<Ret> {
        self.calls.lock().unwrap().push((method, args));
        if let Some(ret) = self
            .returns
            .lock()
            .unwrap()
            .get_mut(method)
            .and_then(VecDeque::pop_front)
        {
            return Some(ret);
        }
        self.always.lock().unwrap().get(method).cloned()
    }

    /// `->willReturn($value)` for the next call of the method.
    pub fn will_return(&self, method: &'static str, value: Ret) {
        self.returns
            .lock()
            .unwrap()
            .entry(method)
            .or_default()
            .push_back(value);
    }

    /// `->method($method)->willReturn($value)` for every call.
    pub fn always_return(&self, method: &'static str, value: Ret) {
        self.always.lock().unwrap().insert(method, value);
    }

    /// The arguments of each call of the method.
    pub fn calls(&self, method: &str) -> Vec<Vec<Arg>> {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .filter(|(m, _)| *m == method)
            .map(|(_, args)| args.clone())
            .collect()
    }

    /// `expects(self::once())->method($method)->with(...$args)`.
    pub fn assert_once_with(&self, method: &str, args: &[Arg]) {
        let calls = self.calls(method);
        assert_eq!(1, calls.len(), "{method} calls: {calls:?}");
        assert_eq!(args, calls[0].as_slice(), "{method}");
    }

    /// `expects(self::exactly($count))->method($method)`.
    pub fn assert_count(&self, method: &str, count: usize) {
        assert_eq!(count, self.calls(method).len(), "{method}");
    }
}

#[allow(
    clippy::needless_pass_by_value,
    reason = "takes what `record()` returns"
)]
fn ret_bool(ret: Option<Ret>) -> bool {
    matches!(ret, Some(Ret::Bool(true)))
}

/// `createMock(Region::class)`, or `createMock(ConcurrentRegion::class)`.
#[derive(Debug, Default)]
pub struct RegionMock {
    pub recorder: Recorder,
    concurrent: bool,
}

impl RegionMock {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// `createMock(ConcurrentRegion::class)`.
    pub fn concurrent() -> Arc<Self> {
        Arc::new(Self {
            recorder: Recorder::default(),
            concurrent: true,
        })
    }

    /// The key argument.
    pub fn key(key: &CacheKey) -> Arg {
        Arg::Text(format!("{key:?}"))
    }

    /// The entry argument.
    pub fn entry(entry: &CacheEntry) -> Arg {
        Arg::Text(format!("{entry:?}"))
    }
}

impl Region for RegionMock {
    fn get_name(&self) -> String {
        match self.recorder.record("getName", vec![]) {
            Some(Ret::Text(name)) => name,
            _ => String::new(),
        }
    }

    fn contains(&self, key: &CacheKey) -> Result<bool, Thrown> {
        Ok(ret_bool(
            self.recorder.record("contains", vec![Self::key(key)]),
        ))
    }

    fn get(&self, key: &CacheKey) -> Result<Option<CacheEntry>, Thrown> {
        Ok(match self.recorder.record("get", vec![Self::key(key)]) {
            Some(Ret::Entry(entry)) => entry,
            _ => None,
        })
    }

    fn get_multiple(
        &self,
        collection: &CollectionCacheEntry,
    ) -> Result<Option<MultipleEntries>, Thrown> {
        self.recorder
            .record("getMultiple", vec![Arg::Text(format!("{collection:?}"))]);
        Ok(None)
    }

    fn put(&self, key: &CacheKey, entry: CacheEntry, _lock: Option<&Lock>) -> Result<bool, Thrown> {
        Ok(ret_bool(
            self.recorder
                .record("put", vec![Self::key(key), Self::entry(&entry)]),
        ))
    }

    fn evict(&self, key: &CacheKey) -> Result<bool, Thrown> {
        Ok(ret_bool(
            self.recorder.record("evict", vec![Self::key(key)]),
        ))
    }

    fn evict_all(&self) -> Result<bool, Thrown> {
        Ok(ret_bool(self.recorder.record("evictAll", vec![])))
    }

    fn as_concurrent(&self) -> Option<&dyn ConcurrentRegion> {
        if self.concurrent { Some(self) } else { None }
    }
}

impl ConcurrentRegion for RegionMock {
    fn lock(&self, key: &CacheKey) -> Result<Option<Lock>, Thrown> {
        Ok(match self.recorder.record("lock", vec![Self::key(key)]) {
            Some(Ret::Lock(lock)) => lock,
            _ => None,
        })
    }

    fn unlock(&self, key: &CacheKey, _lock: &Lock) -> Result<bool, Thrown> {
        Ok(ret_bool(
            self.recorder.record("unlock", vec![Self::key(key)]),
        ))
    }
}

/// `createMock(EntityPersister::class)`.
#[derive(Debug)]
pub struct EntityPersisterMock {
    pub recorder: Recorder,
    class: Arc<ClassMetadata>,
}

impl EntityPersisterMock {
    pub fn new(class: Arc<ClassMetadata>) -> Arc<Self> {
        Arc::new(Self {
            recorder: Recorder::default(),
            class,
        })
    }

    fn text(&self, method: &'static str, args: Vec<Arg>) -> String {
        match self.recorder.record(method, args) {
            Some(Ret::Text(text)) => text,
            _ => String::new(),
        }
    }

    fn object(&self, method: &'static str, args: Vec<Arg>) -> Option<Object> {
        match self.recorder.record(method, args) {
            Some(Ret::Object(object)) => object,
            _ => None,
        }
    }

    fn objects(&self, method: &'static str, args: Vec<Arg>) -> Vec<Object> {
        match self.recorder.record(method, args) {
            Some(Ret::Objects(objects)) => objects,
            _ => Vec::new(),
        }
    }

    fn params(&self, method: &'static str, args: Vec<Arg>) -> Params {
        match self.recorder.record(method, args) {
            Some(Ret::Params(params)) => params,
            _ => (Vec::new(), Vec::new()),
        }
    }

    fn php_array(&self, method: &'static str, args: Vec<Arg>) -> PhpArray {
        match self.recorder.record(method, args) {
            Some(Ret::Array(array)) => array,
            Some(Ret::Objects(objects)) => PhpArray::list(objects.into_iter().map(Value::Object)),
            _ => PhpArray::new(),
        }
    }
}

fn criteria_arg(criteria: &PersisterCriteria<'_>) -> Arg {
    match criteria {
        PersisterCriteria::Array(array) => Arg::array(array),
        PersisterCriteria::Criteria(criteria) => Arg::criteria(criteria),
    }
}

impl EntityPersister for EntityPersisterMock {
    fn count_criteria<'a>(&'a self, criteria: &'a Criteria) -> Fut<'a, i64> {
        let ret = self.recorder.record("count", vec![Arg::criteria(criteria)]);
        Box::pin(async move {
            Ok(match ret {
                Some(Ret::Int(count)) => count,
                _ => 0,
            })
        })
    }

    fn load_criteria<'a>(&'a self, criteria: &'a Criteria) -> Fut<'a, Vec<Value>> {
        let objects = self.objects("loadCriteria", vec![Arg::criteria(criteria)]);
        Box::pin(async move { Ok(objects.into_iter().map(Value::Object).collect()) })
    }

    fn exists<'a>(
        &'a self,
        entity: &'a Object,
        extra_conditions: Option<&'a Criteria>,
    ) -> Fut<'a, bool> {
        CommitPersister::exists(self, entity, extra_conditions)
    }

    fn get_select_condition_statement_sql(
        &self,
        field: &str,
        value: &Value,
        assoc: Option<&AssociationMapping>,
        comparison: Option<&str>,
    ) -> Result<String, Thrown> {
        Ok(self.text(
            "getSelectConditionStatementSQL",
            vec![
                Arg::Value(Value::string(field)),
                Arg::Value(value.clone()),
                Arg::mapping(assoc),
                Arg::Value(comparison.map_or(Value::Null, Value::string)),
            ],
        ))
    }
}

impl CommitPersister for EntityPersisterMock {
    fn add_insert(&self, entity: &Object) {
        self.recorder.record("addInsert", vec![Arg::object(entity)]);
    }

    fn execute_inserts(&self) -> Fut<'_, ()> {
        self.recorder.record("executeInserts", vec![]);
        Box::pin(async { Ok(()) })
    }

    fn update<'a>(&'a self, entity: &'a Object) -> Fut<'a, ()> {
        self.recorder.record("update", vec![Arg::object(entity)]);
        Box::pin(async { Ok(()) })
    }

    fn delete<'a>(&'a self, entity: &'a Object) -> Fut<'a, bool> {
        let ret = ret_bool(self.recorder.record("delete", vec![Arg::object(entity)]));
        Box::pin(async move { Ok(ret) })
    }

    fn exists<'a>(
        &'a self,
        entity: &'a Object,
        extra_conditions: Option<&'a Criteria>,
    ) -> Fut<'a, bool> {
        let ret = ret_bool(self.recorder.record(
            "exists",
            vec![
                Arg::object(entity),
                Arg::Text(extra_conditions.map_or_else(String::new, |c| format!("{c:?}"))),
            ],
        ));
        Box::pin(async move { Ok(ret) })
    }
}

impl EntityPersisterInterface for EntityPersisterMock {
    fn get_class_metadata(&self) -> Arc<ClassMetadata> {
        self.recorder.record("getClassMetadata", vec![]);
        Arc::clone(&self.class)
    }

    fn get_result_set_mapping(&self) -> ResultSetMapping {
        match self.recorder.record("getResultSetMapping", vec![]) {
            Some(Ret::Rsm(rsm)) => *rsm,
            _ => ResultSetMapping::new(),
        }
    }

    fn get_inserts(&self) -> Vec<Object> {
        self.objects("getInserts", vec![])
    }

    fn get_select_sql(
        &self,
        criteria: PersisterCriteria<'_>,
        assoc: Option<&AssociationMapping>,
        lock_mode: Option<LockMode>,
        limit: Option<i64>,
        offset: Option<i64>,
        order_by: Option<&PhpArray>,
    ) -> Result<String, Thrown> {
        Ok(self.text(
            "getSelectSQL",
            vec![
                criteria_arg(&criteria),
                Arg::mapping(assoc),
                Arg::lock_mode(lock_mode),
                Arg::opt_int(limit),
                Arg::opt_int(offset),
                Arg::opt_array(order_by),
            ],
        ))
    }

    fn get_count_sql(&self, criteria: PersisterCriteria<'_>) -> Result<String, Thrown> {
        Ok(self.text("getCountSQL", vec![criteria_arg(&criteria)]))
    }

    fn get_insert_sql(&self) -> Result<String, Thrown> {
        Ok(self.text("getInsertSQL", vec![]))
    }

    fn expand_parameters(&self, criteria: &PhpArray) -> Result<Params, Thrown> {
        Ok(self.params("expandParameters", vec![Arg::array(criteria)]))
    }

    fn expand_criteria_parameters(&self, criteria: &Criteria) -> Result<Params, Thrown> {
        Ok(self.params("expandCriteriaParameters", vec![Arg::criteria(criteria)]))
    }

    fn get_owning_table(&self, field_name: &str) -> Result<String, Thrown> {
        Ok(self.text(
            "getOwningTable",
            vec![Arg::Value(Value::string(field_name))],
        ))
    }

    fn load<'a>(
        &'a self,
        criteria: &'a PhpArray,
        entity: Option<&'a Object>,
        assoc: Option<&'a AssociationMapping>,
        hints: Hints,
        lock_mode: Option<LockMode>,
        limit: Option<i64>,
        order_by: Option<&'a PhpArray>,
    ) -> Fut<'a, Option<Object>> {
        let ret = self.object(
            "load",
            vec![
                Arg::array(criteria),
                Arg::opt_object(entity),
                Arg::mapping(assoc),
                Arg::Text(format!("{hints:?}")),
                Arg::lock_mode(lock_mode),
                Arg::opt_int(limit),
                Arg::opt_array(order_by),
            ],
        );
        Box::pin(async move { Ok(ret) })
    }

    fn load_by_id<'a>(
        &'a self,
        identifier: &'a PhpArray,
        entity: Option<&'a Object>,
    ) -> Fut<'a, Option<Object>> {
        let ret = self.object(
            "loadById",
            vec![Arg::array(identifier), Arg::opt_object(entity)],
        );
        Box::pin(async move { Ok(ret) })
    }

    fn load_all<'a>(
        &'a self,
        criteria: &'a PhpArray,
        order_by: Option<&'a PhpArray>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Fut<'a, Vec<Object>> {
        let ret = self.objects(
            "loadAll",
            vec![
                Arg::array(criteria),
                Arg::opt_array(order_by),
                Arg::opt_int(limit),
                Arg::opt_int(offset),
            ],
        );
        Box::pin(async move { Ok(ret) })
    }

    fn load_one_to_one_entity<'a>(
        &'a self,
        assoc: &'a AssociationMapping,
        source_entity: &'a Object,
        identifier: Option<&'a PhpArray>,
    ) -> Fut<'a, Option<Object>> {
        let ret = self.object(
            "loadOneToOneEntity",
            vec![
                Arg::mapping(Some(assoc)),
                Arg::object(source_entity),
                Arg::opt_array(identifier),
            ],
        );
        Box::pin(async move { Ok(ret) })
    }

    fn refresh<'a>(
        &'a self,
        id: &'a PhpArray,
        entity: &'a Object,
        lock_mode: Option<LockMode>,
    ) -> Fut<'a, ()> {
        self.recorder.record(
            "refresh",
            vec![
                Arg::array(id),
                Arg::object(entity),
                Arg::lock_mode(lock_mode),
            ],
        );
        Box::pin(async { Ok(()) })
    }

    fn count<'a>(&'a self, criteria: &'a PhpArray) -> Fut<'a, i64> {
        let ret = self.recorder.record("count", vec![Arg::array(criteria)]);
        Box::pin(async move {
            Ok(match ret {
                Some(Ret::Int(count)) => count,
                _ => 0,
            })
        })
    }

    fn lock<'a>(&'a self, criteria: &'a PhpArray, lock_mode: LockMode) -> Fut<'a, ()> {
        self.recorder.record(
            "lock",
            vec![Arg::array(criteria), Arg::lock_mode(Some(lock_mode))],
        );
        Box::pin(async { Ok(()) })
    }

    fn get_many_to_many_collection<'a>(
        &'a self,
        assoc: &'a AssociationMapping,
        source_entity: &'a Object,
        offset: Option<i64>,
        limit: Option<i64>,
    ) -> Fut<'a, PhpArray> {
        let ret = self.php_array(
            "getManyToManyCollection",
            vec![
                Arg::mapping(Some(assoc)),
                Arg::object(source_entity),
                Arg::opt_int(offset),
                Arg::opt_int(limit),
            ],
        );
        Box::pin(async move { Ok(ret) })
    }

    fn load_many_to_many_collection<'a>(
        &'a self,
        assoc: &'a AssociationMapping,
        source_entity: &'a Object,
        collection: &'a Arc<PersistentCollection>,
    ) -> Fut<'a, Vec<Object>> {
        let ret = self.objects(
            "loadManyToManyCollection",
            vec![
                Arg::mapping(Some(assoc)),
                Arg::object(source_entity),
                Arg::Value(collection.value()),
            ],
        );
        Box::pin(async move { Ok(ret) })
    }

    fn get_one_to_many_collection<'a>(
        &'a self,
        assoc: &'a AssociationMapping,
        source_entity: &'a Object,
        offset: Option<i64>,
        limit: Option<i64>,
    ) -> Fut<'a, PhpArray> {
        let ret = self.php_array(
            "getOneToManyCollection",
            vec![
                Arg::mapping(Some(assoc)),
                Arg::object(source_entity),
                Arg::opt_int(offset),
                Arg::opt_int(limit),
            ],
        );
        Box::pin(async move { Ok(ret) })
    }

    fn load_one_to_many_collection<'a>(
        &'a self,
        assoc: &'a AssociationMapping,
        source_entity: &'a Object,
        collection: &'a Arc<PersistentCollection>,
    ) -> Fut<'a, Vec<Object>> {
        let ret = self.objects(
            "loadOneToManyCollection",
            vec![
                Arg::mapping(Some(assoc)),
                Arg::object(source_entity),
                Arg::Value(collection.value()),
            ],
        );
        Box::pin(async move { Ok(ret) })
    }
}

/// `createMock(CollectionPersister::class)`.
#[derive(Debug, Default)]
pub struct CollectionPersisterMock {
    pub recorder: Recorder,
}

impl CollectionPersisterMock {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// The collection argument (`identicalTo($collection)`).
    pub fn collection(collection: &Arc<PersistentCollection>) -> Arg {
        Arg::Text(format!("{:p}", Arc::as_ptr(collection)))
    }
}

impl CollectionPersister for CollectionPersisterMock {
    fn delete<'a>(&'a self, collection: &'a Arc<PersistentCollection>) -> Fut<'a, ()> {
        self.recorder
            .record("delete", vec![Self::collection(collection)]);
        Box::pin(async { Ok(()) })
    }

    fn update<'a>(&'a self, collection: &'a Arc<PersistentCollection>) -> Fut<'a, ()> {
        self.recorder
            .record("update", vec![Self::collection(collection)]);
        Box::pin(async { Ok(()) })
    }

    fn count<'a>(&'a self, collection: &'a Arc<PersistentCollection>) -> Fut<'a, i64> {
        let ret = self
            .recorder
            .record("count", vec![Self::collection(collection)]);
        Box::pin(async move {
            Ok(match ret {
                Some(Ret::Int(count)) => count,
                _ => 0,
            })
        })
    }

    fn slice<'a>(
        &'a self,
        collection: &'a Arc<PersistentCollection>,
        offset: i64,
        length: Option<i64>,
    ) -> Fut<'a, PhpArray> {
        let ret = self.recorder.record(
            "slice",
            vec![
                Self::collection(collection),
                Arg::Value(Value::Int(offset)),
                Arg::opt_int(length),
            ],
        );
        Box::pin(async move {
            Ok(match ret {
                Some(Ret::Array(array)) => array,
                Some(Ret::Objects(objects)) => {
                    PhpArray::list(objects.into_iter().map(Value::Object))
                }
                _ => PhpArray::new(),
            })
        })
    }

    fn contains<'a>(
        &'a self,
        collection: &'a Arc<PersistentCollection>,
        element: &'a Object,
    ) -> Fut<'a, bool> {
        let ret = ret_bool(self.recorder.record(
            "contains",
            vec![Self::collection(collection), Arg::object(element)],
        ));
        Box::pin(async move { Ok(ret) })
    }

    fn contains_key<'a>(
        &'a self,
        collection: &'a Arc<PersistentCollection>,
        key: &'a Key,
    ) -> Fut<'a, bool> {
        let ret = ret_bool(self.recorder.record(
            "containsKey",
            vec![Self::collection(collection), Arg::Value(key.to_value())],
        ));
        Box::pin(async move { Ok(ret) })
    }

    fn get<'a>(
        &'a self,
        collection: &'a Arc<PersistentCollection>,
        index: &'a Value,
    ) -> Fut<'a, Option<Object>> {
        let ret = self.recorder.record(
            "get",
            vec![Self::collection(collection), Arg::Value(index.clone())],
        );
        Box::pin(async move {
            Ok(match ret {
                Some(Ret::Object(object)) => object,
                _ => None,
            })
        })
    }

    fn load_criteria<'a>(
        &'a self,
        collection: &'a Arc<PersistentCollection>,
        criteria: &'a Criteria,
    ) -> Fut<'a, Vec<Value>> {
        let ret = self.recorder.record(
            "loadCriteria",
            vec![Self::collection(collection), Arg::criteria(criteria)],
        );
        Box::pin(async move {
            Ok(match ret {
                Some(Ret::Objects(objects)) => objects.into_iter().map(Value::Object).collect(),
                _ => Vec::new(),
            })
        })
    }
}
