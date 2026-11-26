//! Upstream: doctrine/orm tests/Tests/ORM/Cache/DefaultQueryCacheTest.php
//!
//! `CacheFactoryDefaultQueryCacheTest` is `QueryCacheTestFactory`: it
//! returns the test's query cache and region, a new
//! `TimestampRegionMock` for the timestamp region, and builds the rest as
//! `DefaultCacheFactory` does. `assertInstanceOf()` of a key or entry
//! class is its variant. A `getMultiple()` result holds no `false` or
//! `null`: the index is missing. `CacheEntryMock` is a
//! `CacheEntry::Other`. `getAssociationValue()` (called by reflection
//! upstream) is public.

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/cache_region_mock.rs"]
mod cache_region_mock;
#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;
#[path = "common/slc.rs"]
mod slc;

use std::sync::{Arc, Mutex};

use php_value::{Key, Object, PhpArray, Thrown, Value};
use php2rust_doctrine_orm::cache::factory;
use php2rust_doctrine_orm::cache::{
    Cache, CacheEntry, CacheFactory, CacheKey, CachedCollectionPersister, CachedEntityPersister,
    CollectionHydrator, DefaultCacheFactory, DefaultQueryCache, EntityCacheEntry, EntityHydrator,
    MODE_GET, MODE_NORMAL, MODE_PUT, MODE_REFRESH, QueryCache, QueryCacheEntry, QueryCacheKey,
    Region, TimestampRegion,
};
use php2rust_doctrine_orm::collection_object::ArrayCollectionObject;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::mapping::ordered_map::OrderedMap;
use php2rust_doctrine_orm::mapping::{AssociationMapping, ClassMetadata};
use php2rust_doctrine_orm::persisters::EntityPersisterInterface;
use php2rust_doctrine_orm::persisters::collection::CollectionPersister;
use php2rust_doctrine_orm::query::ResultSetMapping;
use php2rust_doctrine_orm::query::result_set_mapping_builder::ResultSetMappingBuilder;

use cache_region_mock::{CacheRegionMock, Return, TimestampRegionMock};

const BOOLEAN_MODEL: &str = "Doctrine\\Tests\\Models\\Generic\\BooleanModel";

/// `CacheFactoryDefaultQueryCacheTest`.
#[derive(Debug)]
struct QueryCacheTestFactory {
    inner: DefaultCacheFactory,
    query_cache: Mutex<Option<Arc<dyn QueryCache>>>,
    region: Arc<CacheRegionMock>,
}

impl CacheFactory for QueryCacheTestFactory {
    fn build_cached_entity_persister(
        &self,
        em: &Arc<EntityManager>,
        persister: Arc<dyn EntityPersisterInterface>,
        metadata: Arc<ClassMetadata>,
    ) -> Result<Arc<dyn CachedEntityPersister>, Thrown> {
        factory::build_cached_entity_persister(self, em, persister, metadata)
    }

    fn build_cached_collection_persister(
        &self,
        em: &Arc<EntityManager>,
        persister: Arc<dyn CollectionPersister>,
        mapping: &AssociationMapping,
    ) -> Result<Arc<dyn CachedCollectionPersister>, Thrown> {
        factory::build_cached_collection_persister(self, em, persister, mapping)
    }

    fn build_query_cache(
        &self,
        _em: &Arc<EntityManager>,
        _region_name: Option<&str>,
    ) -> Result<Arc<dyn QueryCache>, Thrown> {
        Ok(self.query_cache.lock().unwrap().clone().unwrap())
    }

    fn build_collection_hydrator(
        &self,
        em: &Arc<EntityManager>,
        mapping: &AssociationMapping,
    ) -> Arc<dyn CollectionHydrator> {
        self.inner.build_collection_hydrator(em, mapping)
    }

    fn build_entity_hydrator(
        &self,
        em: &Arc<EntityManager>,
        metadata: &Arc<ClassMetadata>,
    ) -> Arc<dyn EntityHydrator> {
        self.inner.build_entity_hydrator(em, metadata)
    }

    fn get_region(&self, _cache: &PhpArray) -> Result<Arc<dyn Region>, Thrown> {
        Ok(self.region.clone())
    }

    fn get_timestamp_region(&self) -> Arc<dyn TimestampRegion> {
        Arc::new(TimestampRegionMock::new())
    }

    fn create_cache(&self, em: &Arc<EntityManager>) -> Result<Arc<dyn Cache>, Thrown> {
        self.inner.create_cache(em)
    }
}

struct Fixture {
    slc: slc::Slc,
    region: Arc<CacheRegionMock>,
    query_cache: Arc<DefaultQueryCache>,
}

/// `setUp()`.
fn set_up() -> Fixture {
    let slc = slc::slc_entity_manager();
    let region = Arc::new(CacheRegionMock::new());
    let query_cache = Arc::new(DefaultQueryCache::new(&slc.em, region.clone()).unwrap());
    let cache_factory = Arc::new(QueryCacheTestFactory {
        inner: DefaultCacheFactory::new(
            slc.cache_config.get_regions_configuration(),
            slc.pool.clone(),
        ),
        query_cache: Mutex::new(Some(query_cache.clone())),
        region: region.clone(),
    });

    slc.em
        .get_configuration()
        .get_second_level_cache_configuration()
        .unwrap()
        .set_cache_factory(cache_factory);

    Fixture {
        slc,
        region,
        query_cache,
    }
}

impl Fixture {
    fn em(&self) -> &Arc<EntityManager> {
        &self.slc.em
    }

    /// `new ResultSetMappingBuilder($this->em)`.
    fn rsm(&self) -> ResultSetMappingBuilder {
        ResultSetMappingBuilder::new(self.em(), None)
    }

    fn register_managed(&self, entity: &Object, id: PhpArray, data: PhpArray) {
        self.em()
            .get_unit_of_work()
            .register_managed(entity, id, data)
            .unwrap();
    }

    fn set_id(&self, class: &str, entity: &Object, id: i64) {
        self.em()
            .get_class_metadata(class)
            .unwrap()
            .read()
            .set_field_value(entity, "id", Value::Int(id))
            .unwrap();
    }

    async fn put(&self, key: &QueryCacheKey, rsm: &ResultSetMapping, result: &[Object]) -> bool {
        self.try_put(key, rsm, result).await.unwrap()
    }

    async fn try_put(
        &self,
        key: &QueryCacheKey,
        rsm: &ResultSetMapping,
        result: &[Object],
    ) -> Result<bool, Thrown> {
        let result = PhpArray::list(result.iter().cloned().map(Value::Object));
        self.query_cache
            .put(key, rsm, &result, &PhpArray::new())
            .await
    }

    async fn get(&self, key: &QueryCacheKey, rsm: &ResultSetMapping) -> Option<PhpArray> {
        self.query_cache
            .get(key, rsm, &PhpArray::new())
            .await
            .unwrap()
    }

    /// `count($this->region->calls['put'])`, with `assertArrayHasKey('put', ...)`.
    fn put_calls(&self) -> Vec<cache_region_mock::Call> {
        self.region.calls("put").expect("the region's put calls")
    }

    /// The `Country` the query cache's region gets for each index.
    fn add_country_entries(&self, entries: &[(i64, &str)]) {
        let multiple = entries
            .iter()
            .enumerate()
            .map(|(index, (id, name))| {
                (
                    Key::Int(i64::try_from(index).unwrap()),
                    CacheEntry::Entity(EntityCacheEntry::new(
                        slc::COUNTRY,
                        slc::array(&[("id", Value::Int(*id)), ("name", Value::string(*name))]),
                    )),
                )
            })
            .collect();
        self.region
            .add_return("getMultiple", Return::Multiple(Some(multiple)));
    }
}

fn key() -> QueryCacheKey {
    QueryCacheKey::with("query.key1", 0, MODE_NORMAL, None)
}

fn key_with(lifetime: i64, cache_mode: i64) -> QueryCacheKey {
    QueryCacheKey::with("query.key1", lifetime, cache_mode, None)
}

fn renamed(columns: &[(&str, &str)]) -> OrderedMap<String> {
    let mut map = OrderedMap::new();
    for (field, column) in columns {
        map.insert(*field, (*column).to_owned());
    }
    map
}

fn is_entity_key(call: &cache_region_mock::Call) -> bool {
    matches!(call.key, Some(CacheKey::Entity(_)))
}

fn is_query_key(call: &cache_region_mock::Call) -> bool {
    matches!(call.key, Some(CacheKey::Query(_)))
}

/// `[['identifier' => ['id' => 1]], ['identifier' => ['id' => 2]]]`.
fn identifiers() -> PhpArray {
    PhpArray::list(
        [1, 2].map(|id| Value::Array(slc::array(&[("identifier", Value::Array(slc::id(id)))]))),
    )
}

fn query_entry(time: Option<f64>) -> CacheEntry {
    CacheEntry::Query(QueryCacheEntry::new(identifiers(), time))
}

fn name(entity: &Object) -> Value {
    orm::get(entity, "name")
}

fn object_at(array: &PhpArray, index: i64) -> Object {
    match array.get(&Key::Int(index)) {
        Some(Value::Object(object)) => object.clone(),
        other => panic!("not an object: {other:?}"),
    }
}

/// `new Country($name)`, `setFieldValue($entity, 'id', $i)` and
/// `registerManaged($entity, ['id' => $i], ['name' => $name])`.
fn managed_countries(f: &Fixture, count: i64) -> Vec<Object> {
    let mut result = Vec::new();
    for i in 0..count {
        let name = format!("Country {i}");
        let entity = slc::country(&name);
        result.push(entity.clone());

        f.set_id(slc::COUNTRY, &entity, i);
        f.register_managed(
            &entity,
            slc::id(i),
            slc::array(&[("name", Value::string(name))]),
        );
    }
    result
}

#[test]
fn testImplementQueryCache() {
    let f = set_up();
    let _: &dyn QueryCache = f.query_cache.as_ref();
}

#[test]
fn testGetRegion() {
    let f = set_up();
    let region: Arc<dyn Region> = f.region.clone();
    assert!(std::ptr::addr_eq(
        Arc::as_ptr(&region),
        Arc::as_ptr(&f.query_cache.get_region())
    ));
}

#[test]
fn testClearShouldEvictRegion() {
    let f = set_up();
    f.query_cache.clear().unwrap();

    assert_eq!(1, f.region.calls("evictAll").unwrap().len());
}

#[tokio::test]
async fn testPutBasicQueryResult() {
    let f = set_up();
    let key = key();
    let mut rsm = f.rsm();

    rsm.add_root_entity_from_class_metadata(slc::COUNTRY, "c", &OrderedMap::new(), None)
        .unwrap();

    let result = managed_countries(&f, 4);

    assert!(f.put(&key, &rsm, &result).await);
    let calls = f.put_calls();
    assert_eq!(5, calls.len());

    for call in &calls[..4] {
        assert!(is_entity_key(call));
        assert!(matches!(call.entry, Some(CacheEntry::Entity(_))));
    }
    assert!(is_query_key(&calls[4]));
    assert!(matches!(calls[4].entry, Some(CacheEntry::Query(_))));
}

#[tokio::test]
async fn testPutToOneAssociationQueryResult() {
    let f = set_up();
    let mut result = Vec::new();
    let key = key();
    let mut rsm = f.rsm();

    rsm.add_root_entity_from_class_metadata(slc::CITY, "c", &OrderedMap::new(), None)
        .unwrap();
    rsm.add_joined_entity_from_class_metadata(
        slc::STATE,
        "s",
        "c",
        "state",
        &renamed(&[("id", "state_id"), ("name", "state_name")]),
        None,
    )
    .unwrap();

    for i in 0..4 {
        let state = slc::state(&format!("State {i}"));
        let city = slc::city(&format!("City {i}"), Some(&state));
        result.push(city.clone());

        f.set_id(slc::CITY, &city, i);
        f.set_id(slc::STATE, &state, i * 2);

        f.register_managed(
            &state,
            slc::array(&[("id", orm::get(&state, "id"))]),
            slc::array(&[("name", name(&city))]),
        );
        f.register_managed(
            &city,
            slc::array(&[("id", orm::get(&city, "id"))]),
            slc::array(&[("name", name(&city)), ("state", Value::Object(state))]),
        );
    }

    assert!(f.put(&key, &rsm, &result).await);
    let calls = f.put_calls();
    assert_eq!(9, calls.len());

    for call in &calls[..8] {
        assert!(is_entity_key(call));
    }
    assert!(is_query_key(&calls[8]));
}

#[tokio::test]
async fn testPutToOneAssociation2LevelsQueryResult() {
    let f = set_up();
    let mut result = Vec::new();
    let key = key();
    let mut rsm = f.rsm();

    rsm.add_root_entity_from_class_metadata(slc::CITY, "c", &OrderedMap::new(), None)
        .unwrap();
    rsm.add_joined_entity_from_class_metadata(
        slc::STATE,
        "s",
        "c",
        "state",
        &renamed(&[("id", "state_id"), ("name", "state_name")]),
        None,
    )
    .unwrap();
    rsm.add_joined_entity_from_class_metadata(
        slc::COUNTRY,
        "co",
        "s",
        "country",
        &renamed(&[("id", "country_id"), ("name", "country_name")]),
        None,
    )
    .unwrap();

    for i in 0..4 {
        let country = slc::country(&format!("Country {i}"));
        let state = slc::state_in(&format!("State {i}"), Some(&country));
        let city = slc::city(&format!("City {i}"), Some(&state));

        result.push(city.clone());

        f.set_id(slc::CITY, &city, i);
        f.set_id(slc::STATE, &state, i * 2);
        f.set_id(slc::COUNTRY, &country, i * 3);

        f.register_managed(
            &country,
            slc::array(&[("id", orm::get(&country, "id"))]),
            slc::array(&[("name", name(&country))]),
        );
        f.register_managed(
            &state,
            slc::array(&[("id", orm::get(&state, "id"))]),
            slc::array(&[("name", name(&state)), ("country", Value::Object(country))]),
        );
        f.register_managed(
            &city,
            slc::array(&[("id", orm::get(&city, "id"))]),
            slc::array(&[("name", name(&city)), ("state", Value::Object(state))]),
        );
    }

    assert!(f.put(&key, &rsm, &result).await);
    let calls = f.put_calls();
    assert_eq!(13, calls.len());

    for call in &calls[..12] {
        assert!(is_entity_key(call));
    }
    assert!(is_query_key(&calls[12]));
}

#[tokio::test]
async fn testPutToOneAssociationNullQueryResult() {
    let f = set_up();
    let mut result = Vec::new();
    let key = key();
    let mut rsm = f.rsm();

    rsm.add_root_entity_from_class_metadata(slc::CITY, "c", &OrderedMap::new(), None)
        .unwrap();
    rsm.add_joined_entity_from_class_metadata(
        slc::STATE,
        "s",
        "c",
        "state",
        &renamed(&[("id", "state_id"), ("name", "state_name")]),
        None,
    )
    .unwrap();

    for i in 0..4 {
        let city = slc::city(&format!("City {i}"), None);
        result.push(city.clone());

        f.set_id(slc::CITY, &city, i);

        f.register_managed(
            &city,
            slc::array(&[("id", orm::get(&city, "id"))]),
            slc::array(&[("name", name(&city)), ("state", Value::Null)]),
        );
    }

    assert!(f.put(&key, &rsm, &result).await);
    let calls = f.put_calls();
    assert_eq!(5, calls.len());

    for call in &calls[..4] {
        assert!(is_entity_key(call));
    }
    assert!(is_query_key(&calls[4]));
}

/// The states with two cities each of `testPutToManyAssociationQueryResult`
/// and `testCancelPutToManyAssociationQueryResult`.
fn state_with_cities(f: &Fixture, name_: &str, id: i64, city_ids: (i64, i64)) -> Object {
    let state = slc::state(name_);
    let city1 = slc::city("City 1", Some(&state));
    let city2 = slc::city("City 2", Some(&state));

    f.set_id(slc::CITY, &city1, city_ids.0);
    f.set_id(slc::CITY, &city2, city_ids.1);
    f.set_id(slc::STATE, &state, id);

    slc::add_city(&state, &city1);
    slc::add_city(&state, &city2);

    for city in [&city1, &city2] {
        f.register_managed(
            city,
            slc::array(&[("id", orm::get(city, "id"))]),
            slc::array(&[
                ("name", name(city)),
                ("state", Value::Object(state.clone())),
            ]),
        );
    }
    f.register_managed(
        &state,
        slc::array(&[("id", orm::get(&state, "id"))]),
        slc::array(&[
            ("name", name(&state)),
            ("cities", orm::get(&state, "cities")),
        ]),
    );
    state
}

fn to_many_rsm(f: &Fixture) -> ResultSetMappingBuilder {
    let mut rsm = f.rsm();
    rsm.add_root_entity_from_class_metadata(slc::STATE, "s", &OrderedMap::new(), None)
        .unwrap();
    rsm.add_joined_entity_from_class_metadata(
        slc::CITY,
        "c",
        "s",
        "cities",
        &renamed(&[("id", "c_id"), ("name", "c_name")]),
        None,
    )
    .unwrap();
    rsm
}

#[tokio::test]
async fn testPutToManyAssociationQueryResult() {
    let f = set_up();
    let mut result = Vec::new();
    let key = key();
    let rsm = to_many_rsm(&f);

    for i in 0..4 {
        result.push(state_with_cities(
            &f,
            &format!("State {i}"),
            i,
            (i + 11, i + 22),
        ));
    }

    assert!(f.put(&key, &rsm, &result).await);
    assert_eq!(13, f.put_calls().len());
}

#[tokio::test]
async fn testGetBasicQueryResult() {
    let f = set_up();
    let mut rsm = f.rsm();
    let key = key();

    f.region
        .add_return("get", Return::Entry(Some(query_entry(None))));

    f.add_country_entries(&[(1, "Foo"), (2, "Bar")]);

    rsm.add_root_entity_from_class_metadata(slc::COUNTRY, "c", &OrderedMap::new(), None)
        .unwrap();

    let result = f.get(&key, &rsm).await.unwrap();

    assert_eq!(2, result.len());
    let (first, second) = (object_at(&result, 0), object_at(&result, 1));
    assert_eq!(slc::COUNTRY, first.class());
    assert_eq!(slc::COUNTRY, second.class());
    assert_eq!(Value::Int(1), orm::get(&first, "id"));
    assert_eq!(Value::Int(2), orm::get(&second, "id"));
    assert_eq!(Value::string("Foo"), name(&first));
    assert_eq!(Value::string("Bar"), name(&second));
}

#[tokio::test]
async fn testGetWithAssociation() {
    let f = set_up();
    let mut rsm = f.rsm();
    let key = key();

    f.region
        .add_return("get", Return::Entry(Some(query_entry(None))));

    f.add_country_entries(&[(1, "Foo"), (2, "Bar")]);

    rsm.add_root_entity_from_class_metadata(slc::COUNTRY, "c", &OrderedMap::new(), None)
        .unwrap();

    let result = f.get(&key, &rsm).await.unwrap();

    assert_eq!(2, result.len());
    let (first, second) = (object_at(&result, 0), object_at(&result, 1));
    assert_eq!(slc::COUNTRY, first.class());
    assert_eq!(slc::COUNTRY, second.class());
    assert_eq!(Value::Int(1), orm::get(&first, "id"));
    assert_eq!(Value::Int(2), orm::get(&second, "id"));
    assert_eq!(Value::string("Foo"), name(&first));
    assert_eq!(Value::string("Bar"), name(&second));
}

#[tokio::test]
async fn testGetWithAssociationCacheMiss() {
    let f = set_up();
    let mut rsm = f.rsm();
    let key = key();

    f.region
        .add_return("get", Return::Entry(Some(query_entry(None))));

    // `[new EntityCacheEntry(Country::class, [...]), false]`.
    f.add_country_entries(&[(1, "Foo")]);

    rsm.add_root_entity_from_class_metadata(slc::COUNTRY, "c", &OrderedMap::new(), None)
        .unwrap();

    assert!(f.get(&key, &rsm).await.is_none());
}

#[tokio::test]
async fn testCancelPutResultIfEntityPutFails() {
    let f = set_up();
    let key = key();
    let mut rsm = f.rsm();

    rsm.add_root_entity_from_class_metadata(slc::COUNTRY, "c", &OrderedMap::new(), None)
        .unwrap();

    let result = managed_countries(&f, 4);

    f.region.add_return("put", Return::Bool(false));

    assert!(!f.put(&key, &rsm, &result).await);
    assert_eq!(1, f.put_calls().len());
}

#[tokio::test]
async fn testCancelPutResultIfAssociationEntityPutFails() {
    let f = set_up();
    let key = key();
    let mut rsm = f.rsm();

    rsm.add_root_entity_from_class_metadata(slc::CITY, "c", &OrderedMap::new(), None)
        .unwrap();
    rsm.add_joined_entity_from_class_metadata(
        slc::STATE,
        "s",
        "c",
        "state",
        &renamed(&[("id", "state_id"), ("name", "state_name")]),
        None,
    )
    .unwrap();

    let state = slc::state("State 1");
    let city = slc::city("City 2", Some(&state));
    let result = vec![city.clone()];

    f.set_id(slc::CITY, &city, 1);
    f.set_id(slc::STATE, &state, 11);

    f.register_managed(
        &state,
        slc::array(&[("id", orm::get(&state, "id"))]),
        slc::array(&[("name", name(&city))]),
    );
    f.register_managed(
        &city,
        slc::array(&[("id", orm::get(&city, "id"))]),
        slc::array(&[("name", name(&city)), ("state", Value::Object(state))]),
    );

    f.region.add_return("put", Return::Bool(true)); // put root entity
    f.region.add_return("put", Return::Bool(false)); // association fails

    assert!(!f.put(&key, &rsm, &result).await);
}

#[tokio::test]
async fn testCancelPutToManyAssociationQueryResult() {
    let f = set_up();
    let key = key();
    let rsm = to_many_rsm(&f);

    let state = state_with_cities(&f, "State", 1, (11, 22));
    let result = vec![state];

    f.region.add_return("put", Return::Bool(true)); // put root entity
    f.region.add_return("put", Return::Bool(false)); // collection association fails

    assert!(!f.put(&key, &rsm, &result).await);
    assert_eq!(2, f.put_calls().len());
}

#[tokio::test]
async fn testIgnoreCacheNonGetMode() {
    let f = set_up();
    let mut rsm = f.rsm();
    let key = key_with(0, MODE_PUT);

    rsm.add_root_entity_from_class_metadata(slc::COUNTRY, "c", &OrderedMap::new(), None)
        .unwrap();

    f.region
        .add_return("get", Return::Entry(Some(query_entry(None))));

    assert!(f.get(&key, &rsm).await.is_none());
}

#[tokio::test]
async fn testIgnoreCacheNonPutMode() {
    let f = set_up();
    let mut rsm = f.rsm();
    let key = key_with(0, MODE_GET);

    rsm.add_root_entity_from_class_metadata(slc::COUNTRY, "c", &OrderedMap::new(), None)
        .unwrap();

    let result = managed_countries(&f, 4);

    assert!(!f.put(&key, &rsm, &result).await);
}

#[tokio::test]
async fn testRefreshModeWritesToCache() {
    let f = set_up();
    let mut rsm = f.rsm();
    let key = key_with(0, MODE_REFRESH);

    rsm.add_root_entity_from_class_metadata(slc::COUNTRY, "c", &OrderedMap::new(), None)
        .unwrap();

    let result = managed_countries(&f, 2);

    assert!(f.put(&key, &rsm, &result).await);
    assert!(f.region.calls("put").is_some());
}

#[tokio::test]
async fn testGetShouldIgnoreOldQueryCacheEntryResult() {
    let f = set_up();
    let mut rsm = f.rsm();
    let key = key_with(50, MODE_NORMAL);
    let entry = query_entry(Some(php_value::clock::microtime() - 100.0));

    f.region.add_return("get", Return::Entry(Some(entry)));

    f.add_country_entries(&[(1, "Foo"), (2, "Bar")]);

    rsm.add_root_entity_from_class_metadata(slc::COUNTRY, "c", &OrderedMap::new(), None)
        .unwrap();

    assert!(f.get(&key, &rsm).await.is_none());
}

#[tokio::test]
async fn testGetShouldIgnoreNonQueryCacheEntryResult() {
    let f = set_up();
    let mut rsm = f.rsm();
    let key = key();
    let entry = CacheEntry::Other {
        class: "Doctrine\\Tests\\Mocks\\CacheEntryMock".to_owned(),
        data: identifiers(),
    };

    f.region.add_return("get", Return::Entry(Some(entry)));

    f.add_country_entries(&[(1, "Foo"), (2, "Bar")]);

    rsm.add_root_entity_from_class_metadata(slc::COUNTRY, "c", &OrderedMap::new(), None)
        .unwrap();

    assert!(f.get(&key, &rsm).await.is_none());
}

#[tokio::test]
async fn testGetShouldIgnoreMissingEntityQueryCacheEntry() {
    let f = set_up();
    let mut rsm = f.rsm();
    let key = key();

    f.region
        .add_return("get", Return::Entry(Some(query_entry(None))));
    // `[null]`.
    f.region
        .add_return("getMultiple", Return::Multiple(Some(Vec::new())));

    rsm.add_root_entity_from_class_metadata(slc::COUNTRY, "c", &OrderedMap::new(), None)
        .unwrap();

    assert!(f.get(&key, &rsm).await.is_none());
}

#[test]
fn testGetAssociationValue() {
    let f = set_up();
    let mut rsm = f.rsm();

    let germany = slc::country("Germany");
    let bavaria = slc::state_in("Bavaria", Some(&germany));
    let wurzburg = slc::city("Würzburg", Some(&bavaria));
    let munich = slc::city("Munich", Some(&bavaria));

    slc::add_city(&bavaria, &munich);
    slc::add_city(&bavaria, &wurzburg);

    slc::add_attraction(&munich, &slc::restaurant("Reinstoff", &munich));
    slc::add_attraction(&munich, &slc::restaurant("Schneider Weisse", &munich));
    slc::add_attraction(&wurzburg, &slc::restaurant("Fischers Fritz", &wurzburg));

    rsm.add_root_entity_from_class_metadata(slc::STATE, "s", &OrderedMap::new(), None)
        .unwrap();
    rsm.add_joined_entity_from_class_metadata(
        slc::CITY,
        "c",
        "s",
        "cities",
        &renamed(&[("id", "c_id"), ("name", "c_name")]),
        None,
    )
    .unwrap();
    rsm.add_joined_entity_from_class_metadata(
        slc::RESTAURANT,
        "a",
        "c",
        "attractions",
        &renamed(&[("id", "a_id"), ("name", "a_name")]),
        None,
    )
    .unwrap();

    let cities = f
        .query_cache
        .get_association_value(f.em(), &rsm, "c", &bavaria)
        .unwrap()
        .unwrap();
    let attractions = f
        .query_cache
        .get_association_value(f.em(), &rsm, "a", &bavaria)
        .unwrap()
        .unwrap();

    let collection = |value: &Value| {
        ArrayCollectionObject::from_value(value)
            .expect("a Collection")
            .lock()
            .to_array()
    };
    let Value::Array(attractions) = attractions else {
        panic!("not an array: {attractions:?}");
    };

    assert_eq!(2, collection(&cities).len());
    assert_eq!(2, attractions.len());

    assert_eq!(2, collection(attractions.get(&Key::Int(0)).unwrap()).len());
    assert_eq!(1, collection(attractions.get(&Key::Int(1)).unwrap()).len());
}

/// `expectException(CacheException::class)`: the class or one of its
/// subclasses (`FeatureNotImplemented`, `NonCacheableEntity`).
fn assert_cache_exception(thrown: &Thrown, message: &str) {
    assert!(
        [
            "Doctrine\\ORM\\Cache\\Exception\\CacheException",
            "Doctrine\\ORM\\Cache\\Exception\\FeatureNotImplemented",
            "Doctrine\\ORM\\Cache\\Exception\\NonCacheableEntity",
        ]
        .contains(&thrown.class.as_str()),
        "{}",
        thrown.class
    );
    assert_eq!(message, thrown.message.to_string());
}

#[tokio::test]
async fn testScalarResultException() {
    let f = set_up();
    let key = key();
    let mut rsm = f.rsm();

    rsm.add_scalar_result("id", "u", "integer");

    let thrown = f.try_put(&key, &rsm, &[]).await.unwrap_err();
    assert_cache_exception(
        &thrown,
        "Second level cache does not support scalar results.",
    );
}

#[tokio::test]
async fn testSupportMultipleRootEntitiesException() {
    let f = set_up();
    let key = key();
    let mut rsm = f.rsm();

    rsm.add_entity_result(slc::CITY, "e1", None);
    rsm.add_entity_result(slc::STATE, "e2", None);

    let thrown = f.try_put(&key, &rsm, &[]).await.unwrap_err();
    assert_cache_exception(
        &thrown,
        "Second level cache does not support multiple root entities.",
    );
}

#[tokio::test]
async fn testNotCacheableEntityException() {
    let f = set_up();
    let key = key();
    let mut rsm = f.rsm();
    let mut result = Vec::new();
    rsm.add_root_entity_from_class_metadata(BOOLEAN_MODEL, "c", &OrderedMap::new(), None)
        .unwrap();

    for i in 0..4 {
        let entity = orm::new_object(BOOLEAN_MODEL);
        let boolean = i % 2 == 0;

        orm::set(&entity, "id", Value::Int(i));
        orm::set(&entity, "booleanField", Value::Bool(boolean));
        result.push(entity.clone());

        f.register_managed(
            &entity,
            slc::id(i),
            slc::array(&[("booleanField", Value::Bool(boolean))]),
        );
    }

    let thrown = f.try_put(&key, &rsm, &result).await.unwrap_err();
    assert_cache_exception(
        &thrown,
        "Entity \"Doctrine\\Tests\\Models\\Generic\\BooleanModel\" not configured as part of the second-level cache.",
    );
}
