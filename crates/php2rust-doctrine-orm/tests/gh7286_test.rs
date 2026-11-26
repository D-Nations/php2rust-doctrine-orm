//! Ticket/GH7286Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH7286Test.php

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

use php_value::{PhpArray, Thrown, Value};
use php2rust_doctrine_orm::configuration::Configuration;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::query::ast::Node;
use php2rust_doctrine_orm::query::ast::functions::FunctionNode;
use php2rust_doctrine_orm::query::context::register_function_class;
use php2rust_doctrine_orm::query::sql_walker::dispatch;
use php2rust_doctrine_orm::query::{Parser, SqlWalker, TokenType};

use orm::*;
use ticket::*;

/// `GH7286CustomConcat extends FunctionNode`. `getSql()` reads the
/// platform from the walker's entity manager (the port's walker has no
/// `getConnection()`).
#[derive(Debug)]
struct GH7286CustomConcat {
    name: String,
    first: Option<Node>,
    second: Option<Node>,
}

impl FunctionNode for GH7286CustomConcat {
    fn class(&self) -> &str {
        "Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH7286CustomConcat"
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn parse(&mut self, parser: &mut Parser<'_>) -> Result<(), Thrown> {
        parser.match_token(TokenType::T_IDENTIFIER)?;
        parser.match_token(TokenType::T_OPEN_PARENTHESIS)?;

        self.first = Some(parser.string_primary()?);
        parser.match_token(TokenType::T_COMMA)?;
        self.second = Some(parser.string_primary()?);

        parser.match_token(TokenType::T_CLOSE_PARENTHESIS)
    }

    fn get_sql(&self, walker: &mut dyn SqlWalker<'_>) -> Result<String, Thrown> {
        let not_parsed = || Thrown::new("Error", "GH7286CustomConcat is not parsed", 0);
        let first = dispatch(walker, self.first.as_ref().ok_or_else(not_parsed)?)?;
        let second = dispatch(walker, self.second.as_ref().ok_or_else(not_parsed)?)?;
        Ok(walker
            .get_entity_manager()
            .get_database_platform()
            .get_concat_expression(&[&first, &second]))
    }
}

/// `setUp()`.
async fn set_up(configure: impl FnOnce(&mut Configuration)) -> Arc<EntityManager> {
    let em = orm::entity_manager_with(orm::connection(), configure);
    orm::create_schema(&em, &[&class("GH7286Entity")]).await;

    for (r#type, version) in [
        (s("foo"), 1),
        (s("foo"), 2),
        (s("bar"), 3),
        (Value::Null, 4),
    ] {
        // new GH7286Entity($type, $version)
        let entity = new("GH7286Entity");
        set(&entity, "type", r#type);
        set(&entity, "version", Value::Int(version));
        em.persist(&entity).unwrap();
    }
    em.flush().await.unwrap();
    em.clear().unwrap();

    em
}

/// `['pair' => $pair]`.
fn pair(pair: &str) -> Value {
    array(&[("pair", s(pair))])
}

#[tokio::test]
async fn testAggregateExpressionInFunction() {
    let em = set_up(|_| {}).await;
    let query = em.create_query(&format!(
        "SELECT CONCAT(e.type, MIN(e.version)) pair FROM {} e WHERE e.type IS NOT NULL GROUP BY e.type ORDER BY e.type",
        class("GH7286Entity")
    ));

    let mut expected = PhpArray::new();
    expected.push(pair("bar3")).unwrap();
    expected.push(pair("foo1")).unwrap();
    let result = query.get_array_result().await.unwrap();
    assert!(Value::Array(expected).identical(&result), "{result:?}");
}

#[tokio::test]
async fn testAggregateFunctionInCustomFunction() {
    register_function_class(
        "Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH7286CustomConcat",
        Arc::new(|name| {
            Box::new(GH7286CustomConcat {
                name: name.to_owned(),
                first: None,
                second: None,
            })
        }),
    );
    // The entity manager takes the configuration by value: the function is
    // added before it is created.
    let em = set_up(|config| {
        config.add_custom_string_function(
            "CC",
            "Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH7286CustomConcat",
        );
    })
    .await;

    let query = em.create_query(&format!(
        "SELECT CC(e.type, MIN(e.version)) pair FROM {} e WHERE e.type IS NOT NULL AND e.type != :type GROUP BY e.type",
        class("GH7286Entity")
    ));
    query.set_parameter("type", s("bar"), None);

    let result = query.get_single_result(None).await.unwrap();
    assert!(pair("foo1").identical(&result), "{result:?}");
}
