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
#[path = "common/query_log.rs"]
mod query_log;

use php_value::compare::loose_equals as loose;
use php_value::{Object, Value};
use php2rust_doctrine_orm::collection_object as coll;
use php2rust_doctrine_orm::exception::{
    ENTITY_IDENTITY_COLLISION_EXCEPTION, ORM_INVALID_ARGUMENT_EXCEPTION,
};
use php2rust_doctrine_orm::persistent_collection::PersistentCollection;
use php2rust_doctrine_orm::query::context::{HINT_REFRESH, Hint};
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
        .await
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
    let _reference = em
        .get_reference(CMS_USER, Value::Int(id + 1))
        .await
        .unwrap();

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

#[tokio::test]
async fn testBasicQuery() {
    let em = set_up(&CMS_MODELS).await;
    let user = cms_user_with("Guilherme", "gblanco", "developer");
    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    let query = em.create_query("select u from Doctrine\\Tests\\Models\\CMS\\CmsUser u");

    let users = list(&query.get_object_result().await.unwrap());
    assert_eq!(users.len(), 1);
    let u0 = obj(&users[0]);
    assert_eq!(get(&u0, "name"), s("Guilherme"));
    assert_eq!(get(&u0, "username"), s("gblanco"));
    assert_eq!(get(&u0, "status"), s("developer"));

    em.clear().unwrap();

    assert_results_are_the_same(&query).await;

    let users_array = query.get_array_result().await.unwrap();
    assert_eq!(list(&users_array).len(), 1);
    assert_eq!(at(&at(&users_array, 0), "name"), s("Guilherme"));
    assert_eq!(at(&at(&users_array, 0), "username"), s("gblanco"));
    assert_eq!(at(&at(&users_array, 0), "status"), s("developer"));

    let users_scalar = query.get_scalar_result().await.unwrap();
    assert_eq!(list(&users_scalar).len(), 1);
    assert_eq!(at(&at(&users_scalar, 0), "u_name"), s("Guilherme"));
    assert_eq!(at(&at(&users_scalar, 0), "u_username"), s("gblanco"));
    assert_eq!(at(&at(&users_scalar, 0), "u_status"), s("developer"));
}

#[tokio::test]
async fn testBasicOneToManyInnerJoin() {
    let em = set_up(&CMS_MODELS).await;
    let user = cms_user_with("Guilherme", "gblanco", "developer");
    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    let query = em.create_query(
        "select u from Doctrine\\Tests\\Models\\CMS\\CmsUser u join u.phonenumbers p",
    );
    let users = list(&query.get_object_result().await.unwrap());
    assert_eq!(users.len(), 0);
}

#[tokio::test]
async fn testBasicOneToManyLeftJoin() {
    let em = set_up(&CMS_MODELS).await;
    let user = cms_user_with("Guilherme", "gblanco", "developer");
    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    let query = em.create_query(
        "select u,p from Doctrine\\Tests\\Models\\CMS\\CmsUser u left join u.phonenumbers p",
    );
    let users = list(&query.get_object_result().await.unwrap());
    assert_eq!(users.len(), 1);
    let u0 = obj(&users[0]);
    assert_eq!(get(&u0, "name"), s("Guilherme"));
    assert_eq!(get(&u0, "username"), s("gblanco"));
    assert_eq!(get(&u0, "status"), s("developer"));
    let phonenumbers = get(&u0, "phonenumbers");
    let pc = php2rust_doctrine_orm::persistent_collection::PersistentCollection::from_value(
        &phonenumbers,
    )
    .expect("a PersistentCollection");
    assert!(pc.is_initialized());
    assert_eq!(pc.count().await.unwrap(), 0);
}

#[tokio::test]
async fn testSingleColumnQuery() {
    let em = set_up(&CMS_MODELS).await;
    let gregoire = cms_user_with("Gregoire", "greg0ire", "developer");
    em.persist(&gregoire).unwrap();
    let bhushan = cms_user_with("Bhushan", "bhushan", "developer");
    em.persist(&bhushan).unwrap();
    em.flush().await.unwrap();

    let query = em.create_query(
        "select u.username from Doctrine\\Tests\\Models\\CMS\\CmsUser u order by u.username DESC",
    );
    let users = list(&query.get_single_column_result().await.unwrap());
    assert_eq!(users, vec![s("greg0ire"), s("bhushan")]);
}

/// `$this->isUninitializedObject($value)`.
fn is_uninitialized(
    em: &php2rust_doctrine_orm::entity_manager::EntityManager,
    value: &Value,
) -> bool {
    em.is_uninitialized_object(value)
}

/// `$collection->isInitialized()` of a persistent collection property.
fn is_initialized(object: &Object, name: &str) -> bool {
    PersistentCollection::from_value(&get(object, name))
        .expect("a persistent collection")
        .is_initialized()
}

/// The user with three phone numbers (`100` to `102`).
fn gblanco_with_three_phonenumbers() -> Object {
    let user = cms_user_with("Guilherme", "gblanco", "developer");

    for i in 0..3 {
        let phone = cms_phonenumber(&(100 + i).to_string());
        add_phonenumber(&user, &phone);
    }
    user
}

async fn single(
    em: &std::sync::Arc<php2rust_doctrine_orm::entity_manager::EntityManager>,
    dql: &str,
) -> Object {
    obj(&em.create_query(dql).get_single_result(None).await.unwrap())
}

#[tokio::test]
async fn testBasicOneToOne() {
    let em = set_up(&CMS_MODELS).await;
    let user = cms_user_with("Roman", "romanb", "developer");

    let address = cms_address("Germany", "12345", "Berlin");

    set(&user, "address", Value::Object(address.clone())); // inverse side
    set(&address, "user", Value::Object(user.clone())); // owning side!

    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    // Check that the foreign key has been set
    let user_id = fetch_one_with(
        &em,
        "SELECT user_id FROM cms_addresses WHERE id=?",
        vec![get(&address, "id")],
    )
    .await;
    assert!(
        matches!(user_id, Value::Int(_))
            || user_id
                .to_php_string()
                .is_some_and(|s| s.to_string_lossy().parse::<f64>().is_ok()),
        "{user_id:?} is not numeric"
    );

    em.clear().unwrap();

    let query =
        em.create_query("select u from \\Doctrine\\Tests\\Models\\CMS\\CmsUser u where u.id=?1");
    query.set_parameter(1, user_id, None);
    let user2 = obj(&query.get_single_result(None).await.unwrap());

    // Address has been eager-loaded because it cant be lazy
    let address2 = get(&user2, "address");
    assert!(matches!(&address2, Value::Object(o) if o.class() == CMS_ADDRESS));
    assert!(!is_uninitialized(&em, &address2));
}

#[tokio::test]
async fn testOneToManyCascadeRemove() {
    let em = set_up(&CMS_MODELS).await;
    let user = gblanco_with_three_phonenumbers();

    em.persist(&user).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let gblanco = single(
        &em,
        "select u from Doctrine\\Tests\\Models\\CMS\\CmsUser u where u.username='gblanco'",
    )
    .await;

    em.remove(&gblanco).await.unwrap();
    em.flush().await.unwrap();

    em.clear().unwrap();

    for dql in [
        "select count(p.phonenumber) from Doctrine\\Tests\\Models\\CMS\\CmsPhonenumber p",
        "select count(u.id) from Doctrine\\Tests\\Models\\CMS\\CmsUser u",
    ] {
        let count = em
            .create_query(dql)
            .get_single_scalar_result()
            .await
            .unwrap();
        assert!(loose(&Value::Int(0), &count), "{count:?}");
    }
}

#[tokio::test]
async fn testFlushDoesNotIssueUnnecessaryUpdates() {
    let (em, log) = query_log::set_up_logged(&CMS_MODELS).await;
    let user = cms_user_with("Guilherme", "gblanco", "developer");

    let address = cms_address("Germany", "12345", "Berlin");

    set(&address, "user", Value::Object(user.clone()));
    set(&user, "address", Value::Object(address.clone()));

    let article = cms_article("A Test Article!", "Lorem ipsum dolor sunt.");
    // $article->setAuthor($user)
    set(&article, "user", Value::Object(user.clone()));

    em.persist(&article).unwrap();
    em.persist(&user).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();

    let user2 = single(
        &em,
        "select u,a,ad from Doctrine\\Tests\\Models\\CMS\\CmsUser u join u.articles a join u.address ad",
    )
    .await;

    assert_eq!(count_of(&user2, "articles").await, 1);
    assert!(matches!(get(&user2, "address"), Value::Object(o) if o.class() == CMS_ADDRESS));

    log.reset_and_enable();
    em.flush().await.unwrap();

    assert_eq!(log.queries().len(), 0, "{:?}", log.queries());
}

#[tokio::test]
async fn testDqlRefreshResetsCollection() {
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
    let dql = "SELECT u FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u WHERE u.id = ?1";
    let query = em.create_query(dql);
    query.set_parameter(1, get(&user, "id"), None);
    query.set_hint(HINT_REFRESH, Hint::Value(Value::Bool(true)));
    let user = obj(&query.get_single_result(None).await.unwrap());

    assert_eq!(count_of(&user, "phonenumbers").await, 1);
}

#[tokio::test]
async fn testAddToCollectionDoesNotInitialize() {
    let em = set_up(&CMS_MODELS).await;
    let user = gblanco_with_three_phonenumbers();

    em.persist(&user).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    assert_eq!(count_of(&user, "phonenumbers").await, 3);

    let gblanco = single(
        &em,
        "select u from Doctrine\\Tests\\Models\\CMS\\CmsUser u where u.username='gblanco'",
    )
    .await;

    assert!(!is_initialized(&gblanco, "phonenumbers"));

    let new_phone = cms_phonenumber("555");
    add_phonenumber(&gblanco, &new_phone);

    assert!(!is_initialized(&gblanco, "phonenumbers"));
    em.persist(&gblanco).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();

    let gblanco2 = single(
        &em,
        "select u, p from Doctrine\\Tests\\Models\\CMS\\CmsUser u join u.phonenumbers p where u.username='gblanco'",
    )
    .await;
    assert_eq!(count_of(&gblanco2, "phonenumbers").await, 4);
}

#[tokio::test]
async fn testInitializeCollectionWithNewObjectsRetainsNewObjects() {
    let em = set_up(&CMS_MODELS).await;
    let user = gblanco_with_three_phonenumbers();

    em.persist(&user).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    assert_eq!(count_of(&user, "phonenumbers").await, 3);

    let gblanco = single(
        &em,
        "select u from Doctrine\\Tests\\Models\\CMS\\CmsUser u where u.username='gblanco'",
    )
    .await;

    assert!(!is_initialized(&gblanco, "phonenumbers"));

    let new_phone = cms_phonenumber("555");
    add_phonenumber(&gblanco, &new_phone);

    assert!(!is_initialized(&gblanco, "phonenumbers"));
    assert_eq!(count_of(&gblanco, "phonenumbers").await, 4);
    assert!(is_initialized(&gblanco, "phonenumbers"));

    em.flush().await.unwrap();
    em.clear().unwrap();

    let gblanco2 = single(
        &em,
        "select u, p from Doctrine\\Tests\\Models\\CMS\\CmsUser u join u.phonenumbers p where u.username='gblanco'",
    )
    .await;
    assert_eq!(count_of(&gblanco2, "phonenumbers").await, 4);
}

#[tokio::test]
async fn testSetToOneAssociationWithGetReference() {
    let em = set_up(&CMS_MODELS).await;
    let user = cms_user_with("Guilherme", "gblanco", "developer");
    em.persist(&user).unwrap();

    let address = cms_address("Germany", "12345", "Berlin");
    em.persist(&address).unwrap();

    em.flush().await.unwrap();
    let user_id = get(&user, "id");
    em.clear().unwrap();
    let user = em.find(CMS_USER, user_id).await.unwrap().unwrap();

    // Assume we only got the identifier of the address and now want to attach
    // that address to the user without actually loading it, using getReference().
    let address_ref = em
        .get_reference(CMS_ADDRESS, get(&address, "id"))
        .await
        .unwrap()
        .unwrap();

    // Ugh! Initializes address 'cause of $address->setUser($user)! (PHP
    // initializes the ghost on the property write.)
    em.initialize_object(&address_ref).await.unwrap();
    set_address(&user, &address_ref);

    em.flush().await.unwrap();
    em.clear().unwrap();

    // Assume we only got the identifier of the user and now want to attach
    // the article to the user without actually loading it, using getReference().
    let user_ref = em
        .get_reference(CMS_USER, get(&user, "id"))
        .await
        .unwrap()
        .unwrap();
    assert!(is_uninitialized(&em, &Value::Object(user_ref.clone())));

    let article = cms_article("topic", "text");
    // $article->setAuthor($userRef)
    set(&article, "user", Value::Object(user_ref.clone()));

    em.persist(&article).unwrap();
    em.flush().await.unwrap();

    assert!(is_uninitialized(&em, &Value::Object(user_ref.clone())));

    em.clear().unwrap();

    // Check with a fresh load that the association is indeed there
    let gblanco = single(
        &em,
        "select u, a from Doctrine\\Tests\\Models\\CMS\\CmsUser u join u.articles a where u.username='gblanco'",
    )
    .await;

    assert_eq!(gblanco.class(), CMS_USER);
    let article0 = obj(&coll::get(&get(&gblanco, "articles"), 0).await.unwrap());
    assert_eq!(article0.class(), CMS_ARTICLE);
    assert!(get(&article, "id").identical(&get(&article0, "id")));
    assert!(s("text").identical(&get(&article0, "text")));
}

#[tokio::test]
async fn testAddToToManyAssociationWithGetReference() {
    let em = set_up(&CMS_MODELS).await;
    let group = cms_group("admins");
    em.persist(&group).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    // Assume we only got the identifier of the user and now want to attach
    // the article to the user without actually loading it, using getReference().
    let group_ref = em
        .get_reference(CMS_GROUP, get(&group, "id"))
        .await
        .unwrap()
        .unwrap();
    assert!(is_uninitialized(&em, &Value::Object(group_ref.clone())));

    let user = cms_user();
    set(&user, "name", s("Guilherme"));
    set(&user, "username", s("gblanco"));
    coll::add(&get(&user, "groups"), Value::Object(group_ref.clone())).unwrap();

    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    assert!(is_uninitialized(&em, &Value::Object(group_ref.clone())));

    em.clear().unwrap();

    // Check with a fresh load that the association is indeed there
    let gblanco = single(
        &em,
        "select u, a from Doctrine\\Tests\\Models\\CMS\\CmsUser u join u.groups a where u.username='gblanco'",
    )
    .await;

    assert_eq!(gblanco.class(), CMS_USER);
    let group0 = obj(&coll::get(&get(&gblanco, "groups"), 0).await.unwrap());
    assert_eq!(group0.class(), CMS_GROUP);
    assert!(get(&group, "id").identical(&get(&group0, "id")));
    assert!(s("admins").identical(&get(&group0, "name")));
}

#[tokio::test]
async fn testQueryEntityByReference() {
    let em = set_up(&CMS_MODELS).await;
    let user = cms_user_with("Guilherme", "gblanco", "developer");

    let address = cms_address("Germany", "12345", "Berlin");

    set_address(&user, &address);

    let persisted = user.clone();
    em.wrap_in_transaction(|em| async move {
        em.persist(&persisted)?;
        Ok(())
    })
    .await
    .unwrap();
    em.clear().unwrap();

    let user_ref = em
        .get_reference(CMS_USER, get(&user, "id"))
        .await
        .unwrap()
        .unwrap();
    let query = em.create_query(
        "select a from Doctrine\\Tests\\Models\\CMS\\CmsAddress a where a.user = :user",
    );
    query.set_parameter("user", Value::Object(user_ref.clone()), None);
    let address2 = obj(&query.get_single_result(None).await.unwrap());

    assert!(same_value(&get(&address2, "user"), &user_ref));
    assert!(is_uninitialized(&em, &Value::Object(user_ref.clone())));
    assert!(loose(&s("Germany"), &get(&address2, "country")));
    assert!(loose(&s("Berlin"), &get(&address2, "city")));
    assert!(loose(&s("12345"), &get(&address2, "zip")));
}

#[tokio::test]
async fn testCreateEntityOfProxy() {
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
    em.clear().unwrap();

    let user_id = get(&user, "id");
    let _user = em
        .get_reference(CMS_USER, user_id.clone())
        .await
        .unwrap()
        .unwrap();

    let dql = "SELECT u FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u WHERE u.id = ?1";
    let query = em.create_query(dql);
    query.set_parameter(1, user_id, None);
    let user = obj(&query.get_single_result(None).await.unwrap());

    assert_eq!(count_of(&user, "phonenumbers").await, 1);
}
