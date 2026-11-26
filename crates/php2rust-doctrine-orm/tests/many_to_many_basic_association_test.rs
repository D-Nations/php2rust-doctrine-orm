//! Upstream: doctrine/orm tests/Tests/ORM/Functional/ManyToManyBasicAssociationTest.php

#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;
#[path = "common/query_log.rs"]
mod query_log;

use std::sync::Arc;

use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::persistent_collection::PersistentCollection;

use orm::*;
use query_log::set_up_logged;

fn persistent(user: &Object, name: &str) -> Arc<PersistentCollection> {
    PersistentCollection::from_value(&get(user, name)).expect("a PersistentCollection")
}

async fn add_cms_user_gblanco_with_groups(em: &EntityManager, group_count: usize) -> Object {
    let user = cms_user_with("Guilherme", "gblanco", "developer");
    for i in 0..group_count {
        let group = cms_group(&format!("Developers_{i}"));
        add_group(&user, &group);
    }
    em.persist(&user).unwrap();
    em.flush().await.unwrap();
    assert!(
        !get(&user, "id").is_null(),
        "User 'gblanco' should have an ID assigned after the persist()/flush() operation."
    );
    user
}

#[tokio::test]
async fn test_many_to_many_add_remove() {
    let em = set_up(&CMS_MODELS).await;
    let user = add_cms_user_gblanco_with_groups(&em, 2).await;
    em.clear().unwrap();
    let u_rep = em.get_repository(CMS_USER).unwrap();

    // Get user
    let Value::Object(user) = u_rep.call("findOneById", vec![get(&user, "id")]).await.unwrap() else {
        panic!("Has to return exactly one entry.");
    };
    assert!(!persistent(&user, "groups").is_initialized());

    // Check groups
    assert_eq!(persistent(&user, "groups").count().await.unwrap(), 2);
    assert!(persistent(&user, "groups").is_initialized());

    // Remove first group
    coll::remove(&get(&user, "groups"), 0).await.unwrap();
    em.flush().await.unwrap();

    assert!(!persistent(&user, "groups").is_dirty());

    em.clear().unwrap();

    // Reload same user
    let Value::Object(user2) = u_rep.call("findOneById", vec![get(&user, "id")]).await.unwrap() else {
        panic!("a user");
    };

    // Check groups
    assert_eq!(persistent(&user2, "groups").count().await.unwrap(), 1);
}

#[tokio::test]
async fn test_many_to_many_inverse_side_ignored() {
    let em = set_up(&CMS_MODELS).await;
    let user = add_cms_user_gblanco_with_groups(&em, 0).await;

    let group = cms_group("Humans");

    // modify directly, addUser() would also (properly) set the owning side
    append(&group, "users", Value::Object(user.clone()));

    em.persist(&user).unwrap();
    em.persist(&group).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    // Association should not exist
    let user2 = em.find(CMS_USER, get(&user, "id")).await.unwrap();
    let user2 = user2.expect("Has to return exactly one entry.");
    assert_eq!(count_of(&user2, "groups").await, 0);
}

#[tokio::test]
async fn test_retrieve_many_to_many_and_add_more() {
    let em = set_up(&CMS_MODELS).await;
    let user = add_cms_user_gblanco_with_groups(&em, 2).await;

    let group = cms_group("Developers_Fresh");
    em.persist(&group).unwrap();
    em.flush().await.unwrap();

    em.clear().unwrap();

    let fresh_user = em.find(CMS_USER, get(&user, "id")).await.unwrap().unwrap();

    let new_group = cms_group("12Monkeys");
    add_group(&fresh_user, &new_group);

    assert!(
        !persistent(&fresh_user, "groups").is_initialized(),
        "CmsUser::groups Collection has to be uninitialized for this test."
    );

    em.flush().await.unwrap();

    assert!(
        !persistent(&fresh_user, "groups").is_initialized(),
        "CmsUser::groups Collection has to be uninitialized for this test."
    );
    assert_eq!(count_of(&fresh_user, "groups").await, 3);
    assert_eq!(
        persistent(&fresh_user, "groups").get_snapshot().len(),
        3,
        "Snapshot of CmsUser::groups should contain 3 entries."
    );

    em.clear().unwrap();

    let fresh_user = em.find(CMS_USER, get(&user, "id")).await.unwrap().unwrap();
    assert_eq!(count_of(&fresh_user, "groups").await, 3);
}

#[tokio::test]
async fn test_remove_user_with_many_groups() {
    let (em, log) = set_up_logged(&CMS_MODELS).await;
    let user = add_cms_user_gblanco_with_groups(&em, 10).await;
    let user_id = get(&user, "id");

    em.remove(&user).await.unwrap();

    log.reset_and_enable();

    em.flush().await.unwrap();

    // This takes three queries: One to delete all user -> group join table rows for the user,
    // one to delete all user -> tags join table rows for the user, and a final one to delete the user itself.
    assert_eq!(log.queries().len(), 3, "{:?}", log.queries());

    let new_user = em.find(CMS_USER, user_id).await.unwrap();
    assert!(new_user.is_none());
}

#[tokio::test]
async fn test_remove_group_with_user() {
    let (em, log) = set_up_logged(&CMS_MODELS).await;
    let user = add_cms_user_gblanco_with_groups(&em, 5).await;

    let another_user = cms_user_with("Joe Doe", "joe_doe", "QA Engineer");

    for group in elements(&user, "groups").await {
        add_group(&another_user, &group);
    }

    em.persist(&another_user).unwrap();
    em.flush().await.unwrap();

    for group in elements(&user, "groups").await {
        em.remove(&group).await.unwrap();
    }

    log.reset_and_enable();

    em.flush().await.unwrap();

    // This takes 5 * 2 queries – for each group to be removed, one to remove all join table rows
    // for the CmsGroup -> CmsUser inverse side association (for both users at once),
    // and one for the group itself.
    assert_eq!(log.queries().len(), 10, "{:?}", log.queries());

    // Changes to in-memory collection have been made and flushed
    assert_eq!(count_of(&user, "groups").await, 0);
    assert!(!persistent(&user, "groups").is_dirty());

    em.clear().unwrap();

    // Changes have been made to the database
    let new_user = em.find(CMS_USER, get(&user, "id")).await.unwrap().unwrap();
    assert_eq!(count_of(&new_user, "groups").await, 0);
}

#[tokio::test]
async fn test_dereference_collection_delete() {
    let (em, log) = set_up_logged(&CMS_MODELS).await;
    let user = add_cms_user_gblanco_with_groups(&em, 2).await;
    set(&user, "groups", Value::Null);

    log.reset_and_enable();

    em.flush().await.unwrap();

    // It takes one query to remove all join table rows for the user at once
    assert_eq!(log.queries().len(), 1, "{:?}", log.queries());

    em.clear().unwrap();

    let new_user = em.find(CMS_USER, get(&user, "id")).await.unwrap().unwrap();
    assert_eq!(count_of(&new_user, "groups").await, 0);
}

#[tokio::test]
async fn test_clear_and_reset_collection() {
    let em = set_up(&CMS_MODELS).await;
    let user = add_cms_user_gblanco_with_groups(&em, 2).await;
    let group1 = cms_group("Developers_New1");
    let group2 = cms_group("Developers_New2");

    em.persist(&group1).unwrap();
    em.persist(&group2).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let user = em.find(CMS_USER, get(&user, "id")).await.unwrap().unwrap();

    let collection = ArrayCollectionObject::value(PhpArray::list([
        Value::Object(group1.clone()),
        Value::Object(group2.clone()),
    ]));
    set(&user, "groups", collection);

    em.flush().await.unwrap();

    assert!(
        PersistentCollection::from_value(&get(&user, "groups")).is_some(),
        "UnitOfWork should have replaced ArrayCollection with PersistentCollection."
    );

    em.flush().await.unwrap();
    em.clear().unwrap();

    let user = em.find(CMS_USER, get(&user, "id")).await.unwrap().unwrap();
    assert_eq!(count_of(&user, "groups").await, 2);
    let groups = elements(&user, "groups").await;
    assert_eq!(get(&groups[0], "name"), s("Developers_New1"));
    assert_eq!(get(&groups[1], "name"), s("Developers_New2"));
}

#[tokio::test]
async fn test_initialize_persistent_collection() {
    let em = set_up(&CMS_MODELS).await;
    let user = add_cms_user_gblanco_with_groups(&em, 2).await;
    em.clear().unwrap();

    let user = em.find(CMS_USER, get(&user, "id")).await.unwrap().unwrap();

    assert!(!persistent(&user, "groups").is_initialized(), "Pre-condition: lazy collection");
    let Value::Object(groups) = get(&user, "groups") else {
        panic!("a collection");
    };
    em.get_unit_of_work().initialize_object(&groups).await.unwrap();
    assert!(
        persistent(&user, "groups").is_initialized(),
        "Collection should be initialized after calling UnitOfWork::initializeObject()"
    );
}

#[tokio::test]
async fn test_clear_before_lazy_load() {
    let em = set_up(&CMS_MODELS).await;
    let user = add_cms_user_gblanco_with_groups(&em, 4).await;

    em.clear().unwrap();

    let user = em.find(CMS_USER, get(&user, "id")).await.unwrap().unwrap();
    coll::clear(&get(&user, "groups")).await.unwrap();
    assert_eq!(count_of(&user, "groups").await, 0);

    em.flush().await.unwrap();

    let user = em.find(CMS_USER, get(&user, "id")).await.unwrap().unwrap();
    assert_eq!(count_of(&user, "groups").await, 0);
}
