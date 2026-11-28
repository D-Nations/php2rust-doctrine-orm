//! Functional/Ticket/SwitchContextWithFilter/ChangeFiltersTest.php on
//! `SQLite` (`tests/common/orm.rs`).
//!
//! The assertion messages leave out the last logged query
//! (`generateMessage()`): the port has no SQL logger. The query cache
//! lifetime (`setQueryCacheLifetime(10)`) is not set: the port has no
//! query cache, the query is parsed again each time.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/SwitchContextWithFilter/ChangeFiltersTest.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/SwitchContextWithFilter/AbstractTestCase.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/SwitchContextWithFilter/Entity/Order.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/SwitchContextWithFilter/Entity/User.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/SwitchContextWithFilter/SQLFilter/CompanySQLFilter.php

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

use php_value::{Object, Thrown, Value};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::mapping::ClassMetadataInfo;
use php2rust_doctrine_orm::query::{FilterConstraint, SqlFilter, register_filter_class};

use orm::{get, get_object, new_object, s, set};

const ORDER: &str =
    "Doctrine\\Tests\\ORM\\Functional\\Ticket\\SwitchContextWithFilter\\Entity\\Order";
const USER: &str =
    "Doctrine\\Tests\\ORM\\Functional\\Ticket\\SwitchContextWithFilter\\Entity\\User";
const COMPANY_SQL_FILTER: &str = "Doctrine\\Tests\\ORM\\Functional\\Ticket\\SwitchContextWithFilter\\SQLFilter\\CompanySQLFilter";

const COMPANY_A: &str = "A";
const COMPANY_B: &str = "B";

/// `CompanySQLFilter extends SQLFilter`.
struct CompanySqlFilter;

impl FilterConstraint for CompanySqlFilter {
    fn add_filter_constraint(
        &self,
        filter: &SqlFilter,
        target_entity: &ClassMetadataInfo,
        target_table_alias: &str,
    ) -> Result<String, Thrown> {
        if target_entity.name == USER || target_entity.name == ORDER {
            let field_name = target_entity
                .field_mappings
                .get("company")
                .map_or_default(|mapping| mapping.field_name.clone());
            return Ok(format!(
                "{target_table_alias}.{field_name} = {}",
                filter.get_parameter("company")?
            ));
        }

        Ok(String::new())
    }
}

/// `new User($company)`.
fn user(company: &str) -> Object {
    let user = new_object(USER);
    set(&user, "company", s(company));
    user
}

/// `new Order($user)`.
fn order(user: &Object) -> Object {
    let order = new_object(ORDER);
    set(&order, "user", Value::Object(user.clone()));
    set(&order, "company", get(user, "company"));
    order
}

async fn set_up() -> Arc<EntityManager> {
    register_filter_class(
        COMPANY_SQL_FILTER,
        Arc::new(|em| SqlFilter::new(em, COMPANY_SQL_FILTER, Box::new(CompanySqlFilter))),
    );
    orm::set_up(&[ORDER, USER]).await
}

struct Ids {
    order_id: Value,
    user_id: Value,
}

async fn prepare_data(em: &Arc<EntityManager>) -> (Ids, Ids) {
    let user1 = user(COMPANY_A);
    let order1 = order(&user1);
    let user2 = user(COMPANY_B);
    let order2 = order(&user2);

    em.persist(&user1).unwrap();
    em.persist(&order1).unwrap();
    em.persist(&user2).unwrap();
    em.persist(&order2).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    (
        Ids {
            order_id: get(&order1, "id"),
            user_id: get(&user1, "id"),
        },
        Ids {
            order_id: get(&order2, "id"),
            user_id: get(&user2, "id"),
        },
    )
}

fn add_filter(em: &EntityManager) {
    em.get_configuration()
        .add_filter(COMPANY_SQL_FILTER, COMPANY_SQL_FILTER);
}

async fn find_order(em: &Arc<EntityManager>, id: &Value) -> Object {
    em.find(ORDER, id.clone()).await.unwrap().expect("an order")
}

#[tokio::test]
async fn testUseEnableDisableFilter() {
    let em = set_up().await;
    add_filter(&em);
    em.get_filters()
        .enable(COMPANY_SQL_FILTER)
        .unwrap()
        .set_parameter("company", s(COMPANY_A), None)
        .unwrap();

    let (company_a, company_b) = prepare_data(&em).await;

    let order1 = find_order(&em, &company_a.order_id).await;

    let user1 = get_object(&order1, "user").expect("Order1->User1 not found");
    assert_eq!(
        get(&user1, "id"),
        company_a.user_id,
        "Order1->User1 != User1"
    );

    em.get_filters().disable(COMPANY_SQL_FILTER).unwrap();
    em.get_filters()
        .enable(COMPANY_SQL_FILTER)
        .unwrap()
        .set_parameter("company", s(COMPANY_B), None)
        .unwrap();

    let order2 = find_order(&em, &company_b.order_id).await;

    let user2 = get_object(&order2, "user").expect("Order2->User2 not found");
    assert_eq!(
        get(&user2, "id"),
        company_b.user_id,
        "Order2->User2 != User2"
    );
}

#[tokio::test]
async fn testUseChangeFilterParameters() {
    let em = set_up().await;
    add_filter(&em);
    let filter = em.get_filters().enable(COMPANY_SQL_FILTER).unwrap();

    let (company_a, company_b) = prepare_data(&em).await;

    filter.set_parameter("company", s(COMPANY_A), None).unwrap();

    let order1 = find_order(&em, &company_a.order_id).await;

    let user1 = get_object(&order1, "user").expect("Order1->User1 not found");
    assert_eq!(
        get(&user1, "id"),
        company_a.user_id,
        "Order1->User1 != User1"
    );

    filter.set_parameter("company", s(COMPANY_B), None).unwrap();

    let order2 = find_order(&em, &company_b.order_id).await;

    let user2 = get_object(&order2, "user").expect("Order2->User2 not found");
    assert_eq!(
        get(&user2, "id"),
        company_b.user_id,
        "Order2->User2 != User2"
    );
}

/// `$getOrderByIdCache($orderId)`.
async fn get_order_by_id_cache(em: &Arc<EntityManager>, order_id: &Value) -> Option<Object> {
    let mut qb = em.create_query_builder();
    qb.select(vec![s("orderMaster, user")])
        .unwrap()
        .from(ORDER, "orderMaster", None)
        .unwrap()
        .inner_join("orderMaster.user", "user", None, None, None)
        .unwrap()
        .r#where(vec![s("orderMaster.id = :orderId")])
        .unwrap()
        .set_parameter("orderId", order_id.clone(), None)
        .set_cacheable(true);
    match qb
        .get_query()
        .unwrap()
        .get_one_or_null_result(None)
        .await
        .unwrap()
    {
        Value::Object(order) => Some(order),
        Value::Null => None,
        other => panic!("not an order: {other:?}"),
    }
}

fn assert_order(order: Option<&Object>, user_id: &Value) {
    let order = order.expect("an order");
    assert_eq!(order.class(), ORDER);
    let user = get_object(order, "user").expect("a user");
    assert_eq!(user.class(), USER);
    assert_eq!(&get(&user, "id"), user_id);
}

#[tokio::test]
async fn testUseQueryBuilder() {
    let em = set_up().await;
    add_filter(&em);
    let filter = em.get_filters().enable(COMPANY_SQL_FILTER).unwrap();

    let (company_a, company_b) = prepare_data(&em).await;

    filter.set_parameter("company", s(COMPANY_A), None).unwrap();

    let order = get_order_by_id_cache(&em, &company_b.order_id).await;
    assert!(order.is_none());

    let order = get_order_by_id_cache(&em, &company_a.order_id).await;

    assert_order(order.as_ref(), &company_a.user_id);

    filter.set_parameter("company", s(COMPANY_B), None).unwrap();

    let order = get_order_by_id_cache(&em, &company_a.order_id).await;
    assert!(order.is_none());

    let order = get_order_by_id_cache(&em, &company_b.order_id).await;

    assert_order(order.as_ref(), &company_b.user_id);
}
