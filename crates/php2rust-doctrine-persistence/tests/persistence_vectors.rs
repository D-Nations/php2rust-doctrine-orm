//! Replays the answers real PHP 8.4.27 gave with doctrine/persistence
//! (`oracle/recorders/persistence.php`, `scripts/record.sh persistence`)
//! for what the upstream tests leave open: `EnumReflectionProperty`'s
//! `setValue()` and `getValue()` over PHP values, the messages of PHP's
//! reflection exceptions, and `AbstractManagerRegistry::getManager()`'s
//! `AssertionError`.
//!
//! PHP's `TypeError`s for user functions end with `, called in <file> on
//! line <line>`, which the port leaves out: such a message matches when
//! PHP's starts with the port's and that tail. Assigning an enum of
//! another class to a typed property is the property's `TypeError`
//! (`Cannot assign ...`), which the fixture object does not check: those
//! cases are left out.
//!
//! Upstream: none

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

mod common;

use std::sync::{Arc, OnceLock};

use common::{FixtureObject, FixtureProperty};
use php_value::object::EnumCase;
use php_value::vectors::{Outcome, decode, outcome};
use php_value::{Thrown, Value};
use php2rust_doctrine_persistence::php_reflection::{
    ClassInfo, PropertyAccessor, PropertyInfo, ReflectionClass, ReflectionProperty, register_class,
};
use php2rust_doctrine_persistence::reflection::EnumReflectionProperty;
use php2rust_doctrine_persistence::{
    AbstractManagerRegistry, ManagerRegistry, ManagerRegistryState, Service,
};
use serde_json::Value as Json;

const HOLDER: &str = "Doctrine\\Tests\\Persistence\\Vectors\\Holder";
const SUIT: &str = "Doctrine\\Tests\\Persistence\\Vectors\\Suit";
const RANK: &str = "Doctrine\\Tests\\Persistence\\Vectors\\Rank";

fn data() -> &'static Json {
    static DATA: OnceLock<Json> = OnceLock::new();
    DATA.get_or_init(|| serde_json::from_str(include_str!("data/persistence.json")).unwrap())
}

fn case(class: &str, name: &str, value: Value) -> php_value::Object {
    EnumCase::new(class, name, Some(value)).object()
}

fn register() {
    common::register_fixtures();
    register_class(ClassInfo::enumeration(
        SUIT,
        vec![
            case(SUIT, "Hearts", Value::from("H")),
            case(SUIT, "Diamonds", Value::from("D")),
            case(SUIT, "Clubs", Value::from("1")),
        ],
    ));
    register_class(ClassInfo::enumeration(
        RANK,
        vec![
            case(RANK, "One", Value::Int(1)),
            case(RANK, "Two", Value::Int(2)),
        ],
    ));
    register_class(
        ClassInfo::class(HOLDER)
            .property(PropertyInfo::typed("suit", SUIT, true).default_value(Value::Null))
            .property(PropertyInfo::typed("suits", "array", true).default_value(Value::Null))
            .property(PropertyInfo::typed("rank", RANK, true).default_value(Value::Null))
            .property(PropertyInfo::typed("ranks", "array", true).default_value(Value::Null))
            .property(PropertyInfo::typed("typed", "int", false)),
    );
}

fn new_holder() -> FixtureObject {
    FixtureObject::new(
        HOLDER,
        vec![
            FixtureProperty::typed("suit", Some(Value::Null)),
            FixtureProperty::typed("suits", Some(Value::Null)),
            FixtureProperty::typed("rank", Some(Value::Null)),
            FixtureProperty::typed("ranks", Some(Value::Null)),
            FixtureProperty::typed("typed", None),
        ],
    )
}

fn property(name: &str, enum_type: &str) -> EnumReflectionProperty {
    let original: Arc<dyn PropertyAccessor> =
        Arc::new(ReflectionProperty::new(HOLDER, name).unwrap());
    EnumReflectionProperty::new(original, enum_type)
}

/// The port's result against PHP's outcome.
#[track_caller]
fn assert_outcome(expected: &Outcome, actual: Result<Value, Thrown>, context: &str) {
    match (expected, actual) {
        (Outcome::Value(expected), Ok(actual)) => {
            assert!(
                expected.identical(&actual),
                "{context}: {expected:?} !== {actual:?}"
            );
        }
        (Outcome::Throws { class, message }, Err(thrown)) => {
            assert_eq!(*class, thrown.class, "{context}");
            let ours = String::from_utf8_lossy(thrown.message.as_bytes()).into_owned();
            assert!(
                *message == ours || message.starts_with(&format!("{ours}, called in ")),
                "{context}: PHP {message:?}, port {ours:?}"
            );
        }
        (expected, actual) => panic!("{context}: PHP {expected:?}, port {actual:?}"),
    }
}

#[test]
fn enum_reflection_property_set_value() {
    register();
    let mut count = 0;
    for entry in data()["set"].as_array().unwrap() {
        let name = entry[0].as_str().unwrap();
        let enum_type = entry[1].as_str().unwrap();
        let value = decode(&entry[2]).unwrap();
        let set = outcome(&entry[3]).unwrap();
        let get = outcome(&entry[4]).unwrap();
        if matches!(&set, Outcome::Throws { message, .. } if message.starts_with("Cannot assign "))
        {
            continue;
        }
        let context = format!("{name} = {value:?}");

        let holder = new_holder();
        let property = property(name, enum_type);
        let result = property
            .set_value(&holder, value.clone())
            .map(|()| holder.get(name).unwrap());
        assert_outcome(&set, result, &context);
        assert_outcome(&get, property.get_value(&holder), &context);
        count += 1;
    }
    assert_eq!(count, 70);
}

#[test]
fn enum_reflection_property_get_value() {
    register();
    for entry in data()["get"].as_array().unwrap() {
        let name = entry[0].as_str().unwrap();
        let raw = decode(&entry[1]).unwrap();
        let expected = outcome(&entry[2]).unwrap();

        let holder = new_holder();
        holder.set(name, raw.clone());
        let property = property(name, SUIT);
        assert_outcome(
            &expected,
            property.get_value(&holder),
            &format!("{name} = {raw:?}"),
        );
    }
}

#[test]
fn reflection_messages() {
    register();
    let reflection = &data()["reflection"];
    let holder = ReflectionClass::new(HOLDER).unwrap();
    let cases: [(&str, Result<Value, Thrown>); 6] = [
        (
            "class",
            ReflectionClass::new("Missing\\Cls").map(|_| Value::Null),
        ),
        (
            "property",
            ReflectionProperty::new(HOLDER, "nope").map(|_| Value::Null),
        ),
        ("method", holder.get_method("nope").map(|_| Value::Null)),
        (
            "interface",
            holder.implements_interface("Missing").map(Value::Bool),
        ),
        (
            "not_interface",
            holder.implements_interface(HOLDER).map(Value::Bool),
        ),
        (
            "uninitialized",
            ReflectionProperty::new(HOLDER, "typed")
                .unwrap()
                .get_value(&new_holder()),
        ),
    ];
    for (name, result) in cases {
        assert_outcome(&outcome(&reflection[name]).unwrap(), result, name);
    }
    let code = ReflectionClass::new("Missing\\Cls").unwrap_err().code;
    assert_eq!(reflection["class_code"].as_i64(), Some(code));
}

/// `Registry`, whose services are not object managers.
struct Registry {
    state: ManagerRegistryState,
}

impl AbstractManagerRegistry for Registry {
    fn registry_state(&self) -> &ManagerRegistryState {
        &self.state
    }

    fn get_service(&self, _name: &str) -> Result<Service, Thrown> {
        Ok(Service::Connection(Arc::new(())))
    }

    fn reset_service(&self, _name: &str) {}
}

#[test]
fn manager_registry_assertion() {
    let registry = Registry {
        state: ManagerRegistryState::new(
            "ORM",
            Vec::new(),
            vec![("default".to_owned(), "default".to_owned())],
            "default",
            "default",
            None,
        ),
    };
    let Err(thrown) = registry.get_manager(None) else {
        panic!("an AssertionError");
    };
    let expected = &data()["assertion"];
    assert_eq!(expected[0].as_str(), Some(thrown.class.as_str()));
    assert_eq!(
        expected[1].as_str(),
        Some(String::from_utf8_lossy(thrown.message.as_bytes()).as_ref())
    );
    assert_eq!(expected[2].as_i64(), Some(thrown.code));
}
