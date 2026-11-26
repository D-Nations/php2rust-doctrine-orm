//! Upstream: doctrine/orm tests/Tests/ORM/Functional/ManyToManyBasicAssociationTest.php

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

use std::sync::Arc;

use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_collections::{Criteria, SortDirection};
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
async fn testManyToManyAddRemove() {
    let em = set_up(&CMS_MODELS).await;
    let user = add_cms_user_gblanco_with_groups(&em, 2).await;
    em.clear().unwrap();
    let u_rep = em.get_repository(CMS_USER).unwrap();

    // Get user
    let Value::Object(user) = u_rep
        .call("findOneById", vec![get(&user, "id")])
        .await
        .unwrap()
    else {
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
    let Value::Object(user2) = u_rep
        .call("findOneById", vec![get(&user, "id")])
        .await
        .unwrap()
    else {
        panic!("a user");
    };

    // Check groups
    assert_eq!(persistent(&user2, "groups").count().await.unwrap(), 1);
}

#[tokio::test]
async fn testManyToManyInverseSideIgnored() {
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
async fn testRetrieveManyToManyAndAddMore() {
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
async fn testRemoveUserWithManyGroups() {
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
async fn testRemoveGroupWithUser() {
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
async fn testDereferenceCollectionDelete() {
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
async fn testClearAndResetCollection() {
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
async fn testInitializePersistentCollection() {
    let em = set_up(&CMS_MODELS).await;
    let user = add_cms_user_gblanco_with_groups(&em, 2).await;
    em.clear().unwrap();

    let user = em.find(CMS_USER, get(&user, "id")).await.unwrap().unwrap();

    assert!(
        !persistent(&user, "groups").is_initialized(),
        "Pre-condition: lazy collection"
    );
    let Value::Object(groups) = get(&user, "groups") else {
        panic!("a collection");
    };
    em.get_unit_of_work()
        .initialize_object(&groups)
        .await
        .unwrap();
    assert!(
        persistent(&user, "groups").is_initialized(),
        "Collection should be initialized after calling UnitOfWork::initializeObject()"
    );
}

#[tokio::test]
async fn testClearBeforeLazyLoad() {
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

async fn assert_gblanco_group_count_is(em: &Arc<EntityManager>, expected_group_count: i64) {
    let count_dql = "SELECT count(g.id) FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u JOIN u.groups g WHERE u.username = 'gblanco'";
    assert_loose_eq(
        &Value::Int(expected_group_count),
        &em.create_query(count_dql)
            .get_single_scalar_result()
            .await
            .unwrap(),
    );
}

#[tokio::test]
async fn testBasicManyToManyJoin() {
    use php2rust_doctrine_orm::unit_of_work::STATE_MANAGED;

    let em = set_up(&CMS_MODELS).await;
    add_cms_user_gblanco_with_groups(&em, 1).await;
    em.clear().unwrap();

    assert_eq!(0, em.get_unit_of_work().size());

    let query =
        em.create_query("select u, g from Doctrine\\Tests\\Models\\CMS\\CmsUser u join u.groups g");

    let result = query.get_object_result().await.unwrap();

    assert_eq!(2, em.get_unit_of_work().size());
    let user0 = obj(&at(&result, 0));
    assert_eq!(user0.class(), CMS_USER);
    assert_loose_eq(&s("Guilherme"), &get(&user0, "name"));
    assert_eq!(1, count_of(&user0, "groups").await);
    let groups = elements(&user0, "groups").await;
    assert_loose_eq(&s("Developers_0"), &get(&groups[0], "name"));

    let uow = em.get_unit_of_work();
    assert_eq!(STATE_MANAGED, uow.get_entity_state(&user0).await.unwrap());
    assert_eq!(
        STATE_MANAGED,
        uow.get_entity_state(&groups[0]).await.unwrap()
    );

    persistent(&user0, "groups");
    persistent(&groups[0], "users");

    coll::clear(&get(&groups[0], "users")).await.unwrap();
    coll::clear(&get(&user0, "groups")).await.unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();

    let query =
        em.create_query("select u, g from Doctrine\\Tests\\Models\\CMS\\CmsUser u join u.groups g");
    assert_eq!(0, list(&query.get_object_result().await.unwrap()).len());
}

#[tokio::test]
async fn testManyToManyCollectionClearAndAdd() {
    let em = set_up(&CMS_MODELS).await;
    let group_count = 10;
    let user = add_cms_user_gblanco_with_groups(&em, group_count).await;

    let groups = coll::get_values(&get(&user, "groups")).await.unwrap();
    coll::clear(&get(&user, "groups")).await.unwrap();

    for group in groups {
        coll::add(&get(&user, "groups"), group).unwrap();
    }

    assert!(persistent(&user, "groups").is_dirty());

    assert_eq!(
        group_count,
        count_of(&user, "groups").await,
        "There should be 10 groups in the collection."
    );

    em.flush().await.unwrap();

    assert_gblanco_group_count_is(&em, 10).await;
}

#[tokio::test]
async fn testManyToManyCollectionClearing() {
    let (em, log) = set_up_logged(&CMS_MODELS).await;
    let user = add_cms_user_gblanco_with_groups(&em, 10).await;

    // Check that there are indeed 10 links in the association table
    assert_gblanco_group_count_is(&em, 10).await;

    coll::clear(&get(&user, "groups")).await.unwrap();

    log.reset_and_enable();
    em.flush().await.unwrap();

    // Deletions of entire collections happen in a single query (the log
    // has no transaction commands)
    assert_eq!(1, log.queries().len(), "{:?}", log.queries());

    // Check that the links in the association table have been deleted
    assert_gblanco_group_count_is(&em, 0).await;
}

#[tokio::test]
async fn testUnsetManyToMany() {
    let em = set_up(&CMS_MODELS).await;
    let user = add_cms_user_gblanco_with_groups(&em, 1).await;

    // inverse side
    // owning side!
    let group = obj(&coll::get(&get(&user, "groups"), 0).await.unwrap());
    coll::remove(&get(&group, "users"), 0).await.unwrap();
    coll::remove(&get(&user, "groups"), 0).await.unwrap();

    em.flush().await.unwrap();

    // Check that the link in the association table has been deleted
    assert_gblanco_group_count_is(&em, 0).await;
}

#[tokio::test]
async fn testWorkWithDqlHydratedEmptyCollection() {
    let em = set_up(&CMS_MODELS).await;
    let user = add_cms_user_gblanco_with_groups(&em, 0).await;
    let group = cms_group("Developers0");
    em.persist(&group).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();

    let new_user = em
        .create_query(
            "SELECT u, g FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u LEFT JOIN u.groups g WHERE u.id = ?1",
        )
        .set_parameter(1, get(&user, "id"), None)
        .get_single_result(None)
        .await
        .unwrap();
    let new_user = obj(&new_user);
    assert_eq!(0, count_of(&new_user, "groups").await);
    persistent(&new_user, "groups").get_mapping().unwrap();

    add_group(&new_user, &group);

    em.flush().await.unwrap();
    em.clear().unwrap();

    let new_user = em.find(CMS_USER, get(&user, "id")).await.unwrap().unwrap();
    assert_eq!(1, count_of(&new_user, "groups").await);
}

/// `Criteria::create(true)` (`Criteria::ASC` is not defined in collections 3).
fn criteria() -> Criteria {
    Criteria::create()
}

/// `$this->_em->find($user::class, $user->id)`.
async fn find_user(em: &EntityManager, user: &Object) -> Object {
    em.find(CMS_USER, get(user, "id")).await.unwrap().unwrap()
}

fn name_of(group: &Value) -> Value {
    match group {
        Value::Object(o) => get(o, "name"),
        other => panic!("an object, got {other:?}"),
    }
}

#[tokio::test]
async fn testMatching() {
    let em = set_up(&CMS_MODELS).await;
    let user = add_cms_user_gblanco_with_groups(&em, 2).await;
    em.clear().unwrap();

    let user = find_user(&em, &user).await;

    let groups = get(&user, "groups");
    assert!(
        !persistent(&user, "groups").is_initialized(),
        "Pre-condition: lazy collection"
    );

    let mut criteria = criteria();
    criteria.r#where(Criteria::expr().eq("name", s("Developers_0")));
    let result = coll::matching(&groups, &criteria).await.unwrap();

    assert_eq!(coll::count(&result).await.unwrap(), 1);

    let first_group = coll::first(&result).await.unwrap();
    assert_eq!(name_of(&first_group), s("Developers_0"));

    assert!(
        !persistent(&user, "groups").is_initialized(),
        "Post-condition: matching does not initialize collection"
    );
}

#[tokio::test]
async fn testMatchingWithLimit() {
    let em = set_up(&CMS_MODELS).await;
    let user = add_cms_user_gblanco_with_groups(&em, 2).await;
    em.clear().unwrap();

    let user = find_user(&em, &user).await;

    let groups = get(&user, "groups");
    assert!(
        !persistent(&user, "groups").is_initialized(),
        "Pre-condition: lazy collection"
    );

    let mut criteria = criteria();
    criteria.set_max_results(Some(1));
    let result = coll::matching(&groups, &criteria).await.unwrap();

    assert_eq!(coll::count(&result).await.unwrap(), 1);

    assert!(
        !persistent(&user, "groups").is_initialized(),
        "Post-condition: matching does not initialize collection"
    );
}

#[tokio::test]
async fn testMatchingWithOffset() {
    let em = set_up(&CMS_MODELS).await;
    let user = add_cms_user_gblanco_with_groups(&em, 2).await;
    em.clear().unwrap();

    let user = find_user(&em, &user).await;

    let groups = get(&user, "groups");
    assert!(
        !persistent(&user, "groups").is_initialized(),
        "Pre-condition: lazy collection"
    );

    let mut criteria = criteria();
    criteria.set_first_result(1);
    let result = coll::matching(&groups, &criteria).await.unwrap();

    assert_eq!(coll::count(&result).await.unwrap(), 1);

    let first_group = coll::first(&result).await.unwrap();
    assert_eq!(name_of(&first_group), s("Developers_1"));

    assert!(
        !persistent(&user, "groups").is_initialized(),
        "Post-condition: matching does not initialize collection"
    );
}

#[tokio::test]
async fn testMatchingWithLimitAndOffset() {
    let em = set_up(&CMS_MODELS).await;
    let user = add_cms_user_gblanco_with_groups(&em, 5).await;
    em.clear().unwrap();

    let user = find_user(&em, &user).await;

    let groups = get(&user, "groups");
    assert!(
        !persistent(&user, "groups").is_initialized(),
        "Pre-condition: lazy collection"
    );

    let mut criteria = criteria();
    criteria.set_first_result(1).set_max_results(Some(3));
    let result = coll::matching(&groups, &criteria).await.unwrap();

    assert_eq!(coll::count(&result).await.unwrap(), 3);

    let first_group = coll::first(&result).await.unwrap();
    assert_eq!(name_of(&first_group), s("Developers_1"));

    // `$result->last()`.
    let last_group = coll::get_values(&result).await.unwrap().pop().unwrap();
    assert_eq!(name_of(&last_group), s("Developers_3"));

    assert!(
        !persistent(&user, "groups").is_initialized(),
        "Post-condition: matching does not initialize collection"
    );
}

#[tokio::test]
async fn testMatchingWithInCondition() {
    let em = set_up(&CMS_MODELS).await;
    let user = add_cms_user_gblanco_with_groups(&em, 2).await;
    em.clear().unwrap();

    let user = find_user(&em, &user).await;

    let groups = get(&user, "groups");
    assert!(
        !persistent(&user, "groups").is_initialized(),
        "Pre-condition: lazy collection"
    );

    let mut criteria = criteria();
    criteria.r#where(Criteria::expr().r#in("name", PhpArray::list([s("Developers_1")])));
    let result = coll::matching(&groups, &criteria).await.unwrap();

    assert_eq!(coll::count(&result).await.unwrap(), 1);
    assert_eq!(
        name_of(&coll::get(&result, 0).await.unwrap()),
        s("Developers_1")
    );

    assert!(
        !persistent(&user, "groups").is_initialized(),
        "Post-condition: matching does not initialize collection"
    );
}

#[tokio::test]
async fn testMatchingWithNotInCondition() {
    let em = set_up(&CMS_MODELS).await;
    let user = add_cms_user_gblanco_with_groups(&em, 2).await;
    em.clear().unwrap();

    let user = find_user(&em, &user).await;

    let groups = get(&user, "groups");
    assert!(
        !persistent(&user, "groups").is_initialized(),
        "Pre-condition: lazy collection"
    );

    let mut criteria = criteria();
    criteria.r#where(Criteria::expr().not_in("name", PhpArray::list([s("Developers_0")])));
    let result = coll::matching(&groups, &criteria).await.unwrap();

    assert_eq!(coll::count(&result).await.unwrap(), 1);
    assert_eq!(
        name_of(&coll::get(&result, 0).await.unwrap()),
        s("Developers_1")
    );

    assert!(
        !persistent(&user, "groups").is_initialized(),
        "Post-condition: matching does not initialize collection"
    );
}

/// `->matching($criteria)->map(fn ($x) => $x->getName())->toArray()`.
async fn matched_names(collection: &Value, criteria: &Criteria) -> Vec<Value> {
    let result = coll::matching(collection, criteria).await.unwrap();
    coll::to_array(&result)
        .await
        .unwrap()
        .values()
        .map(name_of)
        .collect()
}

#[tokio::test]
async fn testManyToManyOrderByIsNotIgnored() {
    let em = set_up(&CMS_MODELS).await;
    let user = add_cms_user_gblanco_with_groups(&em, 1).await;

    let group1 = cms_group("C");
    let group2 = cms_group("A");
    let group3 = cms_group("B");

    add_group(&user, &group1);
    add_group(&user, &group2);
    add_group(&user, &group3);

    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    em.clear().unwrap();

    let user = find_user(&em, &user).await;

    let mut criteria = criteria();
    criteria.order_by([("name", SortDirection::Ascending)]);

    assert_eq!(
        vec![s("A"), s("B"), s("C"), s("Developers_0")],
        matched_names(&get(&user, "groups"), &criteria).await
    );
}

#[tokio::test]
async fn testManyToManyOrderByHonorsFieldNameColumnNameAliases() {
    let em = set_up(&CMS_MODELS).await;
    let user = cms_user_with("Guilherme", "gblanco", "developer");

    let tag1 = cms_tag("C");
    let tag2 = cms_tag("A");
    let tag3 = cms_tag("B");

    add_tag(&user, &tag1);
    add_tag(&user, &tag2);
    add_tag(&user, &tag3);

    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    em.clear().unwrap();

    let user = find_user(&em, &user).await;

    let mut criteria = criteria();
    criteria.order_by([("name", SortDirection::Ascending)]);

    assert_eq!(
        vec![s("A"), s("B"), s("C")],
        matched_names(&get(&user, "tags"), &criteria).await
    );
}
