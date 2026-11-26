//! Functional/Ticket/SwitchContextWithFilterAndIndexedRelation/ChangeFiltersTest.php
//! on `SQLite` (`tests/common/orm.rs`).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/SwitchContextWithFilterAndIndexedRelation/ChangeFiltersTest.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/SwitchContextWithFilterAndIndexedRelation/Company.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/SwitchContextWithFilterAndIndexedRelation/Category.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/SwitchContextWithFilterAndIndexedRelation/CategoryTypeSQLFilter.php

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

use php_value::{Object, PhpArray, Thrown, Value};
use php2rust_doctrine_orm::collection_object::ArrayCollectionObject;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::mapping::ClassMetadataInfo;
use php2rust_doctrine_orm::query::{FilterConstraint, SqlFilter, register_filter_class};

use orm::{count_of, elements, get, new_object, s, set};

const COMPANY: &str =
    "Doctrine\\Tests\\ORM\\Functional\\Ticket\\SwitchContextWithFilterAndIndexedRelation\\Company";
const CATEGORY: &str =
    "Doctrine\\Tests\\ORM\\Functional\\Ticket\\SwitchContextWithFilterAndIndexedRelation\\Category";
const CATEGORY_TYPE_SQL_FILTER: &str = "Doctrine\\Tests\\ORM\\Functional\\Ticket\\SwitchContextWithFilterAndIndexedRelation\\CategoryTypeSQLFilter";

const COMPANY_A: &str = "A";
const CAT_BAR: &str = "bar";
const CAT_FOO: &str = "foo";

/// `CategoryTypeSQLFilter extends SQLFilter`.
struct CategoryTypeSqlFilter;

impl FilterConstraint for CategoryTypeSqlFilter {
    fn add_filter_constraint(
        &self,
        filter: &SqlFilter,
        target_entity: &ClassMetadataInfo,
        target_table_alias: &str,
    ) -> Result<String, Thrown> {
        if target_entity.name == CATEGORY {
            let field_name = target_entity
                .field_mappings
                .get("type")
                .map(|mapping| mapping.field_name.clone())
                .unwrap_or_default();
            return Ok(format!(
                "{target_table_alias}.{field_name} = {}",
                filter.get_parameter("type")?
            ));
        }

        Ok(String::new())
    }
}

/// `new Category($name, $type)`.
fn category(name: &str, type_: &str) -> Object {
    let category = new_object(CATEGORY);
    set(&category, "name", s(name));
    set(&category, "type", s(type_));
    category
}

/// `new Company($name, $categories)`.
fn company(name: &str, categories: &[&Object]) -> Object {
    let company = new_object(COMPANY);
    set(&company, "name", s(name));
    let mut elements = PhpArray::new();
    for category in categories {
        elements.push(Value::Object((*category).clone())).unwrap();
    }
    set(
        &company,
        "categories",
        ArrayCollectionObject::value(elements),
    );
    company
}

async fn set_up() -> Arc<EntityManager> {
    register_filter_class(
        CATEGORY_TYPE_SQL_FILTER,
        Arc::new(|em| {
            SqlFilter::new(
                em,
                CATEGORY_TYPE_SQL_FILTER,
                Box::new(CategoryTypeSqlFilter),
            )
        }),
    );
    orm::set_up(&[COMPANY, CATEGORY]).await
}

async fn prepare_data(em: &Arc<EntityManager>) {
    let cat1 = category("cat1", CAT_FOO);
    let cat2 = category("cat2", CAT_BAR);
    let company_a = company(COMPANY_A, &[&cat1, &cat2]);

    em.persist(&cat1).unwrap();
    em.persist(&cat2).unwrap();
    em.persist(&company_a).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();
}

/// `$company->categories->map(fn (Category $c) => $c->type)->getValues()`.
async fn types(company: &Object) -> Vec<Value> {
    elements(company, "categories")
        .await
        .iter()
        .map(|c| get(c, "type"))
        .collect()
}

#[tokio::test]
async fn testIndexAliasUpdatedWithUpdatedFilter() {
    let em = set_up().await;
    prepare_data(&em).await;

    let company = em
        .get_repository(COMPANY)
        .unwrap()
        .find_one_by(&PhpArray::new(), None)
        .await
        .unwrap()
        .expect("a company");

    assert_eq!(count_of(&company, "categories").await, 2);
    assert_eq!(types(&company).await, vec![s(CAT_FOO), s(CAT_BAR)]);

    em.clear().unwrap();
    em.get_configuration()
        .add_filter(CATEGORY_TYPE_SQL_FILTER, CATEGORY_TYPE_SQL_FILTER);
    em.get_filters()
        .enable(CATEGORY_TYPE_SQL_FILTER)
        .unwrap()
        .set_parameter("type", s(CAT_FOO), None)
        .unwrap();

    let company = em
        .get_repository(COMPANY)
        .unwrap()
        .find_one_by(&PhpArray::new(), None)
        .await
        .unwrap()
        .expect("a company");

    assert_eq!(count_of(&company, "categories").await, 1);
    assert_eq!(types(&company).await, vec![s(CAT_FOO)]);
}
