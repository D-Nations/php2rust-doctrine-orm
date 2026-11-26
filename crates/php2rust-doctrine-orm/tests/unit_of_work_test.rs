//! Upstream: doctrine/orm tests/Tests/ORM/UnitOfWorkTest.php
//!
//! The unit of work of an entity manager on a stubbed connection (no
//! rows, `lastInsertId()` 1, 2, 3, ...), as upstream's `EntityManagerMock`
//! on its stubbed driver; the persisters are the real ones, which run
//! their statements on the stub. A data provider's rows run in one test.
//! `EntityPersisterMock` is a `CommitPersister` that records its calls,
//! set with `UnitOfWork::set_entity_persister()` (upstream's
//! `UnitOfWorkMock::setEntityPersister()`).

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

use std::sync::{Arc, Mutex, PoisonError, Weak};

use php_value::object::StdClass;
use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_collections::Criteria;
use php2rust_doctrine_orm::collection_object::ArrayCollectionObject;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::exception::{
    ENTITY_IDENTITY_COLLISION_EXCEPTION, ENTITY_NOT_FOUND_EXCEPTION, OPTIMISTIC_LOCK_EXCEPTION,
    ORM_INVALID_ARGUMENT_EXCEPTION,
};
use php2rust_doctrine_orm::mapping::ClassMetadata;
use php2rust_doctrine_orm::mapping::class_metadata::GENERATOR_TYPE_IDENTITY;
use php2rust_doctrine_orm::persistent_collection::PersistentCollection;
use php2rust_doctrine_orm::persisters::CommitPersister;
use php2rust_doctrine_orm::unit_of_work::Fut;
use php2rust_doctrine_orm::unit_of_work::{
    STATE_DETACHED, STATE_MANAGED, STATE_NEW, STATE_REMOVED, UnitOfWork,
};

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

/// The recorded calls of an `EntityPersisterMock`.
#[derive(Default)]
struct MockState {
    count_of_execute_insert_calls: usize,
    inserts: Vec<Object>,
    updates: Vec<Object>,
    deletes: Vec<Object>,
    identity_column_value_counter: i64,
    mock_id_generator_type: Option<i64>,
    post_insert_ids: Vec<(i64, Object)>,
    exists_called: bool,
}

/// `EntityPersisterMock`: an entity persister used for mocking during
/// tests.
struct EntityPersisterMock {
    em: Weak<EntityManager>,
    class: Arc<ClassMetadata>,
    state: Mutex<MockState>,
}

impl std::fmt::Debug for EntityPersisterMock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EntityPersisterMock")
            .finish_non_exhaustive()
    }
}

impl EntityPersisterMock {
    /// `new EntityPersisterMock($em, $em->getClassMetadata($class))`,
    /// set as the class's persister.
    fn set(em: &Arc<EntityManager>, class_name: &str) -> Arc<Self> {
        let persister = Arc::new(Self {
            em: Arc::downgrade(em),
            class: em.get_class_metadata(class_name).unwrap(),
            state: Mutex::new(MockState::default()),
        });
        em.get_unit_of_work()
            .set_entity_persister(class_name, persister.clone());
        persister
    }

    fn st(&self) -> std::sync::MutexGuard<'_, MockState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn set_mock_id_generator_type(&self, gen_type: i64) {
        self.st().mock_id_generator_type = Some(gen_type);
    }

    fn get_inserts(&self) -> Vec<Object> {
        self.st().inserts.clone()
    }

    fn get_updates(&self) -> Vec<Object> {
        self.st().updates.clone()
    }

    fn get_deletes(&self) -> Vec<Object> {
        self.st().deletes.clone()
    }

    fn reset(&self) {
        let mut st = self.st();
        st.count_of_execute_insert_calls = 0;
        st.exists_called = false;
        st.identity_column_value_counter = 0;
        st.inserts.clear();
        st.updates.clear();
        st.deletes.clear();
    }

    fn is_exists_called(&self) -> bool {
        self.st().exists_called
    }

    fn count_of_execute_insert_calls(&self) -> usize {
        self.st().count_of_execute_insert_calls
    }
}

impl CommitPersister for EntityPersisterMock {
    fn add_insert(&self, entity: &Object) {
        let identity = self.class.read().is_id_generator_identity();
        let mut st = self.st();
        st.inserts.push(entity.clone());
        if st.mock_id_generator_type != Some(GENERATOR_TYPE_IDENTITY) && !identity {
            return;
        }
        let id = st.identity_column_value_counter;
        st.identity_column_value_counter += 1;
        st.post_insert_ids.push((id, entity.clone()));
    }

    fn execute_inserts(&self) -> Fut<'_, ()> {
        Box::pin(async move {
            let post_insert_ids = {
                let mut st = self.st();
                st.count_of_execute_insert_calls += 1;
                st.post_insert_ids.clone()
            };
            let em = self.em.upgrade().expect("the entity manager");
            for (generated_id, entity) in post_insert_ids {
                em.get_unit_of_work()
                    .assign_post_insert_id(&entity, &Value::Int(generated_id))?;
            }
            Ok(())
        })
    }

    fn update<'a>(&'a self, entity: &'a Object) -> Fut<'a, ()> {
        self.st().updates.push(entity.clone());
        Box::pin(async { Ok(()) })
    }

    fn delete<'a>(&'a self, entity: &'a Object) -> Fut<'a, bool> {
        self.st().deletes.push(entity.clone());
        Box::pin(async { Ok(true) })
    }

    fn exists<'a>(
        &'a self,
        _entity: &'a Object,
        _extra_conditions: Option<&'a Criteria>,
    ) -> Fut<'a, bool> {
        self.st().exists_called = true;
        Box::pin(async { Ok(false) })
    }
}

/// `assertIsNumeric()`.
fn assert_is_numeric(value: &Value) {
    assert!(
        matches!(value, Value::Int(_) | Value::Float(_)),
        "{value:?}"
    );
}

const FORUM_AVATAR: &str = "Doctrine\\Tests\\Models\\Forum\\ForumAvatar";
const CMS_PHONENUMBER: &str = "Doctrine\\Tests\\Models\\CMS\\CmsPhonenumber";
const COUNTRY: &str = "Doctrine\\Tests\\Models\\MixedToOneIdentity\\Country";

#[tokio::test]
async fn testSavingSingleEntityWithIdentityColumnForcesInsert() {
    let em = set_up_test();
    let uow = em.get_unit_of_work();
    // Setup fake persister and id generator for identity generation
    let user_persister = EntityPersisterMock::set(&em, FORUM_USER);
    user_persister.set_mock_id_generator_type(GENERATOR_TYPE_IDENTITY);

    // Test
    let user = forum_user(Some("romanb"));
    uow.persist(&user).unwrap();

    // Check
    assert_eq!(0, user_persister.get_inserts().len());
    assert_eq!(0, user_persister.get_updates().len());
    assert_eq!(0, user_persister.get_deletes().len());
    assert!(!uow.is_in_identity_map(&user).unwrap());
    // should no longer be scheduled for insert
    assert!(uow.is_scheduled_for_insert(&user));

    // Now lets check whether a subsequent commit() does anything
    user_persister.reset();

    // Test
    uow.commit().await.unwrap();

    // Check.
    assert_eq!(1, user_persister.get_inserts().len());
    assert_eq!(0, user_persister.get_updates().len());
    assert_eq!(0, user_persister.get_deletes().len());

    // should have an id
    assert_is_numeric(&get(&user, "id"));
}

#[tokio::test]
async fn testCascadedIdentityColumnInsert() {
    let em = set_up_test();
    let uow = em.get_unit_of_work();
    // Setup fake persister and id generator for identity generation
    //ForumUser
    let user_persister = EntityPersisterMock::set(&em, FORUM_USER);
    user_persister.set_mock_id_generator_type(GENERATOR_TYPE_IDENTITY);
    // ForumAvatar
    let avatar_persister = EntityPersisterMock::set(&em, FORUM_AVATAR);
    avatar_persister.set_mock_id_generator_type(GENERATOR_TYPE_IDENTITY);

    // Test
    let user = forum_user(Some("romanb"));
    let avatar = new_object(FORUM_AVATAR);
    set_obj(&user, "avatar", &avatar);
    uow.persist(&user).unwrap(); // save cascaded to avatar

    uow.commit().await.unwrap();

    assert_is_numeric(&get(&user, "id"));
    assert_is_numeric(&get(&avatar, "id"));

    assert_eq!(1, user_persister.get_inserts().len());
    assert_eq!(0, user_persister.get_updates().len());
    assert_eq!(0, user_persister.get_deletes().len());

    assert_eq!(1, avatar_persister.get_inserts().len());
    assert_eq!(0, avatar_persister.get_updates().len());
    assert_eq!(0, avatar_persister.get_deletes().len());
}

#[tokio::test]
async fn testGetEntityStateOnVersionedEntityWithAssignedIdentifier() {
    let em = set_up_test();
    let class_name = class("VersionedAssignedIdentifierEntity");
    let persister = EntityPersisterMock::set(&em, &class_name);

    let e = new_object(&class_name);
    set(&e, "id", Value::Int(42));
    assert_eq!(
        STATE_NEW,
        em.get_unit_of_work().get_entity_state(&e).await.unwrap()
    );
    assert!(!persister.is_exists_called());
}

#[tokio::test]
async fn testGetEntityStateWithAssignedIdentity() {
    let em = set_up_test();
    let uow = em.get_unit_of_work();
    let persister = EntityPersisterMock::set(&em, CMS_PHONENUMBER);

    let ph = new_object(CMS_PHONENUMBER);
    set(&ph, "phonenumber", s("12345"));

    assert_eq!(STATE_NEW, uow.get_entity_state(&ph).await.unwrap());
    assert!(persister.is_exists_called());

    persister.reset();

    // if the entity is already managed the exists() check should be skipped
    uow.register_managed(&ph, array([("phonenumber", s("12345"))]), PhpArray::new())
        .unwrap();
    assert_eq!(STATE_MANAGED, uow.get_entity_state(&ph).await.unwrap());
    assert!(!persister.is_exists_called());
    let ph2 = new_object(CMS_PHONENUMBER);
    set(&ph2, "phonenumber", s("12345"));
    assert_eq!(STATE_DETACHED, uow.get_entity_state(&ph2).await.unwrap());
    assert!(!persister.is_exists_called());
}

#[tokio::test]
async fn testMultipleInsertsAreBatchedInThePersister() {
    let em = set_up_test();
    let uow = em.get_unit_of_work();
    let user_persister = EntityPersisterMock::set(&em, COUNTRY);

    let country1 = new_object(COUNTRY);
    set(&country1, "country", s("Italy"));
    let country2 = new_object(COUNTRY);
    set(&country2, "country", s("Germany"));

    uow.persist(&country1).unwrap();
    uow.persist(&country2).unwrap();
    uow.commit().await.unwrap();

    assert_eq!(2, user_persister.get_inserts().len());
    assert_eq!(1, user_persister.count_of_execute_insert_calls());
}

/// `new CascadePersistedEntity()` (and the two other classes): the
/// constructor's `$this->id = uniqid(self::class, true)`.
fn new_with_uniqid(class_name: &str) -> Object {
    static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let entity = new_object(class_name);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    set(&entity, "id", s(&format!("{class_name}{n}")));
    entity
}

/// The persisters of `CascadePersistedEntity`,
/// `EntityWithCascadingAssociation` and
/// `EntityWithNonCascadingAssociation`, and new entities of them.
fn cascade_fixture(
    em: &Arc<EntityManager>,
) -> ([Arc<EntityPersisterMock>; 3], Object, Object, Object) {
    let persister1 = EntityPersisterMock::set(em, &class("CascadePersistedEntity"));
    let persister2 = EntityPersisterMock::set(em, &class("EntityWithCascadingAssociation"));
    let persister3 = EntityPersisterMock::set(em, &class("EntityWithNonCascadingAssociation"));
    (
        [persister1, persister2, persister3],
        new_with_uniqid(&class("CascadePersistedEntity")),
        new_with_uniqid(&class("EntityWithCascadingAssociation")),
        new_with_uniqid(&class("EntityWithNonCascadingAssociation")),
    )
}

#[tokio::test]
async fn testNewAssociatedEntityPersistenceOfNewEntitiesThroughCascadedAssociationsFirst() {
    let em = set_up_test();
    let uow = em.get_unit_of_work();
    let ([persister1, persister2, persister3], cascade_persisted, cascading, non_cascading) =
        cascade_fixture(&em);

    // First we persist and flush a EntityWithCascadingAssociation with
    // the cascading association not set. Having the "cascading path" involve
    // a non-new object is important to show that the ORM should be considering
    // cascades across entity changesets in subsequent flushes.
    set_obj(&cascading, "cascaded", &cascade_persisted);
    set_obj(&non_cascading, "nonCascaded", &cascade_persisted);

    uow.persist(&cascading).unwrap();
    uow.persist(&non_cascading).unwrap();

    uow.commit().await.unwrap();

    assert_eq!(1, persister1.get_inserts().len());
    assert_eq!(1, persister2.get_inserts().len());
    assert_eq!(1, persister3.get_inserts().len());
}

#[tokio::test]
async fn testNewAssociatedEntityPersistenceOfNewEntitiesThroughNonCascadedAssociationsFirst() {
    let em = set_up_test();
    let uow = em.get_unit_of_work();
    let ([persister1, persister2, persister3], cascade_persisted, cascading, non_cascading) =
        cascade_fixture(&em);

    // First we persist and flush a EntityWithCascadingAssociation with
    // the cascading association not set. Having the "cascading path" involve
    // a non-new object is important to show that the ORM should be considering
    // cascades across entity changesets in subsequent flushes.
    set(&cascading, "cascaded", Value::Null);

    uow.persist(&cascading).unwrap();
    uow.commit().await.unwrap();

    assert_eq!(0, persister1.get_inserts().len());
    assert_eq!(1, persister2.get_inserts().len());
    assert_eq!(0, persister3.get_inserts().len());

    // Note that we have NOT directly persisted the CascadePersistedEntity,
    // and EntityWithNonCascadingAssociation does NOT have a configured
    // cascade-persist.
    set_obj(&non_cascading, "nonCascaded", &cascade_persisted);

    // However, EntityWithCascadingAssociation *does* have a cascade-persist
    // association, which ought to allow us to save the CascadePersistedEntity
    // anyway through that connection.
    set_obj(&cascading, "cascaded", &cascade_persisted);

    uow.persist(&non_cascading).unwrap();
    uow.commit().await.unwrap();

    assert_eq!(1, persister1.get_inserts().len());
    assert_eq!(1, persister2.get_inserts().len());
    assert_eq!(1, persister3.get_inserts().len());
}

#[tokio::test]
async fn testNoUndefinedIndexNoticeOnScheduleForUpdateWithoutChanges() {
    let em = set_up_test();
    let uow = em.get_unit_of_work();
    // Setup fake persister and id generator
    let user_persister = EntityPersisterMock::set(&em, FORUM_USER);
    user_persister.set_mock_id_generator_type(GENERATOR_TYPE_IDENTITY);

    // Create a test user
    let user = forum_user(Some("Jasper"));
    uow.persist(&user).unwrap();
    uow.commit().await.unwrap();

    // Schedule user for update without changes
    uow.schedule_for_update(&user).unwrap();

    assert!(!uow.get_scheduled_entity_updates().is_empty());

    // This commit should not raise an E_NOTICE
    uow.commit().await.unwrap();

    assert!(uow.get_scheduled_entity_updates().is_empty());
}

#[tokio::test]
async fn testPreviousDetectedIllegalNewNonCascadedEntitiesAreCleanedUpOnSubsequentCommits() {
    let em = set_up_test();
    let uow = em.get_unit_of_work();
    let persister1 = EntityPersisterMock::set(&em, &class("CascadePersistedEntity"));
    let persister2 = EntityPersisterMock::set(&em, &class("EntityWithNonCascadingAssociation"));

    let cascade_persisted = new_with_uniqid(&class("CascadePersistedEntity"));
    let non_cascading = new_with_uniqid(&class("EntityWithNonCascadingAssociation"));

    // We explicitly cause the ORM to detect a non-persisted new entity in the association graph:
    set_obj(&non_cascading, "nonCascaded", &cascade_persisted);

    uow.persist(&non_cascading).unwrap();

    match uow.commit().await {
        Ok(()) => panic!("An exception was supposed to be raised"),
        Err(e) => {
            assert_eq!(e.class, ORM_INVALID_ARGUMENT_EXCEPTION);
            assert!(persister1.get_inserts().is_empty());
            assert!(persister2.get_inserts().is_empty());
        }
    }

    uow.clear().unwrap();
    uow.persist(&new_with_uniqid(&class("CascadePersistedEntity")))
        .unwrap();
    uow.commit().await.unwrap();

    // Persistence operations should just recover normally:
    assert_eq!(1, persister1.get_inserts().len());
    assert_eq!(0, persister2.get_inserts().len());
}
