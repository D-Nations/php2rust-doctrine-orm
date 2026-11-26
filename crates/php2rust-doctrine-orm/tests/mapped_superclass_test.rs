//! MappedSuperclassTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa.
//! The `directorytree` model set: entities that extend a mapped
//! superclass.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/MappedSuperclassTest.php

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

use php_value::Object;
use php2rust_doctrine_persistence::php_reflection;

use orm::*;

const ABSTRACT_CONTENT_ITEM: &str = "Doctrine\\Tests\\Models\\DirectoryTree\\AbstractContentItem";
const FILE: &str = "Doctrine\\Tests\\Models\\DirectoryTree\\File";
const DIRECTORY: &str = "Doctrine\\Tests\\Models\\DirectoryTree\\Directory";

/// `new Directory($parent)` / `new File($parent)`.
fn content_item(class: &str, parent: Option<&Object>) -> Object {
    let item = new_object(class);
    if let Some(parent) = parent {
        set_obj(&item, "parentDirectory", parent);
    }
    item
}

#[tokio::test]
async fn testCRUD() {
    let em = set_up(&[ABSTRACT_CONTENT_ITEM, FILE, DIRECTORY]).await;
    let root = content_item(DIRECTORY, None);
    set(&root, "name", s("Root"));
    set(&root, "path", s("/root"));

    let directory = content_item(DIRECTORY, Some(&root));
    set(&directory, "name", s("TestA"));
    set(&directory, "path", s("/root/dir"));

    let file = content_item(FILE, Some(&directory));
    set(&file, "name", s("test-b.html"));

    em.persist(&root).unwrap();
    em.persist(&directory).unwrap();
    em.persist(&file).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();

    let clean_file = em
        .find(file.class(), get(&file, "id"))
        .await
        .unwrap()
        .unwrap();

    let parent = get(&clean_file, "parentDirectory");
    assert!(php_reflection::is_a(obj(&parent).class(), DIRECTORY));
    assert!(em.is_uninitialized_object(&parent));
    assert_loose_eq(&get(&directory, "id"), &get(&obj(&parent), "id"));
    // $cleanFile->getParent()->getParent(): PHP initializes the lazy
    // object when the property is read; the port's reads do not query.
    em.initialize_object(&obj(&parent)).await.unwrap();
    let grand_parent = get_object(&obj(&parent), "parentDirectory").unwrap();
    assert!(php_reflection::is_a(grand_parent.class(), DIRECTORY));
    assert_loose_eq(&get(&root, "id"), &get(&grand_parent, "id"));
}
