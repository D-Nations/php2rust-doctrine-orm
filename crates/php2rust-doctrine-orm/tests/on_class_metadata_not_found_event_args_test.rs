//! Port of `OnClassMetadataNotFoundEventArgsTest`. The object manager is a
//! `SQLite` entity manager and the metadata a real `ClassMetadata`, in
//! place of the stubs.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Event/OnClassMetadataNotFoundEventArgsTest.php

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

use php2rust_doctrine_orm::event::OnClassMetadataNotFoundEventArgs;
use php2rust_doctrine_orm::mapping::ClassMetadata;

#[test]
fn testEventArgsMutability() {
    let object_manager = orm::entity_manager(orm::connection(), None);

    let args = OnClassMetadataNotFoundEventArgs::new("foo", Arc::clone(&object_manager));

    assert_eq!("foo", args.get_class_name());
    assert!(Arc::ptr_eq(&object_manager, args.get_object_manager()));

    assert!(args.get_found_metadata().is_none());

    let metadata = Arc::new(ClassMetadata::new("foo", None, None));

    args.set_found_metadata(Some(Arc::clone(&metadata)));

    assert!(Arc::ptr_eq(&metadata, &args.get_found_metadata().unwrap()));

    args.set_found_metadata(None);

    assert!(args.get_found_metadata().is_none());
}
