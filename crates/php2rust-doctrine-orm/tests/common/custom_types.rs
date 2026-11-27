//! The custom DBAL types of the tests (`Doctrine\Tests\DbalTypes`) and
//! the `CustomTypeParent` model's constructor and methods.
//!
//! Upstream: doctrine/orm tests/Tests/DbalTypes/UpperCaseStringType.php tests/Tests/DbalTypes/NegativeToPositiveType.php
//! Upstream: doctrine/orm tests/Tests/Models/CustomType/CustomTypeParent.php

#![allow(dead_code, reason = "each test binary uses a part of the helpers")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

use std::sync::Arc;

use php_value::{Object, PhpArray, PhpString, Value};
use php2rust_doctrine_dbal::platforms::AbstractPlatform;
use php2rust_doctrine_dbal::types::{ConversionError, Type};
use php2rust_doctrine_dbal::types::{TypeArgument, add_type, has_type, override_type};
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};

use crate::orm::{get, new_object, set};

pub const CUSTOM_TYPE_CHILD: &str = "Doctrine\\Tests\\Models\\CustomType\\CustomTypeChild";
pub const CUSTOM_TYPE_PARENT: &str = "Doctrine\\Tests\\Models\\CustomType\\CustomTypeParent";

/// `Doctrine\Tests\DbalTypes\UpperCaseStringType extends StringType`.
#[derive(Debug)]
pub struct UpperCaseStringType;

impl Type for UpperCaseStringType {
    fn class(&self) -> &'static str {
        "Doctrine\\Tests\\DbalTypes\\UpperCaseStringType"
    }

    fn get_sql_declaration(
        &self,
        column: &PhpArray,
        platform: &dyn AbstractPlatform,
    ) -> Result<PhpString, php2rust_doctrine_dbal::Exception> {
        platform.get_string_type_declaration_sql(column)
    }

    fn convert_to_database_value(
        &self,
        value: &Value,
        _platform: &dyn AbstractPlatform,
    ) -> Result<Value, ConversionError> {
        Ok(value.clone())
    }

    fn convert_to_php_value(
        &self,
        value: &Value,
        _platform: &dyn AbstractPlatform,
    ) -> Result<Value, ConversionError> {
        Ok(value.clone())
    }

    fn convert_to_database_value_sql(
        &self,
        sql_expr: &[u8],
        _platform: &dyn AbstractPlatform,
    ) -> PhpString {
        let mut out = b"UPPER(".to_vec();
        out.extend_from_slice(sql_expr);
        out.push(b')');
        PhpString::from(out)
    }

    fn convert_to_php_value_sql(
        &self,
        sql_expr: &[u8],
        _platform: &dyn AbstractPlatform,
    ) -> PhpString {
        let mut out = b"LOWER(".to_vec();
        out.extend_from_slice(sql_expr);
        out.push(b')');
        PhpString::from(out)
    }
}

/// `Doctrine\Tests\DbalTypes\NegativeToPositiveType extends Type`.
#[derive(Debug)]
pub struct NegativeToPositiveType;

impl Type for NegativeToPositiveType {
    fn class(&self) -> &'static str {
        "Doctrine\\Tests\\DbalTypes\\NegativeToPositiveType"
    }

    fn get_sql_declaration(
        &self,
        column: &PhpArray,
        platform: &dyn AbstractPlatform,
    ) -> Result<PhpString, php2rust_doctrine_dbal::Exception> {
        platform.get_integer_type_declaration_sql(column)
    }

    fn convert_to_database_value_sql(
        &self,
        sql_expr: &[u8],
        _platform: &dyn AbstractPlatform,
    ) -> PhpString {
        let mut sql = b"ABS(".to_vec();
        sql.extend_from_slice(sql_expr);
        sql.push(b')');
        PhpString::from(sql)
    }

    fn convert_to_php_value_sql(
        &self,
        sql_expr: &[u8],
        _platform: &dyn AbstractPlatform,
    ) -> PhpString {
        let mut sql = b"-(".to_vec();
        sql.extend_from_slice(sql_expr);
        sql.push(b')');
        PhpString::from(sql)
    }
}

/// `DBALType::addType($name, $class)`, or `overrideType()` when the
/// type is there.
pub fn add_or_override(name: &str, type_: impl Fn() -> TypeArgument) {
    if has_type(name) {
        override_type(name, type_()).unwrap();
    } else if add_type(name, type_()).is_err() {
        // Another test thread may have added it meanwhile.
        override_type(name, type_()).unwrap();
    }
}

/// `addType()`/`overrideType()` of `negative_to_positive` and
/// `upper_case_string`, as the tests' `setUp()` does.
pub fn register_types() {
    add_or_override("upper_case_string", || {
        TypeArgument::Instance(Arc::new(UpperCaseStringType))
    });
    add_or_override("negative_to_positive", || {
        TypeArgument::Instance(Arc::new(NegativeToPositiveType))
    });
}

/// `new CustomTypeParent()`.
pub fn custom_type_parent() -> Object {
    let parent = new_object(CUSTOM_TYPE_PARENT);
    set(
        &parent,
        "friendsWithMe",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    set(
        &parent,
        "myFriends",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    parent
}

/// `$parent->addMyFriend($friend)`.
pub fn add_my_friend(parent: &Object, friend: &Object) {
    coll::add(&get(parent, "myFriends"), Value::Object(friend.clone())).unwrap();
    coll::add(&get(friend, "friendsWithMe"), Value::Object(parent.clone())).unwrap();
}
