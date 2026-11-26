//! RootTypeWalkerTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on the test entity manager (`tests/common/dql.rs`, `SQLite`), with
//! `Doctrine\Tests\DbalTypes\Rot13Type` registered as `rot13`. The
//! provider's rows run in one test, labelled.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Tools/Pagination/RootTypeWalkerTest.php tests/Tests/DbalTypes/Rot13Type.php

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

#[path = "common/dql.rs"]
mod dql;
#[path = "common/models.rs"]
mod models;

use std::sync::Arc;

use php_value::{PhpArray, PhpString, Value};
use php2rust_doctrine_dbal::Exception;
use php2rust_doctrine_dbal::platforms::AbstractPlatform;
use php2rust_doctrine_dbal::types::{ConversionError, Type, TypeArgument, add_type, has_type};
use php2rust_doctrine_orm::query::context::{HINT_CUSTOM_OUTPUT_WALKER, Hint};
use php2rust_doctrine_orm::tools::pagination::RootTypeWalker;

use dql::TestEntityManager;

/// `str_rot13($value)`.
fn str_rot13(value: &[u8]) -> Vec<u8> {
    value
        .iter()
        .map(|&b| match b {
            b'a'..=b'z' => (b - b'a' + 13) % 26 + b'a',
            b'A'..=b'Z' => (b - b'A' + 13) % 26 + b'A',
            other => other,
        })
        .collect()
}

/// Shifts every letter by 13 places in the alphabet (ROT13 encoding).
#[derive(Debug)]
struct Rot13Type;

impl Rot13Type {
    fn convert(value: &Value) -> Value {
        if value.is_null() {
            return Value::Null;
        }
        let text = value.to_php_string().unwrap_or_default();
        Value::String(PhpString::from(str_rot13(text.as_bytes())))
    }
}

impl Type for Rot13Type {
    fn class(&self) -> &'static str {
        "Doctrine\\Tests\\DbalTypes\\Rot13Type"
    }

    fn convert_to_database_value(
        &self,
        value: &Value,
        _platform: &dyn AbstractPlatform,
    ) -> Result<Value, ConversionError> {
        Ok(Self::convert(value))
    }

    fn convert_to_php_value(
        &self,
        value: &Value,
        _platform: &dyn AbstractPlatform,
    ) -> Result<Value, ConversionError> {
        Ok(Self::convert(value))
    }

    fn get_sql_declaration(
        &self,
        column: &PhpArray,
        platform: &dyn AbstractPlatform,
    ) -> Result<PhpString, Exception> {
        platform.get_string_type_declaration_sql(column)
    }
}

fn set_up() -> TestEntityManager {
    let entity_manager = TestEntityManager::new();
    if !has_type("rot13") {
        // Another test thread may have added it meanwhile.
        let _ = add_type("rot13", TypeArgument::Instance(Arc::new(Rot13Type)));
    }
    entity_manager
}

fn example_queries() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        (
            "Entity with #Id column of special type",
            r"SELECT e.id4 FROM Doctrine\Tests\Models\ValueConversionType\AuxiliaryEntity e",
            "rot13",
        ),
        (
            "Entity where #Id is a to-one relation with special type identifier",
            r"SELECT e FROM Doctrine\Tests\Models\ValueConversionType\OwningManyToOneIdForeignKeyEntity e",
            "rot13",
        ),
        (
            "Simple integer ID in a query with a JOIN",
            r"SELECT u, g FROM Doctrine\Tests\ORM\Tools\Pagination\User u JOIN u.groups g",
            "integer",
        ),
    ]
}

#[test]
fn testResolveTypeMapping() {
    let entity_manager = set_up();
    for (label, dql_query, expected_type) in example_queries() {
        let query = entity_manager.create_query(dql_query);
        query.hint(
            HINT_CUSTOM_OUTPUT_WALKER,
            Hint::OutputWalker(RootTypeWalker::CLASS),
        );

        assert_eq!(expected_type, query.get_single_sql().unwrap(), "{label}");
    }
}
