//! Ticket/DDC736Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the ecommerce models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC736Test.php

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

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use php_value::{Object, Thrown, Value, php_array};
use php2rust_doctrine_orm::abstract_query::HYDRATE_OBJECT;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::query::ast::{DeleteStatement, Node, SelectStatement, UpdateStatement};
use php2rust_doctrine_orm::query::context::{
    DqlQuery, HINT_CUSTOM_TREE_WALKERS, Hint, QueryComponents, TreeWalkerClass,
};
use php2rust_doctrine_orm::query::{ParserResult, TreeWalker, TreeWalkerAdapter};

use orm::*;

/// A cart of a customer, persisted, flushed and cleared.
async fn set_up() -> Arc<EntityManager> {
    let em = orm::set_up(&ECOMMERCE_MODELS).await;

    let cust = new_object(ECOMMERCE_CUSTOMER);
    set(&cust, "name", s("roman"));

    let cart = ecommerce_cart("cash");
    // $cart->setCustomer($cust), which calls $cust->setCart($cart)
    set_obj(&cart, "customer", &cust);
    set_obj(&cust, "cart", &cart);

    em.persist(&cust).unwrap();
    em.persist(&cart).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();
    em
}

fn is_uninitialized(em: &EntityManager, entity: &Object) -> bool {
    em.is_uninitialized_object(&Value::Object(entity.clone()))
}

#[tokio::test]
async fn testReorderEntityFetchJoinForHydration() {
    let em = set_up().await;

    let result = em
        .create_query("select c, c.name, ca, ca.payment from Doctrine\\Tests\\Models\\ECommerce\\ECommerceCart ca join ca.customer c")
        .get_single_result(None)
        .await
        .unwrap();

    let Value::Array(mut result) = result else {
        panic!("{result:?} is not an array");
    };
    let cart2 = obj(&result.get(&0.into()).cloned().unwrap());
    result.remove(&0.into());

    assert_eq!(ECOMMERCE_CART, cart2.class());
    let customer = get_object(&cart2, "customer").unwrap();
    assert!(!is_uninitialized(&em, &customer));
    assert_eq!(ECOMMERCE_CUSTOMER, customer.class());
    assert_loose_eq(
        &Value::Array(php_array! { "name" => s("roman"), "payment" => s("cash") }),
        &Value::Array(result),
    );
}

#[tokio::test]
async fn testDqlTreeWalkerReordering() {
    let em = set_up().await;

    let dql = "select c, c.name, ca, ca.payment from Doctrine\\Tests\\Models\\ECommerce\\ECommerceCart ca join ca.customer c";
    let query = em.create_query(dql);
    query.set_hint(
        HINT_CUSTOM_TREE_WALKERS,
        Hint::TreeWalkers(vec![DisableFetchJoinTreeWalker::CLASS]),
    );
    let result = query.get_result(HYDRATE_OBJECT).await.unwrap();

    let cart2 = obj(&at(&at(&result, 0), 0));
    assert_eq!(ECOMMERCE_CART, cart2.class());
    assert!(is_uninitialized(
        &em,
        &get_object(&cart2, "customer").unwrap()
    ));
}

/// `DisableFetchJoinTreeWalker`.
struct DisableFetchJoinTreeWalker<'a> {
    adapter: TreeWalkerAdapter<'a>,
}

impl DisableFetchJoinTreeWalker<'_> {
    const CLASS: TreeWalkerClass = TreeWalkerClass {
        name: "Doctrine\\Tests\\ORM\\Functional\\Ticket\\DisableFetchJoinTreeWalker",
        construct,
    };
}

fn construct<'a>(
    query: &'a dyn DqlQuery,
    parser_result: Rc<RefCell<ParserResult>>,
    query_components: QueryComponents,
) -> Box<dyn TreeWalker<'a> + 'a> {
    Box::new(DisableFetchJoinTreeWalker {
        adapter: TreeWalkerAdapter::new(query, parser_result, query_components),
    })
}

impl<'a> TreeWalker<'a> for DisableFetchJoinTreeWalker<'a> {
    fn get_query_components(&self) -> &QueryComponents {
        self.adapter.get_query_components()
    }

    fn walk_select_statement(
        &mut self,
        select_statement: &mut SelectStatement,
    ) -> Result<(), Thrown> {
        let expressions = &mut select_statement.select_clause.select_expressions;
        if let Some(key) = expressions
            .iter()
            .position(|select_expr| matches!(&select_expr.expression, Node::Str(e) if e == "c"))
        {
            expressions.remove(key);
        }
        Ok(())
    }

    fn walk_update_statement(&mut self, _: &mut UpdateStatement) -> Result<(), Thrown> {
        Ok(())
    }

    fn walk_delete_statement(&mut self, _: &mut DeleteStatement) -> Result<(), Thrown> {
        Ok(())
    }
}
