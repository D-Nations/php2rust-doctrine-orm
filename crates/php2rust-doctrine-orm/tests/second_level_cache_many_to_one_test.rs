//! Upstream: doctrine/orm tests/Tests/ORM/Functional/SecondLevelCacheManyToOneTest.php
//!
//! The entity manager uses native lazy objects: `$entity->getAction()
//! instanceof CommonProxy` is false. A getter that triggers a lazy load
//! is `initializeObject()`.

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
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

use php_value::Value;

use slc_functional::{
    ACTION, COMPLEX_ACTION, COUNTRY, STATE, TOKEN, action, add_complex_token, add_token,
    assert_instance_of, complex_action, get_one, id, name, set_up, token,
};

#[tokio::test]
async fn testPutOnPersist() {
    let mut f = set_up().await;
    f.load_fixtures_countries().await;
    f.load_fixtures_states().await;
    f.em.clear().unwrap();

    assert!(f.contains_entity(COUNTRY, &get_one(&f.states[0], "country")));
    assert!(f.contains_entity(COUNTRY, &get_one(&f.states[1], "country")));
    assert!(f.contains_entity(STATE, &f.states[0]));
    assert!(f.contains_entity(STATE, &f.states[1]));
}

#[tokio::test]
async fn testPutAndLoadManyToOneRelation() {
    let mut f = set_up().await;
    f.load_fixtures_countries().await;
    f.load_fixtures_states().await;
    f.em.clear().unwrap();

    f.cache.evict_entity_region(STATE).unwrap();
    f.cache.evict_entity_region(COUNTRY).unwrap();

    assert!(!f.contains_entity(STATE, &f.states[0]));
    assert!(!f.contains_entity(STATE, &f.states[1]));
    assert!(!f.contains_entity(COUNTRY, &get_one(&f.states[0], "country")));
    assert!(!f.contains_entity(COUNTRY, &get_one(&f.states[1], "country")));

    let c1 = f.find(STATE, id(&f.states[0])).await.unwrap();
    let c2 = f.find(STATE, id(&f.states[1])).await.unwrap();

    //trigger lazy load
    f.em.initialize_object(&get_one(&c1, "country"))
        .await
        .unwrap();
    f.em.initialize_object(&get_one(&c2, "country"))
        .await
        .unwrap();
    assert!(!name(&get_one(&c1, "country")).is_null());
    assert!(!name(&get_one(&c2, "country")).is_null());

    assert!(f.contains_entity(COUNTRY, &get_one(&f.states[0], "country")));
    assert!(f.contains_entity(COUNTRY, &get_one(&f.states[1], "country")));
    assert!(f.contains_entity(STATE, &f.states[0]));
    assert!(f.contains_entity(STATE, &f.states[1]));

    assert_instance_of(STATE, &c1);
    assert_instance_of(STATE, &c2);
    assert_instance_of(COUNTRY, &get_one(&c1, "country"));
    assert_instance_of(COUNTRY, &get_one(&c2, "country"));

    for (state, c) in [(&f.states[0], &c1), (&f.states[1], &c2)] {
        assert_eq!(id(state), id(c));
        assert_eq!(name(state), name(c));
        assert_eq!(id(&get_one(state, "country")), id(&get_one(c, "country")));
        assert_eq!(
            name(&get_one(state, "country")),
            name(&get_one(c, "country"))
        );
    }

    f.em.clear().unwrap();

    f.reset_query_log();

    let c3 = f.find(STATE, id(&f.states[0])).await.unwrap();
    let c4 = f.find(STATE, id(&f.states[1])).await.unwrap();

    f.assert_query_count(0);

    //trigger lazy load from cache
    f.em.initialize_object(&get_one(&c3, "country"))
        .await
        .unwrap();
    f.em.initialize_object(&get_one(&c4, "country"))
        .await
        .unwrap();
    assert!(!name(&get_one(&c3, "country")).is_null());
    assert!(!name(&get_one(&c4, "country")).is_null());

    assert_instance_of(STATE, &c3);
    assert_instance_of(STATE, &c4);
    assert_instance_of(COUNTRY, &get_one(&c3, "country"));
    assert_instance_of(COUNTRY, &get_one(&c4, "country"));

    assert_eq!(id(&c1), id(&c3));
    assert_eq!(name(&c1), name(&c3));

    assert_eq!(id(&c2), id(&c4));
    assert_eq!(name(&c2), name(&c4));

    for (state, c) in [(&f.states[0], &c3), (&f.states[1], &c4)] {
        assert_eq!(id(&get_one(state, "country")), id(&get_one(c, "country")));
        assert_eq!(
            name(&get_one(state, "country")),
            name(&get_one(c, "country"))
        );
    }
}

#[tokio::test]
async fn testInverseSidePutShouldEvictCollection() {
    let mut f = set_up().await;
    f.load_fixtures_countries().await;
    f.load_fixtures_states().await;

    f.em.clear().unwrap();

    f.cache.evict_entity_region(STATE).unwrap();
    f.cache.evict_entity_region(COUNTRY).unwrap();

    //evict collection on add
    let c3 = f.find(STATE, id(&f.states[0])).await.unwrap();
    let prev = orm::get(&c3, "cities");
    let count = slc_functional::count(&c3, "cities").await;
    let city = slc::city("Buenos Aires", Some(&c3));

    slc::add_city(&c3, &city);

    f.em.persist(&city).unwrap();
    f.em.persist(&c3).unwrap();
    f.em.flush().await.unwrap();
    f.em.clear().unwrap();

    let state = f.find(STATE, id(&c3)).await.unwrap();
    f.reset_query_log();

    // Association was cleared from EM
    assert!(!prev.identical(&orm::get(&state, "cities")));

    // New association has one more item (cache was evicted)
    assert_eq!(count + 1, slc_functional::count(&state, "cities").await);
    f.assert_query_count(0);
}

#[tokio::test]
async fn testShouldNotReloadWhenAssociationIsMissing() {
    let mut f = set_up().await;
    f.load_fixtures_countries().await;
    f.load_fixtures_states().await;
    f.em.clear().unwrap();

    let state_id1 = id(&f.states[0]);
    let state_id2 = id(&f.states[3]);

    let country_id1 = id(&get_one(&f.states[0], "country"));
    let country_id2 = id(&get_one(&f.states[3], "country"));

    let contains = |class: &str, id: &Value| f.cache.contains_entity(class, id).unwrap();

    assert!(contains(COUNTRY, &country_id1));
    assert!(contains(COUNTRY, &country_id2));
    assert!(contains(STATE, &state_id1));
    assert!(contains(STATE, &state_id2));

    f.cache.evict_entity_region(COUNTRY).unwrap();

    assert!(!contains(COUNTRY, &country_id1));
    assert!(!contains(COUNTRY, &country_id2));

    f.em.clear().unwrap();

    f.reset_query_log();

    let state1 = f.find(STATE, state_id1).await.unwrap();
    let state2 = f.find(STATE, state_id2).await.unwrap();

    f.assert_query_count(0);

    assert_instance_of(STATE, &state1);
    assert_instance_of(STATE, &state2);
    assert_instance_of(COUNTRY, &get_one(&state1, "country"));
    assert_instance_of(COUNTRY, &get_one(&state2, "country"));

    f.reset_query_log();

    f.em.initialize_object(&get_one(&state1, "country"))
        .await
        .unwrap();
    f.em.initialize_object(&get_one(&state2, "country"))
        .await
        .unwrap();
    assert!(!name(&get_one(&state1, "country")).is_null());
    assert!(!name(&get_one(&state2, "country")).is_null());
    assert_eq!(country_id1, id(&get_one(&state1, "country")));
    assert_eq!(country_id2, id(&get_one(&state2, "country")));

    f.assert_query_count(2);
}

#[tokio::test]
async fn testPutAndLoadNonCacheableManyToOne() {
    let f = set_up().await;
    assert!(f.cache.get_entity_cache_region(ACTION).unwrap().is_none());
    assert!(f.cache.get_entity_cache_region(TOKEN).unwrap().is_some());

    let token = token("token-hash", None);
    let action = action("exec");
    add_token(&action, &token);

    f.em.persist(&token).unwrap();

    f.em.flush().await.unwrap();
    f.em.clear().unwrap();

    let contains = |class: &str, id: &Value| f.cache.contains_entity(class, id).unwrap();
    assert!(contains(TOKEN, &orm::get(&token, "token")));
    assert!(!contains(TOKEN, &orm::get(&action, "name")));

    f.reset_query_log();
    let entity = f.find(TOKEN, orm::get(&token, "token")).await.unwrap();

    assert_instance_of(TOKEN, &entity);
    assert_eq!(Value::string("token-hash"), orm::get(&entity, "token"));

    assert_instance_of(ACTION, &get_one(&entity, "action"));
    assert_eq!(
        Value::string("exec"),
        orm::get(&get_one(&entity, "action"), "name")
    );

    f.assert_query_count(0);
}

#[tokio::test]
async fn testPutAndLoadNonCacheableCompositeManyToOne() {
    let f = set_up().await;
    assert!(f.cache.get_entity_cache_region(ACTION).unwrap().is_none());
    assert!(
        f.cache
            .get_entity_cache_region(COMPLEX_ACTION)
            .unwrap()
            .is_none()
    );
    assert!(f.cache.get_entity_cache_region(TOKEN).unwrap().is_some());

    let token = token("token-hash", None);

    let action1 = action("login");
    let action2 = action("logout");
    let action3 = action("rememberme");

    let complex_action = complex_action(&action1, &action3, "login,rememberme");

    add_complex_token(&complex_action, &token);

    orm::set_obj(&token, "action", &action2);

    f.em.persist(&token).unwrap();

    f.em.flush().await.unwrap();
    f.em.clear().unwrap();

    let contains = |class: &str, id: &Value| f.cache.contains_entity(class, id).unwrap();
    assert!(contains(TOKEN, &orm::get(&token, "token")));
    assert!(!contains(ACTION, &orm::get(&action1, "name")));
    assert!(!contains(ACTION, &orm::get(&action2, "name")));
    assert!(!contains(ACTION, &orm::get(&action3, "name")));

    f.reset_query_log();

    let entity = f.find(TOKEN, orm::get(&token, "token")).await.unwrap();

    assert_instance_of(TOKEN, &entity);
    assert_eq!(Value::string("token-hash"), orm::get(&entity, "token"));

    f.assert_query_count(0);

    assert_instance_of(ACTION, &get_one(&entity, "action"));
    assert_instance_of(COMPLEX_ACTION, &get_one(&entity, "complexAction"));
    f.assert_query_count(0);

    let complex = get_one(&entity, "complexAction");
    assert_instance_of(ACTION, &get_one(&complex, "action1"));
    assert_instance_of(ACTION, &get_one(&complex, "action2"));
    // `$entity->getAction() instanceof CommonProxy` is false with native
    // lazy objects.
    let expected_query_count = 0;
    f.assert_query_count(expected_query_count);

    assert_eq!(
        Value::string("login"),
        orm::get(&get_one(&complex, "action1"), "name")
    );
    f.assert_query_count(expected_query_count);
    assert_eq!(
        Value::string("rememberme"),
        orm::get(&get_one(&complex, "action2"), "name")
    );
    f.assert_query_count(expected_query_count);
}
