//! The DQL parser and SQL walker on a real `EntityManager` (its
//! `QueryContext`): class metadata, the owning side of an association,
//! the connection's quoting, the quote strategy and a custom function
//! named by class in the `Configuration`. No upstream test of its own;
//! the SQL is the one `SelectSqlGenerationTest` expects.
//!
//! Upstream: none

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use php2rust_doctrine_orm::configuration::Configuration;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::mapping::driver::AttributeDriver;
use php2rust_doctrine_orm::query::Parser;
use php2rust_doctrine_orm::query::ast::functions::AbsFunction;
use php2rust_doctrine_orm::query::context::{
    DqlQuery, Hint, QueryContext, register_function_class,
};
use php2rust_doctrine_orm::query::exec::SqlStatements;

/// The part of `Query` the parser reads, over the entity manager.
struct Query<'em> {
    em: &'em EntityManager,
    dql: String,
    hints: RefCell<HashMap<String, Hint>>,
}

impl DqlQuery for Query<'_> {
    fn get_entity_manager(&self) -> &dyn QueryContext {
        self.em
    }

    fn get_dql(&self) -> String {
        self.dql.clone()
    }

    fn get_hint(&self, name: &str) -> Hint {
        self.hints
            .borrow()
            .get(name)
            .cloned()
            .unwrap_or(Hint::Value(php_value::Value::Bool(false)))
    }

    fn set_hint(&self, name: &str, value: Hint) {
        self.hints.borrow_mut().insert(name.to_owned(), value);
    }
}

fn get_sql(em: &EntityManager, dql: &str) -> String {
    let query = Query {
        em,
        dql: dql.to_owned(),
        hints: RefCell::new(HashMap::new()),
    };
    let parser_result = Parser::new(&query).unwrap().parse().unwrap();
    let executor = parser_result.borrow().prepare_sql_executor(&query).unwrap();
    match executor.get_sql_statements() {
        SqlStatements::One(sql) => sql.clone(),
        SqlStatements::Many(sql) => sql.join("; "),
    }
}

async fn entity_manager() -> Arc<EntityManager> {
    models::register();
    register_function_class(
        "Doctrine\\Tests\\ORM\\Query\\MyAbsFunction",
        Arc::new(|name| Box::new(AbsFunction::new(name))),
    );
    let mut config = Configuration::new();
    config.set_metadata_driver_impl(Arc::new(
        AttributeDriver::new(Vec::<String>::new(), true).unwrap(),
    ));
    config.add_custom_numeric_function("MYABS", "Doctrine\\Tests\\ORM\\Query\\MyAbsFunction");
    let connection = orm::connection();
    // The driver's quoting needs an open connection.
    connection.connect().await.unwrap();
    EntityManager::create(connection, config, None).unwrap()
}

#[tokio::test]
async fn selects_with_a_join_and_a_string_literal() {
    let em = entity_manager().await;
    assert_eq!(
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c1_.phonenumber AS phonenumber_4, c0_.email_id AS email_id_5, c1_.user_id AS user_id_6 FROM cms_users c0_ INNER JOIN cms_phonenumbers c1_ ON c0_.id = c1_.user_id WHERE c0_.name = 'O''Neil'",
        get_sql(
            &em,
            "SELECT u, p FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u JOIN u.phonenumbers p WHERE u.name = 'O''Neil'"
        )
    );
}

#[tokio::test]
async fn resolves_a_custom_function_by_its_class() {
    let em = entity_manager().await;
    assert_eq!(
        "SELECT ABS(c0_.phonenumber) AS sclr_0 FROM cms_phonenumbers c0_",
        get_sql(
            &em,
            "SELECT MYABS(p.phonenumber) FROM Doctrine\\Tests\\Models\\CMS\\CmsPhonenumber p"
        )
    );
}
