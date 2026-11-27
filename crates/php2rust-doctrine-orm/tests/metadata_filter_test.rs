//! Port of `MetadataFilterTest`. The test entity manager is the factory
//! configuration of an attribute driver; `assertContains()` of a
//! metadata compares the references.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Tools/Console/MetadataFilterTest.php

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/models.rs"]
mod models;

use std::sync::Arc;

use php2rust_doctrine_orm::mapping::driver::AttributeDriver;
use php2rust_doctrine_orm::mapping::{ClassMetadata, ClassMetadataFactory, FactoryConfiguration};
use php2rust_doctrine_orm::tools::console::metadata_filter::filter;
use php2rust_doctrine_persistence::mapping::AbstractClassMetadataFactory;

const NS: &str = "Doctrine\\Tests\\ORM\\Tools\\Console\\";

/// `setUp()`.
fn set_up() -> ClassMetadataFactory {
    models::register();
    let driver = Arc::new(AttributeDriver::new(Vec::<String>::new(), true).unwrap());
    let cmf = ClassMetadataFactory::new();
    cmf.set_entity_manager(FactoryConfiguration::new(driver));
    cmf
}

fn metadata(cmf: &ClassMetadataFactory, short: &str) -> Arc<ClassMetadata> {
    cmf.get_metadata_for(&format!("{NS}MetadataFilterTestEntity{short}"))
        .unwrap()
}

fn contains(metadatas: &[Arc<ClassMetadata>], metadata: &Arc<ClassMetadata>) -> bool {
    metadatas.iter().any(|m| Arc::ptr_eq(m, metadata))
}

#[test]
fn testFilterWithEmptyArray() {
    let cmf = set_up();
    let metadata_aaa = metadata(&cmf, "Aaa");
    let metadata_bbb = metadata(&cmf, "Bbb");
    let original_metadatas = vec![Arc::clone(&metadata_aaa), Arc::clone(&metadata_bbb)];

    let metadatas = filter(original_metadatas.clone(), &[] as &[&str]).unwrap();

    assert!(contains(&metadatas, &metadata_aaa));
    assert!(contains(&metadatas, &metadata_bbb));
    assert_eq!(original_metadatas.len(), metadatas.len());
}

#[test]
fn testFilterWithString() {
    let cmf = set_up();
    let metadata_aaa = metadata(&cmf, "Aaa");
    let metadata_bbb = metadata(&cmf, "Bbb");
    let metadata_ccc = metadata(&cmf, "Ccc");
    let original_metadatas = vec![
        Arc::clone(&metadata_aaa),
        Arc::clone(&metadata_bbb),
        Arc::clone(&metadata_ccc),
    ];

    let metadatas = filter(original_metadatas.clone(), &["MetadataFilterTestEntityAaa"]).unwrap();

    assert!(contains(&metadatas, &metadata_aaa));
    assert!(!contains(&metadatas, &metadata_bbb));
    assert!(!contains(&metadatas, &metadata_ccc));
    assert_eq!(1, metadatas.len());

    let metadatas = filter(original_metadatas.clone(), &["MetadataFilterTestEntityBbb"]).unwrap();

    assert!(!contains(&metadatas, &metadata_aaa));
    assert!(contains(&metadatas, &metadata_bbb));
    assert!(!contains(&metadatas, &metadata_ccc));
    assert_eq!(1, metadatas.len());

    let metadatas = filter(original_metadatas, &["MetadataFilterTestEntityCcc"]).unwrap();

    assert!(!contains(&metadatas, &metadata_aaa));
    assert!(!contains(&metadatas, &metadata_bbb));
    assert!(contains(&metadatas, &metadata_ccc));
    assert_eq!(1, metadatas.len());
}

#[test]
fn testFilterWithString2() {
    let cmf = set_up();
    let metadata_foo = metadata(&cmf, "Foo");
    let metadata_foo_bar = metadata(&cmf, "FooBar");
    let metadata_bar = metadata(&cmf, "Bar");
    let original_metadatas = vec![
        Arc::clone(&metadata_foo),
        Arc::clone(&metadata_foo_bar),
        Arc::clone(&metadata_bar),
    ];

    let metadatas = filter(original_metadatas, &["MetadataFilterTestEntityFoo"]).unwrap();

    assert!(contains(&metadatas, &metadata_foo));
    assert!(contains(&metadatas, &metadata_foo_bar));
    assert!(!contains(&metadatas, &metadata_bar));
    assert_eq!(2, metadatas.len());
}

#[test]
fn testFilterWithArray() {
    let cmf = set_up();
    let metadata_aaa = metadata(&cmf, "Aaa");
    let metadata_bbb = metadata(&cmf, "Bbb");
    let metadata_ccc = metadata(&cmf, "Ccc");
    let original_metadatas = vec![
        Arc::clone(&metadata_aaa),
        Arc::clone(&metadata_bbb),
        Arc::clone(&metadata_ccc),
    ];

    let metadatas = filter(
        original_metadatas,
        &["MetadataFilterTestEntityAaa", "MetadataFilterTestEntityCcc"],
    )
    .unwrap();

    assert!(contains(&metadatas, &metadata_aaa));
    assert!(!contains(&metadatas, &metadata_bbb));
    assert!(contains(&metadatas, &metadata_ccc));
    assert_eq!(2, metadatas.len());
}

#[test]
fn testFilterWithRegex() {
    let cmf = set_up();
    let metadata_foo = metadata(&cmf, "Foo");
    let metadata_foo_bar = metadata(&cmf, "FooBar");
    let metadata_bar = metadata(&cmf, "Bar");
    let original_metadatas = vec![
        Arc::clone(&metadata_foo),
        Arc::clone(&metadata_foo_bar),
        Arc::clone(&metadata_bar),
    ];

    let metadatas = filter(original_metadatas.clone(), &["Foo$"]).unwrap();

    assert!(contains(&metadatas, &metadata_foo));
    assert!(!contains(&metadatas, &metadata_foo_bar));
    assert!(!contains(&metadatas, &metadata_bar));
    assert_eq!(1, metadatas.len());

    let metadatas = filter(original_metadatas, &["Bar$"]).unwrap();

    assert!(!contains(&metadatas, &metadata_foo));
    assert!(contains(&metadatas, &metadata_foo_bar));
    assert!(contains(&metadatas, &metadata_bar));
    assert_eq!(2, metadatas.len());
}
