//! `DatabaseDriverTestCase`: the metadata a `DatabaseDriver` reverse
//! engineers from tables, and the DBAL table helpers its tests build the
//! tables with (`new Table()`, `edit()->addPrimaryKeyConstraint()`,
//! `addForeignKeyConstraint()`).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/DatabaseDriverTestCase.php

#![allow(dead_code, reason = "each test binary uses a part of the helpers")]

use php_value::PhpArray;
use php2rust_doctrine_dbal::schema::name::UnqualifiedName;
use php2rust_doctrine_dbal::schema::{
    AbstractSchemaManager, Column, ForeignKeyConstraint, PrimaryKeyConstraint, Table,
};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::mapping::driver::DatabaseDriver;
use php2rust_doctrine_orm::mapping::{ClassMetadata, ClassMetadataInfo};
use php2rust_doctrine_persistence::mapping::driver::MappingDriver;

/// `new Column($name, Type::getType($type), $options)`.
pub fn column(name: &str, type_name: &str, options: &PhpArray) -> Column {
    Column::new(name, type_name, options).unwrap()
}

/// `new Table($name, $columns)`, with `edit()` adding the primary key
/// constraint (when there are primary key columns) and the foreign keys.
pub fn table(
    name: &str,
    columns: Vec<Column>,
    primary_key: &[&str],
    foreign_keys: Vec<ForeignKeyConstraint>,
) -> Table {
    let mut editor = Table::new(name)
        .unwrap()
        .edit()
        .unwrap()
        .set_columns(columns);
    if !primary_key.is_empty() {
        let names = primary_key
            .iter()
            .map(|column| UnqualifiedName::unquoted(column).unwrap())
            .collect();
        editor = editor
            .add_primary_key_constraint(PrimaryKeyConstraint::new(None, names, true).unwrap());
    }
    for foreign_key in foreign_keys {
        editor = editor.add_foreign_key_constraint(foreign_key);
    }
    editor.create().unwrap()
}

/// `new ForeignKeyConstraint($local, $foreignTable, $foreign)`.
pub fn foreign_key(local: &[&str], foreign_table: &str, foreign: &[&str]) -> ForeignKeyConstraint {
    ForeignKeyConstraint::new(
        local.iter().copied(),
        foreign_table,
        foreign.iter().copied(),
        "",
        PhpArray::new(),
    )
    .unwrap()
}

/// `new DatabaseDriver($this->createSchemaManager())`.
pub async fn driver(em: &EntityManager) -> DatabaseDriver {
    let sm = em.get_connection().create_schema_manager().await.unwrap();
    DatabaseDriver::new(&sm).await.unwrap()
}

/// `new ClassMetadata($className)` and `loadMetadataForClass()`.
fn load(driver: &DatabaseDriver, class_name: &str) -> ClassMetadataInfo {
    let class = ClassMetadata::from_info(ClassMetadataInfo::new(class_name, None, None));
    driver.load_metadata_for_class(class_name, &class).unwrap();
    class.read().clone()
}

/// `convertToClassMetadata()`.
pub async fn convert_to_class_metadata(
    em: &EntityManager,
    entity_tables: Vec<Table>,
    many_tables: Vec<Table>,
) -> Vec<(String, ClassMetadataInfo)> {
    let driver = driver(em).await;
    driver.set_tables(entity_tables, many_tables).unwrap();

    driver
        .get_all_class_names()
        .unwrap()
        .into_iter()
        .map(|class_name| {
            let class = load(&driver, &class_name);
            (class_name, class)
        })
        .collect()
}

/// `extractClassMetadata()`.
pub async fn extract_class_metadata(
    em: &EntityManager,
    class_names: &[&str],
) -> Vec<(String, ClassMetadataInfo)> {
    let class_names: Vec<String> = class_names.iter().map(|c| c.to_lowercase()).collect();

    let driver = driver(em).await;

    let metadatas: Vec<(String, ClassMetadataInfo)> = driver
        .get_all_class_names()
        .unwrap()
        .into_iter()
        .filter(|class_name| class_names.contains(&class_name.to_lowercase()))
        .map(|class_name| {
            let class = load(&driver, &class_name);
            (class_name, class)
        })
        .collect();

    let found: Vec<&str> = metadatas.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(
        class_names.len(),
        metadatas.len(),
        "Have not found all classes matching the names '{}' only tables {}",
        class_names.join(", "),
        found.join(", ")
    );

    metadatas
}

/// `$metadatas[$className]`.
pub fn metadata<'a>(
    metadatas: &'a [(String, ClassMetadataInfo)],
    class_name: &str,
) -> &'a ClassMetadataInfo {
    metadatas
        .iter()
        .find(|(name, _)| name == class_name)
        .map_or_else(
            || panic!("no metadata for {class_name}"),
            |(_, class)| class,
        )
}

/// `dropAndCreateTable()`.
pub async fn drop_and_create_table(em: &EntityManager, table: &Table) {
    let sm = em.get_connection().create_schema_manager().await.unwrap();
    let name = table.get_name();
    if sm.table_exists(&name).await.unwrap() {
        sm.drop_table(&name).await.unwrap();
    }
    sm.create_table(table).await.unwrap();
}
