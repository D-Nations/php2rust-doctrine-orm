//! Upstream: doctrine/orm tests/Tests/ORM/Persisters/BasicEntityPersisterTypeValueSqlTest.php
//! Upstream: doctrine/orm tests/Tests/Models/Generic/NonAlphaColumnsEntity.php
//!
//! The entity manager is a `SQLite` one (`tests/common/orm.rs`) whose
//! statements are logged, in place of `EntityManagerMock` and the mocked
//! connections: the protected methods called by reflection are public,
//! `propertyChanged()` (not ported) is the change set that
//! `computeChangeSet()` finds from the registered original data, and the
//! `executeStatement()` and `delete()` calls are the logged statements
//! with their bound values and types.

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/custom_types.rs"]
mod custom_types;
#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;
#[path = "common/query_log.rs"]
mod query_log;

use std::sync::Arc;

use php_value::{PhpArray, Value, php_array};
use php2rust_doctrine_collections::Criteria;
use php2rust_doctrine_collections::expr::Comparison;
use php2rust_doctrine_dbal::ParameterType;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::mapping::AssociationMapping;
use php2rust_doctrine_orm::mapping::association_mapping::{AssociationClass, AssociationKind};
use php2rust_doctrine_orm::persisters::{BasicEntityPersister, PersisterCriteria};

use custom_types::{CUSTOM_TYPE_CHILD, CUSTOM_TYPE_PARENT, add_my_friend, custom_type_parent};
use orm::{new_object, set};
use query_log::QueryLog;

const NON_ALPHA_COLUMNS_ENTITY: &str = "Doctrine\\Tests\\Models\\Generic\\NonAlphaColumnsEntity";

struct Test {
    em: Arc<EntityManager>,
    log: Arc<QueryLog>,
    persister: BasicEntityPersister,
}

async fn set_up() -> Test {
    custom_types::register_types();
    let (em, log) = query_log::set_up_logged(&[
        CUSTOM_TYPE_CHILD,
        CUSTOM_TYPE_PARENT,
        NON_ALPHA_COLUMNS_ENTITY,
    ])
    .await;
    let persister = persister(&em, CUSTOM_TYPE_PARENT);
    Test { em, log, persister }
}

fn persister(em: &Arc<EntityManager>, class: &str) -> BasicEntityPersister {
    BasicEntityPersister::new(em, em.get_class_metadata(class).unwrap())
}

/// `new OneToManyAssociationMapping('foo', 'bar', 'baz')`.
fn one_to_many() -> AssociationMapping {
    AssociationMapping::new(
        AssociationClass::Final(AssociationKind::OneToMany),
        "foo",
        "bar",
        "baz",
    )
}

fn id(value: i64) -> PhpArray {
    php_array!["id" => value]
}

#[tokio::test]
async fn testGetInsertSQLUsesTypeValuesSQL() {
    let t = set_up().await;
    let sql = t.persister.get_insert_sql(&t.em).unwrap();

    assert_eq!(
        "INSERT INTO customtype_parents (customInteger, child_id) VALUES (ABS(?), ?)",
        sql
    );
}

#[tokio::test]
async fn testUpdateUsesTypeValuesSQL() {
    let t = set_up().await;
    let child = new_object(CUSTOM_TYPE_CHILD);

    let parent = custom_type_parent();
    set(&parent, "customInteger", Value::Int(1));
    set(&parent, "child", Value::Object(child.clone()));

    let uow = t.em.get_unit_of_work();
    uow.register_managed(
        &parent,
        id(1),
        php_array!["customInteger" => 0, "child" => Value::Null],
    )
    .unwrap();
    uow.register_managed(&child, id(1), PhpArray::new())
        .unwrap();

    // propertyChanged($parent, 'customInteger', 0, 1) and
    // propertyChanged($parent, 'child', null, $child)
    let class = t.em.get_class_metadata(CUSTOM_TYPE_PARENT).unwrap();
    uow.compute_change_set(&class, &parent).await.unwrap();

    let persister = persister(&t.em, CUSTOM_TYPE_PARENT);

    t.log.reset_and_enable();
    persister.update(&parent).await.unwrap();

    assert_eq!(
        vec!["UPDATE customtype_parents SET customInteger = ABS(?), child_id = ? WHERE id = ?"],
        t.log.queries()
    );
}

#[tokio::test]
async fn testGetSelectConditionSQLUsesTypeValuesSQL() {
    let t = set_up().await;
    let sql = t
        .persister
        .get_select_condition_sql(&t.em, &php_array!["customInteger" => 1, "child" => 1], None)
        .unwrap();

    assert_eq!("t0.customInteger = ABS(?) AND t0.child_id = ?", sql);
}

/// DDC-1719
#[tokio::test]
async fn testStripNonAlphanumericCharactersFromSelectColumnListSQL() {
    let t = set_up().await;
    let persister = persister(&t.em, NON_ALPHA_COLUMNS_ENTITY);

    assert_eq!(
        "t0.\"simple-entity-id\" AS simpleentityid_1, t0.\"simple-entity-value\" AS simpleentityvalue_2",
        persister.get_select_columns_sql(&t.em).unwrap()
    );
}

/// DDC-2073
#[tokio::test]
async fn testSelectConditionStatementIsNull() {
    let t = set_up().await;
    let association_mapping = one_to_many();
    let statement = t
        .persister
        .get_select_condition_statement_sql(
            &t.em,
            "test",
            &Value::Null,
            Some(&association_mapping),
            Some(Comparison::IS),
        )
        .unwrap();
    assert_eq!("test IS NULL", statement);
}

#[tokio::test]
async fn testSelectConditionStatementEqNull() {
    let t = set_up().await;
    let association_mapping = one_to_many();
    let statement = t
        .persister
        .get_select_condition_statement_sql(
            &t.em,
            "test",
            &Value::Null,
            Some(&association_mapping),
            Some(Comparison::EQ),
        )
        .unwrap();
    assert_eq!("test IS NULL", statement);
}

#[tokio::test]
async fn testSelectConditionStatementNeqNull() {
    let t = set_up().await;
    let association_mapping = one_to_many();
    let statement = t
        .persister
        .get_select_condition_statement_sql(
            &t.em,
            "test",
            &Value::Null,
            Some(&association_mapping),
            Some(Comparison::NEQ),
        )
        .unwrap();
    assert_eq!("test IS NOT NULL", statement);
}

/// DDC-3056, GH12254
#[tokio::test]
async fn testSelectConditionStatementWithMultipleValuesContainingNull() {
    let t = set_up().await;
    let sql = |values: PhpArray| {
        t.persister
            .get_select_condition_statement_sql(&t.em, "id", &Value::Array(values), None, None)
            .unwrap()
    };

    assert_eq!("t0.id IS NULL", sql(php_array![Value::Null]));

    assert_eq!(
        "(t0.id IN (?) OR t0.id IS NULL)",
        sql(php_array![Value::Null, 123])
    );

    assert_eq!(
        "(t0.id IN (?) OR t0.id IS NULL)",
        sql(php_array![123, Value::Null])
    );

    assert_eq!(
        "(t0.id IN (?, ?) OR t0.id IS NULL)",
        sql(php_array![123, Value::Null, 234])
    );

    assert_eq!("1=0", sql(PhpArray::new()));
}

#[tokio::test]
async fn testCountCondition() {
    let t = set_up().await;
    let persister = persister(&t.em, NON_ALPHA_COLUMNS_ENTITY);

    // Using a criteria as array
    let criteria = php_array!["value" => "bar"];
    let statement = persister
        .get_count_sql(&t.em, PersisterCriteria::Array(&criteria))
        .unwrap();
    assert_eq!(
        "SELECT COUNT(*) FROM \"not-a-simple-entity\" t0 WHERE t0.\"simple-entity-value\" = ?",
        statement
    );

    // Using a criteria object
    let mut criteria = Criteria::create();
    criteria.r#where(Criteria::expr().eq("value", Value::string("bar")));
    let statement = persister
        .get_count_sql(&t.em, PersisterCriteria::Criteria(&criteria))
        .unwrap();
    assert_eq!(
        "SELECT COUNT(*) FROM \"not-a-simple-entity\" t0 WHERE t0.\"simple-entity-value\" = ?",
        statement
    );
}

#[tokio::test]
async fn testCountEntities() {
    let t = set_up().await;
    assert_eq!(0, t.persister.count(&PhpArray::new()).await.unwrap());
}

#[tokio::test]
async fn testDeleteManyToManyUsesTypeValuesSQL() {
    let t = set_up().await;
    let persister = persister(&t.em, CUSTOM_TYPE_PARENT);

    let friend = custom_type_parent();
    let parent = custom_type_parent();
    add_my_friend(&parent, &friend);

    let uow = t.em.get_unit_of_work();
    uow.register_managed(&parent, id(1), PhpArray::new())
        .unwrap();
    uow.register_managed(&friend, id(2), PhpArray::new())
        .unwrap();

    t.log.reset_and_enable();
    persister.delete(&parent).await.unwrap();

    let delete_calls: Vec<_> = t.log.entries().into_iter().take(2).collect();
    assert_eq!(
        vec![
            (
                "DELETE FROM customtype_parent_friends WHERE friend_customtypeparent_id = ?"
                    .to_owned(),
                php_array![1 => 1],
                vec![ParameterType::Integer],
            ),
            (
                "DELETE FROM customtype_parent_friends WHERE customtypeparent_id = ?".to_owned(),
                php_array![1 => 1],
                vec![ParameterType::Integer],
            ),
        ],
        delete_calls
    );
}
