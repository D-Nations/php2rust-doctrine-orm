//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC2415Test.php
#![allow(non_snake_case)]
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "test helpers")]

#[path = "common/orm.rs"]
mod orm;
#[path = "common/models.rs"]
mod recorded_models;
#[path = "common/ticket.rs"]
mod ticket;
mod models {
    pub fn register() {
        super::recorded_models::register_with_load_metadata(super::loaders);
    }
}

use md5::{Digest, Md5};
use php_value::{Object, Thrown, Value, php_array};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::mapping::class_metadata::GENERATOR_TYPE_CUSTOM;
use php2rust_doctrine_orm::mapping::{ClassMetadata, ClassMetadataInfo};
use php2rust_doctrine_persistence::mapping::driver::{StaticPHPDriver, StaticPHPDriverPaths};
use php2rust_doctrine_persistence::php_reflection::LoadMetadataFn;
use std::fmt::Write as _;
use std::sync::Arc;

fn loader(load: fn(&mut ClassMetadataInfo) -> Result<(), Thrown>) -> LoadMetadataFn {
    Arc::new(move |metadata| {
        let metadata = metadata
            .downcast_ref::<ClassMetadata>()
            .expect("ORM metadata");
        load(&mut metadata.write())
    })
}

fn loaders() -> Vec<(String, LoadMetadataFn)> {
    vec![
        (ticket::class("DDC2415ParentEntity"), loader(parent)),
        (ticket::class("DDC2415ChildEntity"), loader(child)),
    ]
}

fn parent(metadata: &mut ClassMetadataInfo) -> Result<(), Thrown> {
    metadata.map_field(php_array! { "id" => true, "fieldName" => "id", "type" => "string" })?;
    metadata.set_id_generator_type(GENERATOR_TYPE_CUSTOM);
    metadata.set_custom_generator_definition(
        php_array! { "class" => ticket::class("DDC2415Generator") },
    );
    metadata.is_mapped_superclass = true;
    Ok(())
}

fn child(metadata: &mut ClassMetadataInfo) -> Result<(), Thrown> {
    metadata.map_field(php_array! { "fieldName" => "name", "type" => "string" })
}

#[allow(
    clippy::unnecessary_wraps,
    reason = "custom generator callback signature"
)]
fn generate_id(_em: &EntityManager, entity: Option<&Object>) -> Result<Value, Thrown> {
    let name = orm::get(entity.expect("entity"), "name")
        .to_php_string()
        .expect("name");
    Ok(Value::string(Md5::digest(name.as_bytes()).iter().fold(
        String::with_capacity(32),
        |mut hex, byte| {
            write!(hex, "{byte:02x}").unwrap();
            hex
        },
    )))
}

#[tokio::test]
async fn testTicket() {
    models::register();
    php2rust_doctrine_orm::id::register_custom_generator(
        &ticket::class("DDC2415Generator"),
        generate_id,
    );
    let em = orm::entity_manager_with(orm::connection(), |config| {
        config.set_metadata_driver_impl(Arc::new(StaticPHPDriver::new(
            StaticPHPDriverPaths::Paths(vec![]),
        )));
    });
    let parent = em
        .get_class_metadata(&ticket::class("DDC2415ParentEntity"))
        .unwrap();
    let child = em
        .get_class_metadata(&ticket::class("DDC2415ChildEntity"))
        .unwrap();
    assert_eq!(parent.read().generator_type, child.read().generator_type);
    assert_eq!(
        parent.read().custom_generator_definition,
        child.read().custom_generator_definition
    );
    assert_eq!(
        parent
            .read()
            .custom_generator_definition
            .as_ref()
            .unwrap()
            .get(&"class".into()),
        Some(&Value::string(ticket::class("DDC2415Generator")))
    );
    orm::create_schema(
        &em,
        &[
            &ticket::class("DDC2415ParentEntity"),
            &ticket::class("DDC2415ChildEntity"),
        ],
    )
    .await;
    let first = ticket::new("DDC2415ChildEntity");
    let second = ticket::new("DDC2415ChildEntity");
    orm::set(&first, "name", orm::s("ChildEntity 1"));
    orm::set(&second, "name", orm::s("ChildEntity 2"));
    em.persist(&first).unwrap();
    em.persist(&second).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();
    assert_eq!(
        orm::get(&first, "id"),
        orm::s("51908698d220616b3bc8ba06673b951b")
    );
    assert_eq!(
        orm::get(&second, "id"),
        orm::s("39944dcb86f86ce15ae8bcf8546bb0de")
    );
}
