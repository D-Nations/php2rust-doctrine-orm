//! Ticket/GH12225/GH12225Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH12225/GH12225Test.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH12225/AbstractDirectory.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH12225/ConcreteDirectory.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH12225/MyFilter.php

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

use php_value::{PhpArray, Thrown, Value};
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};
use php2rust_doctrine_orm::mapping::ClassMetadataInfo;
use php2rust_doctrine_orm::query::{FilterConstraint, SqlFilter, register_filter_class};

use orm::*;

const NS: &str = "Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH12225\\";

fn class(short: &str) -> String {
    format!("{NS}{short}")
}

/// `MyFilter extends SQLFilter`.
struct MyFilter;

impl FilterConstraint for MyFilter {
    fn add_filter_constraint(
        &self,
        _filter: &SqlFilter,
        _target_entity: &ClassMetadataInfo,
        target_table_alias: &str,
    ) -> Result<String, Thrown> {
        Ok(format!("{target_table_alias}.deleted_at IS NULL"))
    }
}

/// `new ConcreteDirectory($dirKey)`.
fn concrete_directory(dir_key: &str) -> php_value::Object {
    let directory = new_object(&class("ConcreteDirectory"));
    set(&directory, "dirKey", s(dir_key));
    set(
        &directory,
        "children",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    directory
}

#[tokio::test]
async fn testHydrateWithIndexByFilterAndInheritanceMapping() {
    let my_filter = class("MyFilter");
    register_filter_class(
        &my_filter,
        Arc::new(|em| SqlFilter::new(em, &class("MyFilter"), Box::new(MyFilter))),
    );
    let em = orm::set_up(&[&class("AbstractDirectory"), &class("ConcreteDirectory")]).await;

    // Enable the filter
    em.get_configuration().add_filter("my_filter", &my_filter);
    em.get_filters().enable("my_filter").unwrap();

    // Load entities into database
    let parent = concrete_directory("parent");
    let child = concrete_directory("child");
    set_obj(&child, "parent", &parent);
    em.persist(&parent).unwrap();
    em.persist(&child).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let repository = em.get_repository(&class("AbstractDirectory")).unwrap();

    // Fetch entities from database while changing filters
    em.get_filters().suspend("my_filter").unwrap();
    let mut criteria = PhpArray::new();
    criteria.insert("parent", Value::Null);
    let directories = repository
        .find_by(&criteria, None, None, None)
        .await
        .unwrap();
    em.get_filters().restore("my_filter").unwrap();

    // Ensure we got the parent directory
    assert_eq!(1, directories.len());
    assert_loose_eq(&s("parent"), &get(&directories[0], "dirKey"));

    // Try to hydrate all children of the parent directory (toArray is important here to initialize the collection)
    let children = coll::to_array(&get(&directories[0], "children"))
        .await
        .unwrap();
    assert_eq!(1, children.len());
}
