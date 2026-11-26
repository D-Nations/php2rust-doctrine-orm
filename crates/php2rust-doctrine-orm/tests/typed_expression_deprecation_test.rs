//! Port of `TypedExpressionDeprecationTest`: a custom function that
//! implements only the deprecated `TypedExpression` triggers the
//! deprecation; one that implements `ExpressionWithReturnType` does not.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Query/TypedExpressionDeprecationTest.php

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

use php_value::Thrown;
use php2rust_doctrine_dbal::types::{Type, get_type};
use php2rust_doctrine_orm::deprecations;
use php2rust_doctrine_orm::query::ast::Node;
use php2rust_doctrine_orm::query::ast::functions::FunctionNode;
use php2rust_doctrine_orm::query::{Parser, SqlWalker, TokenType};

use dql::{ExpectDeprecation, TestEntityManager};

const CMS_PHONENUMBER: &str = r"Doctrine\Tests\Models\CMS\CmsPhonenumber";
const DEPRECATION: &str = "https://github.com/doctrine/orm/pull/12543";

/// `Types::INTEGER`.
const INTEGER: &str = "integer";

/// `Type::getType($name)`.
fn dbal_type(name: &str) -> Result<Arc<dyn Type>, Thrown> {
    get_type(name).map_err(|e| Thrown::new("Doctrine\\DBAL\\Exception", format!("{e:?}"), 0))
}

/// The `getSql()` and `parse()` both stubs share.
#[derive(Debug)]
struct AbsStub {
    name: String,
    arithmetic_expression: Option<Node>,
}

impl AbsStub {
    fn get_sql(&self, sql_walker: &mut dyn SqlWalker<'_>) -> Result<String, Thrown> {
        let expression = self.arithmetic_expression.as_ref().unwrap();
        Ok(format!(
            "ABS({})",
            sql_walker.walk_simple_arithmetic_expression(expression)?
        ))
    }

    fn parse(&mut self, parser: &mut Parser<'_>) -> Result<(), Thrown> {
        parser.match_token(TokenType::T_IDENTIFIER)?;
        parser.match_token(TokenType::T_OPEN_PARENTHESIS)?;
        self.arithmetic_expression = Some(parser.simple_arithmetic_expression()?);
        parser.match_token(TokenType::T_CLOSE_PARENTHESIS)
    }
}

/// `LegacyTypedFunctionStub`: implements `TypedExpression`.
#[derive(Debug)]
struct LegacyTypedFunctionStub(AbsStub);

impl FunctionNode for LegacyTypedFunctionStub {
    fn class(&self) -> &'static str {
        r"Doctrine\Tests\ORM\Query\LegacyTypedFunctionStub"
    }

    fn name(&self) -> &str {
        &self.0.name
    }

    fn get_sql(&self, sql_walker: &mut dyn SqlWalker<'_>) -> Result<String, Thrown> {
        self.0.get_sql(sql_walker)
    }

    fn parse(&mut self, parser: &mut Parser<'_>) -> Result<(), Thrown> {
        self.0.parse(parser)
    }

    fn return_type(&self) -> Option<Result<Arc<dyn Type>, Thrown>> {
        Some(dbal_type(INTEGER))
    }
}

/// `ModernTypedFunctionStub`: implements `ExpressionWithReturnType` and
/// `TypedExpression`.
#[derive(Debug)]
struct ModernTypedFunctionStub(AbsStub);

impl FunctionNode for ModernTypedFunctionStub {
    fn class(&self) -> &'static str {
        r"Doctrine\Tests\ORM\Query\ModernTypedFunctionStub"
    }

    fn name(&self) -> &str {
        &self.0.name
    }

    fn get_sql(&self, sql_walker: &mut dyn SqlWalker<'_>) -> Result<String, Thrown> {
        self.0.get_sql(sql_walker)
    }

    fn parse(&mut self, parser: &mut Parser<'_>) -> Result<(), Thrown> {
        self.0.parse(parser)
    }

    fn return_type_name(&self) -> Option<String> {
        Some(INTEGER.to_owned())
    }

    fn return_type(&self) -> Option<Result<Arc<dyn Type>, Thrown>> {
        Some(dbal_type(INTEGER))
    }
}

fn stub(name: &str) -> AbsStub {
    AbsStub {
        name: name.to_owned(),
        arithmetic_expression: None,
    }
}

#[test]
fn testImplementingLegacyTypedExpressionTriggersDeprecation() {
    let entity_manager = TestEntityManager::new();
    entity_manager.add_custom_numeric_function(
        "LEGACY_TYPED",
        Arc::new(|name| Box::new(LegacyTypedFunctionStub(stub(name)))),
    );

    let _deprecation = ExpectDeprecation::new(DEPRECATION);

    entity_manager
        .create_query(&format!(
            "SELECT LEGACY_TYPED(p.phonenumber) FROM {CMS_PHONENUMBER} p"
        ))
        .get_sql()
        .unwrap();
}

#[test]
fn testImplementingExpressionWithReturnTypeDoesNotTriggerDeprecation() {
    let entity_manager = TestEntityManager::new();
    entity_manager.add_custom_numeric_function(
        "MODERN_TYPED",
        Arc::new(|name| Box::new(ModernTypedFunctionStub(stub(name)))),
    );

    // `expectNoDeprecationWithIdentifier()`.
    deprecations::enable_tracking_deprecations();

    entity_manager
        .create_query(&format!(
            "SELECT MODERN_TYPED(p.phonenumber) FROM {CMS_PHONENUMBER} p"
        ))
        .get_sql()
        .unwrap();

    let count = deprecations::count(DEPRECATION);
    deprecations::disable();
    assert_eq!(0, count);
}
