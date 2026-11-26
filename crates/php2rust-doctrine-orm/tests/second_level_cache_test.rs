//! Upstream: doctrine/orm tests/Tests/ORM/Functional/SecondLevelCacheTest.php
//!
//! `ListenerSecondLevelCacheTest` throws a `RuntimeException` with its
//! message for the events it has.

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

use std::sync::Arc;

use php_value::{Thrown, Value};
use php2rust_doctrine_event_manager::{
    EventArgs, EventListener, EventListenerRegistry as _, ListenerError,
};
use php2rust_doctrine_orm::events;

use slc_functional::{COUNTRY, STATE, id, name, set_up};

/// `ListenerSecondLevelCacheTest`: a listener that throws for its events.
struct ListenerSecondLevelCacheTest {
    callbacks: Vec<(&'static str, &'static str)>,
}

impl EventListener for ListenerSecondLevelCacheTest {
    fn call(&self, event_name: &str, _event_args: &dyn EventArgs) -> Result<(), ListenerError> {
        for (event, message) in &self.callbacks {
            if *event == event_name {
                return Err(Box::new(Thrown::new("RuntimeException", *message, 0)));
            }
        }
        Ok(())
    }
}

fn add_listener(
    em: &php2rust_doctrine_orm::entity_manager::EntityManager,
    event: &'static str,
    message: &'static str,
) {
    let listener = Arc::new(ListenerSecondLevelCacheTest {
        callbacks: vec![(event, message)],
    });
    em.with_event_manager(|evm| evm.add_event_listener(&[event], listener));
}

#[tokio::test]
async fn testPutOnPersist() {
    let mut f = set_up().await;
    f.load_fixtures_countries().await;
    f.em.clear().unwrap();

    assert!(f.contains_entity(COUNTRY, &f.countries[0]));
    assert!(f.contains_entity(COUNTRY, &f.countries[1]));
    assert_eq!(2, f.logger.get_put_count());
    assert_eq!(
        2,
        f.logger.get_region_put_count(&f.get_entity_region(COUNTRY))
    );
}

#[tokio::test]
async fn testPutAndLoadEntities() {
    let mut f = set_up().await;
    f.load_fixtures_countries().await;
    f.em.clear().unwrap();

    assert_eq!(2, f.logger.get_put_count());
    assert_eq!(
        2,
        f.logger.get_region_put_count(&f.get_entity_region(COUNTRY))
    );

    f.cache.evict_entity_region(COUNTRY).unwrap();

    assert!(!f.contains_entity(COUNTRY, &f.countries[0]));
    assert!(!f.contains_entity(COUNTRY, &f.countries[1]));

    let c1 = f.find(COUNTRY, id(&f.countries[0])).await.unwrap();
    let c2 = f.find(COUNTRY, id(&f.countries[1])).await.unwrap();

    assert!(f.contains_entity(COUNTRY, &f.countries[0]));
    assert!(f.contains_entity(COUNTRY, &f.countries[1]));

    assert_eq!(COUNTRY, c1.class());
    assert_eq!(COUNTRY, c2.class());

    assert_eq!(id(&f.countries[0]), id(&c1));
    assert_eq!(name(&f.countries[0]), name(&c1));

    assert_eq!(id(&f.countries[1]), id(&c2));
    assert_eq!(name(&f.countries[1]), name(&c2));

    f.em.clear().unwrap();

    f.reset_query_log();

    let c3 = f.find(COUNTRY, id(&f.countries[0])).await.unwrap();
    let c4 = f.find(COUNTRY, id(&f.countries[1])).await.unwrap();

    f.assert_query_count(0);
    assert_eq!(2, f.logger.get_hit_count());
    assert_eq!(
        2,
        f.logger.get_region_hit_count(&f.get_entity_region(COUNTRY))
    );

    assert_eq!(COUNTRY, c3.class());
    assert_eq!(COUNTRY, c4.class());

    assert_eq!(id(&c1), id(&c3));
    assert_eq!(name(&c1), name(&c3));

    assert_eq!(id(&c2), id(&c4));
    assert_eq!(name(&c2), name(&c4));
}

#[tokio::test]
async fn testRemoveEntities() {
    let mut f = set_up().await;
    f.load_fixtures_countries().await;
    f.em.clear().unwrap();

    assert_eq!(2, f.logger.get_put_count());

    f.cache.evict_entity_region(COUNTRY).unwrap();
    f.logger.clear_region_stats(&f.get_entity_region(COUNTRY));

    assert!(!f.contains_entity(COUNTRY, &f.countries[0]));
    assert!(!f.contains_entity(COUNTRY, &f.countries[1]));

    let c1 = f.find(COUNTRY, id(&f.countries[0])).await.unwrap();
    let c2 = f.find(COUNTRY, id(&f.countries[1])).await.unwrap();

    assert_eq!(2, f.logger.get_put_count());
    assert_eq!(2, f.logger.get_miss_count());

    assert!(f.contains_entity(COUNTRY, &f.countries[0]));
    assert!(f.contains_entity(COUNTRY, &f.countries[1]));

    assert_eq!(COUNTRY, c1.class());
    assert_eq!(COUNTRY, c2.class());

    assert_eq!(id(&f.countries[0]), id(&c1));
    assert_eq!(name(&f.countries[0]), name(&c1));

    assert_eq!(id(&f.countries[1]), id(&c2));
    assert_eq!(name(&f.countries[1]), name(&c2));

    f.em.remove(&c1).await.unwrap();
    f.em.remove(&c2).await.unwrap();
    f.em.flush().await.unwrap();
    f.em.clear().unwrap();

    assert!(!f.contains_entity(COUNTRY, &f.countries[0]));
    assert!(!f.contains_entity(COUNTRY, &f.countries[1]));

    assert!(f.find(COUNTRY, id(&f.countries[0])).await.is_none());
    assert!(f.find(COUNTRY, id(&f.countries[1])).await.is_none());

    assert_eq!(2, f.logger.get_put_count());
    assert_eq!(2, f.logger.get_miss_count());
}

#[tokio::test]
async fn testUpdateEntities() {
    let mut f = set_up().await;
    f.load_fixtures_countries().await;
    f.load_fixtures_states().await;
    f.em.clear().unwrap();

    assert_eq!(6, f.logger.get_put_count());
    assert_eq!(
        2,
        f.logger.get_region_put_count(&f.get_entity_region(COUNTRY))
    );
    assert_eq!(
        4,
        f.logger.get_region_put_count(&f.get_entity_region(STATE))
    );

    f.cache.evict_entity_region(STATE).unwrap();
    f.logger.clear_region_stats(&f.get_entity_region(STATE));

    assert!(!f.contains_entity(STATE, &f.states[0]));
    assert!(!f.contains_entity(STATE, &f.states[1]));

    let s1 = f.find(STATE, id(&f.states[0])).await.unwrap();
    let s2 = f.find(STATE, id(&f.states[1])).await.unwrap();

    assert_eq!(4, f.logger.get_put_count());
    assert_eq!(
        2,
        f.logger.get_region_put_count(&f.get_entity_region(COUNTRY))
    );
    assert_eq!(
        2,
        f.logger.get_region_put_count(&f.get_entity_region(STATE))
    );

    assert!(f.contains_entity(STATE, &f.states[0]));
    assert!(f.contains_entity(STATE, &f.states[1]));

    assert_eq!(STATE, s1.class());
    assert_eq!(STATE, s2.class());

    assert_eq!(id(&f.states[0]), id(&s1));
    assert_eq!(name(&f.states[0]), name(&s1));

    assert_eq!(id(&f.states[1]), id(&s2));
    assert_eq!(name(&f.states[1]), name(&s2));

    orm::set(&s1, "name", Value::string("NEW NAME 1"));
    orm::set(&s2, "name", Value::string("NEW NAME 2"));

    f.em.persist(&s1).unwrap();
    f.em.persist(&s2).unwrap();
    f.em.flush().await.unwrap();
    f.em.clear().unwrap();

    assert!(f.contains_entity(STATE, &f.states[0]));
    assert!(f.contains_entity(STATE, &f.states[1]));

    assert_eq!(6, f.logger.get_put_count());
    assert_eq!(
        2,
        f.logger.get_region_put_count(&f.get_entity_region(COUNTRY))
    );
    assert_eq!(
        4,
        f.logger.get_region_put_count(&f.get_entity_region(STATE))
    );

    f.reset_query_log();

    let c3 = f.find(STATE, id(&f.states[0])).await.unwrap();
    let c4 = f.find(STATE, id(&f.states[1])).await.unwrap();

    assert_eq!(2, f.logger.get_hit_count());
    assert_eq!(
        2,
        f.logger.get_region_hit_count(&f.get_entity_region(STATE))
    );

    f.assert_query_count(0);

    assert!(f.contains_entity(STATE, &f.states[0]));
    assert!(f.contains_entity(STATE, &f.states[1]));

    assert_eq!(STATE, c3.class());
    assert_eq!(STATE, c4.class());

    assert_eq!(id(&s1), id(&c3));
    assert_eq!(Value::string("NEW NAME 1"), name(&c3));

    assert_eq!(id(&s2), id(&c4));
    assert_eq!(Value::string("NEW NAME 2"), name(&c4));

    assert_eq!(2, f.logger.get_hit_count());
    assert_eq!(
        2,
        f.logger.get_region_hit_count(&f.get_entity_region(STATE))
    );
}

#[tokio::test]
async fn testPostFlushFailure() {
    let f = set_up().await;
    add_listener(&f.em, events::POST_FLUSH, "post flush failure");

    let country = slc::country("Brazil");

    f.cache.evict_entity_region(COUNTRY).unwrap();

    f.em.persist(&country).unwrap();
    let exc = f.em.flush().await.expect_err("Should throw exception");
    assert!(!id(&country).is_null());
    assert_eq!("post flush failure", orm::message(&exc));
    assert!(f.contains_entity(COUNTRY, &country));
}

#[tokio::test]
async fn testPostUpdateFailure() {
    let mut f = set_up().await;
    f.load_fixtures_countries().await;
    f.load_fixtures_states().await;
    f.em.clear().unwrap();

    add_listener(&f.em, events::POST_UPDATE, "post update failure");

    f.cache.evict_entity_region(STATE).unwrap();

    let state_id = id(&f.states[0]);
    let state_name = orm::get(&f.states[0], "name")
        .to_php_string()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let state = f.find(STATE, state_id.clone()).await.unwrap();

    assert!(f.cache.contains_entity(STATE, &state_id).unwrap());
    assert_eq!(STATE, state.class());
    assert_eq!(Value::string(state_name.as_str()), name(&state));

    orm::set(
        &state,
        "name",
        Value::string(format!(
            "{state_name}{}",
            php2rust_doctrine_orm::cache::uniqid("")
        )),
    );

    f.em.persist(&state).unwrap();

    let exc = f.em.flush().await.expect_err("Should throw exception");
    assert_eq!("post update failure", orm::message(&exc));

    f.em.clear().unwrap();

    assert!(f.cache.contains_entity(STATE, &state_id).unwrap());

    let state = f.find(STATE, state_id).await.unwrap();

    assert_eq!(STATE, state.class());
    assert_eq!(Value::string(state_name.as_str()), name(&state));
}

#[tokio::test]
async fn testPostRemoveFailure() {
    let mut f = set_up().await;
    f.load_fixtures_countries().await;
    f.em.clear().unwrap();

    add_listener(&f.em, events::POST_REMOVE, "post remove failure");

    f.cache.evict_entity_region(COUNTRY).unwrap();

    let country_id = id(&f.countries[0]);
    let country = f.find(COUNTRY, country_id.clone()).await.unwrap();

    assert!(f.cache.contains_entity(COUNTRY, &country_id).unwrap());
    assert_eq!(COUNTRY, country.class());

    f.em.remove(&country).await.unwrap();

    let exc = f.em.flush().await.expect_err("Should throw exception");
    assert_eq!("post remove failure", orm::message(&exc));

    f.em.clear().unwrap();

    assert!(
        !f.cache.contains_entity(COUNTRY, &country_id).unwrap(),
        "Removal attempts should clear the cache entry corresponding to the entity"
    );

    assert_eq!(COUNTRY, f.find(COUNTRY, country_id).await.unwrap().class());
}

#[tokio::test]
async fn testCachedNewEntityExists() {
    let mut f = set_up().await;
    f.load_fixtures_countries().await;

    let persister =
        f.em.get_unit_of_work()
            .get_entity_persister_interface(COUNTRY)
            .unwrap();
    f.reset_query_log();

    assert!(
        php2rust_doctrine_orm::persisters::EntityPersister::exists(
            &*persister,
            &f.countries[0],
            None
        )
        .await
        .unwrap()
    );

    f.assert_query_count(0);

    assert!(
        !php2rust_doctrine_orm::persisters::EntityPersister::exists(
            &*persister,
            &slc::country("Foo"),
            None
        )
        .await
        .unwrap()
    );
}
