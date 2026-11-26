//! Functional/CustomFunctionsTest.php on `SQLite` (`tests/common/orm.rs`).
//!
//! The entity manager takes its `Configuration` by value, so the custom
//! functions are added to the configuration before the entity manager is
//! created (PHP adds them after the user is flushed; the query is parsed
//! after both in either case). A callable is given with
//! `add_custom_*_function_callable()`; the class `CustomCount` registers
//! its constructor (`register_function_class()`).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/CustomFunctionsTest.php

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

use php_value::{Thrown, Value};
use php2rust_doctrine_orm::configuration::Configuration;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::query::ast::functions::FunctionNode;
use php2rust_doctrine_orm::query::ast::{AggregateExpression, Node};
use php2rust_doctrine_orm::query::context::{HYDRATE_OBJECT, register_function_class};
use php2rust_doctrine_orm::query::sql_walker::dispatch;
use php2rust_doctrine_orm::query::{Parser, SqlWalker, TokenType};

use orm::{CMS_MODELS, cms_user, s, same_value, set};

const NO_OP: &str = "Doctrine\\Tests\\ORM\\Functional\\NoOp";
const CUSTOM_COUNT: &str = "Doctrine\\Tests\\ORM\\Functional\\CustomCount";

/// `NoOp extends FunctionNode`.
#[derive(Debug)]
struct NoOp {
    name: String,
    field: Option<Node>,
}

impl FunctionNode for NoOp {
    fn class(&self) -> &str {
        NO_OP
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn parse(&mut self, parser: &mut Parser<'_>) -> Result<(), Thrown> {
        parser.match_token(TokenType::T_IDENTIFIER)?;
        parser.match_token(TokenType::T_OPEN_PARENTHESIS)?;
        self.field = Some(parser.arithmetic_primary()?);
        parser.match_token(TokenType::T_CLOSE_PARENTHESIS)
    }

    fn get_sql(&self, sql_walker: &mut dyn SqlWalker<'_>) -> Result<String, Thrown> {
        let field = self
            .field
            .as_ref()
            .ok_or_else(|| Thrown::new("Error", "NoOp::$field is not parsed", 0))?;
        dispatch(sql_walker, field)
    }
}

/// `CustomCount extends FunctionNode`.
#[derive(Debug)]
struct CustomCount {
    name: String,
    aggregate_expression: Option<AggregateExpression>,
}

impl FunctionNode for CustomCount {
    fn class(&self) -> &str {
        CUSTOM_COUNT
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn parse(&mut self, parser: &mut Parser<'_>) -> Result<(), Thrown> {
        self.aggregate_expression = Some(parser.aggregate_expression()?);
        Ok(())
    }

    fn get_sql(&self, sql_walker: &mut dyn SqlWalker<'_>) -> Result<String, Thrown> {
        let aggregate_expression = self.aggregate_expression.as_ref().ok_or_else(|| {
            Thrown::new(
                "Error",
                "CustomCount::$aggregateExpression is not parsed",
                0,
            )
        })?;
        sql_walker.walk_aggregate_expression(aggregate_expression)
    }
}

async fn set_up(configure: impl FnOnce(&mut Configuration)) -> Arc<EntityManager> {
    let em = orm::entity_manager_with(orm::connection(), configure);
    orm::create_schema(&em, &CMS_MODELS).await;
    em
}

#[tokio::test]
async fn testCustomFunctionDefinedWithCallback() {
    let em = set_up(|config| {
        // Instead of defining the function with the class name, we use a callback
        config.add_custom_string_function_callable(
            "FOO",
            Arc::new(|func_name| {
                Box::new(NoOp {
                    name: func_name.to_owned(),
                    field: None,
                })
            }),
        );
        config.add_custom_numeric_function_callable(
            "BAR",
            Arc::new(|func_name| {
                Box::new(NoOp {
                    name: func_name.to_owned(),
                    field: None,
                })
            }),
        );
    })
    .await;

    let user = cms_user();
    set(&user, "name", s("Bob"));
    set(&user, "username", s("Dylan"));
    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    let query = em.create_query(
        "SELECT u FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u WHERE FOO(u.name) = 'Bob' AND BAR(1) = 1",
    );

    let users = match query.get_result(HYDRATE_OBJECT).await.unwrap() {
        Value::Array(users) => users,
        other => panic!("not a list: {other:?}"),
    };

    assert_eq!(users.len(), 1);
    assert!(same_value(users.values().next().unwrap(), &user));
}

#[tokio::test]
async fn testCustomFunctionOverride() {
    register_function_class(
        CUSTOM_COUNT,
        Arc::new(|name| {
            Box::new(CustomCount {
                name: name.to_owned(),
                aggregate_expression: None,
            })
        }),
    );
    let em = set_up(|config| {
        config.add_custom_string_function("COUNT", CUSTOM_COUNT);
    })
    .await;

    let user = cms_user();
    set(&user, "name", s("Bob"));
    set(&user, "username", s("Dylan"));
    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    let query =
        em.create_query("SELECT COUNT(DISTINCT u.id) FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u");

    let users_count = query.get_single_scalar_result().await.unwrap();

    assert!(php_value::compare::loose_equals(
        &Value::Int(1),
        &users_count
    ));
}
