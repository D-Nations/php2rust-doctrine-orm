//! Upstream: doctrine/orm tests/Tests/ORM/Hydration/AbstractHydratorTest.php
//!
//! `buildEnumForTesting()` is [`build_enum`]. `DummyHydrator` is an
//! `AbstractHydrator` implementation; the stubbed result is an empty
//! `SQLite` result of the test entity manager's connection.
//! `testToIterableIfYieldAndBreakBeforeFinishAlwaysCleansUp` is pending:
//! `Query::toIterable()` hydrates the whole result before it returns, so
//! it is not a generator that can stop early.

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

use std::sync::Arc;

use php_value::{PhpArray, Thrown, Value};
use php2rust_doctrine_event_manager::EventListenerIntrospector;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::events;
use php2rust_doctrine_orm::internal::hydration::{
    AbstractHydrator, HydrationContext, build_enum, hydrate_all_with, to_iterable_with,
};
use php2rust_doctrine_orm::query::ResultSetMapping;
use php2rust_doctrine_persistence::php_reflection;

const ACCESS_LEVEL: &str = "Doctrine\\Tests\\Models\\Enums\\AccessLevel";
const USER_STATUS: &str = "Doctrine\\Tests\\Models\\Enums\\UserStatus";

/// `$enum::Case`.
fn case(enum_type: &str, name: &str) -> Value {
    models::register();
    let class = php_reflection::class_info(enum_type).expect("the enum is declared");
    let case = class
        .cases()
        .iter()
        .find(|case| {
            case.property(b"name")
                .is_some_and(|n| n == Value::string(name))
        })
        .cloned()
        .expect("the case is declared");
    Value::Object(case)
}

fn list(values: Vec<Value>) -> Value {
    Value::Array(PhpArray::list(values))
}

#[test]
fn testEnumCastsIntegerBackedEnumValues() {
    models::register();
    let access_level = build_enum(&Value::string("2"), ACCESS_LEVEL).unwrap();
    let user_status = build_enum(&Value::string("active"), USER_STATUS).unwrap();

    assert!(case(ACCESS_LEVEL, "User").identical(&access_level));
    assert!(case(USER_STATUS, "Active").identical(&user_status));
}

#[test]
fn testEnumCastsIntegerBackedEnumArrayValues() {
    models::register();
    let access_levels = build_enum(
        &list(vec![Value::string("1"), Value::string("2")]),
        ACCESS_LEVEL,
    )
    .unwrap();
    let user_status = build_enum(
        &list(vec![Value::string("active"), Value::string("inactive")]),
        USER_STATUS,
    )
    .unwrap();

    assert!(
        list(vec![
            case(ACCESS_LEVEL, "Admin"),
            case(ACCESS_LEVEL, "User")
        ])
        .identical(&access_levels)
    );
    assert!(
        list(vec![
            case(USER_STATUS, "Active"),
            case(USER_STATUS, "Inactive")
        ])
        .identical(&user_status)
    );
}

/// `DummyHydrator`.
#[derive(Default)]
struct DummyHydrator {
    throw_exception: bool,
    has_listener: bool,
}

impl AbstractHydrator for DummyHydrator {
    fn class(&self) -> &'static str {
        "Doctrine\\Tests\\ORM\\Hydration\\DummyHydrator"
    }

    fn hydrate_all_data(
        &mut self,
        _context: &mut HydrationContext<'_, '_>,
    ) -> Result<Value, Thrown> {
        if self.throw_exception {
            return Err(Thrown::new("LogicException", "", 0));
        }

        Ok(Value::Array(PhpArray::new()))
    }

    fn prepare(&mut self, context: &mut HydrationContext<'_, '_>) -> Result<(), Thrown> {
        self.has_listener = context
            .em
            .get_event_manager()
            .has_listeners(events::ON_CLEAR);
        Ok(())
    }
}

/// `setUp()`: the entity manager and its `onClear` listeners.
fn set_up() -> Arc<EntityManager> {
    orm::entity_manager(orm::connection(), None)
}

fn has_on_clear_listeners(em: &EntityManager) -> bool {
    em.get_event_manager().has_listeners(events::ON_CLEAR)
}

/// The result stub, whose `fetchAssociative()` gives `false`.
const NO_ROWS: &str = "SELECT 1 WHERE 0";

/// Verify that the number of added events to the event listener from the
/// abstract hydrator class is equal to the number of removed events
/// (DDC-3146, #1515).
#[tokio::test]
async fn testOnClearEventListenerIsDetachedOnCleanup() {
    let em = set_up();
    let mut hydrator = DummyHydrator::default();
    let rsm = ResultSetMapping::new();
    let connection = em.get_connection().clone();
    let mut result = connection
        .execute_query(
            NO_ROWS,
            &PhpArray::new(),
            &php2rust_doctrine_dbal::ParameterTypes::new(),
        )
        .await
        .unwrap();

    let iterator = to_iterable_with(&mut hydrator, &em, &mut result, &rsm, PhpArray::new());
    let values: Vec<_> = iterator.collect();
    assert!(values.is_empty());
    assert!(hydrator.has_listener);
    assert!(!has_on_clear_listeners(&em));
}

/// #6623.
#[tokio::test]
async fn testHydrateAllRegistersAndClearsAllAttachedListeners() {
    let em = set_up();
    let mut hydrator = DummyHydrator::default();
    let rsm = ResultSetMapping::new();
    let connection = em.get_connection().clone();
    let mut result = connection
        .execute_query(
            NO_ROWS,
            &PhpArray::new(),
            &php2rust_doctrine_dbal::ParameterTypes::new(),
        )
        .await
        .unwrap();

    hydrate_all_with(&mut hydrator, &em, &mut result, &rsm, PhpArray::new()).unwrap();
    assert!(hydrator.has_listener);
    assert!(!has_on_clear_listeners(&em));
}

/// #8482.
#[tokio::test]
async fn testHydrateAllClearsAllAttachedListenersEvenOnError() {
    let em = set_up();
    let mut hydrator = DummyHydrator {
        throw_exception: true,
        ..DummyHydrator::default()
    };
    let rsm = ResultSetMapping::new();
    let connection = em.get_connection().clone();
    let mut result = connection
        .execute_query(
            NO_ROWS,
            &PhpArray::new(),
            &php2rust_doctrine_dbal::ParameterTypes::new(),
        )
        .await
        .unwrap();

    let e = hydrate_all_with(&mut hydrator, &em, &mut result, &rsm, PhpArray::new()).unwrap_err();

    assert!(hydrator.has_listener);
    assert!(!has_on_clear_listeners(&em));
    assert_eq!("LogicException", e.class);
}
