//! Upstream: doctrine/orm tests/Tests/ORM/Functional/QueryTest.php
//!
//! `toIterable()` reads the result whole, so the iterations see every
//! entity of the result hydrated. The query builder test is the query
//! builder's; the partial objects test needs partial objects (lazy ghosts
//! of the selected fields), which the unit of work does not port.

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::similar_names,
    clippy::too_many_lines,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;
#[path = "common/query_log.rs"]
mod query_log;

use php_value::{Object, PhpArray, Thrown, Value};
use php2rust_doctrine_orm::abstract_query::{
    NO_RESULT_EXCEPTION, NON_UNIQUE_RESULT_EXCEPTION, UNEXPECTED_RESULT_EXCEPTION,
};
use php2rust_doctrine_orm::collection_object::ArrayCollectionObject;
use php2rust_doctrine_orm::mapping::class_metadata::FETCH_EAGER;
use php2rust_doctrine_orm::query::Parameter;
use php2rust_doctrine_orm::query::context::{
    HYDRATE_ARRAY, HYDRATE_OBJECT, HYDRATE_SCALAR, HYDRATE_SINGLE_SCALAR,
};
use php2rust_doctrine_persistence::php_reflection;

use orm::*;

const QUERY_EXCEPTION: &str = "Doctrine\\ORM\\Query\\QueryException";

fn user(name: &str, username: &str, status: &str) -> Object {
    cms_user_with(name, username, status)
}

/// `new CmsUser()` with a name and a username only.
fn author(name: &str, username: &str) -> Object {
    let author = cms_user();
    set(&author, "name", s(name));
    set(&author, "username", s(username));
    author
}

fn assert_instance_of(class: &str, value: &Value) {
    let Value::Object(object) = value else {
        panic!("{value:?} is not an instance of {class}");
    };
    assert!(
        object.class() == class || php_reflection::is_a(object.class(), class),
        "{} is not an instance of {class}",
        object.class()
    );
}

fn assert_exception(class: &str, e: &Thrown) {
    assert!(
        e.class == class || php_reflection::is_a(&e.class, class),
        "{e:?} is not an instance of {class}"
    );
}

fn article_value(id: &Value, topic: &str, text: &str, version: i64) -> Value {
    let mut article = PhpArray::new();
    article.insert("id", id.clone());
    article.insert("topic", s(topic));
    article.insert("text", s(text));
    article.insert("version", Value::Int(version));
    Value::Array(article)
}

/// The user with two articles of the join tests.
async fn persist_user_with_articles(
    em: &std::sync::Arc<php2rust_doctrine_orm::entity_manager::EntityManager>,
) -> (Object, Object, Object) {
    let user = user("Guilherme", "gblanco", "developer");

    let article1 = cms_article("Doctrine 2", "This is an introduction to Doctrine 2.");
    add_article(&user, &article1);

    let article2 = cms_article("Symfony 2", "This is an introduction to Symfony 2.");
    add_article(&user, &article2);

    em.persist(&user).unwrap();
    em.persist(&article1).unwrap();
    em.persist(&article2).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();
    (user, article1, article2)
}

/// The author and three articles of the mixed result tests.
async fn persist_author_with_three_articles(
    em: &std::sync::Arc<php2rust_doctrine_orm::entity_manager::EntityManager>,
) {
    let author = author("Ben", "beberlei");

    let article1 = cms_article("Doctrine 2", "This is an introduction to Doctrine 2.");
    set_obj(&article1, "user", &author);

    let article2 = cms_article("Symfony 2", "This is an introduction to Symfony 2.");
    set_obj(&article2, "user", &author);

    let article3 = cms_article("lala 2", "This is an introduction to Symfony 2.");
    set_obj(&article3, "user", &author);

    em.persist(&article1).unwrap();
    em.persist(&article2).unwrap();
    em.persist(&article3).unwrap();
    em.persist(&author).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();
}

#[tokio::test]
async fn testSimpleQueries() {
    let em = set_up(&CMS_MODELS).await;
    let user = user("Guilherme", "gblanco", "developer");
    em.persist(&user).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let query = em.create_query(
        "select u, upper(u.name) from Doctrine\\Tests\\Models\\CMS\\CmsUser u where u.username = 'gblanco'",
    );

    let result = query.get_result(HYDRATE_OBJECT).await.unwrap();

    assert_eq!(list(&result).len(), 1);
    let user0 = at(&at(&result, 0), 0);
    assert_instance_of(CMS_USER, &user0);
    assert_loose_eq(&s("Guilherme"), &get(&obj(&user0), "name"));
    assert_loose_eq(&s("gblanco"), &get(&obj(&user0), "username"));
    assert_loose_eq(&s("developer"), &get(&obj(&user0), "status"));
    assert_loose_eq(&s("GUILHERME"), &at(&at(&result, 0), 1));

    let result_array = query.get_array_result().await.unwrap();
    assert_eq!(list(&result_array).len(), 1);
    assert!(matches!(at(&at(&result_array, 0), 0), Value::Array(_)));
    assert_loose_eq(&s("Guilherme"), &at(&at(&at(&result_array, 0), 0), "name"));
    assert_loose_eq(
        &s("gblanco"),
        &at(&at(&at(&result_array, 0), 0), "username"),
    );
    assert_loose_eq(
        &s("developer"),
        &at(&at(&at(&result_array, 0), 0), "status"),
    );
    assert_loose_eq(&s("GUILHERME"), &at(&at(&result_array, 0), 1));

    let scalar_result = query.get_scalar_result().await.unwrap();
    assert_eq!(list(&scalar_result).len(), 1);
    assert_loose_eq(&s("Guilherme"), &at(&at(&scalar_result, 0), "u_name"));
    assert_loose_eq(&s("gblanco"), &at(&at(&scalar_result, 0), "u_username"));
    assert_loose_eq(&s("developer"), &at(&at(&scalar_result, 0), "u_status"));
    assert_loose_eq(&s("GUILHERME"), &at(&at(&scalar_result, 0), 1));

    let query = em.create_query(
        "select upper(u.name) from Doctrine\\Tests\\Models\\CMS\\CmsUser u where u.username = 'gblanco'",
    );
    assert_loose_eq(
        &s("GUILHERME"),
        &query.get_single_scalar_result().await.unwrap(),
    );
}

#[tokio::test]
async fn testJoinQueries() {
    let em = set_up(&CMS_MODELS).await;
    persist_user_with_articles(&em).await;

    let query = em.create_query(&format!(
        "select u, a from {CMS_USER} u join u.articles a ORDER BY a.topic"
    ));
    let users = query.get_result(HYDRATE_OBJECT).await.unwrap();
    assert_eq!(list(&users).len(), 1);
    assert_instance_of(CMS_USER, &at(&users, 0));
    let user0 = obj(&at(&users, 0));
    let articles = elements(&user0, "articles").await;
    assert_eq!(articles.len(), 2);
    assert_loose_eq(&s("Doctrine 2"), &get(&articles[0], "topic"));
    assert_loose_eq(&s("Symfony 2"), &get(&articles[1], "topic"));
}

#[tokio::test]
async fn testJoinPartialArrayHydration() {
    let em = set_up(&CMS_MODELS).await;
    let (user, article1, article2) = persist_user_with_articles(&em).await;

    let query = em.create_query(&format!(
        "select partial u.{{id, username}}, partial a.{{id, topic}} from {CMS_USER} u join u.articles a ORDER BY a.topic"
    ));
    let users = query.get_array_result().await.unwrap();

    let article = |article: &Object, topic: &str| {
        let mut array = PhpArray::new();
        array.insert("id", get(article, "id"));
        array.insert("topic", s(topic));
        Value::Array(array)
    };
    let mut expected_user = PhpArray::new();
    expected_user.insert("id", get(&user, "id"));
    expected_user.insert("username", s("gblanco"));
    expected_user.insert(
        "articles",
        Value::Array(PhpArray::list([
            article(&article1, "Doctrine 2"),
            article(&article2, "Symfony 2"),
        ])),
    );
    assert_loose_eq(
        &Value::Array(PhpArray::list([Value::Array(expected_user)])),
        &users,
    );
}

#[tokio::test]
async fn testUsingZeroBasedQueryParameterShouldWork() {
    let em = set_up(&CMS_MODELS).await;
    let user = user("Jonathan", "jwage", "developer");
    em.persist(&user).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let q = em.create_query(&format!("SELECT u FROM {CMS_USER} u WHERE u.username = ?0"));
    q.set_parameter(0, s("jwage"), None);
    let user = q.get_single_result(None).await.unwrap();

    assert!(!user.is_null());
}

#[tokio::test]
async fn testUsingUnknownQueryParameterShouldThrowException() {
    let em = set_up(&CMS_MODELS).await;
    let q = em.create_query(&format!("SELECT u FROM {CMS_USER} u WHERE u.name = ?1"));
    q.set_parameter(2, s("jwage"), None);
    let e = q.get_single_result(None).await.unwrap_err();
    assert_exception(QUERY_EXCEPTION, &e);
    assert!(message(&e).contains("Invalid parameter: token 2 is not defined in the query."));
}

#[tokio::test]
async fn testTooManyParametersShouldThrowException() {
    let em = set_up(&CMS_MODELS).await;
    let q = em.create_query(&format!("SELECT u FROM {CMS_USER} u WHERE u.name = ?1"));
    q.set_parameter(1, s("jwage"), None);
    q.set_parameter(2, s("jwage"), None);

    let e = q.get_single_result(None).await.unwrap_err();
    assert_exception(QUERY_EXCEPTION, &e);
    assert!(
        message(&e).contains("Too many parameters: the query defines 1 parameters and you bound 2")
    );
}

#[tokio::test]
async fn testTooFewParametersShouldThrowException() {
    let em = set_up(&CMS_MODELS).await;
    let e = em
        .create_query(&format!("SELECT u FROM {CMS_USER} u WHERE u.name = ?1"))
        .get_single_result(None)
        .await
        .unwrap_err();
    assert_exception(QUERY_EXCEPTION, &e);
    assert!(
        message(&e)
            .contains("Too few parameters: the query defines 1 parameters but you only bound 0")
    );
}

#[tokio::test]
async fn testInvalidInputParameterThrowsException() {
    let em = set_up(&CMS_MODELS).await;
    let e = em
        .create_query(&format!("SELECT u FROM {CMS_USER} u WHERE u.name = ?"))
        .set_parameter(1, s("jwage"), None)
        .get_single_result(None)
        .await
        .unwrap_err();
    assert_exception(QUERY_EXCEPTION, &e);
}

/// `$enum::Case`.
fn case(enum_type: &str, name: &str) -> Value {
    models::register();
    let class = php_reflection::class_info(enum_type).expect("the enum is declared");
    let case = class
        .cases()
        .iter()
        .find(|case| {
            case.property(b"name")
                .is_some_and(|n| n == Value::string(name))
        })
        .cloned()
        .expect("the case is declared");
    Value::Object(case)
}

async fn assert_enum_case_as_parameter(statuses: [&str; 2], case: Value) {
    let em = set_up(&CMS_MODELS).await;
    let john = user("John", "john", statuses[0]);
    em.persist(&john).unwrap();

    let jane = user("Jane", "jane", statuses[1]);
    em.persist(&jane).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();

    let result = em
        .create_query(&format!(
            "SELECT u FROM {CMS_USER} u WHERE u.status = :status"
        ))
        .set_parameter("status", case, None)
        .get_result(HYDRATE_OBJECT)
        .await
        .unwrap();

    assert_eq!(list(&result).len(), 1);
    assert!(s("jane").identical(&get(&obj(&at(&result, 0)), "username")));
}

#[tokio::test]
async fn testUseStringEnumCaseAsParameter() {
    assert_enum_case_as_parameter(
        ["inactive", "active"],
        case("Doctrine\\Tests\\Models\\Enums\\UserStatus", "Active"),
    )
    .await;
}

#[tokio::test]
async fn testUseIntegerEnumCaseAsParameter() {
    assert_enum_case_as_parameter(
        ["1", "2"],
        case("Doctrine\\Tests\\Models\\Enums\\AccessLevel", "User"),
    )
    .await;
}

fn jwage_active() -> PhpArray {
    let mut params = PhpArray::new();
    params.insert(1, s("jwage"));
    params.insert(2, s("active"));
    params
}

#[tokio::test]
async fn testSetParameters() {
    let (em, log) = query_log::set_up_logged(&CMS_MODELS).await;
    log.reset_and_enable();
    let parameters = vec![
        Parameter::new(1, s("jwage"), None),
        Parameter::new(2, s("active"), None),
    ];

    em.create_query(&format!(
        "SELECT u FROM {CMS_USER} u WHERE u.name = ?1 AND u.status = ?2"
    ))
    .set_parameters(parameters)
    .get_result(HYDRATE_OBJECT)
    .await
    .unwrap();

    assert!(Value::Array(jwage_active()).identical(&Value::Array(log.last_params())));
}

#[tokio::test]
async fn testSetParametersBackwardsCompatible() {
    let (em, log) = query_log::set_up_logged(&CMS_MODELS).await;
    log.reset_and_enable();
    let parameters = jwage_active();

    em.create_query(&format!(
        "SELECT u FROM {CMS_USER} u WHERE u.name = ?1 AND u.status = ?2"
    ))
    .set_parameters_array(&parameters)
    .get_result(HYDRATE_OBJECT)
    .await
    .unwrap();

    assert!(Value::Array(parameters).identical(&Value::Array(log.last_params())));
}

/// The two articles of the iteration tests.
async fn persist_two_articles(
    em: &std::sync::Arc<php2rust_doctrine_orm::entity_manager::EntityManager>,
) -> (Object, Object) {
    let article1 = cms_article("Doctrine 2", "This is an introduction to Doctrine 2.");
    let article2 = cms_article("Symfony 2", "This is an introduction to Symfony 2.");

    em.persist(&article1).unwrap();
    em.persist(&article2).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();
    (article1, article2)
}

#[tokio::test]
async fn testIterateResultAsArrayAndParams() {
    let em = set_up(&CMS_MODELS).await;
    let (article1, _) = persist_two_articles(&em).await;
    let article_id = get(&article1, "id");

    let query = em.create_query(&format!("select a from {CMS_ARTICLE} a WHERE a.topic = ?1"));

    let expected_article = article_value(
        &article_id,
        "Doctrine 2",
        "This is an introduction to Doctrine 2.",
        1,
    );

    query.set_parameters(vec![Parameter::new(1, s("Doctrine 2"), None)]);
    let articles = query.to_iterable(None, HYDRATE_ARRAY).await.unwrap();

    assert_eq!(articles.len(), 1);
    assert_loose_eq(
        &Value::Array(PhpArray::list([expected_article])),
        &Value::Array(PhpArray::list(articles)),
    );
}

#[tokio::test]
async fn testIterateResultIterativelyBuildUpUnitOfWork() {
    let em = set_up(&CMS_MODELS).await;
    persist_two_articles(&em).await;

    let query = em.create_query(&format!("select a from {CMS_ARTICLE} a"));
    let articles = query.to_iterable(None, HYDRATE_OBJECT).await.unwrap();

    let mut iterated_count = 0;
    let mut topics = Vec::new();

    for article in articles {
        topics.push(get(&obj(&article), "topic"));

        let identity_map = em.get_unit_of_work().get_identity_map();
        let identity_map_count = identity_map
            .iter()
            .find(|(class, _)| class == CMS_ARTICLE)
            .map_or(0, |(_, entities)| entities.len());
        assert!(identity_map_count > iterated_count);

        iterated_count += 1;
    }

    assert!(
        Value::Array(PhpArray::list([s("Doctrine 2"), s("Symfony 2")]))
            .identical(&Value::Array(PhpArray::list(topics)))
    );
    assert_eq!(2, iterated_count);
}

#[tokio::test]
async fn testToIterableWithMultipleSelectElements() {
    let em = set_up(&CMS_MODELS).await;
    let author = author("Ben", "beberlei");

    let article1 = cms_article("Doctrine 2", "This is an introduction to Doctrine 2.");
    set_obj(&article1, "user", &author);

    let article2 = cms_article("Symfony 2", "This is an introduction to Symfony 2.");
    set_obj(&article2, "user", &author);

    em.persist(&article1).unwrap();
    em.persist(&article2).unwrap();
    em.persist(&author).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();

    let query = em.create_query(&format!(
        "select a, u from {CMS_ARTICLE} a JOIN {CMS_USER} u ON a.user = u"
    ));

    let result = query.to_iterable(None, HYDRATE_OBJECT).await.unwrap();

    assert_eq!(result.len(), 2);

    for row in result {
        assert_eq!(list(&row).len(), 2);
        assert_instance_of(CMS_ARTICLE, &at(&row, 0));
        assert_instance_of(CMS_USER, &at(&row, 1));
    }
}

#[tokio::test]
async fn testToIterableWithMixedResultEntityScalars() {
    let em = set_up(&CMS_MODELS).await;
    persist_author_with_three_articles(&em).await;

    let query = em.create_query(&format!(
        "select a, a.topic, a.text from {CMS_ARTICLE} a order by a.id asc"
    ));
    let it = query.to_iterable(None, HYDRATE_OBJECT).await.unwrap();

    assert_eq!(it.len(), 3);
    assert_loose_eq(&s("Doctrine 2"), &at(&it[0], "topic"));
    assert_loose_eq(&s("Doctrine 2"), &get(&obj(&at(&it[0], 0)), "topic"));
    assert_loose_eq(&s("lala 2"), &at(&it[2], "topic"));
    assert_loose_eq(&s("lala 2"), &get(&obj(&at(&it[2], 0)), "topic"));
}

#[tokio::test]
async fn testToIterableWithMixedResultArbitraryJoinsScalars() {
    let em = set_up(&CMS_MODELS).await;
    persist_author_with_three_articles(&em).await;

    let query = em.create_query(&format!(
        "select a, u, a.topic, a.text from {CMS_ARTICLE} a, {CMS_USER} u WHERE a.user = u order by a.id asc"
    ));
    let it = query.to_iterable(None, HYDRATE_OBJECT).await.unwrap();

    assert_eq!(it.len(), 3);
    assert_loose_eq(&s("Doctrine 2"), &at(&it[0], "topic"));
    assert_loose_eq(&s("Doctrine 2"), &get(&obj(&at(&it[0], 0)), "topic"));
    assert_loose_eq(&s("beberlei"), &get(&obj(&at(&it[0], 1)), "username"));
    assert_loose_eq(&s("lala 2"), &at(&it[2], "topic"));
    assert_loose_eq(&s("lala 2"), &get(&obj(&at(&it[2], 0)), "topic"));
    assert_loose_eq(&s("beberlei"), &get(&obj(&at(&it[2], 1)), "username"));
}

#[tokio::test]
async fn testToIterableWithMixedResultScalarsOnly() {
    let em = set_up(&CMS_MODELS).await;
    persist_author_with_three_articles(&em).await;

    let query = em.create_query(&format!(
        "select a.topic, a.text from {CMS_ARTICLE} a order by a.id asc"
    ));
    let it = query.to_iterable(None, HYDRATE_OBJECT).await.unwrap();

    let row = |topic: &str, text: &str| {
        let mut row = PhpArray::new();
        row.insert("topic", s(topic));
        row.insert("text", s(text));
        Value::Array(row)
    };
    assert_loose_eq(
        &Value::Array(PhpArray::list([
            row("Doctrine 2", "This is an introduction to Doctrine 2."),
            row("Symfony 2", "This is an introduction to Symfony 2."),
            row("lala 2", "This is an introduction to Symfony 2."),
        ])),
        &Value::Array(PhpArray::list(it)),
    );
}

#[tokio::test]
async fn testIterateResultClearEveryCycle() {
    let em = set_up(&CMS_MODELS).await;
    persist_two_articles(&em).await;

    let query = em
        .create_query("select a from Doctrine\\Tests\\Models\\CMS\\CmsArticle a order by a.id asc");

    let articles = query.to_iterable(None, HYDRATE_OBJECT).await.unwrap();
    let mut iterated_count = 0;
    let mut topics = Vec::new();
    for article in articles {
        topics.push(get(&obj(&article), "topic"));

        em.clear().unwrap();

        iterated_count += 1;
    }

    assert!(
        Value::Array(PhpArray::list([s("Doctrine 2"), s("Symfony 2")]))
            .identical(&Value::Array(PhpArray::list(topics)))
    );
    assert_eq!(2, iterated_count);

    em.flush().await.unwrap();
}

#[tokio::test]
async fn testToIterableResultFetchJoinedCollectionThrowsException() {
    let em = set_up(&CMS_MODELS).await;
    // The PHP string is in double quotes: the DQL holds the quotes and
    // the concatenation.
    let query = em.create_query("SELECT u, a FROM ' . CmsUser::class . ' u JOIN u.articles a");
    let e = query.to_iterable(None, HYDRATE_OBJECT).await.unwrap_err();
    assert_exception(QUERY_EXCEPTION, &e);
}

#[tokio::test]
async fn testGetSingleResultThrowsExceptionOnNoResult() {
    let em = set_up(&CMS_MODELS).await;
    let e = em
        .create_query("select a from Doctrine\\Tests\\Models\\CMS\\CmsArticle a")
        .get_single_result(None)
        .await
        .unwrap_err();
    assert_exception(NO_RESULT_EXCEPTION, &e);
}

#[tokio::test]
async fn testGetSingleScalarResultThrowsExceptionOnNoResult() {
    let em = set_up(&CMS_MODELS).await;
    let e = em
        .create_query("select a.id from Doctrine\\Tests\\Models\\CMS\\CmsArticle a")
        .get_single_scalar_result()
        .await
        .unwrap_err();
    assert_exception(NO_RESULT_EXCEPTION, &e);
}

#[tokio::test]
async fn testGetSingleScalarResultThrowsExceptionOnSingleRowWithMultipleColumns() {
    let em = set_up(&CMS_MODELS).await;
    let user = user("Javier", "phansys", "developer");

    em.persist(&user).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();

    let e = em
        .create_query("select u from Doctrine\\Tests\\Models\\CMS\\CmsUser u")
        .set_max_results(Some(1))
        .get_single_scalar_result()
        .await
        .unwrap_err();
    assert_exception(NON_UNIQUE_RESULT_EXCEPTION, &e);
    assert!(message(&e).contains(
        "The query returned a row containing multiple columns. Change the query or use a different result function like getScalarResult()."
    ));
}

#[tokio::test]
async fn testGetSingleScalarResultThrowsExceptionOnNonUniqueResult() {
    let em = set_up(&CMS_MODELS).await;
    persist_user_with_articles(&em).await;

    let e = em
        .create_query("select a.id from Doctrine\\Tests\\Models\\CMS\\CmsArticle a")
        .get_single_scalar_result()
        .await
        .unwrap_err();
    assert_exception(NON_UNIQUE_RESULT_EXCEPTION, &e);
    assert!(message(&e).contains(
        "The query returned multiple rows. Change the query or use a different result function like getScalarResult()."
    ));
}

#[tokio::test]
async fn testModifiedLimitQuery() {
    let em = set_up(&CMS_MODELS).await;
    for i in 0..5 {
        let user = user(
            &format!("Guilherme{i}"),
            &format!("gblanco{i}"),
            "developer",
        );
        em.persist(&user).unwrap();
    }

    em.flush().await.unwrap();
    em.clear().unwrap();

    let query = format!("SELECT u FROM {CMS_USER} u ORDER BY u.username");

    let data = em
        .create_query(&query)
        .set_first_result(1)
        .set_max_results(Some(2))
        .get_result(HYDRATE_OBJECT)
        .await
        .unwrap();

    assert_eq!(list(&data).len(), 2);
    assert_loose_eq(&s("gblanco1"), &get(&obj(&at(&data, 0)), "username"));
    assert_loose_eq(&s("gblanco2"), &get(&obj(&at(&data, 1)), "username"));

    let data = em
        .create_query(&query)
        .set_first_result(3)
        .set_max_results(Some(2))
        .get_result(HYDRATE_OBJECT)
        .await
        .unwrap();

    assert_eq!(list(&data).len(), 2);
    assert_loose_eq(&s("gblanco3"), &get(&obj(&at(&data, 0)), "username"));
    assert_loose_eq(&s("gblanco4"), &get(&obj(&at(&data, 1)), "username"));

    em.create_query(&query)
        .set_first_result(3)
        .set_max_results(Some(2))
        .get_scalar_result()
        .await
        .unwrap();
}

#[tokio::test]
async fn testEntityParameters() {
    let em = set_up(&CMS_MODELS).await;
    let article = cms_article("dr. dolittle", "Once upon a time ...");
    let author = user("anonymous", "anon", "here");
    set_obj(&article, "user", &author);
    em.persist(&author).unwrap();
    em.persist(&article).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let reference = em
        .get_reference(CMS_USER, get(&author, "id"))
        .unwrap()
        .unwrap();
    let q = em.create_query(
        "select a from Doctrine\\Tests\\Models\\CMS\\CmsArticle a where a.topic = :topic and a.user = :user",
    );
    q.set_parameter("user", Value::Object(reference), None)
        .set_parameter("topic", s("dr. dolittle"), None);

    let result = q.get_result(HYDRATE_OBJECT).await.unwrap();
    assert_eq!(1, list(&result).len());
    assert_instance_of(CMS_ARTICLE, &at(&result, 0));
    assert_loose_eq(&s("dr. dolittle"), &get(&obj(&at(&result, 0)), "topic"));
    assert!(em.is_uninitialized_object(&get(&obj(&at(&result, 0)), "user")));
}

#[tokio::test]
async fn testEnableFetchEagerMode() {
    let em = set_up(&CMS_MODELS).await;
    for i in 0..10 {
        let article = cms_article("dr. dolittle", "Once upon a time ...");
        let author = user("anonymous", &format!("anon{i}"), "here");
        set_obj(&article, "user", &author);
        em.persist(&author).unwrap();
        em.persist(&article).unwrap();
    }

    em.flush().await.unwrap();
    em.clear().unwrap();

    let articles = em
        .create_query("select a from Doctrine\\Tests\\Models\\CMS\\CmsArticle a")
        .set_fetch_mode(CMS_ARTICLE, "user", FETCH_EAGER)
        .get_result(HYDRATE_OBJECT)
        .await
        .unwrap();

    assert_eq!(list(&articles).len(), 10);
    for article in list(&articles) {
        assert!(!em.is_uninitialized_object(&article));
    }
}

#[tokio::test]
async fn testgetOneOrNullResult() {
    let em = set_up(&CMS_MODELS).await;
    let user = user("Guilherme", "gblanco", "developer");
    em.persist(&user).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let query = em.create_query(&format!(
        "select u from {CMS_USER} u where u.username = 'gblanco'"
    ));

    let fetched_user = query.get_one_or_null_result(None).await.unwrap();
    assert_instance_of(CMS_USER, &fetched_user);
    assert_loose_eq(&s("gblanco"), &get(&obj(&fetched_user), "username"));

    let query = em.create_query(&format!(
        "select u.username from {CMS_USER} u where u.username = 'gblanco'"
    ));
    let fetched_username = query
        .get_one_or_null_result(Some(HYDRATE_SINGLE_SCALAR))
        .await
        .unwrap();
    assert_loose_eq(&s("gblanco"), &fetched_username);
}

#[tokio::test]
async fn testgetOneOrNullResultSeveralRows() {
    let em = set_up(&CMS_MODELS).await;
    let user1 = user("Guilherme", "gblanco", "developer");
    em.persist(&user1).unwrap();
    let user2 = user("Roman", "romanb", "developer");
    em.persist(&user2).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let query = em.create_query("select u from Doctrine\\Tests\\Models\\CMS\\CmsUser u");

    let e = query.get_one_or_null_result(None).await.unwrap_err();
    assert_exception(NON_UNIQUE_RESULT_EXCEPTION, &e);
}

#[tokio::test]
async fn testgetOneOrNullResultNoRows() {
    let em = set_up(&CMS_MODELS).await;
    let query = em.create_query("select u from Doctrine\\Tests\\Models\\CMS\\CmsUser u");
    assert!(query.get_one_or_null_result(None).await.unwrap().is_null());

    let query = em.create_query(
        "select u.username from Doctrine\\Tests\\Models\\CMS\\CmsUser u where u.username = 'gblanco'",
    );
    assert!(
        query
            .get_one_or_null_result(Some(HYDRATE_SCALAR))
            .await
            .unwrap()
            .is_null()
    );
}

/// The three users of the parameter tests.
async fn persist_three_users(
    em: &std::sync::Arc<php2rust_doctrine_orm::entity_manager::EntityManager>,
) -> (Object, Object, Object) {
    let user1 = user("Benjamin", "beberlei", "developer");
    em.persist(&user1).unwrap();

    let user2 = user("Roman", "romanb", "developer");
    em.persist(&user2).unwrap();

    let user3 = user("Jonathan", "jwage", "developer");
    em.persist(&user3).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();
    (user1, user2, user3)
}

#[tokio::test]
async fn testParameterOrder() {
    let em = set_up(&CMS_MODELS).await;
    let (user1, user2, user3) = persist_three_users(&em).await;

    let query = em.create_query(
        "SELECT u FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u WHERE u.status = :a AND u.id IN (:b)",
    );
    query.set_parameters(vec![
        Parameter::new(
            "b",
            Value::Array(PhpArray::list([
                get(&user1, "id"),
                get(&user2, "id"),
                get(&user3, "id"),
            ])),
            None,
        ),
        Parameter::new("a", s("developer"), None),
    ]);
    let result = query.get_result(HYDRATE_OBJECT).await.unwrap();

    assert_eq!(list(&result).len(), 3);
}

#[tokio::test]
async fn testDqlWithAutoInferOfParameters() {
    let em = set_up(&CMS_MODELS).await;
    persist_three_users(&em).await;

    let query = em.create_query(
        "SELECT u FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u WHERE u.username IN (?0)",
    );
    query.set_parameter(
        0,
        Value::Array(PhpArray::list([s("beberlei"), s("jwage")])),
        None,
    );

    let users = query.execute(None, None).await.unwrap();

    assert_eq!(list(&users).len(), 2);
}

#[tokio::test]
async fn testQueryWithArrayOfEntitiesAsParameter() {
    let em = set_up(&CMS_MODELS).await;
    let (user_a, _, user_c) = persist_three_users(&em).await;

    let query = em.create_query(
        "SELECT u FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u WHERE u IN (?0) OR u.username = ?1",
    );
    query.set_parameter(
        0,
        Value::Array(PhpArray::list([
            Value::Object(user_a),
            Value::Object(user_c),
        ])),
        None,
    );
    query.set_parameter(1, s("beberlei"), None);

    let users = query.execute(None, None).await.unwrap();

    assert_eq!(list(&users).len(), 2);
}

#[tokio::test]
async fn testQueryWithHiddenAsSelectExpression() {
    let em = set_up(&CMS_MODELS).await;
    persist_three_users(&em).await;

    let query = em.create_query(
        "SELECT u, (SELECT COUNT(u2.id) FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u2) AS HIDDEN total FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u",
    );
    let users = query.execute(None, None).await.unwrap();

    assert_eq!(list(&users).len(), 3);
    assert_instance_of(CMS_USER, &at(&users, 0));
}

#[tokio::test]
async fn testSetParameterBindingSingleIdentifierObject() {
    let em = set_up(&CMS_MODELS).await;
    let user_c = user("Jonathan", "jwage", "developer");
    em.persist(&user_c).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();

    let q = em.create_query(
        "SELECT DISTINCT u from Doctrine\\Tests\\Models\\CMS\\CmsUser u WHERE u.id = ?1",
    );
    q.set_parameter(1, Value::Object(user_c.clone()), None);

    assert_loose_eq(
        &Value::Object(user_c),
        q.get_parameter(1).unwrap().get_value(),
    );

    // Parameter is not converted before, but it should be converted during execution. Test should not fail here
    q.get_result(HYDRATE_OBJECT).await.unwrap();
}

#[tokio::test]
async fn testSetCollectionParameterBindingSingleIdentifierObject() {
    let em = set_up(&CMS_MODELS).await;
    let u1 = user("Name1", "username1", "developer");
    em.persist(&u1).unwrap();

    let u2 = user("Name2", "username2", "tester");
    em.persist(&u2).unwrap();

    let u3 = user("Name3", "username3", "tester");
    em.persist(&u3).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();

    let user_collection = ArrayCollectionObject::value(PhpArray::list([
        Value::Object(u1.clone()),
        Value::Object(u2.clone()),
        get(&u3, "id"),
    ]));

    let q = em.create_query(
        "SELECT u FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u WHERE u IN (:users) ORDER BY u.id",
    );
    q.set_parameter("users", user_collection, None);
    let users = q.execute(None, None).await.unwrap();

    assert_eq!(list(&users).len(), 3);
    assert_instance_of(CMS_USER, &at(&users, 0));
    assert_instance_of(CMS_USER, &at(&users, 1));
    assert_instance_of(CMS_USER, &at(&users, 2));

    let result_user1 = obj(&at(&users, 0));
    let result_user2 = obj(&at(&users, 1));
    let result_user3 = obj(&at(&users, 2));

    assert_loose_eq(&get(&u1, "username"), &get(&result_user1, "username"));
    assert_loose_eq(&get(&u2, "username"), &get(&result_user2, "username"));
    assert_loose_eq(&get(&u3, "username"), &get(&result_user3, "username"));
}

#[tokio::test]
async fn testUnexpectedResultException() {
    let em = set_up(&CMS_MODELS).await;
    let dql = "SELECT u FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u";
    let u1 = user("Fabio B. Silva", "FabioBatSilva", "developer");
    let u2 = user("Test", "test", "tester");

    match em.create_query(dql).get_single_result(None).await {
        Ok(_) => panic!("Expected exception \"\\Doctrine\\ORM\\NoResultException\"."),
        Err(exc) => {
            assert_exception(UNEXPECTED_RESULT_EXCEPTION, &exc);
            assert_exception(NO_RESULT_EXCEPTION, &exc);
        }
    }

    em.persist(&u1).unwrap();
    em.persist(&u2).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    match em.create_query(dql).get_single_result(None).await {
        Ok(_) => panic!("Expected exception \"\\Doctrine\\ORM\\NonUniqueResultException\"."),
        Err(exc) => {
            assert_exception(UNEXPECTED_RESULT_EXCEPTION, &exc);
            assert_exception(NON_UNIQUE_RESULT_EXCEPTION, &exc);
        }
    }
}

/// The two users and the phone number of the join component tests.
async fn persist_users_with_phonenumber(
    em: &std::sync::Arc<php2rust_doctrine_orm::entity_manager::EntityManager>,
) {
    let user_a = user("Benjamin", "beberlei", "developer");

    let phonenumber_a = cms_phonenumber("111111");
    add_phonenumber(&user_a, &phonenumber_a);

    let user_b = user("Alexander", "asm89", "developer");

    em.persist(&user_a).unwrap();
    em.persist(&user_b).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();
}

#[tokio::test]
async fn testMultipleJoinComponentsUsingInnerJoin() {
    let em = set_up(&CMS_MODELS).await;
    persist_users_with_phonenumber(&em).await;

    let query = em.create_query(
        "
            SELECT u, p
              FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u
             INNER JOIN Doctrine\\Tests\\Models\\CMS\\CmsPhonenumber p ON u = p.user
        ",
    );
    let users = query.execute(None, None).await.unwrap();

    assert_eq!(list(&users).len(), 2);
    assert_instance_of(CMS_USER, &at(&users, 0));
    assert_instance_of(CMS_PHONENUMBER, &at(&users, 1));
}

#[tokio::test]
async fn testMultipleJoinComponentsUsingLeftJoin() {
    let em = set_up(&CMS_MODELS).await;
    persist_users_with_phonenumber(&em).await;

    let query = em.create_query(
        "
            SELECT u, p
              FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u
              LEFT JOIN Doctrine\\Tests\\Models\\CMS\\CmsPhonenumber p ON u = p.user
        ",
    );
    let users = query.execute(None, None).await.unwrap();

    assert_eq!(list(&users).len(), 4);
    assert_instance_of(CMS_USER, &at(&users, 0));
    assert_instance_of(CMS_PHONENUMBER, &at(&users, 1));
    assert_instance_of(CMS_USER, &at(&users, 2));
    assert!(at(&users, 3).is_null());
}
