//! Ticket/GH12505/GH12505Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH12505/GH12505Test.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH12505/Car.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH12505/Garage.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH12505/SoftDeleteFilter.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH12505/Vehicle.php

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

const NS: &str = "Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH12505\\";

fn class(short: &str) -> String {
    format!("{NS}{short}")
}

/// `SoftDeleteFilter extends SQLFilter`.
struct SoftDeleteFilter;

impl FilterConstraint for SoftDeleteFilter {
    fn add_filter_constraint(
        &self,
        _filter: &SqlFilter,
        target_entity: &ClassMetadataInfo,
        target_table_alias: &str,
    ) -> Result<String, Thrown> {
        if !target_entity.has_field("deletedAt") {
            return Ok(String::new());
        }

        Ok(format!("{target_table_alias}.deleted_at IS NULL"))
    }
}

/// `new Car($name)`.
fn car(name: &str) -> php_value::Object {
    let car = new_object(&class("Car"));
    set(&car, "name", s(name));
    car
}

#[tokio::test]
async fn testHydrateManyToManyIndexedByInheritedIdAfterFilterChange() {
    let soft_delete_filter = class("SoftDeleteFilter");
    register_filter_class(
        &soft_delete_filter,
        Arc::new(|em| SqlFilter::new(em, &class("SoftDeleteFilter"), Box::new(SoftDeleteFilter))),
    );
    let em = orm::set_up(&[&class("Vehicle"), &class("Car"), &class("Garage")]).await;

    em.get_configuration()
        .add_filter("soft_delete", &soft_delete_filter);
    em.get_filters().enable("soft_delete").unwrap();

    let sedan = car("sedan");
    let coupe = car("coupe");
    let garage = new_object(&class("Garage"));
    set(
        &garage,
        "cars",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    append(&garage, "cars", Value::Object(sedan.clone()));
    append(&garage, "cars", Value::Object(coupe.clone()));

    em.persist(&sedan).unwrap();
    em.persist(&coupe).unwrap();
    em.persist(&garage).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    // Load some cars while the filter is suspended ...
    em.get_filters().suspend("soft_delete").unwrap();
    em.get_repository(&class("Car"))
        .unwrap()
        .find_by(&PhpArray::new(), None, None, None)
        .await
        .unwrap();
    em.get_filters().restore("soft_delete").unwrap();

    // ... then load the indexed collection with it enabled again.
    let garage = em
        .get_repository(&class("Garage"))
        .unwrap()
        .find_one_by(&PhpArray::new(), None)
        .await
        .unwrap()
        .expect("a garage");
    let cars = coll::to_array(&get(&garage, "cars")).await.unwrap();
    assert_eq!(2, cars.len());

    // The keys must be the inherited ids.
    for (key, car) in cars.iter() {
        assert!(get(&obj(car), "id").identical(&key.to_value()));
    }
}
