//! Upstream: doctrine/orm tests/Tests/ORM/UnitOfWorkTest.php
//!
//! The unit of work of an entity manager on a stubbed connection (no
//! rows, `lastInsertId()` 1, 2, 3, ...), as upstream's `EntityManagerMock`
//! on its stubbed driver; the persisters are the real ones, which run
//! their statements on the stub. A data provider's rows run in one test.

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
#[path = "common/stub_connection.rs"]
mod stub_connection;

use std::sync::Arc;

use php_value::object::StdClass;
use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_orm::collection_object::ArrayCollectionObject;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::exception::{
    ENTITY_IDENTITY_COLLISION_EXCEPTION, ENTITY_NOT_FOUND_EXCEPTION, OPTIMISTIC_LOCK_EXCEPTION,
    ORM_INVALID_ARGUMENT_EXCEPTION,
};
use php2rust_doctrine_orm::persistent_collection::PersistentCollection;
use php2rust_doctrine_orm::unit_of_work::{STATE_MANAGED, STATE_REMOVED, UnitOfWork};

use orm::*;
use stub_connection::{Failures, stub_connection};

const FORUM_USER: &str = "Doctrine\\Tests\\Models\\Forum\\ForumUser";
const ORM_NS: &str = "Doctrine\\Tests\\ORM\\";

fn class(name: &str) -> String {
    format!("{ORM_NS}{name}")
}

fn set_up_test() -> Arc<EntityManager> {
    entity_manager(stub_connection(Failures::default()), None)
}

fn array<const N: usize>(pairs: [(&str, Value); N]) -> PhpArray {
    let mut array = PhpArray::new();
    for (key, value) in pairs {
        array.insert(key, value);
    }
    array
}

fn forum_user(username: Option<&str>) -> Object {
    let user = new_object(FORUM_USER);
    if let Some(username) = username {
        set(&user, "username", s(username));
    }
    user
}

/// `invalidAssociationValuesDataProvider()`.
fn invalid_association_values() -> Vec<Value> {
    vec![
        s("foo"),
        Value::Array(PhpArray::list([s("foo")])),
        s(""),
        Value::Array(PhpArray::new()),
        StdClass::object(PhpArray::new()),
        ArrayCollectionObject::value(PhpArray::new()),
    ]
}

#[tokio::test]
async fn testRegisterRemovedOnNewEntityIsIgnored() {
    let em = set_up_test();
    let uow: &UnitOfWork = em.get_unit_of_work();
    let user = forum_user(Some("romanb"));
    assert!(!uow.is_scheduled_for_delete(&user));
    uow.schedule_for_delete(&user).unwrap();
    assert!(!uow.is_scheduled_for_delete(&user));
}

#[tokio::test]
async fn testRejectsPersistenceOfObjectsWithInvalidAssociationValue() {
    for invalid_value in invalid_association_values() {
        let em = set_up_test();
        let user = forum_user(Some("John"));
        set(&user, "avatar", invalid_value.clone());

        let e = em.get_unit_of_work().persist(&user).unwrap_err();
        assert_eq!(e.class, ORM_INVALID_ARGUMENT_EXCEPTION, "{invalid_value:?}");
    }
}

#[tokio::test]
async fn testRejectsChangeSetComputationForObjectsWithInvalidAssociationValue() {
    for invalid_value in invalid_association_values() {
        let em = set_up_test();
        let metadata = em.get_class_metadata(FORUM_USER).unwrap();
        let user = forum_user(None);
        em.get_unit_of_work().persist(&user).unwrap();

        set(&user, "username", s("John"));
        set(&user, "avatar", invalid_value.clone());

        let e = em
            .get_unit_of_work()
            .compute_change_set(&metadata, &user)
            .await
            .unwrap_err();
        assert_eq!(e.class, ORM_INVALID_ARGUMENT_EXCEPTION, "{invalid_value:?}");
    }
}

#[tokio::test]
async fn testRemovedAndRePersistedEntitiesAreInTheIdentityMapAndAreNotGarbageCollected() {
    let em = set_up_test();
    let uow = em.get_unit_of_work();
    let entity = forum_user(None);
    set(&entity, "id", Value::Int(123));

    uow.register_managed(&entity, array([("id", Value::Int(123))]), PhpArray::new())
        .unwrap();
    assert_eq!(uow.get_entity_state(&entity).await.unwrap(), STATE_MANAGED);
    assert!(!uow.is_scheduled_for_delete(&entity));
    assert!(uow.is_in_identity_map(&entity).unwrap());

    uow.remove(&entity).await.unwrap();
    assert_eq!(uow.get_entity_state(&entity).await.unwrap(), STATE_REMOVED);
    assert!(uow.is_scheduled_for_delete(&entity));
    assert!(uow.is_in_identity_map(&entity).unwrap());

    uow.persist(&entity).unwrap();
    assert_eq!(uow.get_entity_state(&entity).await.unwrap(), STATE_MANAGED);
    assert!(!uow.is_scheduled_for_delete(&entity));
    assert!(uow.is_in_identity_map(&entity).unwrap());
}

#[tokio::test]
async fn testAddToIdentityMapValidIdentifiers() {
    let string_entity = |id: &str| {
        let entity = new_object(&class("EntityWithStringIdentifier"));
        set(&entity, "id", s(id));
        entity
    };
    let composite = |id1: &str, id2: &str| {
        let entity = new_object(&class("EntityWithCompositeStringIdentifier"));
        set(&entity, "id1", s(id1));
        set(&entity, "id2", s(id2));
        entity
    };
    let boolean = |id: bool| {
        let entity = new_object(&class("EntityWithBooleanIdentifier"));
        set(&entity, "id", Value::Bool(id));
        entity
    };
    let rows = [
        ("empty string, single field", string_entity(""), ""),
        (
            "non-empty string, single field",
            string_entity("id6512a3e6c1f5a"),
            "id6512a3e6c1f5a",
        ),
        ("empty strings, two fields", composite("", ""), " "),
        (
            "non-empty strings, two fields",
            composite("id16512a3e6c1f5b", "id26512a3e6c1f5c"),
            "id16512a3e6c1f5b id26512a3e6c1f5c",
        ),
        ("boolean true", boolean(true), "1"),
        ("boolean false", boolean(false), ""),
    ];
    for (name, entity, id_hash) in rows {
        let em = set_up_test();
        let uow = em.get_unit_of_work();
        uow.persist(&entity).unwrap();
        uow.add_to_identity_map(&entity).unwrap();

        let found = uow.get_by_id_hash(id_hash, entity.class());
        assert!(found.is_some_and(|found| same(&found, &entity)), "{name}");
    }
}

#[tokio::test]
async fn testRegisteringAManagedInstanceRequiresANonEmptyIdentifier() {
    let em = set_up_test();
    let e = em
        .get_unit_of_work()
        .register_managed(
            &new_object(&class("EntityWithBooleanIdentifier")),
            PhpArray::new(),
            PhpArray::new(),
        )
        .unwrap_err();
    assert_eq!(e.class, ORM_INVALID_ARGUMENT_EXCEPTION);
}

#[tokio::test]
async fn testAddToIdentityMapInvalidIdentifiers() {
    let composite = class("EntityWithCompositeStringIdentifier");
    let first_null_string = new_object(&composite);
    set(&first_null_string, "id2", s("id26512a3e6c1f5d"));
    let second_null_string = new_object(&composite);
    set(&second_null_string, "id1", s("id16512a3e6c1f5e"));
    let rows = [
        (
            "null string, single field",
            new_object(&class("EntityWithStringIdentifier")),
            array([("id", Value::Null)]),
        ),
        (
            "null strings, two fields",
            new_object(&composite),
            array([("id1", Value::Null), ("id2", Value::Null)]),
        ),
        (
            "first null string, two fields",
            first_null_string,
            array([("id1", Value::Null), ("id2", s("id26512a3e6c1f5d"))]),
        ),
        (
            "second null string, two fields",
            second_null_string,
            array([("id1", s("id16512a3e6c1f5e")), ("id2", Value::Null)]),
        ),
    ];
    for (name, entity, identifier) in rows {
        let em = set_up_test();
        let e = em
            .get_unit_of_work()
            .register_managed(&entity, identifier, PhpArray::new())
            .unwrap_err();
        assert_eq!(e.class, ORM_INVALID_ARGUMENT_EXCEPTION, "{name}");
    }
}

#[tokio::test]
async fn testCommitThrowOptimisticLockExceptionWhenConnectionCommitFails() {
    // Set another connection mock that fail on commit
    let em = entity_manager(
        stub_connection(Failures {
            commit: Some("commit failed"),
            roll_back: None,
        }),
        None,
    );

    // Create a test user
    let user = forum_user(Some("Jasper"));
    em.get_unit_of_work().persist(&user).unwrap();

    let e = em.get_unit_of_work().commit().await.unwrap_err();
    assert_eq!(e.class, OPTIMISTIC_LOCK_EXCEPTION);
}

#[tokio::test]
async fn testItThrowsWhenLookingUpIdentifierForUnknownEntity() {
    let em = set_up_test();
    let Value::Object(object) = StdClass::object(PhpArray::new()) else {
        panic!("an object");
    };
    let e = em
        .get_unit_of_work()
        .get_entity_identifier(&object)
        .unwrap_err();
    assert_eq!(e.class, ENTITY_NOT_FOUND_EXCEPTION);
}

fn persistent(object: &Object, name: &str) -> Arc<PersistentCollection> {
    PersistentCollection::from_value(&get(object, name)).expect("a PersistentCollection")
}

#[tokio::test]
async fn testRemovedEntityIsRemovedFromManyToManyCollection() {
    let em = set_up_test();
    let uow = em.get_unit_of_work();
    let group = cms_group("test");
    uow.persist(&group).unwrap();

    let user = cms_user();
    set(&user, "name", s("test"));
    append(&user, "groups", Value::Object(group.clone()));
    uow.persist(&user).unwrap();

    uow.commit().await.unwrap();

    assert!(!persistent(&user, "groups").is_dirty());

    uow.remove(&group).await.unwrap();
    uow.commit().await.unwrap();

    // Test that the removed entity has been removed from the many to many collection
    assert_eq!(
        count_of(&user, "groups").await,
        0,
        "the removed entity should have been removed from the many to many collection"
    );

    // Collection is clean, snapshot has been updated
    assert!(!persistent(&user, "groups").is_dirty());
    assert!(persistent(&user, "groups").get_snapshot().is_empty());
}

#[tokio::test]
async fn testRemovedEntityIsRemovedFromOneToManyCollection() {
    let em = set_up_test();
    let uow = em.get_unit_of_work();
    let user = cms_user();
    set(&user, "name", s("test"));

    let phonenumber = cms_phonenumber("0800-123456");
    add_phonenumber(&user, &phonenumber);

    uow.persist(&user).unwrap();
    uow.persist(&phonenumber).unwrap();
    uow.commit().await.unwrap();

    assert!(!persistent(&user, "phonenumbers").is_dirty());

    uow.remove(&phonenumber).await.unwrap();
    uow.commit().await.unwrap();

    // Test that the removed entity has been removed from the one to many collection
    assert_eq!(count_of(&user, "phonenumbers").await, 0);

    // Collection is clean, snapshot has been updated
    assert!(!persistent(&user, "phonenumbers").is_dirty());
    assert!(persistent(&user, "phonenumbers").get_snapshot().is_empty());
}

#[tokio::test]
async fn testItThrowsWhenApplicationProvidedIdsCollide() {
    let em = set_up_test();
    let uow = em.get_unit_of_work();
    // We're using application-provided IDs and assign the same ID twice
    // Note this is about colliding IDs in the identity map in memory.
    // Duplicate database-level IDs would be spotted when the EM is flushed.

    let phone1 = cms_phonenumber("1234");
    uow.persist(&phone1).unwrap();

    let phone2 = cms_phonenumber("1234");

    let e = uow.persist(&phone2).unwrap_err();
    assert_eq!(e.class, ENTITY_IDENTITY_COLLISION_EXCEPTION);
    assert!(message(&e).contains("another object"));
    assert!(message(&e).contains("was already present for the same ID"));
}
