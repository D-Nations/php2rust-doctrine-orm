//! Ticket/DDC2984Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models; `DDC2984UserIdCustomDbalType` and `DDC2984DomainUserId` are
//! Rust types. DBAL 4 has no `ConversionException::conversionFailed()`: PHP's
//! call of it throws an `Error`, which the type returns).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC2984Test.php

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
#[path = "common/ticket.rs"]
mod ticket;

use std::sync::Arc;

use php_value::{Object, PhpArray, PhpObject, PhpString, Thrown, Value};
use php2rust_doctrine_dbal::platforms::AbstractPlatform;
use php2rust_doctrine_dbal::types::{ConversionError, Type};

use orm::*;
use ticket::*;

/// `DDC2984DomainUserId implements Stringable`.
#[derive(Debug)]
struct DDC2984DomainUserId {
    user_id_string: PhpString,
}

impl DDC2984DomainUserId {
    const CLASS: &str = "Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC2984DomainUserId";

    /// `toString()`.
    fn to_string(value: &Value) -> PhpString {
        let Value::Object(object) = value else {
            panic!("a DDC2984DomainUserId")
        };
        object.to_php_string().unwrap()
    }
}

impl PhpObject for DDC2984DomainUserId {
    fn class(&self) -> &str {
        Self::CLASS
    }

    fn properties(&self) -> PhpArray {
        PhpArray::new()
    }

    fn to_php_string(&self) -> Option<PhpString> {
        Some(self.user_id_string.clone())
    }
}

/// `new DDC2984DomainUserId($userIdString)`.
fn domain_user_id(user_id_string: impl Into<PhpString>) -> Value {
    let id: Object = Arc::new(DDC2984DomainUserId {
        user_id_string: user_id_string.into(),
    });
    Value::Object(id)
}

/// `DDC2984UserIdCustomDbalType extends StringType`.
#[derive(Debug)]
struct DDC2984UserIdCustomDbalType;

impl DDC2984UserIdCustomDbalType {
    const TYPE_NAME: &str = "ddc2984_domain_user_id";
}

impl Type for DDC2984UserIdCustomDbalType {
    fn class(&self) -> &'static str {
        "Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC2984UserIdCustomDbalType"
    }

    fn get_sql_declaration(
        &self,
        column: &PhpArray,
        platform: &dyn AbstractPlatform,
    ) -> Result<PhpString, php2rust_doctrine_dbal::Exception> {
        platform.get_string_type_declaration_sql(column)
    }

    fn convert_to_php_value(
        &self,
        value: &Value,
        _platform: &dyn AbstractPlatform,
    ) -> Result<Value, ConversionError> {
        Ok(if value.is_truthy() {
            domain_user_id(value.to_php_string().unwrap_or_default())
        } else {
            Value::Null
        })
    }

    fn convert_to_database_value(
        &self,
        value: &Value,
        _platform: &dyn AbstractPlatform,
    ) -> Result<Value, ConversionError> {
        if !value.is_truthy() {
            return Ok(Value::Null);
        }

        if matches!(value, Value::String(_)) {
            return Ok(value.clone());
        }

        if !matches!(value, Value::Object(object) if object.class() == DDC2984DomainUserId::CLASS) {
            return Err(ConversionError::Thrown(Thrown::new(
                "Error",
                "Call to undefined method Doctrine\\DBAL\\Types\\ConversionException::conversionFailed()",
                0,
            )));
        }

        Ok(Value::String(DDC2984DomainUserId::to_string(value)))
    }
}

/// `$user->sameIdentityAs($other)`.
fn same_identity_as(user: &Object, other: &Object) -> bool {
    DDC2984DomainUserId::to_string(&get(user, "userId"))
        == DDC2984DomainUserId::to_string(&get(other, "userId"))
}

#[tokio::test]
async fn testIssue() {
    if !php2rust_doctrine_dbal::types::has_type(DDC2984UserIdCustomDbalType::TYPE_NAME) {
        ticket::add_type(
            DDC2984UserIdCustomDbalType::TYPE_NAME,
            Arc::new(DDC2984UserIdCustomDbalType),
        );
    }
    let em = ticket::set_up(&["DDC2984User"]).await;

    let user = new("DDC2984User");
    set(&user, "userId", domain_user_id("unique_id_within_a_vo"));
    set(&user, "name", s("Alex"));

    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    let repository = em.get_repository(&class("DDC2984User")).unwrap();

    let same_user = repository
        .find(domain_user_id("unique_id_within_a_vo"))
        .await
        .unwrap()
        .expect("the user");

    //Until know, everything works as expected
    assert!(same_identity_as(&user, &same_user));

    em.clear().unwrap();

    //After clearing the identity map, the UnitOfWork produces the warning described in DDC-2984
    let equal_user = repository
        .find(domain_user_id("unique_id_within_a_vo"))
        .await
        .unwrap()
        .expect("the user");

    assert!(!same(&user, &equal_user));
    assert!(same_identity_as(&user, &equal_user));
}
