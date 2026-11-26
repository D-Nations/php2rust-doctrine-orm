//! `SecondLevelCacheFunctionalTestCase`: an entity manager with the
//! second-level cache (a `DefaultCacheFactory` over a new `ArrayAdapter`,
//! and a `StatisticsCacheLogger`) on a fresh in-memory `SQLite` database
//! with the `cache` model set, its fixtures, and the models' constructors.
//!
//! PHP shares the cache pool between the tests of a run; here each test
//! has its own pool, as it has its own database.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/SecondLevelCacheFunctionalTestCase.php
//! Upstream: doctrine/orm tests/Tests/Models/Cache/Traveler.php tests/Tests/Models/Cache/Travel.php
//! Upstream: doctrine/orm tests/Tests/Models/Cache/TravelerProfile.php tests/Tests/Models/Cache/TravelerProfileInfo.php
//! Upstream: doctrine/orm tests/Tests/Models/Cache/Attraction.php tests/Tests/Models/Cache/Bar.php
//! Upstream: doctrine/orm tests/Tests/Models/Cache/Beach.php tests/Tests/Models/Cache/AttractionInfo.php
//! Upstream: doctrine/orm tests/Tests/Models/Cache/AttractionContactInfo.php tests/Tests/Models/Cache/AttractionLocationInfo.php
//! Upstream: doctrine/orm tests/Tests/Models/Cache/Person.php tests/Tests/Models/Cache/Address.php
//! Upstream: doctrine/orm tests/Tests/Models/Cache/Token.php tests/Tests/Models/Cache/Action.php
//! Upstream: doctrine/orm tests/Tests/Models/Cache/ComplexAction.php tests/Tests/Models/Cache/Login.php
//! Upstream: doctrine/orm tests/Tests/Models/Cache/Client.php

#![allow(dead_code, reason = "each test file uses a subset")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

use std::sync::Arc;

use php_value::datetime::{DateTime, DateTimeImmutable, TimeZone};
use php_value::{Object, PhpArray, Thrown, Value};
use php2rust_doctrine_orm::cache::logging::StatisticsCacheLogger;
use php2rust_doctrine_orm::cache::pool::ArrayAdapter;
use php2rust_doctrine_orm::cache::{Cache, CacheConfiguration, DefaultCacheFactory};
use php2rust_doctrine_orm::collection_object::ArrayCollectionObject;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::mapping::driver::AttributeDriver;
use php2rust_doctrine_orm::mapping::driver::attribute_driver::Paths;
use php2rust_doctrine_persistence::mapping::driver::ClassLocator;
use php2rust_doctrine_persistence::php_reflection::{self, ClassKind};

use crate::orm;
use crate::query_log::{QueryLog, logging_connection};
use crate::slc;

pub use crate::slc::{CITY, COUNTRY, RESTAURANT, STATE};

pub const TRAVELER: &str = "Doctrine\\Tests\\Models\\Cache\\Traveler";
pub const TRAVELER_PROFILE: &str = "Doctrine\\Tests\\Models\\Cache\\TravelerProfile";
pub const TRAVELER_PROFILE_INFO: &str = "Doctrine\\Tests\\Models\\Cache\\TravelerProfileInfo";
pub const TRAVEL: &str = "Doctrine\\Tests\\Models\\Cache\\Travel";
pub const ATTRACTION: &str = "Doctrine\\Tests\\Models\\Cache\\Attraction";
pub const BEACH: &str = "Doctrine\\Tests\\Models\\Cache\\Beach";
pub const BAR: &str = "Doctrine\\Tests\\Models\\Cache\\Bar";
pub const FLIGHT: &str = "Doctrine\\Tests\\Models\\Cache\\Flight";
pub const TOKEN: &str = "Doctrine\\Tests\\Models\\Cache\\Token";
pub const LOGIN: &str = "Doctrine\\Tests\\Models\\Cache\\Login";
pub const CLIENT: &str = "Doctrine\\Tests\\Models\\Cache\\Client";
pub const PERSON: &str = "Doctrine\\Tests\\Models\\Cache\\Person";
pub const ADDRESS: &str = "Doctrine\\Tests\\Models\\Cache\\Address";
pub const ACTION: &str = "Doctrine\\Tests\\Models\\Cache\\Action";
pub const COMPLEX_ACTION: &str = "Doctrine\\Tests\\Models\\Cache\\ComplexAction";
pub const ATTRACTION_INFO: &str = "Doctrine\\Tests\\Models\\Cache\\AttractionInfo";
pub const ATTRACTION_CONTACT_INFO: &str = "Doctrine\\Tests\\Models\\Cache\\AttractionContactInfo";
pub const ATTRACTION_LOCATION_INFO: &str = "Doctrine\\Tests\\Models\\Cache\\AttractionLocationInfo";

/// The `cache` model set.
pub const CACHE_MODELS: [&str; 22] = [
    COUNTRY,
    STATE,
    CITY,
    TRAVELER,
    TRAVELER_PROFILE_INFO,
    TRAVELER_PROFILE,
    TRAVEL,
    ATTRACTION,
    RESTAURANT,
    BEACH,
    BAR,
    FLIGHT,
    TOKEN,
    LOGIN,
    CLIENT,
    PERSON,
    ADDRESS,
    ACTION,
    COMPLEX_ACTION,
    ATTRACTION_INFO,
    ATTRACTION_CONTACT_INFO,
    ATTRACTION_LOCATION_INFO,
];

/// The classes of `tests/Tests/Models/Cache` and `Models/GeoNames`.
#[derive(Debug)]
pub struct FunctionalModelsLocator;

impl ClassLocator for FunctionalModelsLocator {
    fn get_class_names(&self) -> Result<Vec<String>, Thrown> {
        Ok(php_reflection::declared()
            .iter()
            .filter(|class| matches!(class.kind(), ClassKind::Class | ClassKind::Enum))
            .map(|class| class.name().to_owned())
            .filter(|name| {
                name.starts_with(slc::NS) || name.starts_with("Doctrine\\Tests\\Models\\GeoNames\\")
            })
            .collect())
    }
}

/// A test of `SecondLevelCacheFunctionalTestCase`: the entity manager,
/// its cache, the cache logger, the query log, and the fixtures loaded.
pub struct Fixture {
    pub em: Arc<EntityManager>,
    pub cache: Arc<dyn Cache>,
    pub logger: Arc<StatisticsCacheLogger>,
    pub log: Arc<QueryLog>,
    pub pool: Arc<ArrayAdapter>,
    /// `$this->secondLevelCacheFactory`.
    pub factory: Arc<DefaultCacheFactory>,
    pub people: Vec<Object>,
    pub addresses: Vec<Object>,
    pub countries: Vec<Object>,
    pub states: Vec<Object>,
    pub cities: Vec<Object>,
    pub travels: Vec<Object>,
    pub travelers: Vec<Object>,
    pub attractions: Vec<Object>,
    pub attractions_info: Vec<Object>,
    pub travelers_with_profile: Vec<Object>,
}

/// `setUp()`: `enableSecondLevelCache()`, `useModelSet('cache')` and
/// `parent::setUp()`.
pub async fn set_up() -> Fixture {
    set_up_with(&CACHE_MODELS).await
}

/// `setUp()` with more classes in the schema.
pub async fn set_up_with(classes: &[&str]) -> Fixture {
    let pool = Arc::new(ArrayAdapter::new());
    let cache_config = Arc::new(CacheConfiguration::new());
    let factory = Arc::new(DefaultCacheFactory::new(
        cache_config.get_regions_configuration(),
        pool.clone(),
    ));
    let logger = Arc::new(StatisticsCacheLogger::new());
    cache_config.set_cache_logger(logger.clone());
    cache_config.set_cache_factory(factory.clone());
    let (conn, log) = logging_connection();
    let em = orm::entity_manager_with(conn, move |c| {
        let locator: Arc<dyn ClassLocator> = Arc::new(FunctionalModelsLocator);
        c.set_metadata_driver_impl(Arc::new(
            AttributeDriver::new(Paths::Locator(locator), true).unwrap(),
        ));
        c.set_second_level_cache_enabled(true);
        c.set_second_level_cache_configuration(cache_config);
    });
    orm::create_schema(&em, classes).await;
    // $this->getQueryLog()->enable()
    log.reset_and_enable();
    let cache = em.get_cache().unwrap();
    Fixture {
        em,
        cache,
        logger,
        log,
        pool,
        factory,
        people: Vec::new(),
        addresses: Vec::new(),
        countries: Vec::new(),
        states: Vec::new(),
        cities: Vec::new(),
        travels: Vec::new(),
        travelers: Vec::new(),
        attractions: Vec::new(),
        attractions_info: Vec::new(),
        travelers_with_profile: Vec::new(),
    }
}

impl Fixture {
    /// `$this->getQueryLog()->reset()->enable()`.
    pub fn reset_query_log(&self) {
        self.log.reset_and_enable();
    }

    /// `assertQueryCount($expectedCount)`.
    #[track_caller]
    pub fn assert_query_count(&self, expected_count: usize) {
        assert_eq!(
            expected_count,
            self.log.queries().len(),
            "{:#?}",
            self.log.queries()
        );
    }

    /// `getCurrentQueryCount()`.
    pub fn get_current_query_count(&self) -> usize {
        self.log.queries().len()
    }

    async fn persist_all(&self, entities: &[Object]) {
        for entity in entities {
            self.em.persist(entity).unwrap();
        }
        self.em.flush().await.unwrap();
    }

    pub async fn load_fixtures_countries(&mut self) {
        let brazil = slc::country("Brazil");
        let germany = slc::country("Germany");
        self.countries.push(brazil.clone());
        self.countries.push(germany.clone());
        self.persist_all(&[brazil, germany]).await;
    }

    pub async fn load_fixtures_states(&mut self) {
        let saopaulo = slc::state_in("São Paulo", Some(&self.countries[0]));
        let rio = slc::state_in("Rio de janeiro", Some(&self.countries[0]));
        let berlin = slc::state_in("Berlin", Some(&self.countries[1]));
        let bavaria = slc::state_in("Bavaria", Some(&self.countries[1]));
        self.states.push(saopaulo.clone());
        self.states.push(rio.clone());
        self.states.push(bavaria.clone());
        self.states.push(berlin.clone());
        self.persist_all(&[saopaulo, rio, bavaria, berlin]).await;
    }

    pub async fn load_fixtures_cities(&mut self) {
        let saopaulo = slc::city("São Paulo", Some(&self.states[0]));
        let rio = slc::city("Rio de janeiro", Some(&self.states[0]));
        let berlin = slc::city("Berlin", Some(&self.states[1]));
        let munich = slc::city("Munich", Some(&self.states[1]));

        slc::add_city(&self.states[0], &saopaulo);
        slc::add_city(&self.states[0], &rio);
        slc::add_city(&self.states[1], &berlin);
        slc::add_city(&self.states[1], &berlin);

        self.cities.push(saopaulo.clone());
        self.cities.push(rio.clone());
        self.cities.push(munich.clone());
        self.cities.push(berlin.clone());
        self.persist_all(&[saopaulo, rio, munich, berlin]).await;
    }

    pub async fn load_fixtures_traveler(&mut self) {
        let t1 = traveler("Fabio Silva");
        let t2 = traveler("Doctrine Bot");
        self.travelers.push(t1.clone());
        self.travelers.push(t2.clone());
        self.persist_all(&[t1, t2]).await;
    }

    pub async fn load_fixtures_travelers_with_profile(&mut self) {
        let t1 = traveler("Test traveler 1");
        let t2 = traveler("Test traveler 2");
        let p1 = traveler_profile("First Traveler Profile");
        let p2 = traveler_profile("Second Traveler Profile");

        orm::set_obj(&t1, "profile", &p1);
        orm::set_obj(&t2, "profile", &p2);

        self.travelers_with_profile.push(t1.clone());
        self.travelers_with_profile.push(t2.clone());
        self.persist_all(&[p1, p2, t1, t2]).await;
    }

    pub async fn load_fixtures_travelers_profile_info(&mut self) {
        let p1 = orm::get_object(&self.travelers_with_profile[0], "profile").unwrap();
        let p2 = orm::get_object(&self.travelers_with_profile[1], "profile").unwrap();
        let i1 = traveler_profile_info(&p1, "First Profile Info ...");
        let i2 = traveler_profile_info(&p2, "Second  Profile Info ...");

        orm::set_obj(&p1, "info", &i1);
        orm::set_obj(&p2, "info", &i2);

        self.persist_all(&[i1, i2, p1, p2]).await;
    }

    pub async fn load_fixtures_travels(&mut self) {
        let t1 = travel(&self.travelers[0]);
        let t2 = travel(&self.travelers[1]);
        let t3 = travel(&self.travelers[1]);

        add_visited_city(&t1, &self.cities[0]);
        add_visited_city(&t1, &self.cities[1]);
        add_visited_city(&t1, &self.cities[2]);

        add_visited_city(&t2, &self.cities[1]);
        add_visited_city(&t2, &self.cities[3]);

        self.travels.push(t1.clone());
        self.travels.push(t2.clone());
        self.travels.push(t3.clone());
        self.persist_all(&[t1, t2, t3]).await;
    }

    pub async fn load_fixtures_attractions(&mut self) {
        self.attractions = vec![
            attraction(BAR, "Boteco São Bento", &self.cities[0]),
            attraction(BAR, "Prainha Paulista", &self.cities[0]),
            attraction(BEACH, "Copacabana", &self.cities[1]),
            attraction(BEACH, "Ipanema", &self.cities[1]),
            attraction(BAR, "Schneider Weisse", &self.cities[2]),
            attraction(RESTAURANT, "Reinstoff", &self.cities[3]),
            attraction(RESTAURANT, "Fischers Fritz", &self.cities[3]),
        ];

        slc::add_attraction(&self.cities[0], &self.attractions[0]);
        slc::add_attraction(&self.cities[0], &self.attractions[1]);
        slc::add_attraction(&self.cities[1], &self.attractions[2]);
        slc::add_attraction(&self.cities[1], &self.attractions[3]);
        slc::add_attraction(&self.cities[2], &self.attractions[4]);
        slc::add_attraction(&self.cities[3], &self.attractions[5]);
        slc::add_attraction(&self.cities[3], &self.attractions[6]);

        let attractions = self.attractions.clone();
        self.persist_all(&attractions).await;
    }

    pub async fn load_fixtures_attractions_info(&mut self) {
        self.attractions_info = vec![
            attraction_info(
                ATTRACTION_CONTACT_INFO,
                "fone",
                "0000-0000",
                &self.attractions[0],
            ),
            attraction_info(
                ATTRACTION_CONTACT_INFO,
                "fone",
                "1111-1111",
                &self.attractions[1],
            ),
            attraction_info(
                ATTRACTION_LOCATION_INFO,
                "address",
                "Some St 1",
                &self.attractions[2],
            ),
            attraction_info(
                ATTRACTION_LOCATION_INFO,
                "address",
                "Some St 2",
                &self.attractions[3],
            ),
        ];
        let infos = self.attractions_info.clone();
        self.persist_all(&infos).await;
    }

    pub async fn load_fixtures_person_with_address(&mut self) {
        let person1 = person("Guilherme Blanco");
        let address1 = address("Canada");
        orm::set_obj(&person1, "address", &address1);
        orm::set_obj(&address1, "person", &person1);

        let person2 = person("Marco Pivetta");
        let address2 = address("Germany");
        orm::set_obj(&person2, "address", &address2);
        orm::set_obj(&address2, "person", &person2);

        self.people.push(person1.clone());
        self.people.push(person2.clone());
        self.addresses.push(address1.clone());
        self.addresses.push(address2.clone());

        for person in &self.people {
            self.em.persist(person).unwrap();
        }
        for address in &self.addresses {
            self.em.persist(address).unwrap();
        }
        self.em.flush().await.unwrap();
    }

    pub fn get_entity_region(&self, class_name: &str) -> String {
        self.cache
            .get_entity_cache_region(class_name)
            .unwrap()
            .unwrap()
            .get_name()
    }

    pub fn get_collection_region(&self, class_name: &str, association: &str) -> String {
        self.cache
            .get_collection_cache_region(class_name, association)
            .unwrap()
            .unwrap()
            .get_name()
    }

    pub fn get_default_query_region_name(&self) -> String {
        self.cache
            .get_query_cache(None)
            .unwrap()
            .get_region()
            .get_name()
    }

    pub fn evict_regions(&self) {
        self.cache.evict_query_regions().unwrap();
        self.cache.evict_entity_regions().unwrap();
        self.cache.evict_collection_regions().unwrap();
    }

    /// `$this->cache->containsEntity($class, $entity->getId())`.
    pub fn contains_entity(&self, class_name: &str, entity: &Object) -> bool {
        self.cache
            .contains_entity(class_name, &orm::get(entity, "id"))
            .unwrap()
    }

    /// `$this->cache->containsCollection($class, $association, $owner->getId())`.
    pub fn contains_collection(&self, class_name: &str, association: &str, owner: &Object) -> bool {
        self.cache
            .contains_collection(class_name, association, &orm::get(owner, "id"))
            .unwrap()
    }

    /// `$this->_em->find($class, $id)`.
    pub async fn find(&self, class_name: &str, id: Value) -> Option<Object> {
        self.em.find(class_name, id).await.unwrap()
    }
}

/// `$entity->getId()`.
pub fn id(entity: &Object) -> Value {
    orm::get(entity, "id")
}

/// `$entity->getName()`.
pub fn name(entity: &Object) -> Value {
    orm::get(entity, "name")
}

/// `new Traveler($name)`.
pub fn traveler(name: &str) -> Object {
    let traveler = orm::new_object(TRAVELER);
    orm::set(&traveler, "name", Value::string(name));
    orm::set(
        &traveler,
        "travels",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    traveler
}

/// `new TravelerProfile($name)`.
pub fn traveler_profile(name: &str) -> Object {
    let profile = orm::new_object(TRAVELER_PROFILE);
    orm::set(&profile, "name", Value::string(name));
    orm::set(&profile, "info", Value::Null);
    profile
}

/// `new TravelerProfileInfo($profile, $description)`.
pub fn traveler_profile_info(profile: &Object, description: &str) -> Object {
    let info = orm::new_object(TRAVELER_PROFILE_INFO);
    orm::set_obj(&info, "profile", profile);
    orm::set(&info, "description", Value::string(description));
    info
}

/// `new Travel($traveler)`.
pub fn travel(traveler: &Object) -> Object {
    let travel = orm::new_object(TRAVEL);
    orm::set_obj(&travel, "traveler", traveler);
    orm::set(
        &travel,
        "createdAt",
        DateTime::object(DateTimeImmutable::now(TimeZone::Utc)),
    );
    orm::set(
        &travel,
        "visitedCities",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    travel
}

/// `$travel->addVisitedCity($city)`.
pub fn add_visited_city(travel: &Object, city: &Object) {
    slc::add_to(travel, "visitedCities", city);
}

/// `$traveler->addTravel($travel)`.
pub fn add_travel(traveler: &Object, travel: &Object) {
    slc::add_to(traveler, "travels", travel);
    let current = orm::get_object(travel, "traveler");
    if !current.is_some_and(|t| orm::same(&t, traveler)) {
        orm::set_obj(travel, "traveler", traveler);
    }
}

/// `new Bar|Beach|Restaurant($name, $city)`.
pub fn attraction(class: &str, name: &str, city: &Object) -> Object {
    let attraction = orm::new_object(class);
    orm::set(&attraction, "name", Value::string(name));
    orm::set_obj(&attraction, "city", city);
    orm::set(
        &attraction,
        "infos",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    attraction
}

/// `new AttractionContactInfo($fone, $attraction)` and
/// `new AttractionLocationInfo($address, $attraction)`.
pub fn attraction_info(class: &str, field: &str, value: &str, attraction: &Object) -> Object {
    let info = orm::new_object(class);
    // setAttraction(): $attraction->addInfo($this)
    orm::set_obj(&info, "attraction", attraction);
    // (a new info is not in the collection yet)
    slc::add_to(attraction, "infos", &info);
    orm::set(&info, field, Value::string(value));
    info
}

/// `new Person($name)`.
pub fn person(name: &str) -> Object {
    let person = orm::new_object(PERSON);
    orm::set(&person, "name", Value::string(name));
    person
}

/// `new Address($location)`.
pub fn address(location: &str) -> Object {
    let address = orm::new_object(ADDRESS);
    orm::set(&address, "location", Value::string(location));
    address
}

/// The entities of a query result.
pub fn entities(result: &Value) -> Vec<Object> {
    orm::list(result).iter().map(orm::obj).collect()
}

/// `assertInstanceOf($class, $object)`.
#[track_caller]
pub fn assert_instance_of(class: &str, object: &Object) {
    assert!(
        object.class() == class || php_reflection::is_a(object.class(), class),
        "{} is not a {class}",
        object.class()
    );
}

/// `$entity->$field->get($index)` of a collection property.
pub async fn get_at(entity: &Object, field: &str, index: i64) -> Object {
    orm::obj(
        &php2rust_doctrine_orm::collection_object::get(&orm::get(entity, field), index)
            .await
            .unwrap(),
    )
}

/// `$entity->$field->isInitialized()` of a persistent collection.
pub fn is_initialized(entity: &Object, field: &str) -> bool {
    php2rust_doctrine_orm::persistent_collection::PersistentCollection::from_value(&orm::get(
        entity, field,
    ))
    .unwrap()
    .is_initialized()
}

/// `new Login($name)`.
pub fn login(name: &str) -> Object {
    let login = orm::new_object(LOGIN);
    orm::set(&login, "name", Value::string(name));
    login
}

/// `$token->addLogin($login)`.
pub fn add_login(token: &Object, login: &Object) {
    slc::add_to(token, "logins", login);
    orm::set_obj(login, "token", token);
}

/// `count($entity->$field)`.
pub async fn count(entity: &Object, field: &str) -> usize {
    orm::count_of(entity, field).await
}

/// `$entity->$field` of a to-one association.
pub fn get_one(entity: &Object, field: &str) -> Object {
    orm::get_object(entity, field).unwrap()
}

/// `new Token($token, $client)`.
pub fn token(token: &str, client: Option<&Object>) -> Object {
    let entity = orm::new_object(TOKEN);
    orm::set(&entity, "token", Value::string(token));
    orm::set(
        &entity,
        "client",
        client.map_or(Value::Null, |c| Value::Object(c.clone())),
    );
    orm::set(
        &entity,
        "logins",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    // new DateTime(date('Y-m-d H:i:s', strtotime('+7 day')))
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX));
    orm::set(
        &entity,
        "expiresAt",
        DateTime::object(DateTimeImmutable::from_timestamp(
            now + 7 * 86_400,
            0,
            TimeZone::Utc,
        )),
    );
    entity
}

/// `new Action($name)`.
pub fn action(name: &str) -> Object {
    let action = orm::new_object(ACTION);
    orm::set(&action, "name", Value::string(name));
    orm::set(
        &action,
        "tokens",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    action
}

/// `new ComplexAction($action1, $action2, $name)`.
pub fn complex_action(action1: &Object, action2: &Object, name: &str) -> Object {
    let complex = orm::new_object(COMPLEX_ACTION);
    orm::set_obj(&complex, "action1", action1);
    orm::set_obj(&complex, "action2", action2);
    orm::set(&complex, "name", Value::string(name));
    orm::set(
        &complex,
        "tokens",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    complex
}

/// `$action->addToken($token)`: `$token->action = $this`.
pub fn add_token(action: &Object, token: &Object) {
    slc::add_to(action, "tokens", token);
    orm::set_obj(token, "action", action);
}

/// `$complexAction->addToken($token)`: `$token->complexAction = $this`.
pub fn add_complex_token(complex: &Object, token: &Object) {
    slc::add_to(complex, "tokens", token);
    orm::set_obj(token, "complexAction", complex);
}

/// `new Client($name)`.
pub fn client(name: &str) -> Object {
    let client = orm::new_object(CLIENT);
    orm::set(&client, "name", Value::string(name));
    client
}

impl Fixture {
    /// `$entity->$field` of a to-one association, the lazy object
    /// initialized (as a getter on it does).
    pub async fn load_one(&self, entity: &Object, field: &str) -> Object {
        let target = get_one(entity, field);
        self.em.initialize_object(&target).await.unwrap();
        target
    }
}
