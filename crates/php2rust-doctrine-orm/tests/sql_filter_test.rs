//! Functional/SQLFilterTest.php on `SQLite` (`tests/common/orm.rs`).
//!
//! The mocked entity manager, connection and filter collection of the
//! `SQLFilter` unit tests are a real entity manager on an open `SQLite`
//! connection: its driver quotes (`'en'`), and the collection's
//! `setFiltersStateDirty()` is seen as the collection turning dirty. The
//! mocked `ClassMetadata` is a `ClassMetadataInfo` with the name set.
//! Each filter class registers its constructor
//! (`register_filter_class()`), as the configuration names it by class.
//!
//! Pending: the query cache (not ported); see the parity rows.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/SQLFilterTest.php

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/company.rs"]
mod company;
#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;

use std::sync::Arc;

use php_value::{Thrown, Value, php_array};
use php2rust_doctrine_dbal::types::Types;
use php2rust_doctrine_orm::collection_object as coll;
use php2rust_doctrine_orm::configuration::Configuration;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::mapping::ClassMetadataInfo;
use php2rust_doctrine_orm::mapping::class_metadata::FETCH_EXTRA_LAZY;
use php2rust_doctrine_orm::persistent_collection::PersistentCollection;
use php2rust_doctrine_orm::query::context::HYDRATE_OBJECT;
use php2rust_doctrine_orm::query::{FilterConstraint, SqlFilter, register_filter_class};
use php2rust_doctrine_persistence::php_reflection::ReflectionClass;

use orm::{
    CMS_ADDRESS, CMS_ARTICLE, CMS_GROUP, CMS_MODELS, CMS_USER, add_group, cms_address, cms_article,
    cms_group, cms_user_with, get, set,
};

const NS: &str = "Doctrine\\Tests\\ORM\\Functional\\";
const MY_SOFT_DELETE_FILTER: &str = "Doctrine\\Tests\\ORM\\Functional\\MySoftDeleteFilter";
const MY_LOCALE_FILTER: &str = "Doctrine\\Tests\\ORM\\Functional\\MyLocaleFilter";
const CMS_COUNTRY_FILTER: &str = "Doctrine\\Tests\\ORM\\Functional\\CMSCountryFilter";
const CMS_GROUP_PREFIX_FILTER: &str = "Doctrine\\Tests\\ORM\\Functional\\CMSGroupPrefixFilter";
const COMPANY_PERSON_NAME_FILTER: &str =
    "Doctrine\\Tests\\ORM\\Functional\\CompanyPersonNameFilter";
const COMPLETED_CONTRACT_FILTER: &str = "Doctrine\\Tests\\ORM\\Functional\\CompletedContractFilter";
const COMPANY_EVENT_FILTER: &str = "Doctrine\\Tests\\ORM\\Functional\\CompanyEventFilter";
const CMS_ARTICLE_TOPIC_FILTER: &str = "Doctrine\\Tests\\ORM\\Functional\\CMSArticleTopicFilter";
const FILTER_EXCEPTION: &str = "Doctrine\\ORM\\Query\\Filter\\FilterException";

// The filters

/// `MySoftDeleteFilter extends SQLFilter`.
struct MySoftDeleteFilter;

impl FilterConstraint for MySoftDeleteFilter {
    fn add_filter_constraint(
        &self,
        _filter: &SqlFilter,
        target_entity: &ClassMetadataInfo,
        target_table_alias: &str,
    ) -> Result<String, Thrown> {
        if target_entity.name != "MyEntity\\SoftDeleteNewsItem" {
            return Ok(String::new());
        }

        Ok(format!("{target_table_alias}.deleted = 0"))
    }
}

/// `MyLocaleFilter extends SQLFilter`.
struct MyLocaleFilter;

impl FilterConstraint for MyLocaleFilter {
    fn add_filter_constraint(
        &self,
        filter: &SqlFilter,
        target_entity: &ClassMetadataInfo,
        target_table_alias: &str,
    ) -> Result<String, Thrown> {
        let locale_aware = ReflectionClass::new(&target_entity.name)
            .and_then(|class| class.implements_interface("LocaleAware"))
            .unwrap_or(false);
        if !locale_aware {
            return Ok(String::new());
        }

        // getParam uses connection to quote the value.
        Ok(format!(
            "{target_table_alias}.locale = {}",
            filter.get_parameter("locale")?
        ))
    }
}

/// `CMSCountryFilter extends SQLFilter`.
struct CmsCountryFilter;

impl FilterConstraint for CmsCountryFilter {
    fn add_filter_constraint(
        &self,
        filter: &SqlFilter,
        target_entity: &ClassMetadataInfo,
        target_table_alias: &str,
    ) -> Result<String, Thrown> {
        if target_entity.name != CMS_ADDRESS {
            return Ok(String::new());
        }

        // getParam uses connection to quote the value.
        Ok(format!(
            "{target_table_alias}.country IN ({})",
            filter.get_parameter_list("country")?
        ))
    }
}

/// `CMSGroupPrefixFilter extends SQLFilter`.
struct CmsGroupPrefixFilter;

impl FilterConstraint for CmsGroupPrefixFilter {
    fn add_filter_constraint(
        &self,
        filter: &SqlFilter,
        target_entity: &ClassMetadataInfo,
        target_table_alias: &str,
    ) -> Result<String, Thrown> {
        if target_entity.name != CMS_GROUP {
            return Ok(String::new());
        }

        // getParam uses connection to quote the value.
        Ok(format!(
            "{target_table_alias}.name LIKE {}",
            filter.get_parameter("prefix")?
        ))
    }
}

/// `CompanyPersonNameFilter extends SQLFilter`.
struct CompanyPersonNameFilter;

impl FilterConstraint for CompanyPersonNameFilter {
    fn add_filter_constraint(
        &self,
        filter: &SqlFilter,
        target_entity: &ClassMetadataInfo,
        target_table_alias: &str,
    ) -> Result<String, Thrown> {
        if target_entity.name != company::COMPANY_PERSON {
            return Ok(String::new());
        }

        Ok(format!(
            "{target_table_alias}.name LIKE {}",
            filter.get_parameter("name")?
        ))
    }
}

/// `CompletedContractFilter extends SQLFilter`.
struct CompletedContractFilter;

impl FilterConstraint for CompletedContractFilter {
    fn add_filter_constraint(
        &self,
        filter: &SqlFilter,
        target_entity: &ClassMetadataInfo,
        target_table_alias: &str,
    ) -> Result<String, Thrown> {
        if target_entity.name != company::COMPANY_CONTRACT {
            return Ok(String::new());
        }

        Ok(format!(
            "{target_table_alias}.completed = {}",
            filter.get_parameter("completed")?
        ))
    }
}

/// `CompanyEventFilter extends SQLFilter`.
struct CompanyEventFilter;

impl FilterConstraint for CompanyEventFilter {
    fn add_filter_constraint(
        &self,
        filter: &SqlFilter,
        target_entity: &ClassMetadataInfo,
        target_table_alias: &str,
    ) -> Result<String, Thrown> {
        if target_entity.name != company::COMPANY_EVENT {
            return Ok(String::new());
        }

        Ok(format!(
            "{target_table_alias}.id = {}",
            filter.get_parameter("id")?
        ))
    }
}

/// `CMSArticleTopicFilter extends SQLFilter`.
struct CmsArticleTopicFilter;

impl FilterConstraint for CmsArticleTopicFilter {
    fn add_filter_constraint(
        &self,
        filter: &SqlFilter,
        target_entity: &ClassMetadataInfo,
        target_table_alias: &str,
    ) -> Result<String, Thrown> {
        if target_entity.name != CMS_ARTICLE {
            return Ok(String::new());
        }

        // getParam uses connection to quote the value.
        Ok(format!(
            "{target_table_alias}.topic = {}",
            filter.get_parameter("topic")?
        ))
    }
}

fn register_filters() {
    fn register(class: &'static str, constraint: fn() -> Box<dyn FilterConstraint>) {
        register_filter_class(
            class,
            Arc::new(move |em| SqlFilter::new(em, class, constraint())),
        );
    }
    register(MY_SOFT_DELETE_FILTER, || Box::new(MySoftDeleteFilter));
    register(MY_LOCALE_FILTER, || Box::new(MyLocaleFilter));
    register(CMS_COUNTRY_FILTER, || Box::new(CmsCountryFilter));
    register(CMS_GROUP_PREFIX_FILTER, || Box::new(CmsGroupPrefixFilter));
    register(COMPANY_PERSON_NAME_FILTER, || {
        Box::new(CompanyPersonNameFilter)
    });
    register(COMPLETED_CONTRACT_FILTER, || {
        Box::new(CompletedContractFilter)
    });
    register(COMPANY_EVENT_FILTER, || Box::new(CompanyEventFilter));
    register(CMS_ARTICLE_TOPIC_FILTER, || Box::new(CmsArticleTopicFilter));
}

// The test case

/// `setUp()`: the cms and company model sets.
async fn set_up() -> Arc<EntityManager> {
    register_filters();
    company::register_listeners();
    let mut models = CMS_MODELS.to_vec();
    models.extend(company::COMPANY_MODELS);
    orm::set_up(&models).await
}

/// `getEntityManager()`: a new entity manager on an open connection.
async fn get_entity_manager() -> Arc<EntityManager> {
    register_filters();
    let em = orm::entity_manager(orm::connection(), None);
    em.get_connection().connect().await.unwrap();
    em
}

fn configure_filters(em: &EntityManager) {
    // Add filters to the configuration of the EM
    let config = em.get_configuration();
    config.add_filter("locale", &format!("\\{NS}MyLocaleFilter"));
    config.add_filter("soft_delete", &format!("\\{NS}MySoftDeleteFilter"));
}

#[derive(Default)]
struct Fixture {
    user_id: Value,
    user_id2: Value,
    article_id2: Value,
    group_id: Value,
    group_id2: Value,
}

async fn load_fixture_data(em: &Arc<EntityManager>) -> Fixture {
    let user = cms_user_with("Roman", "romanb", "developer");

    let address = cms_address("Germany", "12345", "Berlin");

    set(&user, "address", Value::Object(address.clone())); // inverse side
    set(&address, "user", Value::Object(user.clone())); // owning side!

    let group = cms_group("foo_group");
    add_group(&user, &group);

    let article1 = cms_article("Test1", "Test");
    set(&article1, "user", Value::Object(user.clone()));

    let article2 = cms_article("Test2", "Test");
    set(&article2, "user", Value::Object(user.clone()));

    em.persist(&article1).unwrap();
    em.persist(&article2).unwrap();

    em.persist(&user).unwrap();

    let user2 = cms_user_with("Guilherme", "gblanco", "developer");

    let address2 = cms_address("France", "12345", "Paris");

    set(&user, "address", Value::Object(address2.clone())); // inverse side
    set(&address2, "user", Value::Object(user2.clone())); // owning side!

    add_group(&user2, &group);
    let group2 = cms_group("bar_group");
    add_group(&user2, &group2);

    em.persist(&user2).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    Fixture {
        user_id: get(&user, "id"),
        user_id2: get(&user2, "id"),
        article_id2: get(&article2, "id"),
        group_id: get(&group, "id"),
        group_id2: get(&group2, "id"),
    }
}

/// `loadLazyFixtureData()`.
async fn load_lazy_fixture_data(em: &Arc<EntityManager>) -> Fixture {
    {
        let class = em.get_class_metadata(CMS_USER).unwrap();
        let mut class = class.write();
        for field in ["articles", "groups"] {
            class.association_mappings.get_mut(field).unwrap().fetch = Some(FETCH_EXTRA_LAZY);
        }
    }
    load_fixture_data(em).await
}

fn use_cms_article_topic_filter(em: &EntityManager) {
    let conf = em.get_configuration();
    conf.add_filter("article_topic", &format!("\\{NS}CMSArticleTopicFilter"));
    em.get_filters()
        .enable("article_topic")
        .unwrap()
        .set_parameter("topic", Value::string("Test1"), Some(Types::STRING))
        .unwrap();
}

fn is_initialized(object: &php_value::Object, name: &str) -> bool {
    PersistentCollection::from_value(&get(object, name))
        .expect("a persistent collection")
        .is_initialized()
}

async fn count_of(object: &php_value::Object, name: &str) -> usize {
    coll::count(&get(object, name)).await.unwrap()
}

async fn contains(object: &php_value::Object, name: &str, element: &php_value::Object) -> bool {
    coll::contains(&get(object, name), &Value::Object(element.clone()))
        .await
        .unwrap()
}

async fn slice_count(object: &php_value::Object, name: &str) -> usize {
    coll::slice(&get(object, name), 0, Some(10))
        .await
        .unwrap()
        .len()
}

async fn find(em: &Arc<EntityManager>, class: &str, id: &Value) -> php_value::Object {
    em.find(class, id.clone()).await.unwrap().unwrap()
}

fn use_cms_group_prefix_filter(em: &EntityManager) {
    let conf = em.get_configuration();
    conf.add_filter("group_prefix", &format!("\\{NS}CMSGroupPrefixFilter"));
    em.get_filters()
        .enable("group_prefix")
        .unwrap()
        .set_parameter("prefix", Value::string("foo%"), Some(Types::STRING))
        .unwrap();
}

async fn count_result(query: &php2rust_doctrine_orm::query::Query) -> usize {
    match query.get_result(HYDRATE_OBJECT).await.unwrap() {
        Value::Array(rows) => rows.len(),
        other => panic!("not a result list: {other:?}"),
    }
}

#[test]
fn testConfigureFilter() {
    let config = Configuration::new();

    config.add_filter(
        "locale",
        "\\Doctrine\\Tests\\ORM\\Functional\\MyLocaleFilter",
    );

    assert_eq!(
        config.get_filter_class_name("locale").as_deref(),
        Some("\\Doctrine\\Tests\\ORM\\Functional\\MyLocaleFilter")
    );
    assert_eq!(config.get_filter_class_name("foo"), None);
}

#[tokio::test]
async fn testEntityManagerEnableFilter() {
    let em = get_entity_manager().await;
    configure_filters(&em);

    // Enable an existing filter
    let filter = em.get_filters().enable("locale").unwrap();
    assert_eq!(filter.class(), MY_LOCALE_FILTER);

    // Enable the filter again
    let filter2 = em.get_filters().enable("locale").unwrap();
    assert!(Arc::ptr_eq(&filter, &filter2));

    // Enable a non-existing filter
    let e = em.get_filters().enable("foo").unwrap_err();
    assert_eq!(e.class, "InvalidArgumentException");
}

#[tokio::test]
async fn testEntityManagerEnabledFilters() {
    let em = get_entity_manager().await;

    // No enabled filters
    assert!(em.get_filters().get_enabled_filters().is_empty());

    configure_filters(&em);
    em.get_filters().enable("locale").unwrap();
    em.get_filters().enable("soft_delete").unwrap();

    // Two enabled filters
    assert_eq!(em.get_filters().get_enabled_filters().len(), 2);
}

#[tokio::test]
async fn testEntityManagerDisableFilter() {
    let em = get_entity_manager().await;
    configure_filters(&em);

    // Enable the filter
    let filter = em.get_filters().enable("locale").unwrap();

    // Disable it
    assert!(Arc::ptr_eq(
        &filter,
        &em.get_filters().disable("locale").unwrap()
    ));
    assert_eq!(em.get_filters().get_enabled_filters().len(), 0);

    // Disable a non-existing filter
    let e = em.get_filters().disable("foo").unwrap_err();
    assert_eq!(e.class, "InvalidArgumentException");

    // Disable a non-enabled filter
    let e = em.get_filters().disable("locale").unwrap_err();
    assert_eq!(e.class, "InvalidArgumentException");
}

#[tokio::test]
async fn testEntityManagerGetFilter() {
    let em = get_entity_manager().await;
    configure_filters(&em);

    // Enable the filter
    let filter = em.get_filters().enable("locale").unwrap();

    // Get the filter
    assert!(Arc::ptr_eq(
        &filter,
        &em.get_filters().get_filter("locale").unwrap()
    ));

    // Get a non-enabled filter
    let e = em.get_filters().get_filter("soft_delete").unwrap_err();
    assert_eq!(e.class, "InvalidArgumentException");
}

#[tokio::test]
async fn testEntityManagerIsFilterEnabled() {
    let em = get_entity_manager().await;
    configure_filters(&em);

    // Check for an enabled filter
    em.get_filters().enable("locale").unwrap();
    assert!(em.get_filters().is_enabled("locale"));

    // Check for a disabled filter
    em.get_filters().disable("locale").unwrap();
    assert!(!em.get_filters().is_enabled("locale"));

    // Check a non-existing filter
    assert!(!em.get_filters().is_enabled("foo_filter"));
}

#[tokio::test]
async fn testSQLFilterGetSetParameter() {
    let em = get_entity_manager().await;
    assert!(em.get_filters().is_clean());

    let filter = SqlFilter::new(&em, MY_LOCALE_FILTER, Box::new(MyLocaleFilter));

    filter
        .set_parameter("locale", Value::string("en"), Some(Types::STRING))
        .unwrap();
    // `setFiltersStateDirty()` was called.
    assert!(!em.get_filters().is_clean());

    assert_eq!(filter.get_parameter("locale").unwrap(), "'en'");
}

#[tokio::test]
async fn testSQLFilterGetConnection() {
    let em = get_entity_manager().await;

    let filter = SqlFilter::new(&em, MY_LOCALE_FILTER, Box::new(MyLocaleFilter));

    assert!(Arc::ptr_eq(
        &filter.get_connection().unwrap(),
        em.get_connection()
    ));
}

#[tokio::test]
async fn testSQLFilterSetParameterInfersType() {
    let em = get_entity_manager().await;
    assert!(em.get_filters().is_clean());

    let filter = SqlFilter::new(&em, MY_LOCALE_FILTER, Box::new(MyLocaleFilter));

    filter
        .set_parameter("locale", Value::string("en"), None)
        .unwrap();
    assert!(!em.get_filters().is_clean());

    assert_eq!(filter.get_parameter("locale").unwrap(), "'en'");
}

#[tokio::test]
async fn testSQLFilterSetArrayParameterInfersType() {
    let em = get_entity_manager().await;
    assert!(em.get_filters().is_clean());

    let filter = SqlFilter::new(&em, MY_LOCALE_FILTER, Box::new(MyLocaleFilter));

    filter
        .set_parameter_list("locale", php_array!["en", "es"], None)
        .unwrap();
    assert!(!em.get_filters().is_clean());

    assert_eq!(filter.get_parameter_list("locale").unwrap(), "'en','es'");
}

#[tokio::test]
async fn testSQLFilterAddConstraint() {
    let em = get_entity_manager().await;
    let filter = SqlFilter::new(&em, MY_SOFT_DELETE_FILTER, Box::new(MySoftDeleteFilter));

    // Test for an entity that gets extra filter data
    let mut target_entity = ClassMetadataInfo::new("MyEntity\\SoftDeleteNewsItem", None, None);
    assert_eq!(
        filter.add_filter_constraint(&target_entity, "t1_").unwrap(),
        "t1_.deleted = 0"
    );

    // Test for an entity that doesn't get extra filter data
    target_entity.name = "MyEntity\\NoSoftDeleteNewsItem".to_owned();
    assert_eq!(
        filter.add_filter_constraint(&target_entity, "t1_").unwrap(),
        ""
    );
}

#[tokio::test]
async fn testSQLFilterToString() {
    let em = get_entity_manager().await;

    let filter = SqlFilter::new(&em, MY_LOCALE_FILTER, Box::new(MyLocaleFilter));
    filter
        .set_parameter("locale", Value::string("en"), Some(Types::STRING))
        .unwrap();
    filter
        .set_parameter("foo", Value::string("bar"), Some(Types::STRING))
        .unwrap();

    let filter2 = SqlFilter::new(&em, MY_LOCALE_FILTER, Box::new(MyLocaleFilter));
    filter2
        .set_parameter("foo", Value::string("bar"), Some(Types::STRING))
        .unwrap();
    filter2
        .set_parameter("locale", Value::string("en"), Some(Types::STRING))
        .unwrap();

    let parameters = php_array![
        "foo" => php_array!["value" => "bar", "type" => Types::STRING, "is_list" => false],
        "locale" => php_array!["value" => "en", "type" => Types::STRING, "is_list" => false],
    ];

    assert_eq!(
        php_value::serialize::serialize(&Value::Array(parameters)),
        filter.to_php_string()
    );
    assert_eq!(filter.to_php_string(), filter2.to_php_string());
}

#[tokio::test]
async fn testQueryGenerationDependsOnFilters() {
    let em = set_up().await;
    let query = em.create_query("select a from Doctrine\\Tests\\Models\\CMS\\CmsAddress a");
    let first_sql_query = query.get_single_sql().unwrap();

    let conf = em.get_configuration();
    conf.add_filter(
        "country",
        "\\Doctrine\\Tests\\ORM\\Functional\\CMSCountryFilter",
    );
    em.get_filters()
        .enable("country")
        .unwrap()
        .set_parameter_list("country", php_array!["en"], Some(Types::STRING))
        .unwrap();

    assert_ne!(first_sql_query, query.get_single_sql().unwrap());
}

#[tokio::test]
async fn testRepositoryFind() {
    let em = set_up().await;
    let f = load_fixture_data(&em).await;

    let repository = em.get_repository(CMS_GROUP).unwrap();
    assert!(repository.find(f.group_id.clone()).await.unwrap().is_some());
    assert!(
        repository
            .find(f.group_id2.clone())
            .await
            .unwrap()
            .is_some()
    );

    use_cms_group_prefix_filter(&em);
    em.clear().unwrap();

    assert!(repository.find(f.group_id).await.unwrap().is_some());
    assert!(repository.find(f.group_id2).await.unwrap().is_none());
}

#[tokio::test]
async fn testRepositoryFindAll() {
    let em = set_up().await;
    load_fixture_data(&em).await;

    let repository = em.get_repository(CMS_GROUP).unwrap();
    assert_eq!(repository.find_all().await.unwrap().len(), 2);

    use_cms_group_prefix_filter(&em);
    em.clear().unwrap();

    assert_eq!(repository.find_all().await.unwrap().len(), 1);
}

#[tokio::test]
async fn testRepositoryFindBy() {
    let em = set_up().await;
    let f = load_fixture_data(&em).await;

    let repository = em.get_repository(CMS_GROUP).unwrap();
    let criteria = php_array!["id" => f.group_id2];
    assert_eq!(
        repository
            .find_by(&criteria, None, None, None)
            .await
            .unwrap()
            .len(),
        1
    );

    use_cms_group_prefix_filter(&em);
    em.clear().unwrap();

    assert_eq!(
        repository
            .find_by(&criteria, None, None, None)
            .await
            .unwrap()
            .len(),
        0
    );
}

#[tokio::test]
async fn testRepositoryFindByX() {
    let em = set_up().await;
    let f = load_fixture_data(&em).await;

    let repository = em.get_repository(CMS_GROUP).unwrap();
    let found = repository
        .call("findById", vec![f.group_id2.clone()])
        .await
        .unwrap();
    assert_eq!(found.as_array().unwrap().len(), 1);

    use_cms_group_prefix_filter(&em);
    em.clear().unwrap();

    let found = repository
        .call("findById", vec![f.group_id2])
        .await
        .unwrap();
    assert_eq!(found.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn testRepositoryFindOneBy() {
    let em = set_up().await;
    let f = load_fixture_data(&em).await;

    let repository = em.get_repository(CMS_GROUP).unwrap();
    let criteria = php_array!["id" => f.group_id2];
    assert!(
        repository
            .find_one_by(&criteria, None)
            .await
            .unwrap()
            .is_some()
    );

    use_cms_group_prefix_filter(&em);
    em.clear().unwrap();

    assert!(
        repository
            .find_one_by(&criteria, None)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn testRepositoryFindOneByX() {
    let em = set_up().await;
    let f = load_fixture_data(&em).await;

    let repository = em.get_repository(CMS_GROUP).unwrap();
    let found = repository
        .call("findOneById", vec![f.group_id2.clone()])
        .await
        .unwrap();
    assert!(!found.is_null());

    use_cms_group_prefix_filter(&em);
    em.clear().unwrap();

    let found = repository
        .call("findOneById", vec![f.group_id2])
        .await
        .unwrap();
    assert!(found.is_null());
}

#[tokio::test]
async fn testToOneFilter() {
    let em = set_up().await;
    load_fixture_data(&em).await;

    let query = em.create_query(
        "select ux, ua from Doctrine\\Tests\\Models\\CMS\\CmsUser ux JOIN ux.address ua",
    );

    // We get two users before enabling the filter
    assert_eq!(count_result(&query).await, 2);

    let conf = em.get_configuration();
    conf.add_filter(
        "country",
        "\\Doctrine\\Tests\\ORM\\Functional\\CMSCountryFilter",
    );
    em.get_filters()
        .enable("country")
        .unwrap()
        .set_parameter_list("country", php_array!["Germany"], Some(Types::STRING))
        .unwrap();

    // We get one user after enabling the filter
    assert_eq!(count_result(&query).await, 1);

    em.get_filters()
        .enable("country")
        .unwrap()
        .set_parameter_list(
            "country",
            php_array!["Germany", "France"],
            Some(Types::STRING),
        )
        .unwrap();

    assert_eq!(count_result(&query).await, 2);
}

#[tokio::test]
async fn testOneToOneInverseSideWithFilter() {
    let em = set_up().await;
    let f = load_fixture_data(&em).await;

    let conf = em.get_configuration();
    conf.add_filter(
        "country",
        "\\Doctrine\\Tests\\ORM\\Functional\\CMSCountryFilter",
    );
    em.get_filters()
        .enable("country")
        .unwrap()
        .set_parameter_list("country", php_array!["Germany"], Some(Types::STRING))
        .unwrap();

    let user = em.find(CMS_USER, f.user_id).await.unwrap().unwrap();
    assert!(get(&user, "address").is_truthy());

    let user2 = em.find(CMS_USER, f.user_id2).await.unwrap().unwrap();
    assert!(!get(&user2, "address").is_truthy());
}

#[tokio::test]
async fn testOneToManyExtraLazyCountWithFilter() {
    let em = set_up().await;
    let f = load_lazy_fixture_data(&em).await;
    let user = find(&em, CMS_USER, &f.user_id).await;

    assert!(!is_initialized(&user, "articles"));
    assert_eq!(count_of(&user, "articles").await, 2);

    use_cms_article_topic_filter(&em);

    assert_eq!(count_of(&user, "articles").await, 1);
}

#[tokio::test]
async fn testOneToManyExtraLazyContainsWithFilter() {
    let em = set_up().await;
    let f = load_lazy_fixture_data(&em).await;
    let user = find(&em, CMS_USER, &f.user_id).await;
    let filtered_article = find(&em, CMS_ARTICLE, &f.article_id2).await;

    assert!(!is_initialized(&user, "articles"));
    assert!(contains(&user, "articles", &filtered_article).await);

    use_cms_article_topic_filter(&em);

    assert!(!contains(&user, "articles", &filtered_article).await);
}

#[tokio::test]
async fn testOneToManyExtraLazySliceWithFilter() {
    let em = set_up().await;
    let f = load_lazy_fixture_data(&em).await;
    let user = find(&em, CMS_USER, &f.user_id).await;

    assert!(!is_initialized(&user, "articles"));
    assert_eq!(slice_count(&user, "articles").await, 2);

    use_cms_article_topic_filter(&em);

    assert_eq!(slice_count(&user, "articles").await, 1);
}

#[tokio::test]
async fn testManyToManyExtraLazyCountWithFilter() {
    let em = set_up().await;
    let f = load_lazy_fixture_data(&em).await;

    let user = find(&em, CMS_USER, &f.user_id2).await;

    assert!(!is_initialized(&user, "groups"));
    assert_eq!(count_of(&user, "groups").await, 2);

    use_cms_group_prefix_filter(&em);

    assert_eq!(count_of(&user, "groups").await, 1);
}

#[tokio::test]
async fn testManyToManyExtraLazyContainsWithFilter() {
    let em = set_up().await;
    let f = load_lazy_fixture_data(&em).await;
    let user = find(&em, CMS_USER, &f.user_id2).await;
    let filtered_article = find(&em, CMS_GROUP, &f.group_id2).await;

    assert!(!is_initialized(&user, "groups"));
    assert!(contains(&user, "groups", &filtered_article).await);

    use_cms_group_prefix_filter(&em);

    assert!(!contains(&user, "groups", &filtered_article).await);
}

#[tokio::test]
async fn testManyToManyExtraLazySliceWithFilter() {
    let em = set_up().await;
    let f = load_lazy_fixture_data(&em).await;
    let user = find(&em, CMS_USER, &f.user_id2).await;

    assert!(!is_initialized(&user, "groups"));
    assert_eq!(slice_count(&user, "groups").await, 2);

    use_cms_group_prefix_filter(&em);

    assert_eq!(slice_count(&user, "groups").await, 1);
}

#[tokio::test]
async fn testManyToManyFilter() {
    let em = set_up().await;
    load_fixture_data(&em).await;
    let query = em.create_query(
        "select ux, ug from Doctrine\\Tests\\Models\\CMS\\CmsUser ux JOIN ux.groups ug",
    );

    // We get two users before enabling the filter
    assert_eq!(count_result(&query).await, 2);

    let conf = em.get_configuration();
    conf.add_filter(
        "group_prefix",
        "\\Doctrine\\Tests\\ORM\\Functional\\CMSGroupPrefixFilter",
    );
    em.get_filters()
        .enable("group_prefix")
        .unwrap()
        .set_parameter("prefix", Value::string("bar_%"), Some(Types::STRING))
        .unwrap();

    // We get one user after enabling the filter
    assert_eq!(count_result(&query).await, 1);
}

#[tokio::test]
async fn testWhereFilter() {
    let em = set_up().await;
    load_fixture_data(&em).await;
    let query =
        em.create_query("select ug from Doctrine\\Tests\\Models\\CMS\\CmsGroup ug WHERE 1=1");

    // We get two users before enabling the filter
    assert_eq!(count_result(&query).await, 2);

    let conf = em.get_configuration();
    conf.add_filter(
        "group_prefix",
        "\\Doctrine\\Tests\\ORM\\Functional\\CMSGroupPrefixFilter",
    );
    em.get_filters()
        .enable("group_prefix")
        .unwrap()
        .set_parameter("prefix", Value::string("bar_%"), Some(Types::STRING))
        .unwrap();

    // We get one user after enabling the filter
    assert_eq!(count_result(&query).await, 1);
}

#[tokio::test]
async fn testWhereOrFilter() {
    let em = set_up().await;
    load_fixture_data(&em).await;
    let query = em
        .create_query("select ug from Doctrine\\Tests\\Models\\CMS\\CmsGroup ug WHERE 1=1 OR 1=1");

    // We get two users before enabling the filter
    assert_eq!(count_result(&query).await, 2);

    let conf = em.get_configuration();
    conf.add_filter(
        "group_prefix",
        "\\Doctrine\\Tests\\ORM\\Functional\\CMSGroupPrefixFilter",
    );
    em.get_filters()
        .enable("group_prefix")
        .unwrap()
        .set_parameter("prefix", Value::string("bar_%"), Some(Types::STRING))
        .unwrap();

    // We get one user after enabling the filter
    assert_eq!(count_result(&query).await, 1);
}

// The company model set

/// `getRepository($class)->findAll()`'s count.
async fn find_all_count(em: &Arc<EntityManager>, class: &str) -> usize {
    em.get_repository(class)
        .unwrap()
        .find_all()
        .await
        .unwrap()
        .len()
}

#[derive(Default)]
struct CompanyFixture {
    manager_id: Value,
    manager_id2: Value,
    contract_id1: Value,
    contract_id2: Value,
    organization_id: Value,
    event_id1: Value,
    event_id2: Value,
}

#[tokio::test]
async fn testJoinSubclassPersisterFilterOnlyOnRootTableWhenFetchingSubEntity() {
    let em = set_up().await;
    load_company_joined_subclass_fixture_data(&em).await;
    // Persister
    assert_eq!(find_all_count(&em, company::COMPANY_MANAGER).await, 2);
    // SQLWalker
    let query =
        em.create_query("SELECT cm FROM Doctrine\\Tests\\Models\\Company\\CompanyManager cm");
    assert_eq!(count_result(&query).await, 2);

    // Enable the filter
    use_person_name_filter(&em, "Guilh%");

    let managers = em
        .get_repository(company::COMPANY_MANAGER)
        .unwrap()
        .find_all()
        .await
        .unwrap();
    assert_eq!(managers.len(), 1);
    orm::assert_loose_eq(&Value::string("Guilherme"), &get(&managers[0], "name"));

    let query =
        em.create_query("SELECT cm FROM Doctrine\\Tests\\Models\\Company\\CompanyManager cm");
    assert_eq!(count_result(&query).await, 1);
}

#[tokio::test]
async fn testJoinSubclassPersisterFilterOnlyOnRootTableWhenFetchingRootEntity() {
    let em = set_up().await;
    load_company_joined_subclass_fixture_data(&em).await;
    assert_eq!(find_all_count(&em, company::COMPANY_PERSON).await, 3);
    let query =
        em.create_query("SELECT cp FROM Doctrine\\Tests\\Models\\Company\\CompanyPerson cp");
    assert_eq!(count_result(&query).await, 3);

    // Enable the filter
    use_person_name_filter(&em, "Guilh%");

    let persons = em
        .get_repository(company::COMPANY_PERSON)
        .unwrap()
        .find_all()
        .await
        .unwrap();
    assert_eq!(persons.len(), 1);
    orm::assert_loose_eq(&Value::string("Guilherme"), &get(&persons[0], "name"));

    let query =
        em.create_query("SELECT cp FROM Doctrine\\Tests\\Models\\Company\\CompanyPerson cp");
    assert_eq!(count_result(&query).await, 1);
}

async fn load_company_joined_subclass_fixture_data(em: &Arc<EntityManager>) {
    let manager = company::company_manager("Roman", 42, "persisters", "testlead");

    let manager2 = company::company_manager("Guilherme", 42, "parsers", "devlead");

    let person = company::company_person("Benjamin");

    em.persist(&manager).unwrap();
    em.persist(&manager2).unwrap();
    em.persist(&person).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();
}

#[tokio::test]
async fn testSingleTableInheritanceFilterOnlyOnRootTableWhenFetchingSubEntity() {
    let em = set_up().await;
    load_company_single_table_inheritance_fixture_data(&em).await;
    // Persister
    assert_eq!(
        find_all_count(&em, company::COMPANY_FLEX_ULTRA_CONTRACT).await,
        2
    );
    // SQLWalker
    let query = em.create_query(
        "SELECT cfc FROM Doctrine\\Tests\\Models\\Company\\CompanyFlexUltraContract cfc",
    );
    assert_eq!(count_result(&query).await, 2);

    // Enable the filter
    use_completed_contract_filter(&em);

    assert_eq!(
        find_all_count(&em, company::COMPANY_FLEX_ULTRA_CONTRACT).await,
        1
    );
    let query = em.create_query(
        "SELECT cfc FROM Doctrine\\Tests\\Models\\Company\\CompanyFlexUltraContract cfc",
    );
    assert_eq!(count_result(&query).await, 1);
}

#[tokio::test]
async fn testSingleTableInheritanceFilterOnlyOnRootTableWhenFetchingRootEntity() {
    let em = set_up().await;
    load_company_single_table_inheritance_fixture_data(&em).await;
    assert_eq!(find_all_count(&em, company::COMPANY_FLEX_CONTRACT).await, 4);
    let query = em
        .create_query("SELECT cfc FROM Doctrine\\Tests\\Models\\Company\\CompanyFlexContract cfc");
    assert_eq!(count_result(&query).await, 4);

    // Enable the filter
    use_completed_contract_filter(&em);

    assert_eq!(find_all_count(&em, company::COMPANY_FLEX_CONTRACT).await, 2);
    let query = em
        .create_query("SELECT cfc FROM Doctrine\\Tests\\Models\\Company\\CompanyFlexContract cfc");
    assert_eq!(count_result(&query).await, 2);
}

async fn load_company_single_table_inheritance_fixture_data(
    em: &Arc<EntityManager>,
) -> CompanyFixture {
    let contract1 = company::company_contract(company::COMPANY_FLEX_ULTRA_CONTRACT);
    let contract2 = company::company_contract(company::COMPANY_FLEX_ULTRA_CONTRACT);
    company::mark_completed(&contract2);

    let contract3 = company::company_contract(company::COMPANY_FLEX_CONTRACT);
    let contract4 = company::company_contract(company::COMPANY_FLEX_CONTRACT);
    company::mark_completed(&contract4);

    let manager = company::company_manager("Alexander", 42, "Doctrine", "Filterer");

    let manager2 = company::company_manager("Benjamin", 1337, "Doctrine", "Maintainer");

    for contract in [&contract1, &contract2, &contract3, &contract4] {
        // `$contract->addManager($manager)`
        orm::append(contract, "managers", Value::Object(manager.clone()));
    }

    orm::append(&contract1, "managers", Value::Object(manager2.clone()));

    orm::set_obj(&contract1, "salesPerson", &manager);
    orm::set_obj(&contract2, "salesPerson", &manager);

    em.persist(&manager).unwrap();
    em.persist(&manager2).unwrap();
    em.persist(&contract1).unwrap();
    em.persist(&contract2).unwrap();
    em.persist(&contract3).unwrap();
    em.persist(&contract4).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    CompanyFixture {
        manager_id: get(&manager, "id"),
        manager_id2: get(&manager2, "id"),
        contract_id1: get(&contract1, "id"),
        contract_id2: get(&contract2, "id"),
        ..CompanyFixture::default()
    }
}

fn use_completed_contract_filter(em: &EntityManager) {
    let conf = em.get_configuration();
    conf.add_filter(
        "completed_contract",
        "\\Doctrine\\Tests\\ORM\\Functional\\CompletedContractFilter",
    );
    em.get_filters()
        .enable("completed_contract")
        .unwrap()
        .set_parameter("completed", Value::Bool(true), Some(Types::BOOLEAN))
        .unwrap();
}

#[tokio::test]
async fn testManyToManyExtraLazyCountWithFilterOnSTI() {
    let em = set_up().await;
    let f = load_company_single_table_inheritance_fixture_data(&em).await;

    let manager = find(&em, company::COMPANY_MANAGER, &f.manager_id).await;

    assert!(!is_initialized(&manager, "managedContracts"));
    assert_eq!(orm::count_of(&manager, "managedContracts").await, 4);

    // Enable the filter
    use_completed_contract_filter(&em);

    assert!(!is_initialized(&manager, "managedContracts"));
    assert_eq!(orm::count_of(&manager, "managedContracts").await, 2);
}

#[tokio::test]
async fn testManyToManyExtraLazyContainsWithFilterOnSTI() {
    let em = set_up().await;
    let f = load_company_single_table_inheritance_fixture_data(&em).await;

    let manager = find(&em, company::COMPANY_MANAGER, &f.manager_id).await;
    let contract1 = find(&em, company::COMPANY_CONTRACT, &f.contract_id1).await;
    let contract2 = find(&em, company::COMPANY_CONTRACT, &f.contract_id2).await;

    assert!(!is_initialized(&manager, "managedContracts"));
    assert!(contains(&manager, "managedContracts", &contract1).await);
    assert!(contains(&manager, "managedContracts", &contract2).await);

    // Enable the filter
    use_completed_contract_filter(&em);

    assert!(!is_initialized(&manager, "managedContracts"));
    assert!(!contains(&manager, "managedContracts", &contract1).await);
    assert!(contains(&manager, "managedContracts", &contract2).await);
}

#[tokio::test]
async fn testManyToManyExtraLazySliceWithFilterOnSTI() {
    let em = set_up().await;
    let f = load_company_single_table_inheritance_fixture_data(&em).await;

    let manager = find(&em, company::COMPANY_MANAGER, &f.manager_id).await;

    assert!(!is_initialized(&manager, "managedContracts"));
    assert_eq!(slice_count(&manager, "managedContracts").await, 4);

    // Enable the filter
    use_completed_contract_filter(&em);

    assert!(!is_initialized(&manager, "managedContracts"));
    assert_eq!(slice_count(&manager, "managedContracts").await, 2);
}

fn use_person_name_filter(em: &EntityManager, name: &str) {
    // Enable the filter
    let conf = em.get_configuration();
    conf.add_filter(
        "person_name",
        "\\Doctrine\\Tests\\ORM\\Functional\\CompanyPersonNameFilter",
    );
    em.get_filters()
        .enable("person_name")
        .unwrap()
        .set_parameter("name", Value::string(name), Some(Types::STRING))
        .unwrap();
}

#[tokio::test]
async fn testManyToManyExtraLazyCountWithFilterOnCTI() {
    let em = set_up().await;
    let f = load_company_single_table_inheritance_fixture_data(&em).await;

    let contract = find(&em, company::COMPANY_FLEX_ULTRA_CONTRACT, &f.contract_id1).await;

    assert!(!is_initialized(&contract, "managers"));
    assert_eq!(orm::count_of(&contract, "managers").await, 2);

    // Enable the filter
    use_person_name_filter(&em, "Benjamin");

    assert!(!is_initialized(&contract, "managers"));
    assert_eq!(orm::count_of(&contract, "managers").await, 1);
}

#[tokio::test]
async fn testManyToManyExtraLazyContainsWithFilterOnCTI() {
    let em = set_up().await;
    let f = load_company_single_table_inheritance_fixture_data(&em).await;

    let contract = find(&em, company::COMPANY_FLEX_ULTRA_CONTRACT, &f.contract_id1).await;
    let manager1 = find(&em, company::COMPANY_MANAGER, &f.manager_id).await;
    let manager2 = find(&em, company::COMPANY_MANAGER, &f.manager_id2).await;

    assert!(!is_initialized(&contract, "managers"));
    assert!(contains(&contract, "managers", &manager1).await);
    assert!(contains(&contract, "managers", &manager2).await);

    // Enable the filter
    use_person_name_filter(&em, "Benjamin");

    assert!(!is_initialized(&contract, "managers"));
    assert!(!contains(&contract, "managers", &manager1).await);
    assert!(contains(&contract, "managers", &manager2).await);
}

#[tokio::test]
async fn testManyToManyExtraLazySliceWithFilterOnCTI() {
    let em = set_up().await;
    let f = load_company_single_table_inheritance_fixture_data(&em).await;

    let contract = find(&em, company::COMPANY_FLEX_ULTRA_CONTRACT, &f.contract_id1).await;

    assert!(!is_initialized(&contract, "managers"));
    assert_eq!(slice_count(&contract, "managers").await, 2);

    // Enable the filter
    use_person_name_filter(&em, "Benjamin");

    assert!(!is_initialized(&contract, "managers"));
    assert_eq!(slice_count(&contract, "managers").await, 1);
}

#[tokio::test]
async fn testOneToManyExtraLazyCountWithFilterOnSTI() {
    let em = set_up().await;
    let f = load_company_single_table_inheritance_fixture_data(&em).await;

    let manager = find(&em, company::COMPANY_MANAGER, &f.manager_id).await;

    assert!(!is_initialized(&manager, "soldContracts"));
    assert_eq!(orm::count_of(&manager, "soldContracts").await, 2);

    // Enable the filter
    use_completed_contract_filter(&em);

    assert!(!is_initialized(&manager, "soldContracts"));
    assert_eq!(orm::count_of(&manager, "soldContracts").await, 1);
}

#[tokio::test]
async fn testOneToManyExtraLazyContainsWithFilterOnSTI() {
    let em = set_up().await;
    let f = load_company_single_table_inheritance_fixture_data(&em).await;

    let manager = find(&em, company::COMPANY_MANAGER, &f.manager_id).await;
    let contract1 = find(&em, company::COMPANY_CONTRACT, &f.contract_id1).await;
    let contract2 = find(&em, company::COMPANY_CONTRACT, &f.contract_id2).await;

    assert!(!is_initialized(&manager, "soldContracts"));
    assert!(contains(&manager, "soldContracts", &contract1).await);
    assert!(contains(&manager, "soldContracts", &contract2).await);

    // Enable the filter
    use_completed_contract_filter(&em);

    assert!(!is_initialized(&manager, "soldContracts"));
    assert!(!contains(&manager, "soldContracts", &contract1).await);
    assert!(contains(&manager, "soldContracts", &contract2).await);
}

#[tokio::test]
async fn testOneToManyExtraLazySliceWithFilterOnSTI() {
    let em = set_up().await;
    let f = load_company_single_table_inheritance_fixture_data(&em).await;

    let manager = find(&em, company::COMPANY_MANAGER, &f.manager_id).await;

    assert!(!is_initialized(&manager, "soldContracts"));
    assert_eq!(slice_count(&manager, "soldContracts").await, 2);

    // Enable the filter
    use_completed_contract_filter(&em);

    assert!(!is_initialized(&manager, "soldContracts"));
    assert_eq!(slice_count(&manager, "soldContracts").await, 1);
}

async fn load_company_organization_event_joined_subclass_fixture_data(
    em: &Arc<EntityManager>,
) -> CompanyFixture {
    let organization = company::company_organization();

    let event1 = company::company_event(company::COMPANY_AUCTION, "foo");

    let event2 = company::company_event(company::COMPANY_AUCTION, "bar");

    company::add_event(&organization, &event1);
    company::add_event(&organization, &event2);

    em.persist(&organization).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    CompanyFixture {
        organization_id: get(&organization, "id"),
        event_id1: get(&event1, "id"),
        event_id2: get(&event2, "id"),
        ..CompanyFixture::default()
    }
}

fn use_company_event_id_filter(em: &EntityManager, f: &CompanyFixture) {
    // Enable the filter
    let conf = em.get_configuration();
    conf.add_filter("event_id", COMPANY_EVENT_FILTER);
    em.get_filters()
        .enable("event_id")
        .unwrap()
        .set_parameter("id", f.event_id2.clone(), None)
        .unwrap();
}

#[tokio::test]
async fn testOneToManyExtraLazyCountWithFilterOnCTI() {
    let em = set_up().await;
    let f = load_company_organization_event_joined_subclass_fixture_data(&em).await;

    let organization = find(&em, company::COMPANY_ORGANIZATION, &f.organization_id).await;

    assert!(!is_initialized(&organization, "events"));
    assert_eq!(orm::count_of(&organization, "events").await, 2);

    // Enable the filter
    use_company_event_id_filter(&em, &f);

    assert!(!is_initialized(&organization, "events"));
    assert_eq!(orm::count_of(&organization, "events").await, 1);
}

#[tokio::test]
async fn testOneToManyExtraLazyContainsWithFilterOnCTI() {
    let em = set_up().await;
    let f = load_company_organization_event_joined_subclass_fixture_data(&em).await;

    let organization = find(&em, company::COMPANY_ORGANIZATION, &f.organization_id).await;

    let event1 = find(&em, company::COMPANY_EVENT, &f.event_id1).await;
    let event2 = find(&em, company::COMPANY_EVENT, &f.event_id2).await;

    assert!(!is_initialized(&organization, "events"));
    assert!(contains(&organization, "events", &event1).await);
    assert!(contains(&organization, "events", &event2).await);

    // Enable the filter
    use_company_event_id_filter(&em, &f);

    assert!(!is_initialized(&organization, "events"));
    assert!(!contains(&organization, "events", &event1).await);
    assert!(contains(&organization, "events", &event2).await);
}

#[tokio::test]
async fn testOneToManyExtraLazySliceWithFilterOnCTI() {
    let em = set_up().await;
    let f = load_company_organization_event_joined_subclass_fixture_data(&em).await;

    let organization = find(&em, company::COMPANY_ORGANIZATION, &f.organization_id).await;

    assert!(!is_initialized(&organization, "events"));
    assert_eq!(slice_count(&organization, "events").await, 2);

    // Enable the filter
    use_company_event_id_filter(&em, &f);

    assert!(!is_initialized(&organization, "events"));
    assert_eq!(slice_count(&organization, "events").await, 1);
}

#[tokio::test]
async fn testRetrieveSingleAsListThrowsException() {
    let em = set_up().await;
    let conf = em.get_configuration();
    conf.add_filter(
        "country",
        "\\Doctrine\\Tests\\ORM\\Functional\\CMSCountryFilter",
    );

    em.get_filters().enable("country").unwrap();

    em.get_filters()
        .get_filter("country")
        .unwrap()
        .set_parameter("country", Value::string("DE"), None)
        .unwrap();
    let e = em
        .get_filters()
        .get_filter("country")
        .unwrap()
        .get_parameter_list("country")
        .unwrap_err();
    assert_eq!(e.class, FILTER_EXCEPTION);
    assert_eq!(
        orm::message(&e),
        "Cannot convert single SQL filter parameter \"country\" into a list value."
    );
}

#[tokio::test]
async fn testRetrieveListAsSingleThrowsException() {
    let em = set_up().await;
    let conf = em.get_configuration();
    conf.add_filter(
        "country",
        "\\Doctrine\\Tests\\ORM\\Functional\\CMSCountryFilter",
    );

    em.get_filters().enable("country").unwrap();

    em.get_filters()
        .get_filter("country")
        .unwrap()
        .set_parameter_list("country", php_array!["DE"], None)
        .unwrap();
    let e = em
        .get_filters()
        .get_filter("country")
        .unwrap()
        .get_parameter("country")
        .unwrap_err();
    assert_eq!(e.class, FILTER_EXCEPTION);
    assert_eq!(
        orm::message(&e),
        "Cannot convert list-based SQL filter parameter \"country\" into a single value."
    );
}
