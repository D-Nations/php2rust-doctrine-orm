//! Upstream: doctrine/orm tests/Tests/ORM/Functional/SecondLevelCacheOneToOneTest.php
//!
//! A getter on a lazy object (`$t1->getProfile()->getName()`) is
//! `initializeObject()` and a read of the property; the identifier of a
//! lazy object is read without initializing it, as PHP does.

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
    ADDRESS, CLIENT, Fixture, PERSON, TOKEN, TRAVELER, TRAVELER_PROFILE, TRAVELER_PROFILE_INFO,
    assert_instance_of, client, get_one, id, name, set_up, token,
};

async fn load_travelers_with_profile(f: &mut Fixture, with_info: bool) {
    f.load_fixtures_countries().await;
    f.load_fixtures_states().await;
    f.load_fixtures_cities().await;
    f.load_fixtures_travelers_with_profile().await;
    if with_info {
        f.load_fixtures_travelers_profile_info().await;
    }
}

#[tokio::test]
async fn testPutOneToOneOnUnidirectionalPersist() {
    let mut f = set_up().await;
    load_travelers_with_profile(&mut f, false).await;

    f.em.clear().unwrap();

    let entity1 = &f.travelers_with_profile[0];
    let entity2 = &f.travelers_with_profile[1];

    assert!(f.contains_entity(TRAVELER, entity1));
    assert!(f.contains_entity(TRAVELER, entity2));
    assert!(f.contains_entity(TRAVELER_PROFILE, &get_one(entity1, "profile")));
    assert!(f.contains_entity(TRAVELER_PROFILE, &get_one(entity2, "profile")));
}

#[tokio::test]
async fn testPutOneToOneOnBidirectionalPersist() {
    let mut f = set_up().await;
    load_travelers_with_profile(&mut f, true).await;

    f.em.clear().unwrap();

    let entity1 = &f.travelers_with_profile[0];
    let entity2 = &f.travelers_with_profile[1];

    assert!(f.contains_entity(TRAVELER, entity1));
    assert!(f.contains_entity(TRAVELER, entity2));
    assert!(f.contains_entity(TRAVELER_PROFILE, &get_one(entity1, "profile")));
    assert!(f.contains_entity(TRAVELER_PROFILE, &get_one(entity2, "profile")));
    assert!(f.contains_entity(
        TRAVELER_PROFILE_INFO,
        &get_one(&get_one(entity1, "profile"), "info")
    ));
    assert!(f.contains_entity(
        TRAVELER_PROFILE_INFO,
        &get_one(&get_one(entity2, "profile"), "info")
    ));
}

#[tokio::test]
async fn testPutAndLoadOneToOneUnidirectionalRelation() {
    let mut f = set_up().await;
    load_travelers_with_profile(&mut f, true).await;

    f.em.clear().unwrap();

    f.cache.evict_entity_region(TRAVELER).unwrap();
    f.cache.evict_entity_region(TRAVELER_PROFILE).unwrap();

    let entity1 = f.travelers_with_profile[0].clone();
    let entity2 = f.travelers_with_profile[1].clone();

    assert!(!f.contains_entity(TRAVELER, &entity1));
    assert!(!f.contains_entity(TRAVELER, &entity2));
    assert!(!f.contains_entity(TRAVELER_PROFILE, &get_one(&entity1, "profile")));
    assert!(!f.contains_entity(TRAVELER_PROFILE, &get_one(&entity2, "profile")));

    let t1 = f.find(TRAVELER, id(&entity1)).await.unwrap();
    let t2 = f.find(TRAVELER, id(&entity2)).await.unwrap();

    assert!(f.contains_entity(TRAVELER, &entity1));
    assert!(f.contains_entity(TRAVELER, &entity2));
    // The inverse side its not cached
    assert!(!f.contains_entity(TRAVELER_PROFILE, &get_one(&entity1, "profile")));
    assert!(!f.contains_entity(TRAVELER_PROFILE, &get_one(&entity2, "profile")));

    assert_instance_of(TRAVELER, &t1);
    assert_instance_of(TRAVELER, &t2);
    assert_instance_of(TRAVELER_PROFILE, &get_one(&t1, "profile"));
    assert_instance_of(TRAVELER_PROFILE, &get_one(&t2, "profile"));

    for (entity, t) in [(&entity1, &t1), (&entity2, &t2)] {
        assert_eq!(id(entity), id(t));
        assert_eq!(name(entity), name(t));
        assert_eq!(id(&get_one(entity, "profile")), id(&get_one(t, "profile")));
        assert_eq!(
            name(&get_one(entity, "profile")),
            name(&f.load_one(t, "profile").await)
        );
    }

    // its all cached now
    assert!(f.contains_entity(TRAVELER, &entity1));
    assert!(f.contains_entity(TRAVELER, &entity2));
    assert!(f.contains_entity(TRAVELER_PROFILE, &get_one(&entity1, "profile")));
    assert!(f.contains_entity(TRAVELER_PROFILE, &get_one(&entity1, "profile")));

    f.em.clear().unwrap();

    f.reset_query_log();
    // load from cache
    let t3 = f.find(TRAVELER, id(&entity1)).await.unwrap();
    let t4 = f.find(TRAVELER, id(&entity2)).await.unwrap();

    assert_instance_of(TRAVELER, &t3);
    assert_instance_of(TRAVELER, &t4);
    assert_instance_of(TRAVELER_PROFILE, &get_one(&t3, "profile"));
    assert_instance_of(TRAVELER_PROFILE, &get_one(&t4, "profile"));

    assert_eq!(
        id(&get_one(&entity1, "profile")),
        id(&get_one(&t3, "profile"))
    );
    assert_eq!(
        id(&get_one(&entity2, "profile")),
        id(&get_one(&t4, "profile"))
    );

    assert_eq!(
        name(&get_one(&entity1, "profile")),
        name(&f.load_one(&t3, "profile").await)
    );
    assert_eq!(
        name(&get_one(&entity2, "profile")),
        name(&f.load_one(&t4, "profile").await)
    );

    f.assert_query_count(0);
}

#[tokio::test]
async fn testPutAndLoadOneToOneBidirectionalRelation() {
    let mut f = set_up().await;
    load_travelers_with_profile(&mut f, true).await;

    f.em.clear().unwrap();

    f.cache.evict_entity_region(TRAVELER).unwrap();
    f.cache.evict_entity_region(TRAVELER_PROFILE).unwrap();
    f.cache.evict_entity_region(TRAVELER_PROFILE_INFO).unwrap();

    let entity1 = get_one(&f.travelers_with_profile[0], "profile");
    let entity2 = get_one(&f.travelers_with_profile[1], "profile");

    assert!(!f.contains_entity(TRAVELER_PROFILE, &entity1));
    assert!(!f.contains_entity(TRAVELER_PROFILE, &entity2));
    assert!(!f.contains_entity(TRAVELER_PROFILE_INFO, &get_one(&entity1, "info")));
    assert!(!f.contains_entity(TRAVELER_PROFILE_INFO, &get_one(&entity2, "info")));

    let p1 = f.find(TRAVELER_PROFILE, id(&entity1)).await.unwrap();
    let p2 = f.find(TRAVELER_PROFILE, id(&entity2)).await.unwrap();

    for (entity, p) in [(&entity1, &p1), (&entity2, &p2)] {
        assert_eq!(id(entity), id(p));
        assert_eq!(name(entity), name(p));
        assert_eq!(id(&get_one(entity, "info")), id(&get_one(p, "info")));
        assert_eq!(
            orm::get(&get_one(entity, "info"), "description"),
            orm::get(&f.load_one(p, "info").await, "description")
        );
    }

    assert!(f.contains_entity(TRAVELER_PROFILE, &entity1));
    assert!(f.contains_entity(TRAVELER_PROFILE, &entity2));
    assert!(f.contains_entity(TRAVELER_PROFILE_INFO, &get_one(&entity1, "info")));
    assert!(f.contains_entity(TRAVELER_PROFILE_INFO, &get_one(&entity2, "info")));

    f.em.clear().unwrap();

    f.reset_query_log();

    let p3 = f.find(TRAVELER_PROFILE, id(&entity1)).await.unwrap();
    let p4 = f.find(TRAVELER_PROFILE, id(&entity2)).await.unwrap();

    assert_instance_of(TRAVELER_PROFILE, &p3);
    assert_instance_of(TRAVELER_PROFILE, &p4);
    assert_instance_of(TRAVELER_PROFILE_INFO, &get_one(&p3, "info"));
    assert_instance_of(TRAVELER_PROFILE_INFO, &get_one(&p4, "info"));

    for (entity, p) in [(&entity1, &p3), (&entity2, &p4)] {
        assert_eq!(id(entity), id(p));
        assert_eq!(name(entity), name(p));
        assert_eq!(id(&get_one(entity, "info")), id(&get_one(p, "info")));
        assert_eq!(
            orm::get(&get_one(entity, "info"), "description"),
            orm::get(&f.load_one(p, "info").await, "description")
        );
    }

    f.assert_query_count(0);
}

#[tokio::test]
async fn testInverseSidePutAndLoadOneToOneBidirectionalRelation() {
    let mut f = set_up().await;
    f.load_fixtures_person_with_address().await;

    f.em.clear().unwrap();

    f.cache.evict_entity_region(PERSON).unwrap();
    f.cache.evict_entity_region(ADDRESS).unwrap();

    let entity1 = get_one(&f.addresses[0], "person");
    let entity2 = get_one(&f.addresses[1], "person");

    assert!(!f.contains_entity(PERSON, &entity1));
    assert!(!f.contains_entity(PERSON, &entity2));
    assert!(!f.contains_entity(ADDRESS, &get_one(&entity1, "address")));
    assert!(!f.contains_entity(ADDRESS, &get_one(&entity2, "address")));

    let p1 = f.find(PERSON, id(&entity1)).await.unwrap();
    let p2 = f.find(PERSON, id(&entity2)).await.unwrap();

    let assert_person = |entity: &php_value::Object, p: &php_value::Object| {
        assert_eq!(id(entity), id(p));
        assert_eq!(name(entity), name(p));
        assert_eq!(id(&get_one(entity, "address")), id(&get_one(p, "address")));
        assert_eq!(
            orm::get(&get_one(entity, "address"), "location"),
            orm::get(&get_one(p, "address"), "location")
        );
    };

    assert_person(&entity1, &p1);
    assert_person(&entity2, &p2);

    assert!(f.contains_entity(PERSON, &entity1));
    assert!(f.contains_entity(PERSON, &entity2));
    // The inverse side its not cached
    assert!(!f.contains_entity(ADDRESS, &get_one(&entity1, "address")));
    assert!(!f.contains_entity(ADDRESS, &get_one(&entity2, "address")));

    f.em.clear().unwrap();

    f.reset_query_log();

    let p3 = f.find(PERSON, id(&entity1)).await.unwrap();
    let p4 = f.find(PERSON, id(&entity2)).await.unwrap();

    assert_instance_of(PERSON, &p3);
    assert_instance_of(PERSON, &p4);
    assert_instance_of(ADDRESS, &get_one(&p3, "address"));
    assert_instance_of(ADDRESS, &get_one(&p4, "address"));

    assert_person(&entity1, &p3);
    assert_person(&entity2, &p4);

    f.assert_query_count(2);
}

#[tokio::test]
async fn testPutAndLoadNonCacheableOneToOne() {
    let f = set_up().await;
    assert!(f.cache.get_entity_cache_region(CLIENT).unwrap().is_none());
    assert!(f.cache.get_entity_cache_region(TOKEN).unwrap().is_some());

    let client = client("FabioBatSilva");
    let token = token("token-hash", Some(&client));

    f.em.persist(&client).unwrap();
    f.em.persist(&token).unwrap();
    f.em.flush().await.unwrap();
    f.em.clear().unwrap();

    f.reset_query_log();

    assert!(
        f.cache
            .contains_entity(TOKEN, &orm::get(&token, "token"))
            .unwrap()
    );
    assert!(!f.contains_entity(CLIENT, &client));

    let entity = f.find(TOKEN, orm::get(&token, "token")).await.unwrap();

    assert_instance_of(TOKEN, &entity);
    assert_instance_of(CLIENT, &get_one(&entity, "client"));
    assert_eq!(Value::string("token-hash"), orm::get(&entity, "token"));
    f.assert_query_count(0);

    assert_eq!(
        Value::string("FabioBatSilva"),
        orm::get(&f.load_one(&entity, "client").await, "name")
    );
    f.assert_query_count(1);
}
