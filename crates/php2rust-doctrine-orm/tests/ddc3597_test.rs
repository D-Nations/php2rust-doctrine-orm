//! Ticket/DDC3597Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the Models/DDC3597 models).
//!
//! The lifecycle callbacks of `DDC3597Root` are registered with
//! `register_method`.
//!
//! Upstream: doctrine/orm tests/Tests/Models/DDC3597/DDC3597Root.php
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC3597Test.php

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

use php_value::Value;
use php_value::datetime::{DateTime, DateTimeImmutable, TimeZone};
use php2rust_doctrine_orm::object::register_method;

use orm::*;

const ROOT: &str = "Doctrine\\Tests\\Models\\DDC3597\\DDC3597Root";
const MEDIA: &str = "Doctrine\\Tests\\Models\\DDC3597\\DDC3597Media";
const IMAGE: &str = "Doctrine\\Tests\\Models\\DDC3597\\DDC3597Image";
const DIMENSION: &str = "Doctrine\\Tests\\Models\\DDC3597\\Embeddable\\DDC3597Dimension";

fn register_callbacks() {
    // $this->updatedAt = $this->createdAt = new DateTime();
    register_method(
        ROOT,
        "prePersist",
        Arc::new(|entity, _| {
            let now = DateTime::object(DateTimeImmutable::now(TimeZone::Utc));
            set(entity, "createdAt", now.clone());
            set(entity, "updatedAt", now);
            Ok(())
        }),
    );
    // $this->updatedAt = new DateTime();
    register_method(
        ROOT,
        "preUpdate",
        Arc::new(|entity, _| {
            set(
                entity,
                "updatedAt",
                DateTime::object(DateTimeImmutable::now(TimeZone::Utc)),
            );
            Ok(())
        }),
    );
}

/// `DDC3597Root::getId()`: `(int) $this->id` (the deletion sets a
/// generated identifier to `null`).
fn get_id(entity: &php_value::Object) -> Value {
    match get(entity, "id") {
        Value::Null => Value::Int(0),
        id => id,
    }
}

#[tokio::test]
async fn testSaveImageEntity() {
    let em = orm::set_up(&[ROOT, MEDIA, IMAGE]).await;
    register_callbacks();

    // new DDC3597Image('foobar')
    let image_entity = new_object(IMAGE);
    set(&image_entity, "distributionHash", s("foobar"));
    set(&image_entity, "size", Value::Int(0));
    let dimension = new_object(DIMENSION);
    set(&dimension, "width", Value::Int(0));
    set(&dimension, "height", Value::Int(0));
    set_obj(&image_entity, "dimension", &dimension);

    set(&image_entity, "format", s("JPG"));
    set(&image_entity, "size", Value::Int(123));
    set(&dimension, "width", Value::Int(300));
    set(&dimension, "height", Value::Int(500));

    em.persist(&image_entity).unwrap();
    em.flush().await.unwrap(); //before this fix, it will fail with a exception

    em.clear().unwrap();

    //request entity
    let image_entity = em
        .find(IMAGE, get_id(&image_entity))
        .await
        .unwrap()
        .unwrap();
    assert!(image_entity.instance_of(IMAGE));

    //cleanup
    em.remove(&image_entity).await.unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    //check delete
    let image_entity = em.find(IMAGE, get_id(&image_entity)).await.unwrap();
    assert!(image_entity.is_none());
}
