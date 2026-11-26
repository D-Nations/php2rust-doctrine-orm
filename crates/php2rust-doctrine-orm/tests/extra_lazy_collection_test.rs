//! ExtraLazyCollectionTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa.
//! `setUp()` changes the loaded metadata of `CmsUser` and `CmsGroup`
//! (fetch mode and `indexBy`) as upstream does; the entity manager of a
//! test is its own, so `tearDown()` has nothing to undo.
//! `foreach ($collection as ...)` is `toArray()`,
//! `assertQueryCount()` counts the statements in the query log.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/ExtraLazyCollectionTest.php

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

use std::sync::Arc;

use php_value::{Key, Object, PhpArray, Value};
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::mapping::class_metadata::FETCH_EXTRA_LAZY;
use php2rust_doctrine_orm::persistent_collection::PersistentCollection;

use orm::*;
use query_log::QueryLog;

struct Fixture {
    em: Arc<EntityManager>,
    log: Arc<QueryLog>,
    user_id: Value,
    user_id2: Value,
    username: Value,
    group_id: Value,
    article_id: Value,
    groupname: Value,
    topic: Value,
    phonenumber: Value,
    ddc2504_other_class_id: Value,
    ddc2504_child_class_id: Value,
}

const DDC2504_ROOT_CLASS: &str = "Doctrine\\Tests\\Models\\DDC2504\\DDC2504RootClass";
const DDC2504_CHILD_CLASS: &str = "Doctrine\\Tests\\Models\\DDC2504\\DDC2504ChildClass";
const DDC2504_OTHER_CLASS: &str = "Doctrine\\Tests\\Models\\DDC2504\\DDC2504OtherClass";

/// `new DDC2504ChildClass()`.
fn ddc2504_child_class() -> Object {
    new_object(DDC2504_CHILD_CLASS)
}

/// `new DDC2504OtherClass()`.
fn ddc2504_other_class() -> Object {
    let other = new_object(DDC2504_OTHER_CLASS);
    set(
        &other,
        "childClasses",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    other
}

impl Fixture {
    /// `$this->getQueryLog()->reset()->enable()`.
    fn reset_log(&self) {
        self.log.reset_and_enable();
    }

    /// `assertQueryCount($count)`.
    fn assert_query_count(&self, count: usize, message: &str) {
        let queries = self.log.queries();
        assert_eq!(queries.len(), count, "{message} {queries:?}");
    }

    async fn find(&self, class: &str, id: &Value) -> Object {
        self.em.find(class, id.clone()).await.unwrap().unwrap()
    }

    async fn user(&self) -> Object {
        self.find(CMS_USER, &self.user_id).await
    }

    async fn group(&self) -> Object {
        self.find(CMS_GROUP, &self.group_id).await
    }

    async fn other_class(&self) -> Object {
        self.find(DDC2504_OTHER_CLASS, &self.ddc2504_other_class_id)
            .await
    }
}

/// The persistent collection of a property.
fn collection(object: &Object, name: &str) -> Arc<PersistentCollection> {
    PersistentCollection::from_value(&get(object, name)).expect("a persistent collection")
}

fn is_initialized(object: &Object, name: &str) -> bool {
    collection(object, name).is_initialized()
}

/// `foreach ($object->$name as $element) {}`.
async fn iterate(object: &Object, name: &str) {
    coll::to_array(&get(object, name)).await.unwrap();
}

/// `assertContainsOnly($class, $array)`.
fn assert_contains_only(class: &str, values: &php_value::PhpArray) {
    for value in values.values() {
        assert!(
            matches!(value, Value::Object(o) if o.class() == class),
            "{value:?} is not a {class}"
        );
    }
}

/// `$class->associationMappings[$field]->fetch` and `->indexBy`.
fn set_mapping(em: &EntityManager, class: &str, field: &str, fetch: i64, index_by: &str) {
    let metadata = em.get_class_metadata(class).unwrap();
    let mut info = metadata.write();
    let mapping = info.association_mappings.get_mut(field).unwrap();
    mapping.fetch = Some(fetch);
    mapping.index_by = Some(index_by.to_owned());
}

/// `$class->associationMappings[$field]->indexBy = $index_by`.
fn set_index_by(em: &EntityManager, class: &str, field: &str, index_by: &str) {
    let metadata = em.get_class_metadata(class).unwrap();
    let mut info = metadata.write();
    info.association_mappings.get_mut(field).unwrap().index_by = Some(index_by.to_owned());
}

/// `new CmsUser()` with a username, name and status.
fn user(username: &str, name: &str) -> Object {
    cms_user_with(name, username, "active")
}

async fn set_up() -> Fixture {
    // useModelSet('tweet'), useModelSet('cms'), useModelSet('ddc2504'):
    // the cms and ddc2504 models (the tweet models are not used here).
    let mut models = CMS_MODELS.to_vec();
    models.extend([DDC2504_ROOT_CLASS, DDC2504_CHILD_CLASS, DDC2504_OTHER_CLASS]);
    let (em, log) = query_log::set_up_logged(&models).await;

    set_mapping(&em, CMS_USER, "groups", FETCH_EXTRA_LAZY, "name");
    set_mapping(&em, CMS_USER, "articles", FETCH_EXTRA_LAZY, "topic");
    set_mapping(
        &em,
        CMS_USER,
        "phonenumbers",
        FETCH_EXTRA_LAZY,
        "phonenumber",
    );
    set_mapping(&em, CMS_GROUP, "users", FETCH_EXTRA_LAZY, "username");

    load_fixture(em, log).await
}

async fn load_fixture(em: Arc<EntityManager>, log: Arc<QueryLog>) -> Fixture {
    let user1 = user("beberlei", "Benjamin");
    let user2 = user("jwage", "Jonathan");
    let user3 = user("romanb", "Roman");
    let user4 = user("gblanco", "Guilherme");

    em.persist(&user1).unwrap();
    em.persist(&user2).unwrap();
    em.persist(&user3).unwrap();
    em.persist(&user4).unwrap();

    let group1 = cms_group("Test1");
    let group2 = cms_group("Test2");
    let group3 = cms_group("Test3");

    add_group(&user1, &group1);
    add_group(&user1, &group2);
    add_group(&user1, &group3);

    add_group(&user2, &group1);
    add_group(&user3, &group1);
    add_group(&user4, &group1);

    em.persist(&group1).unwrap();
    em.persist(&group2).unwrap();
    em.persist(&group3).unwrap();

    // `$article->setAuthor($user1)`.
    let article1 = cms_article("Test1", "Test1");
    set_obj(&article1, "user", &user1);
    let article2 = cms_article("Test2", "Test2");
    set_obj(&article2, "user", &user1);

    em.persist(&article1).unwrap();
    em.persist(&article2).unwrap();

    let phonenumber1 = cms_phonenumber("12345");
    let phonenumber2 = cms_phonenumber("67890");

    em.persist(&phonenumber1).unwrap();
    em.persist(&phonenumber2).unwrap();

    add_phonenumber(&user1, &phonenumber1);

    // DDC-2504
    let other_class = ddc2504_other_class();
    let child_class1 = ddc2504_child_class();
    let child_class2 = ddc2504_child_class();

    set_obj(&child_class1, "other", &other_class);
    set_obj(&child_class2, "other", &other_class);

    append(
        &other_class,
        "childClasses",
        Value::Object(child_class1.clone()),
    );
    append(
        &other_class,
        "childClasses",
        Value::Object(child_class2.clone()),
    );

    em.persist(&child_class1).unwrap();
    em.persist(&child_class2).unwrap();
    em.persist(&other_class).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();

    Fixture {
        em,
        log,
        article_id: get(&article1, "id"),
        user_id: get(&user1, "id"),
        user_id2: get(&user2, "id"),
        username: get(&user1, "username"),
        group_id: get(&group1, "id"),
        groupname: get(&group1, "name"),
        topic: get(&article1, "topic"),
        phonenumber: get(&phonenumber1, "phonenumber"),
        ddc2504_other_class_id: get(&other_class, "id"),
        ddc2504_child_class_id: get(&child_class1, "id"),
    }
}

fn key(value: &Value) -> Key {
    Key::from_value(value).unwrap()
}

#[tokio::test]
async fn testCountNotInitializesCollection() {
    let f = set_up().await;
    let user = f.user().await;
    f.reset_log();

    assert!(!is_initialized(&user, "groups"));
    assert_eq!(count_of(&user, "groups").await, 3);
    assert!(!is_initialized(&user, "groups"));

    iterate(&user, "groups").await;

    f.assert_query_count(
        2,
        "Expecting two queries to be fired for count, then iteration.",
    );
}

#[tokio::test]
async fn testCountWhenNewEntityPresent() {
    let f = set_up().await;
    let user = f.user().await;

    let new_group = cms_group("Test4");

    add_group(&user, &new_group);
    f.em.persist(&new_group).unwrap();

    assert!(!is_initialized(&user, "groups"));
    assert_eq!(count_of(&user, "groups").await, 4);
    assert!(!is_initialized(&user, "groups"));
}

#[tokio::test]
async fn testCountWhenInitialized() {
    let f = set_up().await;
    let user = f.user().await;
    f.reset_log();

    iterate(&user, "groups").await;

    assert!(is_initialized(&user, "groups"));
    assert_eq!(count_of(&user, "groups").await, 3);
    f.assert_query_count(
        1,
        "Should only execute one query to initialize collection, no extra query for count() more.",
    );
}

#[tokio::test]
async fn testCountInverseCollection() {
    let f = set_up().await;
    let group = f.group().await;
    assert!(!is_initialized(&group, "users"), "Pre-Condition");

    assert_eq!(count_of(&group, "users").await, 4);
    assert!(
        !is_initialized(&group, "users"),
        "Extra Lazy collection should not be initialized by counting the collection."
    );
}

#[tokio::test]
async fn testCountOneToMany() {
    let f = set_up().await;
    let user = f.user().await;
    assert!(!is_initialized(&user, "groups"), "Pre-Condition");

    assert_eq!(count_of(&user, "articles").await, 2);
}

#[tokio::test]
async fn testCountOneToManyJoinedInheritance() {
    let f = set_up().await;
    let other_class = f.other_class().await;

    assert!(
        !is_initialized(&other_class, "childClasses"),
        "Pre-Condition"
    );
    assert_eq!(count_of(&other_class, "childClasses").await, 2);
}

#[tokio::test]
async fn testFirstWhenInitialized() {
    let f = set_up().await;
    let user = f.user().await;
    f.reset_log();
    coll::to_array(&get(&user, "groups")).await.unwrap();

    assert!(is_initialized(&user, "groups"));
    let first = coll::first(&get(&user, "groups")).await.unwrap();
    assert!(matches!(&first, Value::Object(o) if o.class() == CMS_GROUP));
    f.assert_query_count(
        1,
        "Should only execute one query to initialize collection, no extra query for first().",
    );
}

async fn remove_all_groups(f: &Fixture) {
    for group in
        f.em.get_repository(CMS_GROUP)
            .unwrap()
            .find_all()
            .await
            .unwrap()
    {
        f.em.remove(&group).await.unwrap();
    }

    f.em.flush().await.unwrap();
}

#[tokio::test]
async fn testFirstOnEmptyCollectionWhenInitialized() {
    let f = set_up().await;
    remove_all_groups(&f).await;

    let user = f.user().await;
    f.reset_log();
    coll::to_array(&get(&user, "groups")).await.unwrap();

    assert!(is_initialized(&user, "groups"));
    assert!(matches!(
        coll::first(&get(&user, "groups")).await.unwrap(),
        Value::Bool(false)
    ));
    f.assert_query_count(
        1,
        "Should only execute one query to initialize collection, no extra query for first().",
    );
}

#[tokio::test]
async fn testFirstWhenNotInitialized() {
    let f = set_up().await;
    let user = f.user().await;
    f.reset_log();

    assert!(!is_initialized(&user, "groups"));
    let first = coll::first(&get(&user, "groups")).await.unwrap();
    assert!(matches!(&first, Value::Object(o) if o.class() == CMS_GROUP));
    assert!(!is_initialized(&user, "groups"));
    f.assert_query_count(1, "Should only execute one query for first().");
}

#[tokio::test]
async fn testFirstOnEmptyCollectionWhenNotInitialized() {
    let f = set_up().await;
    remove_all_groups(&f).await;

    let user = f.user().await;
    f.reset_log();

    assert!(!is_initialized(&user, "groups"));
    assert!(matches!(
        coll::first(&get(&user, "groups")).await.unwrap(),
        Value::Bool(false)
    ));
    assert!(!is_initialized(&user, "groups"));
    f.assert_query_count(1, "Should only execute one query for first().");
}

#[tokio::test]
async fn testFullSlice() {
    let f = set_up().await;
    let user = f.user().await;
    assert!(
        !is_initialized(&user, "groups"),
        "Pre-Condition: Collection is not initialized."
    );

    let some_groups = coll::slice(&get(&user, "groups"), 0, None).await.unwrap();
    assert_eq!(some_groups.len(), 3);
}

#[tokio::test]
async fn testSlice() {
    let f = set_up().await;
    let user = f.user().await;
    assert!(
        !is_initialized(&user, "groups"),
        "Pre-Condition: Collection is not initialized."
    );

    f.reset_log();

    let some_groups = coll::slice(&get(&user, "groups"), 0, Some(2))
        .await
        .unwrap();

    assert_contains_only(CMS_GROUP, &some_groups);
    assert_eq!(some_groups.len(), 2);
    assert!(
        !is_initialized(&user, "groups"),
        "Slice should not initialize the collection if it wasn't before!"
    );

    let other_group = coll::slice(&get(&user, "groups"), 2, Some(1))
        .await
        .unwrap();

    assert_contains_only(CMS_GROUP, &other_group);
    assert_eq!(other_group.len(), 1);
    assert!(!is_initialized(&user, "groups"));

    iterate(&user, "groups").await;

    assert!(is_initialized(&user, "groups"));
    assert_eq!(count_of(&user, "groups").await, 3);

    f.assert_query_count(3, "");
}

#[tokio::test]
async fn testSliceInitializedCollection() {
    let f = set_up().await;
    let user = f.user().await;
    f.reset_log();

    iterate(&user, "groups").await;

    let some_groups = coll::slice(&get(&user, "groups"), 0, Some(2))
        .await
        .unwrap();

    f.assert_query_count(1, "");

    assert_eq!(some_groups.len(), 2);
    let mut some_groups = some_groups.values();
    assert!(
        coll::contains(&get(&user, "groups"), some_groups.next().unwrap())
            .await
            .unwrap()
    );
    assert!(
        coll::contains(&get(&user, "groups"), some_groups.next().unwrap())
            .await
            .unwrap()
    );
}
#[tokio::test]
async fn testSliceInverseCollection() {
    let f = set_up().await;
    let group = f.group().await;
    assert!(!is_initialized(&group, "users"), "Pre-Condition");
    f.reset_log();

    let some_users = coll::slice(&get(&group, "users"), 0, Some(2))
        .await
        .unwrap();
    let other_users = coll::slice(&get(&group, "users"), 2, Some(2))
        .await
        .unwrap();

    for user in some_users.values().chain(other_users.values()) {
        assert!(
            matches!(user, Value::Object(o) if o.class() == CMS_USER),
            "{user:?}"
        );
    }
    assert_eq!(2, some_users.len());
    assert_eq!(2, other_users.len());

    // +2 queries executed by slice
    f.assert_query_count(
        2,
        "Slicing two parts should only execute two additional queries.",
    );
}

#[tokio::test]
async fn testSliceOneToMany() {
    let f = set_up().await;
    let user = f.user().await;
    assert!(
        !is_initialized(&user, "articles"),
        "Pre-Condition: Collection is not initialized."
    );

    f.reset_log();

    let _some_article = coll::slice(&get(&user, "articles"), 0, Some(1))
        .await
        .unwrap();
    let _other_article = coll::slice(&get(&user, "articles"), 1, Some(1))
        .await
        .unwrap();

    f.assert_query_count(2, "");
}

#[tokio::test]
async fn testContainsOneToMany() {
    let f = set_up().await;
    let user = f.user().await;
    assert!(
        !is_initialized(&user, "articles"),
        "Pre-Condition: Collection is not initialized."
    );

    // Test One to Many existence retrieved from DB
    let article = f.find(CMS_ARTICLE, &f.article_id).await;
    f.reset_log();

    assert!(contains(&user, "articles", &article).await);
    assert!(
        !is_initialized(&user, "articles"),
        "Post-Condition: Collection is not initialized."
    );
    f.assert_query_count(1, "");

    // Test One to Many existence with state new
    let article = cms_article("Testnew", "blub");

    f.reset_log();
    assert!(!contains(&user, "articles", &article).await);
    f.assert_query_count(
        0,
        "Checking for contains of new entity should cause no query to be executed.",
    );

    // Test One to Many existence with state clear
    f.em.persist(&article).unwrap();
    f.em.flush().await.unwrap();

    f.reset_log();
    assert!(!contains(&user, "articles", &article).await);
    f.assert_query_count(
        1,
        "Checking for contains of persisted entity should cause one query to be executed.",
    );
    assert!(
        !is_initialized(&user, "articles"),
        "Post-Condition: Collection is not initialized."
    );

    // Test One to Many existence with state managed
    let article = cms_article(
        "How to not fail anymore on tests",
        "That is simple! Just write more tests!",
    );

    f.em.persist(&article).unwrap();

    f.reset_log();

    assert!(!contains(&user, "articles", &article).await);
    f.assert_query_count(
        0,
        "Checking for contains of managed entity (but not persisted) should cause no query to be executed.",
    );
    assert!(
        !is_initialized(&user, "articles"),
        "Post-Condition: Collection is not initialized."
    );
}

#[tokio::test]
async fn testLazyOneToManyJoinedInheritanceIsLazilyInitialized() {
    let f = set_up().await;
    let other_class = f.other_class().await;

    assert!(
        !is_initialized(&other_class, "childClasses"),
        "Collection is not initialized."
    );
}

#[tokio::test]
async fn testContainsOnOneToManyJoinedInheritanceWillNotInitializeCollectionWhenMatchingItemIsFound()
 {
    let f = set_up().await;
    let other_class = f.other_class().await;

    // Test One to Many existence retrieved from DB
    let child_class = f.find(DDC2504_CHILD_CLASS, &f.ddc2504_child_class_id).await;
    f.reset_log();

    assert!(contains(&other_class, "childClasses", &child_class).await);
    assert!(
        !is_initialized(&other_class, "childClasses"),
        "Collection is not initialized."
    );
    f.assert_query_count(1, "Search operation was performed via SQL");
}

#[tokio::test]
async fn testContainsOnOneToManyJoinedInheritanceWillNotCauseQueriesWhenNonPersistentItemIsMatched()
{
    let f = set_up().await;
    let other_class = f.other_class().await;
    f.reset_log();

    assert!(!contains(&other_class, "childClasses", &ddc2504_child_class()).await);
    f.assert_query_count(
        0,
        "Checking for contains of new entity should cause no query to be executed.",
    );
}

#[tokio::test]
async fn testContainsOnOneToManyJoinedInheritanceWillNotInitializeCollectionWithClearStateMatchingItem()
 {
    let f = set_up().await;
    let other_class = f.other_class().await;
    let child_class = ddc2504_child_class();

    // Test One to Many existence with state clear
    f.em.persist(&child_class).unwrap();
    f.em.flush().await.unwrap();

    f.reset_log();
    assert!(!contains(&other_class, "childClasses", &child_class).await);
    f.assert_query_count(
        1,
        "Checking for contains of persisted entity should cause one query to be executed.",
    );
    assert!(
        !is_initialized(&other_class, "childClasses"),
        "Post-Condition: Collection is not initialized."
    );
}

#[tokio::test]
async fn testContainsOnOneToManyJoinedInheritanceWillNotInitializeCollectionWithNewStateNotMatchingItem()
 {
    let f = set_up().await;
    let other_class = f.other_class().await;
    let child_class = ddc2504_child_class();

    f.em.persist(&child_class).unwrap();

    f.reset_log();

    assert!(!contains(&other_class, "childClasses", &child_class).await);
    f.assert_query_count(
        0,
        "Checking for contains of managed entity (but not persisted) should cause no query to be executed.",
    );
    assert!(
        !is_initialized(&other_class, "childClasses"),
        "Post-Condition: Collection is not initialized."
    );
}

#[tokio::test]
async fn testCountingOnOneToManyJoinedInheritanceWillNotInitializeCollection() {
    let f = set_up().await;
    let other_class = f.other_class().await;

    assert_eq!(count_of(&other_class, "childClasses").await, 2);

    assert!(!is_initialized(&other_class, "childClasses"));
}

#[tokio::test]
async fn testContainsManyToMany() {
    let f = set_up().await;
    let user = f.user().await;
    assert!(
        !is_initialized(&user, "groups"),
        "Pre-Condition: Collection is not initialized."
    );

    // Test Many to Many existence retrieved from DB
    let group = f.group().await;
    f.reset_log();

    assert!(contains(&user, "groups", &group).await);
    f.assert_query_count(
        1,
        "Checking for contains of managed entity should cause one query to be executed.",
    );
    assert!(
        !is_initialized(&user, "groups"),
        "Post-Condition: Collection is not initialized."
    );

    // Test Many to Many existence with state new
    let group = cms_group("A New group!");

    f.reset_log();

    assert!(!contains(&user, "groups", &group).await);
    f.assert_query_count(
        0,
        "Checking for contains of new entity should cause no query to be executed.",
    );
    assert!(
        !is_initialized(&user, "groups"),
        "Post-Condition: Collection is not initialized."
    );

    // Test Many to Many existence with state clear
    f.em.persist(&group).unwrap();
    f.em.flush().await.unwrap();

    f.reset_log();

    assert!(!contains(&user, "groups", &group).await);
    f.assert_query_count(
        1,
        "Checking for contains of persisted entity should cause one query to be executed.",
    );
    assert!(
        !is_initialized(&user, "groups"),
        "Post-Condition: Collection is not initialized."
    );

    // Test Many to Many existence with state managed
    let group = cms_group("My managed group");

    f.em.persist(&group).unwrap();

    f.reset_log();

    assert!(!contains(&user, "groups", &group).await);
    f.assert_query_count(
        0,
        "Checking for contains of managed entity (but not persisted) should cause no query to be executed.",
    );
    assert!(
        !is_initialized(&user, "groups"),
        "Post-Condition: Collection is not initialized."
    );
}

#[tokio::test]
async fn testContainsManyToManyInverse() {
    let f = set_up().await;
    let group = f.group().await;
    assert!(
        !is_initialized(&group, "users"),
        "Pre-Condition: Collection is not initialized."
    );

    let user = f.user().await;

    f.reset_log();
    assert!(contains(&group, "users", &user).await);
    f.assert_query_count(
        1,
        "Checking for contains of managed entity should cause one query to be executed.",
    );
    assert!(
        !is_initialized(&user, "groups"),
        "Post-Condition: Collection is not initialized."
    );

    let new_user = cms_user();
    set(&new_user, "name", s("A New group!"));

    f.reset_log();
    assert!(!contains(&group, "users", &new_user).await);
    f.assert_query_count(
        0,
        "Checking for contains of new entity should cause no query to be executed.",
    );
    assert!(
        !is_initialized(&user, "groups"),
        "Post-Condition: Collection is not initialized."
    );
}

#[tokio::test]
async fn testCountAfterAddThenFlush() {
    let f = set_up().await;
    let user = f.user().await;

    let new_group = cms_group("Test4");

    add_group(&user, &new_group);
    f.em.persist(&new_group).unwrap();

    assert!(!is_initialized(&user, "groups"));
    assert_eq!(count_of(&user, "groups").await, 4);
    assert!(!is_initialized(&user, "groups"));

    f.em.flush().await.unwrap();

    assert_eq!(count_of(&user, "groups").await, 4);
}

#[tokio::test]
async fn testSliceOnDirtyCollection() {
    let f = set_up().await;
    let user = f.user().await;

    let new_group = cms_group("Test4");

    add_group(&user, &new_group);
    f.em.persist(&new_group).unwrap();

    f.reset_log();
    let groups = coll::slice(&get(&user, "groups"), 0, Some(10))
        .await
        .unwrap();

    assert_eq!(groups.len(), 4);
    f.assert_query_count(1, "");
}

#[tokio::test]
async fn testGetIndexByIdentifier() {
    let f = set_up().await;
    let user = f.user().await;

    f.reset_log();
    let phonenumber = coll::get(&get(&user, "phonenumbers"), key(&f.phonenumber))
        .await
        .unwrap();

    assert!(!is_initialized(&user, "phonenumbers"));
    f.assert_query_count(1, "");
    let found = f.find(CMS_PHONENUMBER, &f.phonenumber).await;
    assert!(same_value(&phonenumber, &found));

    let _article = coll::get(&get(&user, "phonenumbers"), key(&f.phonenumber))
        .await
        .unwrap();
    f.assert_query_count(
        1,
        "Getting the same entity should not cause an extra query to be executed",
    );
}

#[tokio::test]
async fn testGetIndexByOneToMany() {
    let f = set_up().await;
    let user = f.user().await;

    f.reset_log();

    let article = coll::get(&get(&user, "articles"), key(&f.topic))
        .await
        .unwrap();

    assert!(!is_initialized(&user, "articles"));
    f.assert_query_count(1, "");
    let found = f.find(CMS_ARTICLE, &f.article_id).await;
    assert!(same_value(&article, &found));
}
#[tokio::test]
async fn testGetIndexByManyToManyInverseSide() {
    let f = set_up().await;
    let group = f.group().await;

    f.reset_log();

    let user = coll::get(&get(&group, "users"), key(&f.username))
        .await
        .unwrap();

    assert!(!is_initialized(&group, "users"));
    f.assert_query_count(1, "");
    let found = f.user().await;
    assert!(same_value(&user, &found));
}

#[tokio::test]
async fn testGetIndexByManyToManyOwningSide() {
    let f = set_up().await;
    let user = f.user().await;

    f.reset_log();

    let group = coll::get(&get(&user, "groups"), key(&f.groupname))
        .await
        .unwrap();

    assert!(!is_initialized(&user, "groups"));
    f.assert_query_count(1, "");
    let found = f.group().await;
    assert!(same_value(&group, &found));
}

#[tokio::test]
async fn testGetNonExistentIndexBy() {
    let f = set_up().await;
    let user = f.user().await;
    assert!(
        coll::get(&get(&user, "articles"), -1)
            .await
            .unwrap()
            .is_null()
    );
    assert!(
        coll::get(&get(&user, "groups"), -1)
            .await
            .unwrap()
            .is_null()
    );
}

#[tokio::test]
async fn testContainsKeyIndexByOneToMany() {
    let f = set_up().await;
    let user = f.user().await;

    f.reset_log();

    let contains = coll::contains_key(&get(&user, "articles"), key(&f.topic))
        .await
        .unwrap();

    assert!(contains);
    assert!(!is_initialized(&user, "articles"));
    f.assert_query_count(1, "");
}

#[tokio::test]
async fn testContainsKeyIndexByOneToManyJoinedInheritance() {
    let f = set_up().await;
    set_index_by(&f.em, DDC2504_OTHER_CLASS, "childClasses", "id");

    let other_class = f.other_class().await;

    f.reset_log();

    let contains = coll::contains_key(
        &get(&other_class, "childClasses"),
        key(&f.ddc2504_child_class_id),
    )
    .await
    .unwrap();

    assert!(contains);
    assert!(!is_initialized(&other_class, "childClasses"));
    f.assert_query_count(1, "");
}

#[tokio::test]
async fn testContainsKeyIndexByManyToMany() {
    let f = set_up().await;
    let user = f.find(CMS_USER, &f.user_id2).await;

    let group = f.group().await;

    f.reset_log();

    let contains = coll::contains_key(&get(&user, "groups"), key(&get(&group, "name")))
        .await
        .unwrap();

    assert!(contains, "The item is not into collection");
    assert!(
        !is_initialized(&user, "groups"),
        "The collection must not be initialized"
    );
    f.assert_query_count(1, "");
}

#[tokio::test]
async fn testContainsKeyIndexByManyToManyNonOwning() {
    let f = set_up().await;
    let user = f.find(CMS_USER, &f.user_id2).await;
    let group = f.group().await;

    f.reset_log();

    let contains = coll::contains_key(&get(&group, "users"), key(&get(&user, "username")))
        .await
        .unwrap();

    assert!(contains, "The item is not into collection");
    assert!(
        !is_initialized(&group, "users"),
        "The collection must not be initialized"
    );
    f.assert_query_count(1, "");
}

#[tokio::test]
async fn testContainsKeyIndexByWithPkManyToMany() {
    let f = set_up().await;
    set_index_by(&f.em, CMS_USER, "groups", "id");

    let user = f.find(CMS_USER, &f.user_id2).await;

    f.reset_log();

    let contains = coll::contains_key(&get(&user, "groups"), key(&f.group_id))
        .await
        .unwrap();

    assert!(contains, "The item is not into collection");
    assert!(
        !is_initialized(&user, "groups"),
        "The collection must not be initialized"
    );
    f.assert_query_count(1, "");
}

#[tokio::test]
async fn testContainsKeyIndexByWithPkManyToManyNonOwning() {
    let f = set_up().await;
    set_index_by(&f.em, CMS_GROUP, "users", "id");

    let group = f.group().await;

    f.reset_log();

    let contains = coll::contains_key(&get(&group, "users"), key(&f.user_id2))
        .await
        .unwrap();

    assert!(contains, "The item is not into collection");
    assert!(
        !is_initialized(&group, "users"),
        "The collection must not be initialized"
    );
    f.assert_query_count(1, "");
}

#[tokio::test]
async fn testContainsKeyNonExistentIndexByOneToMany() {
    let f = set_up().await;
    let user = f.find(CMS_USER, &f.user_id2).await;

    f.reset_log();

    let contains = coll::contains_key(&get(&user, "articles"), "NonExistentTopic")
        .await
        .unwrap();

    assert!(!contains);
    assert!(!is_initialized(&user, "articles"));
    f.assert_query_count(1, "");
}

#[tokio::test]
async fn testContainsKeyNonExistentIndexByManyToMany() {
    let f = set_up().await;
    let user = f.find(CMS_USER, &f.user_id2).await;

    f.reset_log();

    let contains = coll::contains_key(&get(&user, "groups"), "NonExistentTopic")
        .await
        .unwrap();

    assert!(!contains);
    assert!(!is_initialized(&user, "groups"));
    f.assert_query_count(1, "");
}
