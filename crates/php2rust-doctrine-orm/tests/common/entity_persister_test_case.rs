//! `EntityPersisterTestCase`'s tests, for each cached entity persister:
//! `entity_persister_test_case!` declares them in a test file, over the
//! file's `create_persister()`.
//!
//! `createMock()` expectations are checked after the call: a mock
//! records its calls. `assertInstanceOf()` of an interface is the type;
//! `identicalTo()` of an association mapping compares its class, field
//! and target.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Cache/Persister/Entity/EntityPersisterTestCase.php

#![allow(dead_code, reason = "each test file uses a subset")]
#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

use std::sync::Arc;

use php_value::{PhpArray, Value};
use php2rust_doctrine_collections::{ArrayCollection, Criteria};
use php2rust_doctrine_dbal::LockMode;
use php2rust_doctrine_orm::cache::{AbstractEntityPersister, CachedEntityPersister, Region};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::mapping::association_mapping::{AssociationClass, AssociationKind};
use php2rust_doctrine_orm::mapping::{AssociationMapping, ClassMetadata};
use php2rust_doctrine_orm::persistent_collection::PersistentCollection;
use php2rust_doctrine_orm::persisters::{
    CommitPersister, EntityPersister, EntityPersisterInterface,
};
use php2rust_doctrine_orm::query::result_set_mapping_builder::ResultSetMappingBuilder;
use php2rust_doctrine_orm::unit_of_work::Hints;

use crate::slc::{self, Arg, EntityPersisterMock, RegionMock, Ret, Slc};

/// `createPersister($em, $persister, $region, $metadata)`.
pub type CreatePersister = fn(
    &Arc<EntityManager>,
    Arc<dyn EntityPersisterInterface>,
    Arc<dyn Region>,
    Arc<ClassMetadata>,
) -> AbstractEntityPersister;

/// The test's entity manager, region mock and entity persister mock.
pub struct Fixture {
    pub slc: Slc,
    pub region: Arc<RegionMock>,
    pub entity_persister: Arc<EntityPersisterMock>,
    create: CreatePersister,
}

impl Fixture {
    /// `setUp()` with `createRegion()`.
    pub fn new(create: CreatePersister, region: Arc<RegionMock>) -> Self {
        let slc = slc::slc_entity_manager();
        let class = slc.em.get_class_metadata(slc::COUNTRY).unwrap();
        Self {
            slc,
            region,
            entity_persister: EntityPersisterMock::new(class),
            create,
        }
    }

    pub fn em(&self) -> &Arc<EntityManager> {
        &self.slc.em
    }

    /// `createPersisterDefault()`.
    pub fn create_persister_default(&self) -> AbstractEntityPersister {
        (self.create)(
            &self.slc.em,
            self.entity_persister.clone(),
            self.region.clone(),
            self.slc.em.get_class_metadata(slc::COUNTRY).unwrap(),
        )
    }

    /// `registerManaged($entity, ['id' => 1], ['id' => 1, 'name' => 'Foo'])`.
    pub fn register_managed(&self, entity: &php_value::Object) {
        self.slc
            .em
            .get_unit_of_work()
            .register_managed(
                entity,
                slc::id(1),
                slc::array(&[("id", Value::Int(1)), ("name", Value::string("Foo"))]),
            )
            .unwrap();
    }
}

/// `new OneToOneInverseSideMapping('foo', 'bar', 'baz')`.
pub fn association_mapping() -> AssociationMapping {
    AssociationMapping::new(
        AssociationClass::Final(AssociationKind::OneToOneInverseSide),
        "foo",
        "bar",
        "baz",
    )
}

/// `(object) []`.
pub fn std_object() -> php_value::Object {
    Arc::new(php_value::object::StdClass::default())
}

fn criteria() -> Criteria {
    Criteria::create()
}

pub fn testImplementsEntityPersister(f: &Fixture) {
    let persister: Arc<dyn CachedEntityPersister> = Arc::new(f.create_persister_default());
    let _: Arc<dyn EntityPersisterInterface> = persister.clone();
    let _: Arc<dyn php2rust_doctrine_orm::cache::CachedPersister> = persister;
}

pub fn testInvokeAddInsert(f: &Fixture) {
    let persister = f.create_persister_default();
    let entity = slc::country("Foo");
    persister.add_insert(&entity);
    f.entity_persister
        .recorder
        .assert_once_with("addInsert", &[Arg::object(&entity)]);
}

pub fn testInvokeGetInserts(f: &Fixture) {
    let persister = f.create_persister_default();
    let entity = slc::country("Foo");
    f.entity_persister
        .recorder
        .will_return("getInserts", Ret::Objects(vec![entity.clone()]));
    let inserts = persister.get_inserts();
    assert_eq!(1, inserts.len());
    assert!(Arc::ptr_eq(&entity, &inserts[0]));
    f.entity_persister.recorder.assert_count("getInserts", 1);
}

pub fn testInvokeGetSelectSQL(f: &Fixture) {
    let persister = f.create_persister_default();
    let association_mapping = association_mapping();
    f.entity_persister.recorder.will_return(
        "getSelectSQL",
        Ret::Text("SELECT * FROM foo WERE name = ?".to_owned()),
    );
    let criteria = slc::array(&[("name", Value::string("Foo"))]);
    let order_by = PhpArray::list([Value::Int(4)]);
    assert_eq!(
        "SELECT * FROM foo WERE name = ?",
        persister
            .get_select_sql(
                (&criteria).into(),
                Some(&association_mapping),
                Some(LockMode::Optimistic),
                Some(2),
                Some(3),
                Some(&order_by),
            )
            .unwrap()
    );
    f.entity_persister.recorder.assert_once_with(
        "getSelectSQL",
        &[
            Arg::array(&criteria),
            Arg::mapping(Some(&association_mapping)),
            Arg::lock_mode(Some(LockMode::Optimistic)),
            Arg::opt_int(Some(2)),
            Arg::opt_int(Some(3)),
            Arg::opt_array(Some(&order_by)),
        ],
    );
}

pub fn testInvokeGetInsertSQL(f: &Fixture) {
    let persister = f.create_persister_default();
    f.entity_persister
        .recorder
        .will_return("getInsertSQL", Ret::Text("INSERT INTO foo (?)".to_owned()));
    assert_eq!("INSERT INTO foo (?)", persister.get_insert_sql().unwrap());
    f.entity_persister.recorder.assert_count("getInsertSQL", 1);
}

pub fn testInvokeExpandParameters(f: &Fixture) {
    let persister = f.create_persister_default();
    let criteria = slc::array(&[("name", Value::string("Foo"))]);
    f.entity_persister.recorder.will_return(
        "expandParameters",
        Ret::Params((vec![Value::string("Foo")], Vec::new())),
    );
    let (params, _) = persister.expand_parameters(&criteria).unwrap();
    assert_eq!(vec![Value::string("Foo")], params);
    f.entity_persister
        .recorder
        .assert_once_with("expandParameters", &[Arg::array(&criteria)]);
}

pub fn testInvokeExpandCriteriaParameters(f: &Fixture) {
    let persister = f.create_persister_default();
    let criteria = criteria();
    f.entity_persister.recorder.will_return(
        "expandCriteriaParameters",
        Ret::Params((vec![Value::string("Foo")], Vec::new())),
    );
    let (params, _) = persister.expand_criteria_parameters(&criteria).unwrap();
    assert_eq!(vec![Value::string("Foo")], params);
    f.entity_persister
        .recorder
        .assert_once_with("expandCriteriaParameters", &[Arg::criteria(&criteria)]);
}

pub fn testInvokeSelectConditionStatementSQL(f: &Fixture) {
    let persister = f.create_persister_default();
    let association_mapping = association_mapping();
    f.entity_persister.recorder.will_return(
        "getSelectConditionStatementSQL",
        Ret::Text("name = 1".to_owned()),
    );
    assert_eq!(
        "name = 1",
        persister
            .get_select_condition_statement_sql(
                "id",
                &Value::Int(1),
                Some(&association_mapping),
                Some("=")
            )
            .unwrap()
    );
    f.entity_persister.recorder.assert_once_with(
        "getSelectConditionStatementSQL",
        &[
            Arg::Value(Value::string("id")),
            Arg::Value(Value::Int(1)),
            Arg::mapping(Some(&association_mapping)),
            Arg::Value(Value::string("=")),
        ],
    );
}

pub async fn testInvokeExecuteInserts(f: &Fixture) {
    let persister = f.create_persister_default();
    persister.execute_inserts().await.unwrap();
    f.entity_persister
        .recorder
        .assert_count("executeInserts", 1);
}

pub async fn testInvokeUpdate(f: &Fixture) {
    let persister = f.create_persister_default();
    let entity = slc::country("Foo");
    f.register_managed(&entity);
    persister.update(&entity).await.unwrap();
    f.entity_persister
        .recorder
        .assert_once_with("update", &[Arg::object(&entity)]);
}

pub async fn testInvokeDelete(f: &Fixture) {
    let persister = f.create_persister_default();
    let entity = slc::country("Foo");
    f.entity_persister
        .recorder
        .will_return("delete", Ret::Bool(true));
    f.register_managed(&entity);
    assert!(persister.delete(&entity).await.unwrap());
    f.entity_persister
        .recorder
        .assert_once_with("delete", &[Arg::object(&entity)]);
}

pub fn testInvokeGetOwningTable(f: &Fixture) {
    let persister = f.create_persister_default();
    f.entity_persister
        .recorder
        .will_return("getOwningTable", Ret::Text("t".to_owned()));
    assert_eq!("t", persister.get_owning_table("name").unwrap());
    f.entity_persister
        .recorder
        .assert_once_with("getOwningTable", &[Arg::Value(Value::string("name"))]);
}

pub async fn testInvokeLoad(f: &Fixture) {
    let persister = f.create_persister_default();
    let entity = slc::country("Foo");
    let association_mapping = association_mapping();
    // `[1]`: hints that are not empty.
    let hints = Hints {
        read_only: true,
        ..Hints::default()
    };
    let order_by = PhpArray::list([Value::Int(4)]);
    f.entity_persister
        .recorder
        .will_return("load", Ret::Object(Some(entity.clone())));
    let loaded = persister
        .load(
            &slc::id(1),
            Some(&entity),
            Some(&association_mapping),
            hints.clone(),
            Some(LockMode::PessimisticRead),
            Some(3),
            Some(&order_by),
        )
        .await
        .unwrap()
        .unwrap();
    assert!(Arc::ptr_eq(&entity, &loaded));
    f.entity_persister.recorder.assert_once_with(
        "load",
        &[
            Arg::array(&slc::id(1)),
            Arg::object(&entity),
            Arg::mapping(Some(&association_mapping)),
            Arg::Text(format!("{hints:?}")),
            Arg::lock_mode(Some(LockMode::PessimisticRead)),
            Arg::opt_int(Some(3)),
            Arg::opt_array(Some(&order_by)),
        ],
    );
}

pub async fn testInvokeLoadAll(f: &Fixture) {
    let mut rsm = ResultSetMappingBuilder::new(f.em(), None);
    let persister = f.create_persister_default();
    let entity = slc::country("Foo");
    rsm.add_entity_result(slc::COUNTRY, "c", None);
    f.register_managed(&entity);
    f.entity_persister
        .recorder
        .always_return("expandParameters", Ret::Params((Vec::new(), Vec::new())));
    f.entity_persister
        .recorder
        .will_return("loadAll", Ret::Objects(vec![entity.clone()]));
    f.entity_persister.recorder.will_return(
        "getResultSetMapping",
        Ret::Rsm(rsm.into_result_set_mapping()),
    );
    let order_by = PhpArray::list([Value::Int(0)]);
    let loaded = persister
        .load_all(&slc::id(1), Some(&order_by), Some(1), Some(2))
        .await
        .unwrap();
    assert_eq!(1, loaded.len());
    assert!(Arc::ptr_eq(&entity, &loaded[0]));
    f.entity_persister.recorder.assert_once_with(
        "loadAll",
        &[
            Arg::array(&slc::id(1)),
            Arg::opt_array(Some(&order_by)),
            Arg::opt_int(Some(1)),
            Arg::opt_int(Some(2)),
        ],
    );
    f.entity_persister
        .recorder
        .assert_count("getResultSetMapping", 1);
}

pub async fn testInvokeLoadById(f: &Fixture) {
    let persister = f.create_persister_default();
    let entity = slc::country("Foo");
    f.entity_persister
        .recorder
        .will_return("loadById", Ret::Object(Some(entity.clone())));
    let loaded = persister
        .load_by_id(&slc::id(1), Some(&entity))
        .await
        .unwrap()
        .unwrap();
    assert!(Arc::ptr_eq(&entity, &loaded));
    f.entity_persister
        .recorder
        .assert_once_with("loadById", &[Arg::array(&slc::id(1)), Arg::object(&entity)]);
}

pub async fn testInvokeLoadOneToOneEntity(f: &Fixture) {
    let persister = f.create_persister_default();
    let entity = slc::country("Foo");
    let owner = std_object();
    let association_mapping = association_mapping();
    let identifier = slc::id(11);
    f.entity_persister
        .recorder
        .will_return("loadOneToOneEntity", Ret::Object(Some(entity.clone())));
    let loaded = persister
        .load_one_to_one_entity(&association_mapping, &owner, Some(&identifier))
        .await
        .unwrap()
        .unwrap();
    assert!(Arc::ptr_eq(&entity, &loaded));
    f.entity_persister.recorder.assert_once_with(
        "loadOneToOneEntity",
        &[
            Arg::mapping(Some(&association_mapping)),
            Arg::object(&owner),
            Arg::opt_array(Some(&identifier)),
        ],
    );
}

pub async fn testInvokeRefresh(f: &Fixture) {
    let persister = f.create_persister_default();
    let entity = slc::country("Foo");
    persister.refresh(&slc::id(1), &entity, None).await.unwrap();
    f.entity_persister.recorder.assert_once_with(
        "refresh",
        &[
            Arg::array(&slc::id(1)),
            Arg::object(&entity),
            Arg::lock_mode(None),
        ],
    );
}

pub async fn testInvokeLoadCriteria(f: &Fixture) {
    let mut rsm = ResultSetMappingBuilder::new(f.em(), None);
    let persister = f.create_persister_default();
    let entity = slc::country("Foo");
    let criteria = criteria();
    f.register_managed(&entity);
    rsm.add_entity_result(slc::COUNTRY, "c", None);
    f.entity_persister.recorder.always_return(
        "expandCriteriaParameters",
        Ret::Params((Vec::new(), Vec::new())),
    );
    f.entity_persister.recorder.will_return(
        "getResultSetMapping",
        Ret::Rsm(rsm.into_result_set_mapping()),
    );
    f.entity_persister
        .recorder
        .will_return("loadCriteria", Ret::Objects(vec![entity.clone()]));
    let loaded = EntityPersister::load_criteria(&persister, &criteria)
        .await
        .unwrap();
    assert_eq!(vec![Value::Object(entity)], loaded);
    f.entity_persister
        .recorder
        .assert_count("getResultSetMapping", 1);
    f.entity_persister
        .recorder
        .assert_once_with("loadCriteria", &[Arg::criteria(&criteria)]);
}

pub async fn testInvokeGetManyToManyCollection(f: &Fixture) {
    let persister = f.create_persister_default();
    let entity = slc::country("Foo");
    let owner = std_object();
    let association_mapping = association_mapping();
    f.entity_persister.recorder.will_return(
        "getManyToManyCollection",
        Ret::Objects(vec![entity.clone()]),
    );
    let collection = persister
        .get_many_to_many_collection(&association_mapping, &owner, Some(1), Some(2))
        .await
        .unwrap();
    assert_eq!(PhpArray::list([Value::Object(entity)]), collection);
    f.entity_persister.recorder.assert_once_with(
        "getManyToManyCollection",
        &[
            Arg::mapping(Some(&association_mapping)),
            Arg::object(&owner),
            Arg::opt_int(Some(1)),
            Arg::opt_int(Some(2)),
        ],
    );
}

pub async fn testInvokeGetOneToManyCollection(f: &Fixture) {
    let persister = f.create_persister_default();
    let entity = slc::country("Foo");
    let owner = std_object();
    let association_mapping = association_mapping();
    f.entity_persister
        .recorder
        .will_return("getOneToManyCollection", Ret::Objects(vec![entity.clone()]));
    let collection = persister
        .get_one_to_many_collection(&association_mapping, &owner, Some(1), Some(2))
        .await
        .unwrap();
    assert_eq!(PhpArray::list([Value::Object(entity)]), collection);
    f.entity_persister.recorder.assert_once_with(
        "getOneToManyCollection",
        &[
            Arg::mapping(Some(&association_mapping)),
            Arg::object(&owner),
            Arg::opt_int(Some(1)),
            Arg::opt_int(Some(2)),
        ],
    );
}

fn persistent_collection(f: &Fixture) -> Arc<PersistentCollection> {
    let mapping = f.em().get_class_metadata(slc::COUNTRY).unwrap();
    PersistentCollection::new(
        Some(f.em()),
        Some(mapping),
        ArrayCollection::new(PhpArray::new()),
    )
}

pub async fn testInvokeLoadManyToManyCollection(f: &Fixture) {
    let assoc = association_mapping();
    let coll = persistent_collection(f);
    let persister = f.create_persister_default();
    let entity = slc::country("Foo");
    let owner = std_object();
    f.entity_persister.recorder.will_return(
        "loadManyToManyCollection",
        Ret::Objects(vec![entity.clone()]),
    );
    let loaded = persister
        .load_many_to_many_collection(&assoc, &owner, &coll)
        .await
        .unwrap();
    assert_eq!(1, loaded.len());
    assert!(Arc::ptr_eq(&entity, &loaded[0]));
    f.entity_persister.recorder.assert_once_with(
        "loadManyToManyCollection",
        &[
            Arg::mapping(Some(&assoc)),
            Arg::object(&owner),
            Arg::Value(coll.value()),
        ],
    );
}

pub async fn testInvokeLoadOneToManyCollection(f: &Fixture) {
    let assoc = association_mapping();
    let coll = persistent_collection(f);
    let persister = f.create_persister_default();
    let entity = slc::country("Foo");
    let owner = std_object();
    f.entity_persister.recorder.will_return(
        "loadOneToManyCollection",
        Ret::Objects(vec![entity.clone()]),
    );
    let loaded = persister
        .load_one_to_many_collection(&assoc, &owner, &coll)
        .await
        .unwrap();
    assert_eq!(1, loaded.len());
    assert!(Arc::ptr_eq(&entity, &loaded[0]));
    f.entity_persister.recorder.assert_once_with(
        "loadOneToManyCollection",
        &[
            Arg::mapping(Some(&assoc)),
            Arg::object(&owner),
            Arg::Value(coll.value()),
        ],
    );
}

pub async fn testInvokeLock(f: &Fixture) {
    let identifier = slc::id(1);
    let persister = f.create_persister_default();
    persister
        .lock(&identifier, LockMode::Optimistic)
        .await
        .unwrap();
    f.entity_persister.recorder.assert_once_with(
        "lock",
        &[
            Arg::array(&identifier),
            Arg::lock_mode(Some(LockMode::Optimistic)),
        ],
    );
}

pub async fn testInvokeExists(f: &Fixture) {
    let entity = slc::country("Foo");
    let persister = f.create_persister_default();
    f.entity_persister
        .recorder
        .will_return("exists", Ret::Bool(true));
    assert!(
        CommitPersister::exists(&persister, &entity, None)
            .await
            .unwrap()
    );
    f.entity_persister
        .recorder
        .assert_once_with("exists", &[Arg::object(&entity), Arg::Text(String::new())]);
}

/// Declares `EntityPersisterTestCase`'s tests (those named) over
/// `create_persister()` and `create_region()`.
#[macro_export]
macro_rules! entity_persister_test_case {
    (sync: [$($sync:ident),* $(,)?], async: [$($async:ident),* $(,)?] $(,)?) => {
        $(
            #[tokio::test]
            async fn $sync() {
                let f = entity_persister_test_case::Fixture::new(create_persister, create_region());
                entity_persister_test_case::$sync(&f);
            }
        )*
        $(
            #[tokio::test]
            async fn $async() {
                let f = entity_persister_test_case::Fixture::new(create_persister, create_region());
                entity_persister_test_case::$async(&f).await;
            }
        )*
    };
}
