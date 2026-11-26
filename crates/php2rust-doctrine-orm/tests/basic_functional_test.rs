//! Upstream: doctrine/orm tests/Tests/ORM/Functional/BasicFunctionalTest.php

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

use php_value::{Object, Value};
use php2rust_doctrine_orm::collection_object as coll;
use php2rust_doctrine_orm::exception::{
    ENTITY_IDENTITY_COLLISION_EXCEPTION, ORM_INVALID_ARGUMENT_EXCEPTION,
};
use php2rust_doctrine_orm::unit_of_work::{STATE_MANAGED, STATE_NEW, STATE_REMOVED};

use orm::*;

#[tokio::test]
async fn testBasicUnitsOfWorkWithOneToManyAssociation() {
    let em = set_up(&CMS_MODELS).await;
    // Create
    let user = cms_user_with("Roman", "romanb", "developer");
    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    let id = get(&user, "id");
    assert!(matches!(id, Value::Int(_)), "{id:?}");
    assert!(em.contains(&user).unwrap());

    // Read
    let user2 = em.find(CMS_USER, id).await.unwrap().unwrap();
    assert!(same(&user, &user2));

    // Add a phonenumber
    let ph = cms_phonenumber("12345");
    add_phonenumber(&user, &ph);
    em.flush().await.unwrap();
    assert!(em.contains(&ph).unwrap());
    assert!(em.contains(&user).unwrap());

    // Update name
    set(&user, "name", s("guilherme"));
    em.flush().await.unwrap();
    assert_eq!(get(&user, "name"), s("guilherme"));

    // Add another phonenumber
    let ph2 = cms_phonenumber("6789");
    add_phonenumber(&user, &ph2);
    em.flush().await.unwrap();
    assert!(em.contains(&ph2).unwrap());

    // Delete
    em.remove(&user).await.unwrap();
    let uow = em.get_unit_of_work();
    assert!(uow.is_scheduled_for_delete(&user));
    assert!(uow.is_scheduled_for_delete(&ph));
    assert!(uow.is_scheduled_for_delete(&ph2));
    em.flush().await.unwrap();
    assert!(!uow.is_scheduled_for_delete(&user));
    assert!(!uow.is_scheduled_for_delete(&ph));
    assert!(!uow.is_scheduled_for_delete(&ph2));
    assert_eq!(uow.get_entity_state(&user).await.unwrap(), STATE_NEW);
    assert_eq!(uow.get_entity_state(&ph).await.unwrap(), STATE_NEW);
    assert_eq!(uow.get_entity_state(&ph2).await.unwrap(), STATE_NEW);
}

#[tokio::test]
async fn testOneToManyAssociationModification() {
    let em = set_up(&CMS_MODELS).await;
    let user = cms_user_with("Roman", "romanb", "developer");
    let ph1 = cms_phonenumber("0301234");
    let ph2 = cms_phonenumber("987654321");
    add_phonenumber(&user, &ph1);
    add_phonenumber(&user, &ph2);
    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    // Remove the first element from the collection
    coll::remove(&get(&user, "phonenumbers"), 0).await.unwrap();
    set(&ph1, "user", Value::Null); // owning side!

    em.flush().await.unwrap();

    assert_eq!(count_of(&user, "phonenumbers").await, 1);
    assert!(get(&ph1, "user").is_null());
}

#[tokio::test]
async fn testRemove() {
    let em = set_up(&CMS_MODELS).await;
    let user = cms_user_with("Guilherme", "gblanco", "developer");
    let uow = em.get_unit_of_work();

    assert_eq!(
        uow.get_entity_state(&user).await.unwrap(),
        STATE_NEW,
        "State should be UnitOfWork::STATE_NEW"
    );
    em.persist(&user).unwrap();
    assert_eq!(
        uow.get_entity_state(&user).await.unwrap(),
        STATE_MANAGED,
        "State should be UnitOfWork::STATE_MANAGED"
    );
    em.remove(&user).await.unwrap();
    assert_eq!(
        uow.get_entity_state(&user).await.unwrap(),
        STATE_NEW,
        "State should be UnitOfWork::STATE_NEW"
    );

    em.persist(&user).unwrap();
    em.flush().await.unwrap();
    let id = get(&user, "id");

    em.remove(&user).await.unwrap();
    assert_eq!(
        uow.get_entity_state(&user).await.unwrap(),
        STATE_REMOVED,
        "State should be UnitOfWork::STATE_REMOVED"
    );
    em.flush().await.unwrap();
    assert_eq!(
        uow.get_entity_state(&user).await.unwrap(),
        STATE_NEW,
        "State should be UnitOfWork::STATE_NEW"
    );

    assert!(em.find(CMS_USER, id).await.unwrap().is_none());
}

#[tokio::test]
async fn testOneToManyOrphanRemoval() {
    let em = set_up(&CMS_MODELS).await;
    let user = cms_user_with("Guilherme", "gblanco", "developer");
    for i in 0..3 {
        let phone = new_object(CMS_PHONENUMBER);
        set(&phone, "phonenumber", Value::Int(100 + i));
        add_phonenumber(&user, &phone);
    }
    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    coll::remove(&get(&user, "phonenumbers"), 0).await.unwrap();
    assert_eq!(count_of(&user, "phonenumbers").await, 2);

    em.flush().await.unwrap();

    // Check that there are just 2 phonenumbers left
    let count = fetch_one(&em, "SELECT COUNT(*) FROM cms_phonenumbers").await;
    assert_eq!(count, Value::Int(2)); // only 2 remaining

    // check that clear() removes the others via orphan removal
    coll::clear(&get(&user, "phonenumbers")).await.unwrap();
    em.flush().await.unwrap();
    assert_eq!(
        fetch_one(&em, "select count(*) from cms_phonenumbers").await,
        Value::Int(0)
    );
}

#[tokio::test]
async fn testBasicRefresh() {
    let em = set_up(&CMS_MODELS).await;
    let user = cms_user_with("Guilherme", "gblanco", "developer");
    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    set(&user, "status", s("mascot"));
    assert_eq!(get(&user, "status"), s("mascot"));
    em.refresh(&user).await.unwrap();
    assert_eq!(get(&user, "status"), s("developer"));
}

/// DDC-833
#[tokio::test]
async fn testRefreshResetsCollection() {
    let em = set_up(&CMS_MODELS).await;
    let user = cms_user_with("Guilherme", "gblanco", "developer");
    // Add a phonenumber
    let ph1 = cms_phonenumber("12345");
    add_phonenumber(&user, &ph1);
    // Add a phonenumber
    let ph2 = cms_phonenumber("54321");

    em.persist(&user).unwrap();
    em.persist(&ph1).unwrap();
    em.persist(&ph2).unwrap();
    em.flush().await.unwrap();

    add_phonenumber(&user, &ph2);

    assert_eq!(count_of(&user, "phonenumbers").await, 2);
    em.refresh(&user).await.unwrap();

    assert_eq!(count_of(&user, "phonenumbers").await, 1);
}

#[tokio::test]
async fn testTextColumnSaveAndRetrieve() {
    let em = set_up(&CMS_MODELS).await;
    let user = cms_user_with("Guilherme", "gblanco", "developer");
    em.persist(&user).unwrap();

    let article = cms_article("A Test Article!", "Lorem ipsum dolor sunt.");
    set_obj(&article, "user", &user);

    em.persist(&article).unwrap();
    em.flush().await.unwrap();
    let article_id = get(&article, "id");

    em.clear().unwrap();

    // test find() with leading backslash at the same time
    let article_new = em
        .find(&format!("\\{CMS_ARTICLE}"), article_id.clone())
        .await
        .unwrap()
        .unwrap();
    assert!(em.contains(&article_new).unwrap());
    assert_eq!(get(&article_new, "text"), s("Lorem ipsum dolor sunt."));

    assert!(!same(&article, &article_new));

    set(
        &article_new,
        "text",
        s("Lorem ipsum dolor sunt. And stuff!"),
    );

    em.flush().await.unwrap();
    em.clear().unwrap();

    let article_new = em.find(CMS_ARTICLE, article_id).await.unwrap().unwrap();
    assert_eq!(
        get(&article_new, "text"),
        s("Lorem ipsum dolor sunt. And stuff!")
    );
    assert!(em.contains(&article_new).unwrap());
}

#[tokio::test]
async fn testRemoveEntityByReference() {
    let em = set_up(&CMS_MODELS).await;
    let user = cms_user_with("Guilherme", "gblanco", "developer");
    em.persist(&user).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let user_ref = em
        .get_reference(CMS_USER, get(&user, "id"))
        .unwrap()
        .unwrap();
    em.remove(&user_ref).await.unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    assert_eq!(
        fetch_one(&em, "select count(*) from cms_users").await,
        Value::Int(0)
    );
}

fn bonn_address(user: &Object) -> Object {
    let address = cms_address("Germany", "12354", "Bonn");
    set(&address, "street", s("somestreet"));
    set_obj(&address, "user", user);
    address
}

#[tokio::test]
async fn testOneToOneNullUpdate() {
    let em = set_up(&CMS_MODELS).await;
    let user = cms_user_with("Benjamin E.", "beberlei", "active");
    let address = bonn_address(&user);

    em.persist(&address).unwrap();
    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    let sql = format!(
        "select 1 from cms_addresses where user_id = {}",
        get(&user, "id").to_php_string().unwrap().to_string_lossy()
    );
    assert_eq!(fetch_one(&em, &sql).await, Value::Int(1));

    set(&address, "user", Value::Null);
    em.flush().await.unwrap();

    assert_ne!(fetch_one(&em, &sql).await, Value::Int(1));
}

#[tokio::test]
async fn testNewAssociatedEntityDuringFlushThrowsException() {
    let em = set_up(&CMS_MODELS).await;
    let user = cms_user_with("Benjamin E.", "beberlei", "active");
    let address = bonn_address(&user);

    em.persist(&address).unwrap();

    // flushing without persisting $user should raise an exception
    let e = em.flush().await.unwrap_err();
    assert_eq!(e.class, ORM_INVALID_ARGUMENT_EXCEPTION);
}

#[tokio::test]
async fn testNewAssociatedEntityDuringFlushThrowsException2() {
    let em = set_up(&CMS_MODELS).await;
    let user = cms_user_with("Benjamin E.", "beberlei", "active");
    let address = bonn_address(&user);

    em.persist(&address).unwrap();
    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    let u2 = cms_user_with("Benjamin E.", "beberlei", "inactive");
    set_obj(&address, "user", &u2);

    // flushing without persisting $u2 should raise an exception
    let e = em.flush().await.unwrap_err();
    assert_eq!(e.class, ORM_INVALID_ARGUMENT_EXCEPTION);
}

#[tokio::test]
async fn testNewAssociatedEntityDuringFlushThrowsException3() {
    let em = set_up(&CMS_MODELS).await;
    let art = cms_article("topic", "the text");
    let com = cms_comment("Good", "Really good!");
    add_comment(&art, &com);

    em.persist(&art).unwrap();

    // flushing without persisting $com should raise an exception
    let e = em.flush().await.unwrap_err();
    assert_eq!(e.class, ORM_INVALID_ARGUMENT_EXCEPTION);
}

#[tokio::test]
async fn testOneToOneOrphanRemoval() {
    let em = set_up(&CMS_MODELS).await;
    let user = cms_user_with("Benjamin E.", "beberlei", "active");
    let address = bonn_address(&user);
    set_obj(&user, "address", &address);

    em.persist(&address).unwrap();
    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    set(&user, "address", Value::Null);

    em.flush().await.unwrap();

    assert_eq!(
        fetch_one(&em, "select count(*) from cms_addresses").await,
        Value::Int(0)
    );

    // check orphan removal through replacement
    set_obj(&user, "address", &address);
    set_obj(&address, "user", &user);

    em.flush().await.unwrap();
    assert_eq!(
        fetch_one(&em, "select count(*) from cms_addresses").await,
        Value::Int(1)
    );

    // remove $address to free up unique key id
    em.remove(&address).await.unwrap();
    em.flush().await.unwrap();

    let new_address = cms_address("NewGermany", "12354", "NewBonn");
    set(&new_address, "street", s("somenewstreet"));
    set_obj(&new_address, "user", &user);
    set_obj(&user, "address", &new_address);

    em.flush().await.unwrap();
    assert_eq!(
        fetch_one(&em, "select count(*) from cms_addresses").await,
        Value::Int(1)
    );
}

#[tokio::test]
async fn testFlushSingleManagedEntity() {
    let em = set_up(&CMS_MODELS).await;
    let user = cms_user_with("Dominik", "domnikl", "developer");
    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    set(&user, "status", s("administrator"));
    em.flush().await.unwrap();
    em.clear().unwrap();

    let user = em.find(CMS_USER, get(&user, "id")).await.unwrap().unwrap();
    assert_eq!(get(&user, "status"), s("administrator"));
}

#[tokio::test]
async fn testFlushAndCascadePersist() {
    let em = set_up(&CMS_MODELS).await;
    let user = cms_user_with("Dominik", "domnikl", "developer");
    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    let address = cms_address("Germany", "12354", "Springfield");
    set(&address, "street", s("Foo Street"));
    set_obj(&address, "user", &user);
    set_obj(&user, "address", &address);

    em.flush().await.unwrap();

    assert!(
        em.contains(&address).unwrap(),
        "Other user is contained in EntityManager"
    );
    assert!(
        matches!(get(&address, "id"), Value::Int(id) if id > 0),
        "other user has an id"
    );
}

#[tokio::test]
async fn testFlushSingleAndNoCascade() {
    let em = set_up(&CMS_MODELS).await;
    let user = cms_user_with("Dominik", "domnikl", "developer");
    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    let article1 = cms_article("Foo", "Foo Text");
    set_obj(&article1, "user", &user);
    append(&user, "articles", Value::Object(article1.clone()));

    let e = em.flush().await.unwrap_err();
    assert_eq!(e.class, ORM_INVALID_ARGUMENT_EXCEPTION);
    assert!(message(&e).contains(
        "A new entity was found through the relationship 'Doctrine\\Tests\\Models\\CMS\\CmsUser#articles'"
    ));
}

#[tokio::test]
async fn testFlushSingleNewEntityThenRemove() {
    let em = set_up(&CMS_MODELS).await;
    let user = cms_user_with("Dominik", "domnikl", "developer");
    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    let user_id = get(&user, "id");

    em.remove(&user).await.unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    assert!(em.find(CMS_USER, user_id).await.unwrap().is_none());
}

#[tokio::test]
async fn testWrongAssociationInstance() {
    let em = set_up(&CMS_MODELS).await;
    let user = cms_user_with("Dominik", "domnikl", "developer");
    set_obj(&user, "address", &user);

    // persist() throws already (cascadePersist() checks the type).
    let e = em.persist(&user).unwrap_err();
    assert_eq!(e.class, ORM_INVALID_ARGUMENT_EXCEPTION);
    assert_eq!(
        message(&e),
        "Expected value of type \"Doctrine\\Tests\\Models\\CMS\\CmsAddress\" for association field \"Doctrine\\Tests\\Models\\CMS\\CmsUser#$address\", got \"Doctrine\\Tests\\Models\\CMS\\CmsUser\" instead."
    );
}

#[tokio::test]
async fn testItThrowsWhenReferenceUsesIdAssignedByDatabase() {
    let em = set_up(&CMS_MODELS).await;
    let user = cms_user();
    set(&user, "name", s("test"));
    set(&user, "username", s("test"));
    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    // Obtain a reference object for the next ID. This is a user error - references
    // should be fetched only for existing IDs
    let Value::Int(id) = get(&user, "id") else {
        panic!("an id");
    };
    let _reference = em.get_reference(CMS_USER, Value::Int(id + 1)).unwrap();

    let user2 = cms_user();
    set(&user2, "name", s("test2"));
    set(&user2, "username", s("test2"));

    // depending on ID generation strategy, the ID may be asssigned already here
    // and the entity be put in the identity map
    em.persist(&user2).unwrap();

    // post insert IDs will be assigned during flush
    let e = em.flush().await.unwrap_err();
    assert_eq!(e.class, ENTITY_IDENTITY_COLLISION_EXCEPTION);
    assert!(message(&e).contains("another object of class"));
    assert!(message(&e).contains("was already present for the same ID"));
}
