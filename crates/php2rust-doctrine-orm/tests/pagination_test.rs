//! Functional/PaginationTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (`tests/common/orm.rs`). A data provider's rows run in one
//! test, labelled.
//! `testDifferentResultLengthsDoNotRequireExtraQueryCacheEntries` sets the
//! query cache that `OrmFunctionalTestCase` configures (`self::$queryCache`)
//! on its entity manager. `ReflectionMethod($paginator,
//! 'getCountQuery')` is `OffsetPaginator::get_count_query()`.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/PaginationTest.php
//! Upstream: doctrine/orm tests/Tests/DbalTypes/CustomIdObjectType.php tests/Tests/DbalTypes/CustomIdObject.php
//! Upstream: doctrine/orm tests/Tests/Models/Pagination/Company.php tests/Tests/Models/Pagination/Logo.php
//! Upstream: doctrine/orm tests/Tests/Models/Pagination/Department.php tests/Tests/Models/Pagination/User1.php
//! Upstream: doctrine/orm tests/Tests/Models/CustomType/CustomIdObjectTypeParent.php

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

#[path = "common/company.rs"]
mod company;
#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use php_value::{Object, PhpArray, PhpString, Thrown, Value};
use php2rust_doctrine_dbal::Exception;
use php2rust_doctrine_dbal::platforms::AbstractPlatform;
use php2rust_doctrine_dbal::types::{ConversionError, Type, TypeArgument, add_type, has_type};
use php2rust_doctrine_orm::cache::pool::ArrayAdapter;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::query::ast::{
    ComparisonExpression, ConditionalPrimary, DeleteStatement, LITERAL_STRING, Literal, Node,
    PathExpression, SelectStatement, TYPE_STATE_FIELD, UpdateStatement, WhereClause,
};
use php2rust_doctrine_orm::query::context::{
    DqlQuery, HINT_CUSTOM_OUTPUT_WALKER, HINT_CUSTOM_TREE_WALKERS, Hint, QueryComponents,
    TreeWalkerClass,
};
use php2rust_doctrine_orm::query::{
    ParserResult, Query, SqlOutputWalker, TreeWalker, TreeWalkerAdapter,
};
use php2rust_doctrine_orm::tools::pagination::{OffsetPaginator, Page, Window, WindowPage};

use orm::{CMS_ARTICLE, CMS_GROUP, CMS_MODELS, cms_email, cms_group, cms_user_with, get, s, set};

const COMPANY: &str = "Doctrine\\Tests\\Models\\Pagination\\Company";
const DEPARTMENT: &str = "Doctrine\\Tests\\Models\\Pagination\\Department";
const LOGO: &str = "Doctrine\\Tests\\Models\\Pagination\\Logo";
const PAGINATION_USER: &str = "Doctrine\\Tests\\Models\\Pagination\\User";
const USER1: &str = "Doctrine\\Tests\\Models\\Pagination\\User1";
const CUSTOM_ID_OBJECT: &str = "Doctrine\\Tests\\DbalTypes\\CustomIdObject";
const CUSTOM_ID_OBJECT_TYPE_PARENT: &str =
    "Doctrine\\Tests\\Models\\CustomType\\CustomIdObjectTypeParent";
const CUSTOM_ID_OBJECT_TYPE_CHILD: &str =
    "Doctrine\\Tests\\Models\\CustomType\\CustomIdObjectTypeChild";

/// Page size holding every row of the fixtures, for the single page scenarios.
const WHOLE_RESULT_SET: i64 = 9;

/// `CustomIdObjectType`.
#[derive(Debug)]
struct CustomIdObjectType;

impl CustomIdObjectType {
    const NAME: &'static str = "CustomIdObject";
}

impl Type for CustomIdObjectType {
    fn class(&self) -> &'static str {
        "Doctrine\\Tests\\DbalTypes\\CustomIdObjectType"
    }

    /// `$value->id`.
    fn convert_to_database_value(
        &self,
        value: &Value,
        _platform: &dyn AbstractPlatform,
    ) -> Result<Value, ConversionError> {
        Ok(value.as_object().map_or(Value::Null, |id| get(id, "id")))
    }

    /// `new CustomIdObject($value)`.
    fn convert_to_php_value(
        &self,
        value: &Value,
        _platform: &dyn AbstractPlatform,
    ) -> Result<Value, ConversionError> {
        let text = value.to_php_string().unwrap_or_default();
        Ok(Value::Object(custom_id_object(&String::from_utf8_lossy(
            text.as_bytes(),
        ))))
    }

    fn get_sql_declaration(
        &self,
        column: &PhpArray,
        platform: &dyn AbstractPlatform,
    ) -> Result<PhpString, Exception> {
        platform.get_string_type_declaration_sql(column)
    }
}

/// `new CustomIdObject($id)`.
fn custom_id_object(id: &str) -> Object {
    let object = orm::new_object(CUSTOM_ID_OBJECT);
    set(&object, "id", s(id));
    object
}

/// `setUp()`: the cms, pagination, company and `custom_id_object_type` model sets,
/// the `CustomIdObject` type and `populate()`.
async fn set_up() -> Arc<EntityManager> {
    if !has_type(CustomIdObjectType::NAME) {
        // Another test thread may have added it meanwhile.
        let _ = add_type(
            CustomIdObjectType::NAME,
            TypeArgument::Instance(Arc::new(CustomIdObjectType)),
        );
    }

    let mut classes: Vec<&str> = CMS_MODELS.to_vec();
    classes.extend([
        COMPANY,
        DEPARTMENT,
        LOGO,
        PAGINATION_USER,
        USER1,
        CUSTOM_ID_OBJECT_TYPE_PARENT,
        CUSTOM_ID_OBJECT_TYPE_CHILD,
    ]);
    classes.extend(company::COMPANY_MODELS);
    company::register_listeners();
    let em = orm::set_up(&classes).await;
    populate(&em).await;
    em
}

async fn populate(em: &Arc<EntityManager>) {
    let mut groups = Vec::new();
    for j in 0..3 {
        let group = cms_group(&format!("group{j}"));
        em.persist(&group).unwrap();
        groups.push(group);
    }

    for i in 0..9 {
        let user = cms_user_with(&format!("Name{i}"), &format!("username{i}"), "active");
        let email = cms_email(&format!("email{i}"));
        set(&user, "email", Value::Object(email.clone()));
        set(&email, "user", Value::Object(user.clone()));
        for group in &groups {
            orm::add_group(&user, group);
        }

        em.persist(&user).unwrap();
        for j in 0..=i {
            let article = orm::cms_article(&format!("topic{i}{j}"), &format!("text{i}{j}"));
            // `setAuthor($user)`
            set(&article, "user", Value::Object(user.clone()));
            set(&article, "version", Value::Int(0));
            em.persist(&article).unwrap();
        }
    }

    for i in 0..9 {
        let company = orm::new_object(COMPANY);
        set(&company, "name", s(&format!("name{i}")));
        let logo = orm::new_object(LOGO);
        set(&logo, "image", s(&format!("image{i}")));
        set(&logo, "imageWidth", Value::Int(100 + i));
        set(&logo, "imageHeight", Value::Int(100 + i));
        set(&logo, "company", Value::Object(company.clone()));
        set(&company, "logo", Value::Object(logo));
        let mut departments = Vec::new();
        for j in 0..3 {
            let department = orm::new_object(DEPARTMENT);
            set(&department, "name", s(&format!("name{i}{j}")));
            set(&department, "company", Value::Object(company.clone()));
            departments.push(Value::Object(department));
        }
        // `$company->departments[] = $department` on `null`: an array.
        set(
            &company,
            "departments",
            Value::Array(PhpArray::list(departments)),
        );

        em.persist(&company).unwrap();
    }

    for i in 0..9 {
        let user = orm::new_object(USER1);
        set(&user, "name", s(&format!("name{i}")));
        set(&user, "email", s(&format!("email{i}")));
        em.persist(&user).unwrap();
    }

    let manager = company::company_manager("Roman B.", 100_000, "IT", "Foo");

    em.persist(&manager).unwrap();

    em.flush().await.unwrap();
}

/// `[[true], [false]]`.
const USE_OUTPUT_WALKERS: [bool; 2] = [true, false];
/// `[[true], [false]]`.
const FETCH_JOIN_COLLECTION: [bool; 2] = [true, false];
/// `[[true, false], [true, true], [false, false], [false, true]]`.
const USE_OUTPUT_WALKERS_AND_FETCH_JOIN_COLLECTION: [(bool, bool); 4] =
    [(true, false), (true, true), (false, false), (false, true)];

fn window(first_result: i64, max_results: i64) -> Window {
    Window::new(first_result, max_results).unwrap()
}

async fn paginate(
    paginator: OffsetPaginator,
    query: &Query,
    window: Window,
) -> Result<WindowPage, Thrown> {
    paginator.paginate_window(query.into(), window).await
}

fn message(e: &Thrown) -> String {
    orm::message(e)
}

#[tokio::test]
async fn testCountSimpleWithoutJoin() {
    for use_output_walkers in USE_OUTPUT_WALKERS {
        let em = set_up().await;
        let dql = "SELECT u FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u";
        let query = em.create_query(dql);

        let page = paginate(
            OffsetPaginator::new(true, Some(use_output_walkers)),
            &query,
            window(0, WHOLE_RESULT_SET),
        )
        .await
        .unwrap();
        assert_eq!(9, page.get_total_count(), "{use_output_walkers}");
    }
}

#[tokio::test]
async fn testCountWithFetchJoin() {
    for use_output_walkers in USE_OUTPUT_WALKERS {
        let em = set_up().await;
        let dql = "SELECT u,g FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u JOIN u.groups g";
        let query = em.create_query(dql);

        let page = paginate(
            OffsetPaginator::new(true, Some(use_output_walkers)),
            &query,
            window(0, WHOLE_RESULT_SET),
        )
        .await
        .unwrap();
        assert_eq!(9, page.get_total_count(), "{use_output_walkers}");
    }
}

#[tokio::test]
async fn testOffsetPaginatorReturnsFirstPage() {
    let em = set_up().await;
    let dql = "SELECT u FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u ORDER BY u.id ASC";
    let query = em.create_query(dql);

    let page = paginate(OffsetPaginator::new(true, None), &query, window(0, 4))
        .await
        .unwrap();

    assert_eq!(4, page.count());
    assert_eq!(9, page.get_total_count());
    assert!(!page.has_previous_page());
    assert!(page.has_next_page());
    assert!(page.has_to_paginate());
    assert_eq!(1, page.get_page_number());
    assert_eq!(3, page.get_page_count());
    assert_eq!(4, page.get_next_window().unwrap().get_first_result());
    assert_eq!(8, page.get_last_window().unwrap().get_first_result());
}

#[tokio::test]
async fn testOffsetPaginatorReturnsLastPage() {
    let em = set_up().await;
    let dql = "SELECT u FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u ORDER BY u.id ASC";
    let query = em.create_query(dql);

    let page = paginate(OffsetPaginator::new(true, None), &query, window(8, 4))
        .await
        .unwrap();

    assert_eq!(1, page.count());
    assert_eq!(9, page.get_total_count());
    assert!(page.has_previous_page());
    assert!(!page.has_next_page());
    assert_eq!(4, page.get_previous_window().unwrap().get_first_result());
    assert_eq!(8, page.get_last_window().unwrap().get_first_result());
}

#[tokio::test]
async fn testOffsetPaginatorWithFromPageNumberAndSize() {
    let em = set_up().await;
    let dql = "SELECT u FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u ORDER BY u.id ASC";
    let query = em.create_query(dql);

    let page = paginate(
        OffsetPaginator::default(),
        &query,
        Window::from_page_number_and_size(2, 4).unwrap(),
    )
    .await
    .unwrap();

    assert_eq!(4, page.count());
    assert_eq!(2, page.get_page_number());
    assert!(page.has_previous_page());
    assert!(page.has_next_page());
}

#[tokio::test]
async fn testOffsetPaginatorWithFetchJoin() {
    let em = set_up().await;
    let dql = "SELECT u, g FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u JOIN u.groups g ORDER BY u.id ASC";
    let query = em.create_query(dql);

    let page = paginate(OffsetPaginator::new(true, None), &query, window(0, 4))
        .await
        .unwrap();

    assert_eq!(4, page.count());
    assert_eq!(9, page.get_total_count());
}

#[tokio::test]
async fn testOffsetPaginatorIsReusableAcrossQueriesAndPages() {
    let em = set_up().await;
    let users =
        em.create_query("SELECT u FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u ORDER BY u.id ASC");
    let groups =
        em.create_query("SELECT g FROM Doctrine\\Tests\\Models\\CMS\\CmsGroup g ORDER BY g.id ASC");

    let paginator = OffsetPaginator::default();

    let first_page = paginate(paginator, &users, window(0, 4)).await.unwrap();
    assert_eq!(4, first_page.count());

    let second_page = paginate(paginator, &users, first_page.get_next_window().unwrap())
        .await
        .unwrap();
    assert_eq!(4, second_page.count());
    assert_eq!(2, second_page.get_page_number());

    // the first page is unaffected by the second pagination
    assert_eq!(1, first_page.get_page_number());

    assert_eq!(
        9,
        paginate(paginator, &users, window(0, 4))
            .await
            .unwrap()
            .get_total_count()
    );
    assert_eq!(
        3,
        paginate(paginator, &groups, window(0, 4))
            .await
            .unwrap()
            .get_total_count()
    );
}

#[tokio::test]
async fn testOffsetPaginatorRejectsANonWindowPosition() {
    use php2rust_doctrine_orm::tools::pagination::PaginatorInterface;

    let em = set_up().await;
    let dql = "SELECT u FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u ORDER BY u.id ASC";
    let query = em.create_query(dql);

    let e = OffsetPaginator::default()
        .paginate((&query).into(), &Value::Int(42))
        .await
        .unwrap_err();
    assert_eq!("InvalidArgumentException", e.class);
}

#[tokio::test]
async fn testPageItemsAreAListEvenForIndexByQueries() {
    let em = set_up().await;
    let dql = "SELECT u FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u INDEX BY u.id";
    let query = em.create_query(dql);

    let page = paginate(OffsetPaginator::new(false, None), &query, window(0, 3))
        .await
        .unwrap();

    // `array_keys($page->getItems())`: the items are a list.
    assert_eq!(3, page.get_items().len());
}

#[tokio::test]
async fn testCountComplexWithOutputWalker() {
    let em = set_up().await;
    let dql = "SELECT g, COUNT(u.id) AS userCount FROM Doctrine\\Tests\\Models\\CMS\\CmsGroup g LEFT JOIN g.users u GROUP BY g HAVING COUNT(u.id) > 0";
    let query = em.create_query(dql);

    let page = paginate(
        OffsetPaginator::new(true, Some(true)),
        &query,
        window(0, WHOLE_RESULT_SET),
    )
    .await
    .unwrap();
    assert_eq!(3, page.get_total_count());
}

#[tokio::test]
async fn testCountComplexWithoutOutputWalker() {
    let em = set_up().await;
    let dql = "SELECT g, COUNT(u.id) AS userCount FROM Doctrine\\Tests\\Models\\CMS\\CmsGroup g LEFT JOIN g.users u GROUP BY g HAVING COUNT(u.id) > 0";
    let query = em.create_query(dql);

    let e = paginate(
        OffsetPaginator::new(false, Some(false)),
        &query,
        window(0, WHOLE_RESULT_SET),
    )
    .await
    .unwrap_err();
    assert_eq!("RuntimeException", e.class);
    assert_eq!(
        "Cannot count query that uses a HAVING clause. Use the output walkers for pagination",
        message(&e)
    );
}

#[tokio::test]
async fn testCountWithComplexScalarOrderBy() {
    for use_output_walkers in USE_OUTPUT_WALKERS {
        let em = set_up().await;
        let dql = "SELECT l FROM Doctrine\\Tests\\Models\\Pagination\\Logo l ORDER BY l.imageWidth * l.imageHeight DESC";
        let query = em.create_query(dql);

        // no collection is joined, and without output walkers the identifier
        // subquery would be a SELECT DISTINCT ordered by an expression that is
        // not in its select list, which MySQL and PostgreSQL reject
        let page = paginate(
            OffsetPaginator::new(false, Some(use_output_walkers)),
            &query,
            window(0, WHOLE_RESULT_SET),
        )
        .await
        .unwrap();
        assert_eq!(9, page.get_total_count(), "{use_output_walkers}");
    }
}

#[tokio::test]
async fn testIterateSimpleWithoutJoin() {
    for (use_output_walkers, fetch_join_collection) in USE_OUTPUT_WALKERS_AND_FETCH_JOIN_COLLECTION
    {
        let label = format!("{use_output_walkers}, {fetch_join_collection}");
        let em = set_up().await;
        let dql = "SELECT u FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u";
        let query = em.create_query(dql);

        let paginator = OffsetPaginator::new(fetch_join_collection, Some(use_output_walkers));
        assert_eq!(
            9,
            paginate(paginator, &query, window(0, WHOLE_RESULT_SET))
                .await
                .unwrap()
                .count(),
            "{label}"
        );

        // Test with limit
        assert_eq!(
            3,
            paginate(paginator, &query, window(0, 3))
                .await
                .unwrap()
                .count(),
            "{label}"
        );

        // Test with limit and offset
        assert_eq!(
            3,
            paginate(paginator, &query, window(4, 3))
                .await
                .unwrap()
                .count(),
            "{label}"
        );
    }
}

/// The ascending and descending orders a test checks, with their
/// windows, counts and first items (`iterateWithOrderAsc()` ...).
#[derive(Clone, Copy)]
enum Order {
    Asc,
    AscWithLimit,
    AscWithLimitAndOffset,
    Desc,
    DescWithLimit,
    DescWithLimitAndOffset,
}

impl Order {
    fn direction(self) -> &'static str {
        match self {
            Self::Asc | Self::AscWithLimit | Self::AscWithLimitAndOffset => " ASC",
            Self::Desc | Self::DescWithLimit | Self::DescWithLimitAndOffset => " DESC",
        }
    }

    fn window(self) -> Window {
        match self {
            Self::Asc | Self::Desc => window(0, WHOLE_RESULT_SET),
            Self::AscWithLimit | Self::DescWithLimit => window(0, 3),
            Self::AscWithLimitAndOffset | Self::DescWithLimitAndOffset => window(3, 3),
        }
    }

    fn expected_count(self) -> usize {
        match self {
            Self::Asc | Self::Desc => 9,
            _ => 3,
        }
    }

    fn expected_first(self) -> &'static str {
        match self {
            Self::Asc | Self::AscWithLimit => "0",
            Self::AscWithLimitAndOffset => "3",
            Self::Desc | Self::DescWithLimit => "8",
            Self::DescWithLimitAndOffset => "5",
        }
    }
}

/// `iterateWithOrder*($useOutputWalkers, $fetchJoinCollection, $baseDql, $checkField)`.
async fn iterate_with_order(
    em: &Arc<EntityManager>,
    use_output_walkers: bool,
    fetch_join_collection: bool,
    base_dql: &str,
    check_field: &str,
    order: Order,
) -> Result<(), Thrown> {
    let dql = format!("{base_dql}{}", order.direction());
    let query = em.create_query(&dql);

    let page = paginate(
        OffsetPaginator::new(fetch_join_collection, Some(use_output_walkers)),
        &query,
        order.window(),
    )
    .await?;

    let label = format!("{dql} ({use_output_walkers}, {fetch_join_collection})");
    assert_eq!(order.expected_count(), page.count(), "{label}");
    let first = page.get_items()[0].as_object().expect("an entity").clone();
    assert!(
        php_value::compare::loose_equals(
            &get(&first, check_field),
            &s(&format!("{check_field}{}", order.expected_first())),
        ),
        "{label}: {:?}",
        get(&first, check_field)
    );
    Ok(())
}

/// Runs the orders on a new database for each provider row.
async fn iterate_with_orders(
    rows: &[(bool, bool)],
    base_dql: &str,
    check_field: &str,
    orders: &[Order],
) {
    for &(use_output_walkers, fetch_join_collection) in rows {
        let em = set_up().await;
        for &order in orders {
            iterate_with_order(
                &em,
                use_output_walkers,
                fetch_join_collection,
                base_dql,
                check_field,
                order,
            )
            .await
            .unwrap();
        }
    }
}

fn output_walkers_rows(fetch_join_collection: bool) -> Vec<(bool, bool)> {
    USE_OUTPUT_WALKERS
        .iter()
        .map(|&w| (w, fetch_join_collection))
        .collect()
}

fn fetch_join_collection_rows(use_output_walkers: bool) -> Vec<(bool, bool)> {
    FETCH_JOIN_COLLECTION
        .iter()
        .map(|&f| (use_output_walkers, f))
        .collect()
}

const PLAIN: [Order; 2] = [Order::Asc, Order::Desc];
const WITH_LIMIT: [Order; 2] = [Order::AscWithLimit, Order::DescWithLimit];
const WITH_LIMIT_AND_OFFSET: [Order; 2] =
    [Order::AscWithLimitAndOffset, Order::DescWithLimitAndOffset];

#[tokio::test]
async fn testIterateSimpleWithoutJoinWithOrder() {
    // Ascending
    let dql = "SELECT u FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u ORDER BY u.username";
    iterate_with_orders(
        &USE_OUTPUT_WALKERS_AND_FETCH_JOIN_COLLECTION,
        dql,
        "username",
        &PLAIN,
    )
    .await;
}

#[tokio::test]
async fn testIterateSimpleWithoutJoinWithOrderAndLimit() {
    // Ascending
    let dql = "SELECT u FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u ORDER BY u.username";
    iterate_with_orders(
        &USE_OUTPUT_WALKERS_AND_FETCH_JOIN_COLLECTION,
        dql,
        "username",
        &WITH_LIMIT,
    )
    .await;
}

#[tokio::test]
async fn testIterateSimpleWithoutJoinWithOrderAndLimitAndOffset() {
    // Ascending
    let dql = "SELECT u FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u ORDER BY u.username";
    iterate_with_orders(
        &USE_OUTPUT_WALKERS_AND_FETCH_JOIN_COLLECTION,
        dql,
        "username",
        &WITH_LIMIT_AND_OFFSET,
    )
    .await;
}

#[tokio::test]
async fn testIterateSimpleWithOutputWalkerWithoutJoinWithComplexOrder() {
    // Ascending
    let dql = "SELECT l FROM Doctrine\\Tests\\Models\\Pagination\\Logo l ORDER BY l.imageWidth * l.imageHeight";
    iterate_with_orders(&fetch_join_collection_rows(true), dql, "image", &PLAIN).await;
}

#[tokio::test]
async fn testIterateSimpleWithOutputWalkerWithoutJoinWithComplexOrderAndLimit() {
    // Ascending
    let dql = "SELECT l FROM Doctrine\\Tests\\Models\\Pagination\\Logo l ORDER BY l.imageWidth * l.imageHeight";
    iterate_with_orders(&fetch_join_collection_rows(true), dql, "image", &WITH_LIMIT).await;
}

#[tokio::test]
async fn testIterateSimpleWithOutputWalkerWithoutJoinWithComplexOrderAndLimitAndOffset() {
    // Ascending
    let dql = "SELECT l FROM Doctrine\\Tests\\Models\\Pagination\\Logo l ORDER BY l.imageWidth * l.imageHeight";
    iterate_with_orders(
        &fetch_join_collection_rows(true),
        dql,
        "image",
        &WITH_LIMIT_AND_OFFSET,
    )
    .await;
}

#[tokio::test]
async fn testIterateWithFetchJoin() {
    for use_output_walkers in USE_OUTPUT_WALKERS {
        let em = set_up().await;
        let dql = "SELECT u,g FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u JOIN u.groups g";
        let query = em.create_query(dql);

        let page = paginate(
            OffsetPaginator::new(true, Some(use_output_walkers)),
            &query,
            window(0, WHOLE_RESULT_SET),
        )
        .await
        .unwrap();
        assert_eq!(9, page.count(), "{use_output_walkers}");
    }
}

#[tokio::test]
async fn testIterateWithFetchJoinWithOrder() {
    let dql = "SELECT u,g FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u JOIN u.groups g ORDER BY u.username";
    iterate_with_orders(&output_walkers_rows(true), dql, "username", &PLAIN).await;
}

#[tokio::test]
async fn testIterateWithFetchJoinWithOrderAndLimit() {
    let dql = "SELECT u,g FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u JOIN u.groups g ORDER BY u.username";
    iterate_with_orders(&output_walkers_rows(true), dql, "username", &WITH_LIMIT).await;
}

#[tokio::test]
async fn testIterateWithFetchJoinWithOrderAndLimitAndOffset() {
    let dql = "SELECT u,g FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u JOIN u.groups g ORDER BY u.username";
    iterate_with_orders(
        &output_walkers_rows(true),
        dql,
        "username",
        &WITH_LIMIT_AND_OFFSET,
    )
    .await;
}

#[tokio::test]
async fn testIterateWithRegularJoinWithOrderByColumnFromJoined() {
    let dql =
        "SELECT u FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u JOIN u.email e ORDER BY e.email";
    iterate_with_orders(
        &USE_OUTPUT_WALKERS_AND_FETCH_JOIN_COLLECTION,
        dql,
        "username",
        &PLAIN,
    )
    .await;
}

#[tokio::test]
async fn testIterateWithRegularJoinWithOrderByColumnFromJoinedWithLimit() {
    let dql =
        "SELECT u FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u JOIN u.email e ORDER BY e.email";
    iterate_with_orders(
        &USE_OUTPUT_WALKERS_AND_FETCH_JOIN_COLLECTION,
        dql,
        "username",
        &WITH_LIMIT,
    )
    .await;
}

#[tokio::test]
async fn testIterateWithRegularJoinWithOrderByColumnFromJoinedWithLimitAndOffset() {
    let dql =
        "SELECT u FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u JOIN u.email e ORDER BY e.email";
    iterate_with_orders(
        &USE_OUTPUT_WALKERS_AND_FETCH_JOIN_COLLECTION,
        dql,
        "username",
        &WITH_LIMIT_AND_OFFSET,
    )
    .await;
}

#[tokio::test]
async fn testIterateWithOutputWalkersWithRegularJoinWithComplexOrderByReferencingJoined() {
    // long function name is loooooooooooong

    let dql = "SELECT c FROM Doctrine\\Tests\\Models\\Pagination\\Company c JOIN c.logo l ORDER BY l.imageHeight * l.imageWidth";
    iterate_with_orders(&fetch_join_collection_rows(true), dql, "name", &PLAIN).await;
}

#[tokio::test]
async fn testIterateWithOutputWalkersWithRegularJoinWithComplexOrderByReferencingJoinedWithLimit() {
    // long function name is loooooooooooong

    let dql = "SELECT c FROM Doctrine\\Tests\\Models\\Pagination\\Company c JOIN c.logo l ORDER BY l.imageHeight * l.imageWidth";
    iterate_with_orders(&fetch_join_collection_rows(true), dql, "name", &WITH_LIMIT).await;
}

#[tokio::test]
async fn testIterateWithOutputWalkersWithRegularJoinWithComplexOrderByReferencingJoinedWithLimitAndOffset()
 {
    // long function name is loooooooooooong

    let dql = "SELECT c FROM Doctrine\\Tests\\Models\\Pagination\\Company c JOIN c.logo l ORDER BY l.imageHeight * l.imageWidth";
    iterate_with_orders(
        &fetch_join_collection_rows(true),
        dql,
        "name",
        &WITH_LIMIT_AND_OFFSET,
    )
    .await;
}

#[tokio::test]
async fn testIterateWithFetchJoinWithOrderByColumnFromJoined() {
    let dql = "SELECT u,g,e FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u JOIN u.groups g JOIN u.email e ORDER BY e.email";
    iterate_with_orders(&output_walkers_rows(true), dql, "username", &PLAIN).await;
}

#[tokio::test]
async fn testIterateWithFetchJoinWithOrderByColumnFromJoinedWithLimit() {
    let dql = "SELECT u,g,e FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u JOIN u.groups g JOIN u.email e ORDER BY e.email";
    iterate_with_orders(&output_walkers_rows(true), dql, "username", &WITH_LIMIT).await;
}

#[tokio::test]
async fn testIterateWithFetchJoinWithOrderByColumnFromJoinedWithLimitAndOffset() {
    let dql = "SELECT u,g,e FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u JOIN u.groups g JOIN u.email e ORDER BY e.email";
    iterate_with_orders(
        &output_walkers_rows(true),
        dql,
        "username",
        &WITH_LIMIT_AND_OFFSET,
    )
    .await;
}

#[tokio::test]
async fn testIterateWithOutputWalkersWithFetchJoinWithComplexOrderByReferencingJoined() {
    let dql = "SELECT c,l FROM Doctrine\\Tests\\Models\\Pagination\\Company c JOIN c.logo l ORDER BY l.imageWidth * l.imageHeight";
    iterate_with_orders(&fetch_join_collection_rows(true), dql, "name", &PLAIN).await;
}

#[tokio::test]
async fn testIterateWithOutputWalkersWithFetchJoinWithComplexOrderByReferencingJoinedWithLimit() {
    let dql = "SELECT c,l FROM Doctrine\\Tests\\Models\\Pagination\\Company c JOIN c.logo l ORDER BY l.imageWidth * l.imageHeight";
    iterate_with_orders(&fetch_join_collection_rows(true), dql, "name", &WITH_LIMIT).await;
}

#[tokio::test]
async fn testIterateWithOutputWalkersWithFetchJoinWithComplexOrderByReferencingJoinedWithLimitAndOffset()
 {
    let dql = "SELECT c,l FROM Doctrine\\Tests\\Models\\Pagination\\Company c JOIN c.logo l ORDER BY l.imageWidth * l.imageHeight";
    iterate_with_orders(
        &fetch_join_collection_rows(true),
        dql,
        "name",
        &WITH_LIMIT_AND_OFFSET,
    )
    .await;
}

#[tokio::test]
async fn testIterateWithOutputWalkersWithFetchJoinWithComplexOrderByReferencingJoinedWithLimitAndOffsetWithInheritanceType()
 {
    let dql = "SELECT u FROM Doctrine\\Tests\\Models\\Pagination\\User u ORDER BY u.id";
    iterate_with_orders(&fetch_join_collection_rows(true), dql, "name", &WITH_LIMIT).await;
}

#[tokio::test]
async fn testIterateComplexWithOutputWalker() {
    let em = set_up().await;
    let dql = "SELECT g, COUNT(u.id) AS userCount FROM Doctrine\\Tests\\Models\\CMS\\CmsGroup g LEFT JOIN g.users u GROUP BY g HAVING COUNT(u.id) > 0";
    let query = em.create_query(dql);

    let page = paginate(
        OffsetPaginator::new(true, Some(true)),
        &query,
        window(0, WHOLE_RESULT_SET),
    )
    .await
    .unwrap();
    assert_eq!(3, page.count());
}

#[tokio::test]
async fn testJoinedClassTableInheritance() {
    let em = set_up().await;
    let dql =
        "SELECT c FROM Doctrine\\Tests\\Models\\Company\\CompanyManager c ORDER BY c.startDate";
    let query = em.create_query(dql);

    let page = paginate(
        OffsetPaginator::new(true, None),
        &query,
        window(0, WHOLE_RESULT_SET),
    )
    .await
    .unwrap();
    assert_eq!(1, page.count());
}

#[tokio::test]
async fn testIterateWithFetchJoinOneToManyWithOrderByColumnFromBoth() {
    // without output walkers, ordering by a column of a fetch joined to-many
    // association is rejected, see the WithLimitWithoutOutputWalker test below
    let em = set_up().await;
    let dql = "SELECT c, d FROM Doctrine\\Tests\\Models\\Pagination\\Company c JOIN c.departments d ORDER BY c.name";
    let dql_asc = format!("{dql} ASC, d.name");
    let dql_desc = format!("{dql} DESC, d.name");
    iterate_with_order_dql(&em, true, true, &dql_asc, "name", Order::Asc)
        .await
        .unwrap();
    iterate_with_order_dql(&em, true, true, &dql_desc, "name", Order::Desc)
        .await
        .unwrap();
}

/// `iterateWithOrder()` with the whole DQL (the direction is in it).
async fn iterate_with_order_dql(
    em: &Arc<EntityManager>,
    use_output_walkers: bool,
    fetch_join_collection: bool,
    dql: &str,
    check_field: &str,
    order: Order,
) -> Result<(), Thrown> {
    let query = em.create_query(dql);

    let page = paginate(
        OffsetPaginator::new(fetch_join_collection, Some(use_output_walkers)),
        &query,
        order.window(),
    )
    .await?;

    assert_eq!(order.expected_count(), page.count(), "{dql}");
    let first = page.get_items()[0].as_object().expect("an entity").clone();
    assert!(
        php_value::compare::loose_equals(
            &get(&first, check_field),
            &s(&format!("{check_field}{}", order.expected_first())),
        ),
        "{dql}: {:?}",
        get(&first, check_field)
    );
    Ok(())
}

#[tokio::test]
async fn testIterateWithFetchJoinOneToManyWithOrderByColumnFromBothWithLimitWithOutputWalker() {
    let em = set_up().await;
    let dql = "SELECT c, d FROM Doctrine\\Tests\\Models\\Pagination\\Company c JOIN c.departments d ORDER BY c.name";
    let dql_asc = format!("{dql} ASC, d.name");
    let dql_desc = format!("{dql} DESC, d.name");
    iterate_with_order_dql(&em, true, true, &dql_asc, "name", Order::AscWithLimit)
        .await
        .unwrap();
    iterate_with_order_dql(&em, true, true, &dql_desc, "name", Order::DescWithLimit)
        .await
        .unwrap();
}

#[tokio::test]
async fn testIterateWithFetchJoinOneToManyWithOrderByColumnFromBothWithLimitWithoutOutputWalker() {
    let em = set_up().await;
    let dql = "SELECT c, d FROM Doctrine\\Tests\\Models\\Pagination\\Company c JOIN c.departments d ORDER BY c.name";
    let dql_asc = format!("{dql} ASC, d.name");
    let e = iterate_with_order_dql(&em, false, true, &dql_asc, "name", Order::AscWithLimit)
        .await
        .unwrap_err();
    assert_eq!("RuntimeException", e.class);
    assert_eq!(
        "Cannot select distinct identifiers from query with LIMIT and ORDER BY on a column from a fetch joined to-many association. Use output walkers.",
        message(&e)
    );
}

#[tokio::test]
async fn testIterateWithFetchJoinOneToManyWithOrderByColumnFromRoot() {
    let dql = "SELECT c, d FROM Doctrine\\Tests\\Models\\Pagination\\Company c JOIN c.departments d ORDER BY c.name";
    iterate_with_orders(&output_walkers_rows(true), dql, "name", &PLAIN).await;
}

#[tokio::test]
async fn testIterateWithFetchJoinOneToManyWithOrderByColumnFromRootWithLimit() {
    let dql = "SELECT c, d FROM Doctrine\\Tests\\Models\\Pagination\\Company c JOIN c.departments d ORDER BY c.name";
    iterate_with_orders(&output_walkers_rows(true), dql, "name", &WITH_LIMIT).await;
}

#[tokio::test]
async fn testIterateWithFetchJoinOneToManyWithOrderByColumnFromRootWithLimitAndOffset() {
    let dql = "SELECT c, d FROM Doctrine\\Tests\\Models\\Pagination\\Company c JOIN c.departments d ORDER BY c.name";
    iterate_with_orders(
        &output_walkers_rows(true),
        dql,
        "name",
        &WITH_LIMIT_AND_OFFSET,
    )
    .await;
}

#[tokio::test]
async fn testIterateWithFetchJoinOneToManyWithOrderByColumnFromJoined() {
    // without output walkers, ordering by a column of a fetch joined to-many
    // association is rejected, see the WithLimitWithoutOutputWalker test below
    let dql = "SELECT c, d FROM Doctrine\\Tests\\Models\\Pagination\\Company c JOIN c.departments d ORDER BY d.name";
    iterate_with_orders(&[(true, true)], dql, "name", &PLAIN).await;
}

#[tokio::test]
async fn testIterateWithFetchJoinOneToManyWithOrderByColumnFromJoinedWithLimitWithOutputWalker() {
    let dql = "SELECT c, d FROM Doctrine\\Tests\\Models\\Pagination\\Company c JOIN c.departments d ORDER BY d.name";
    iterate_with_orders(&[(true, true)], dql, "name", &WITH_LIMIT).await;
}

#[tokio::test]
async fn testIterateWithFetchJoinOneToManyWithOrderByColumnFromJoinedWithLimitWithoutOutputWalker()
{
    let em = set_up().await;
    let dql = "SELECT c, d FROM Doctrine\\Tests\\Models\\Pagination\\Company c JOIN c.departments d ORDER BY d.name";

    let e = iterate_with_order(&em, false, true, dql, "name", Order::AscWithLimit)
        .await
        .unwrap_err();
    assert_eq!("RuntimeException", e.class);
    assert_eq!(
        "Cannot select distinct identifiers from query with LIMIT and ORDER BY on a column from a fetch joined to-many association. Use output walkers.",
        message(&e)
    );
}

#[tokio::test]
async fn testCountWithCountSubqueryInWhereClauseWithOutputWalker() {
    let em = set_up().await;
    let dql = "SELECT u FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u WHERE ((SELECT COUNT(s.id) FROM Doctrine\\Tests\\Models\\CMS\\CmsUser s) = 9) ORDER BY u.id desc";
    let query = em.create_query(dql);

    let page = paginate(
        OffsetPaginator::new(true, Some(true)),
        &query,
        window(0, WHOLE_RESULT_SET),
    )
    .await
    .unwrap();
    assert_eq!(9, page.get_total_count());
}

#[tokio::test]
async fn testIterateWithCountSubqueryInWhereClause() {
    let em = set_up().await;
    let dql = "SELECT u FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u WHERE ((SELECT COUNT(s.id) FROM Doctrine\\Tests\\Models\\CMS\\CmsUser s) = 9) ORDER BY u.id desc";
    let query = em.create_query(dql);

    let page = paginate(
        OffsetPaginator::new(true, Some(true)),
        &query,
        window(0, WHOLE_RESULT_SET),
    )
    .await
    .unwrap();
    let users = page.get_items();

    assert_eq!(9, users.len());
    for (i, user) in users.iter().enumerate() {
        let user = user.as_object().expect("a CmsUser");
        assert!(php_value::compare::loose_equals(
            &get(user, "username"),
            &s(&format!("username{}", 8 - i)),
        ));
    }
}

#[tokio::test]
async fn testDetectOutputWalker() {
    let em = set_up().await;
    // This query works using the output walkers but causes an exception using the TreeWalker
    let dql = "SELECT g, COUNT(u.id) AS userCount FROM Doctrine\\Tests\\Models\\CMS\\CmsGroup g LEFT JOIN g.users u GROUP BY g HAVING COUNT(u.id) > 0";
    let query = em.create_query(dql);

    // If the paginator detects the custom output walker it should fall back to using the
    // Tree walkers for pagination, which leads to an exception. If the query works, the output walkers were used
    query.set_hint(
        HINT_CUSTOM_OUTPUT_WALKER,
        Hint::OutputWalker(SqlOutputWalker::CLASS),
    );

    let e = paginate(
        OffsetPaginator::new(false, None),
        &query,
        window(0, WHOLE_RESULT_SET),
    )
    .await
    .unwrap_err();
    assert_eq!("RuntimeException", e.class);
    assert_eq!(
        "Cannot count query that uses a HAVING clause. Use the output walkers for pagination",
        message(&e)
    );
}

/// Test using a paginator when the entity attribute name and corresponding column name are not the same.
#[tokio::test]
async fn testPaginationWithColumnAttributeNameDifference() {
    let em = set_up().await;
    let dql = "SELECT c FROM Doctrine\\Tests\\Models\\Pagination\\Company c ORDER BY c.id";
    let query = em.create_query(dql);

    let page = paginate(
        OffsetPaginator::default(),
        &query,
        window(0, WHOLE_RESULT_SET),
    )
    .await
    .unwrap();

    assert_eq!(9, page.count());
}

#[tokio::test]
async fn testCloneQuery() {
    let em = set_up().await;
    let dql = "SELECT u FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u";
    let query = em.create_query(dql);

    paginate(
        OffsetPaginator::default(),
        &query,
        window(0, WHOLE_RESULT_SET),
    )
    .await
    .unwrap();

    assert!(query.get_parameters().is_empty());
}

#[tokio::test]
async fn testQueryWalkerIsKept() {
    let em = set_up().await;
    let dql = "SELECT u FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u";
    let query = em.create_query(dql);
    query.set_hint(
        HINT_CUSTOM_TREE_WALKERS,
        Hint::TreeWalkers(vec![CustomPaginationTestTreeWalker::CLASS]),
    );

    let page = paginate(
        OffsetPaginator::new(true, Some(false)),
        &query,
        window(0, WHOLE_RESULT_SET),
    )
    .await
    .unwrap();
    assert_eq!(1, page.count());
    assert_eq!(1, page.get_total_count());
}

/// GH-7890
#[tokio::test]
async fn testCustomIdTypeWithoutOutputWalker() {
    let em = set_up().await;
    let parent = orm::new_object(CUSTOM_ID_OBJECT_TYPE_PARENT);
    set(&parent, "id", Value::Object(custom_id_object("foo")));
    set(
        &parent,
        "children",
        php2rust_doctrine_orm::collection_object::ArrayCollectionObject::value(PhpArray::new()),
    );
    em.persist(&parent).unwrap();
    em.flush().await.unwrap();

    let dql = "SELECT p FROM Doctrine\\Tests\\Models\\CustomType\\CustomIdObjectTypeParent p";
    let query = em.create_query(dql);

    let page = paginate(
        OffsetPaginator::new(true, Some(false)),
        &query,
        window(0, 1),
    )
    .await
    .unwrap();
    let matched_items = page.get_items();

    assert_eq!(1, matched_items.len());
    let item = matched_items[0].as_object().expect("an entity");
    assert_eq!(CUSTOM_ID_OBJECT_TYPE_PARENT, item.class());
    // `(string) $matchedItems[0]->id`: `CustomIdObject::__toString()`
    // gives its `id`.
    let id = get(item, "id");
    let id = id.as_object().expect("a CustomIdObject");
    assert!(get(id, "id").identical(&s("foo")));
}

#[tokio::test]
async fn testCountQueryStripsParametersInSelect() {
    let em = set_up().await;
    let query = em.create_query(
        "SELECT u, (CASE WHEN u.id < :vipMaxId THEN 1 ELSE 0 END) AS hidden promotedFirst
            FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u
            WHERE u.id < :id or 1=1",
    );
    query.set_parameter("vipMaxId", 10, None);
    query.set_parameter("id", 100, None);

    let paginator = OffsetPaginator::default();
    let window = window(0, WHOLE_RESULT_SET);

    assert_eq!(
        2,
        paginator
            .get_count_query(&query)
            .unwrap()
            .get_parameters()
            .len()
    );
    assert_eq!(
        9,
        paginate(paginator, &query, window)
            .await
            .unwrap()
            .get_total_count()
    );

    query.set_hint(
        HINT_CUSTOM_OUTPUT_WALKER,
        Hint::OutputWalker(SqlOutputWalker::CLASS),
    );

    // if select part of query is replaced with count(...) paginator should remove
    // parameters from query object not used in new query.
    assert_eq!(
        1,
        paginator
            .get_count_query(&query)
            .unwrap()
            .get_parameters()
            .len()
    );
    assert_eq!(
        9,
        paginate(paginator, &query, window)
            .await
            .unwrap()
            .get_total_count()
    );
}

#[tokio::test]
async fn testPaginationWithSubSelectOrderByExpression() {
    for (use_output_walker, fetch_join_collection) in USE_OUTPUT_WALKERS_AND_FETCH_JOIN_COLLECTION {
        let em = set_up().await;
        let query = em.create_query(
            "SELECT u,
    (
        SELECT MAX(a.version)
        FROM Doctrine\\Tests\\Models\\CMS\\CmsArticle a
        WHERE a.user = u
    ) AS HIDDEN max_version
FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u
ORDER BY max_version DESC",
        );

        let page = paginate(
            OffsetPaginator::new(fetch_join_collection, Some(use_output_walker)),
            &query,
            window(0, WHOLE_RESULT_SET),
        )
        .await
        .unwrap();

        assert_eq!(
            9,
            page.count(),
            "{use_output_walker}, {fetch_join_collection}"
        );
    }
}

/// `CustomPaginationTestTreeWalker`: replaces the WHERE clause with
/// `u.name = 'Name1'`.
struct CustomPaginationTestTreeWalker<'a> {
    adapter: TreeWalkerAdapter<'a>,
}

impl CustomPaginationTestTreeWalker<'_> {
    const CLASS: TreeWalkerClass = TreeWalkerClass {
        name: "Doctrine\\Tests\\ORM\\Functional\\CustomPaginationTestTreeWalker",
        construct: construct_custom_pagination_test_tree_walker,
    };
}

fn construct_custom_pagination_test_tree_walker<'a>(
    query: &'a dyn DqlQuery,
    parser_result: Rc<RefCell<ParserResult>>,
    query_components: QueryComponents,
) -> Box<dyn TreeWalker<'a> + 'a> {
    Box::new(CustomPaginationTestTreeWalker {
        adapter: TreeWalkerAdapter::new(query, parser_result, query_components),
    })
}

impl<'a> TreeWalker<'a> for CustomPaginationTestTreeWalker<'a> {
    fn get_query_components(&self) -> &QueryComponents {
        self.adapter.get_query_components()
    }

    fn walk_select_statement(
        &mut self,
        select_statement: &mut SelectStatement,
    ) -> Result<(), Thrown> {
        let mut path = PathExpression::new(TYPE_STATE_FIELD, "u", Some("name"));
        path.type_ = Some(TYPE_STATE_FIELD);

        let condition = ConditionalPrimary {
            simple_conditional_expression: Some(Node::ComparisonExpression(Box::new(
                ComparisonExpression {
                    left_expression: Node::path(path),
                    operator: "=".to_owned(),
                    right_expression: Node::Literal(Box::new(Literal {
                        type_: LITERAL_STRING,
                        value: Some("Name1".to_owned()),
                    })),
                },
            ))),
            conditional_expression: None,
        };

        select_statement.where_clause = Some(WhereClause {
            conditional_expression: Node::ConditionalPrimary(Box::new(condition)),
        });
        Ok(())
    }

    fn walk_update_statement(&mut self, _: &mut UpdateStatement) -> Result<(), Thrown> {
        Ok(())
    }

    fn walk_delete_statement(&mut self, _: &mut DeleteStatement) -> Result<(), Thrown> {
        Ok(())
    }
}

#[allow(
    dead_code,
    reason = "the CmsGroup and CmsArticle names of the model set"
)]
const _CMS: [&str; 2] = [CMS_GROUP, CMS_ARTICLE];

#[tokio::test]
async fn testDifferentResultLengthsDoNotRequireExtraQueryCacheEntries() {
    let em = set_up().await;
    let query_cache = Arc::new(ArrayAdapter::new());
    em.get_configuration().set_query_cache(query_cache.clone());

    let dql =
        "SELECT u FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u WHERE u.id >= :id ORDER BY u.id";
    let query = em.create_query(dql);

    let window = window(0, 10);

    query.set_parameter("id", 1, None);
    let initial_result = paginate(OffsetPaginator::new(true, None), &query, window)
        .await
        .unwrap()
        .get_items()
        .to_vec(); // exercise the paginator
    assert_eq!(9, initial_result.len());

    let initial_query_count = query_cache.get_values().len();

    query.set_parameter("id", get(&orm::obj(&initial_result[1]), "id"), None); // skip the first result element
    assert_eq!(
        8,
        paginate(OffsetPaginator::new(true, None), &query, window)
            .await
            .unwrap()
            .count()
    ); // exercise the paginator again, with a smaller result set

    let new_count = query_cache.get_values().len();

    assert_eq!(initial_query_count, new_count);
}
