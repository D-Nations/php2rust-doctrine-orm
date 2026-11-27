//! Upstream: doctrine/orm tests/Tests/ORM/ConfigurationTest.php
//!
//! The stubs (`createStub()`) are real implementations of the interface;
//! `assertSame()` compares the `Arc`s, and `assertInstanceOf()` of an
//! interface is the type. `#[IgnoreDeprecations]` tests ignore the proxy
//! deprecations. `enableNativeLazyObjects(false)` triggers the
//! deprecation, then is refused (the port only has native lazy objects).

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

use std::sync::Arc;

use php_value::{Object, Thrown};
use php2rust_doctrine_orm::cache::CacheConfiguration;
use php2rust_doctrine_orm::cache::pool::{ArrayAdapter, CacheItemPool};
use php2rust_doctrine_orm::configuration::{Configuration, proxy_factory};
use php2rust_doctrine_orm::deprecations;
use php2rust_doctrine_orm::mapping::driver::attribute_driver::AttributeDriver;
use php2rust_doctrine_orm::mapping::entity_listener_resolver::EntityListenerResolver;
use php2rust_doctrine_orm::mapping::typed_field_mapper::DefaultTypedFieldMapper;
use php2rust_doctrine_orm::mapping::{AnsiQuoteStrategy, UnderscoreNamingStrategy};

const CONFIGURATION_TEST: &str = "Doctrine\\Tests\\ORM\\ConfigurationTest";
const CLASS_METADATA_FACTORY: &str = "Doctrine\\ORM\\Mapping\\ClassMetadataFactory";

/// `assertSame()` of two `Arc`s of a trait object.
fn same<T: ?Sized, U: ?Sized>(a: &Arc<T>, b: &Arc<U>) -> bool {
    std::ptr::addr_eq(Arc::as_ptr(a), Arc::as_ptr(b))
}

fn set_up() -> Configuration {
    Configuration::new()
}

#[test]
fn testSetGetProxyDir() {
    let mut configuration = set_up();
    assert_eq!(None, configuration.get_proxy_dir()); // defaults

    configuration.set_proxy_dir(env!("CARGO_MANIFEST_DIR"));
    assert_eq!(
        Some(env!("CARGO_MANIFEST_DIR")),
        configuration.get_proxy_dir()
    );
}

#[test]
fn testSetGetAutoGenerateProxyClasses() {
    let mut configuration = set_up();
    assert_eq!(
        proxy_factory::AUTOGENERATE_ALWAYS,
        configuration.get_auto_generate_proxy_classes()
    ); // defaults

    configuration.set_auto_generate_proxy_classes(false);
    assert_eq!(
        proxy_factory::AUTOGENERATE_NEVER,
        configuration.get_auto_generate_proxy_classes()
    );

    configuration.set_auto_generate_proxy_classes(true);
    assert_eq!(
        proxy_factory::AUTOGENERATE_ALWAYS,
        configuration.get_auto_generate_proxy_classes()
    );

    configuration.set_auto_generate_proxy_classes(proxy_factory::AUTOGENERATE_FILE_NOT_EXISTS);
    assert_eq!(
        proxy_factory::AUTOGENERATE_FILE_NOT_EXISTS,
        configuration.get_auto_generate_proxy_classes()
    );
}

#[test]
fn testSetGetProxyNamespace() {
    let mut configuration = set_up();
    assert_eq!(None, configuration.get_proxy_namespace()); // defaults

    configuration.set_proxy_namespace("Doctrine\\Tests\\ORM");
    assert_eq!(
        Some("Doctrine\\Tests\\ORM"),
        configuration.get_proxy_namespace()
    );
}

#[test]
fn testSetGetMetadataDriverImpl() {
    let mut configuration = set_up();
    assert!(configuration.get_metadata_driver_impl().is_none()); // defaults

    let metadata_driver = Arc::new(AttributeDriver::new(Vec::<String>::new(), true).unwrap());
    configuration.set_metadata_driver_impl(metadata_driver.clone());
    assert!(same(
        &metadata_driver,
        configuration.get_metadata_driver_impl().unwrap()
    ));
}

#[test]
fn testSetGetQueryCache() {
    let configuration = set_up();
    assert!(configuration.get_query_cache().is_none()); // defaults
    let query_cache: Arc<dyn CacheItemPool> = Arc::new(ArrayAdapter::new());
    configuration.set_query_cache(query_cache.clone());
    assert!(same(
        &query_cache,
        &configuration.get_query_cache().unwrap()
    ));
}

#[test]
fn testSetGetHydrationCache() {
    let configuration = set_up();
    assert!(configuration.get_hydration_cache().is_none()); // defaults
    let hydration_cache: Arc<dyn CacheItemPool> = Arc::new(ArrayAdapter::new());
    configuration.set_hydration_cache(hydration_cache.clone());
    assert!(same(
        &hydration_cache,
        &configuration.get_hydration_cache().unwrap()
    ));
}

#[test]
fn testSetGetMetadataCache() {
    let configuration = set_up();
    assert!(configuration.get_metadata_cache().is_none());
    let cache: Arc<dyn CacheItemPool> = Arc::new(ArrayAdapter::new());
    configuration.set_metadata_cache(cache.clone());
    assert!(same(&cache, &configuration.get_metadata_cache().unwrap()));
}

#[test]
fn testAddGetCustomStringFunction() {
    let mut configuration = set_up();
    configuration.add_custom_string_function("FunctionName", CONFIGURATION_TEST);
    assert_eq!(
        Some(CONFIGURATION_TEST),
        configuration
            .get_custom_string_function("FunctionName")
            .as_deref()
    );
    assert_eq!(
        None,
        configuration.get_custom_string_function("NonExistingFunction")
    );
    configuration.set_custom_string_functions(&[("OtherFunctionName", CONFIGURATION_TEST)]);
    assert_eq!(
        Some(CONFIGURATION_TEST),
        configuration
            .get_custom_string_function("OtherFunctionName")
            .as_deref()
    );
}

#[test]
fn testAddGetCustomNumericFunction() {
    let mut configuration = set_up();
    configuration.add_custom_numeric_function("FunctionName", CONFIGURATION_TEST);
    assert_eq!(
        Some(CONFIGURATION_TEST),
        configuration
            .get_custom_numeric_function("FunctionName")
            .as_deref()
    );
    assert_eq!(
        None,
        configuration.get_custom_numeric_function("NonExistingFunction")
    );
    configuration.set_custom_numeric_functions(&[("OtherFunctionName", CONFIGURATION_TEST)]);
    assert_eq!(
        Some(CONFIGURATION_TEST),
        configuration
            .get_custom_numeric_function("OtherFunctionName")
            .as_deref()
    );
}

#[test]
fn testAddGetCustomDatetimeFunction() {
    let mut configuration = set_up();
    configuration.add_custom_datetime_function("FunctionName", CONFIGURATION_TEST);
    assert_eq!(
        Some(CONFIGURATION_TEST),
        configuration
            .get_custom_datetime_function("FunctionName")
            .as_deref()
    );
    assert_eq!(
        None,
        configuration.get_custom_datetime_function("NonExistingFunction")
    );
    configuration.set_custom_datetime_functions(&[("OtherFunctionName", CONFIGURATION_TEST)]);
    assert_eq!(
        Some(CONFIGURATION_TEST),
        configuration
            .get_custom_datetime_function("OtherFunctionName")
            .as_deref()
    );
}

#[test]
fn testAddGetCustomHydrationMode() {
    let mut configuration = set_up();
    assert_eq!(None, configuration.get_custom_hydration_mode("NonExisting"));

    configuration.add_custom_hydration_mode("HydrationModeName", CONFIGURATION_TEST);

    assert_eq!(
        Some(CONFIGURATION_TEST),
        configuration.get_custom_hydration_mode("HydrationModeName")
    );
}

#[test]
fn testSetCustomHydrationModes() {
    let mut configuration = set_up();
    configuration.add_custom_hydration_mode("HydrationModeName", CONFIGURATION_TEST);

    assert_eq!(
        Some(CONFIGURATION_TEST),
        configuration.get_custom_hydration_mode("HydrationModeName")
    );

    configuration.set_custom_hydration_modes(&[("AnotherHydrationModeName", CONFIGURATION_TEST)]);

    assert_eq!(
        None,
        configuration.get_custom_hydration_mode("HydrationModeName")
    );
    assert_eq!(
        Some(CONFIGURATION_TEST),
        configuration.get_custom_hydration_mode("AnotherHydrationModeName")
    );
}

#[test]
fn testSetGetClassMetadataFactoryName() {
    let mut configuration = set_up();
    assert_eq!(
        CLASS_METADATA_FACTORY,
        configuration.get_class_metadata_factory_name()
    );
    configuration.set_class_metadata_factory_name(CONFIGURATION_TEST);
    assert_eq!(
        CONFIGURATION_TEST,
        configuration.get_class_metadata_factory_name()
    );
}

#[test]
fn testAddGetFilters() {
    let configuration = set_up();
    assert_eq!(
        configuration.get_filter_class_name("NonExistingFilter"),
        None
    );
    configuration.add_filter("FilterName", CONFIGURATION_TEST);
    assert_eq!(
        configuration.get_filter_class_name("FilterName").as_deref(),
        Some(CONFIGURATION_TEST)
    );
}

#[test]
fn testSetGetNamingStrategy() {
    let mut configuration = set_up();
    // `assertInstanceOf(NamingStrategy::class, ...)`: the type.
    let _default = configuration.get_naming_strategy();
    let naming_strategy = Arc::new(UnderscoreNamingStrategy::new(0));
    configuration.set_naming_strategy(naming_strategy.clone());
    assert!(same(&naming_strategy, &configuration.get_naming_strategy()));
}

#[test]
fn testSetGetQuoteStrategy() {
    let mut configuration = set_up();
    // `assertInstanceOf(QuoteStrategy::class, ...)`: the type.
    let _default = configuration.get_quote_strategy();
    let quote_strategy = Arc::new(AnsiQuoteStrategy::new());
    configuration.set_quote_strategy(quote_strategy.clone());
    assert!(same(&quote_strategy, &configuration.get_quote_strategy()));
}

/// `createStub(EntityListenerResolver::class)`.
#[derive(Debug)]
struct EntityListenerResolverStub;

impl EntityListenerResolver for EntityListenerResolverStub {
    fn clear(&self, _class_name: Option<&str>) {}

    fn resolve(&self, class_name: &str) -> Result<Object, Thrown> {
        Err(Thrown::new(
            "Error",
            format!("Class \"{class_name}\" not found"),
            0,
        ))
    }

    fn register(&self, _object: Object) {}
}

#[test]
fn testSetGetEntityListenerResolver() {
    let mut configuration = set_up();
    // `assertInstanceOf(DefaultEntityListenerResolver::class, ...)`.
    assert!(
        format!("{:?}", configuration.get_entity_listener_resolver())
            .starts_with("DefaultEntityListenerResolver"),
    );
    let resolver = Arc::new(EntityListenerResolverStub);
    configuration.set_entity_listener_resolver(resolver.clone());
    assert!(same(
        &resolver,
        &configuration.get_entity_listener_resolver()
    ));
}

#[test]
fn testSetGetSecondLevelCacheConfig() {
    let mut configuration = set_up();
    let mock_class = Arc::new(CacheConfiguration::new());

    assert!(
        configuration
            .get_second_level_cache_configuration()
            .is_none()
    );
    configuration.set_second_level_cache_configuration(mock_class.clone());
    assert!(same(
        &mock_class,
        &configuration
            .get_second_level_cache_configuration()
            .unwrap()
    ));
}

#[test]
fn testSetGetTypedFieldMapper() {
    let mut configuration = set_up();
    assert!(configuration.get_typed_field_mapper().is_none());

    let default_typed_field_mapper = Arc::new(DefaultTypedFieldMapper::default());
    configuration.set_typed_field_mapper(Some(default_typed_field_mapper.clone()));

    assert!(same(
        &default_typed_field_mapper,
        &configuration.get_typed_field_mapper().unwrap()
    ));
}

#[test]
fn testDisablingNativeLazyObjectsIsDeprecated() {
    let mut configuration = set_up();
    deprecations::enable_tracking_deprecations();
    let before = deprecations::count("https://github.com/doctrine/orm/pull/12005");

    let _ = configuration.enable_native_lazy_objects(false);

    assert!(deprecations::count("https://github.com/doctrine/orm/pull/12005") > before);
}
