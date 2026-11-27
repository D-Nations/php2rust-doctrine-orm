//! Port of `GH10473Test`: the listener's `addEventSubscriber()` is
//! `ResolveTargetEntityListener::subscribe()`.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH10473Test.php

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
#[path = "common/ticket.rs"]
mod ticket;

use std::sync::Arc;

use php_value::PhpArray;
use php2rust_doctrine_orm::tools::ResolveTargetEntityListener;

#[test]
fn testMappedSuperclassAssociationsCanBeResolvedToEntities() {
    models::register();
    let em = orm::entity_manager(orm::connection(), None);

    let resolve_target_entity = Arc::new(ResolveTargetEntityListener::new());

    resolve_target_entity.add_resolve_target_entity(
        &ticket::class("GH10473BaseUser"),
        &ticket::class("GH10473UserImplementation"),
        PhpArray::new(),
    );

    resolve_target_entity.subscribe(&em).unwrap();

    let user_implementation = ticket::class("GH10473UserImplementation");
    let social_media_account = ticket::class("GH10473SocialMediaAccount");

    let user_metadata = em.get_class_metadata(&user_implementation).unwrap();
    let user_metadata = user_metadata.read();

    assert!(!user_metadata.is_mapped_superclass);
    assert!(user_metadata.is_inheritance_type_none());

    let social_media_accounts_mapping = user_metadata
        .get_association_mapping("socialMediaAccounts")
        .unwrap();
    assert_eq!(None, social_media_accounts_mapping.inherited);
    assert!(social_media_accounts_mapping.is_to_many());
    assert!(!social_media_accounts_mapping.is_owning_side());
    assert_eq!(
        social_media_account,
        social_media_accounts_mapping.target_entity
    );
    assert_eq!(
        Some("user"),
        social_media_accounts_mapping.mapped_by.as_deref()
    );

    let created_by_mapping = user_metadata.get_association_mapping("createdBy").unwrap();
    assert_eq!(None, created_by_mapping.inherited);
    assert!(created_by_mapping.is_to_one());
    assert!(created_by_mapping.is_owning_side());
    assert_eq!(user_implementation, created_by_mapping.target_entity);
    assert_eq!(
        Some("createdUsers"),
        created_by_mapping.inversed_by.as_deref()
    );

    let created_users_mapping = user_metadata
        .get_association_mapping("createdUsers")
        .unwrap();
    assert_eq!(None, created_users_mapping.inherited);
    assert!(created_users_mapping.is_to_many());
    assert!(!created_users_mapping.is_owning_side());
    assert_eq!(user_implementation, created_users_mapping.target_entity);
    assert_eq!(
        Some("createdBy"),
        created_users_mapping.mapped_by.as_deref()
    );

    let social_media_account_metadata = em.get_class_metadata(&social_media_account).unwrap();
    let social_media_account_metadata = social_media_account_metadata.read();

    assert!(!social_media_account_metadata.is_mapped_superclass);
    assert!(social_media_account_metadata.is_inheritance_type_none());

    let user_mapping = social_media_account_metadata
        .get_association_mapping("user")
        .unwrap();
    assert_eq!(None, user_mapping.inherited);
    assert!(user_mapping.is_to_one());
    assert!(user_mapping.is_owning_side());
    assert_eq!(user_implementation, user_mapping.target_entity);
    assert_eq!(
        Some("socialMediaAccounts"),
        user_mapping.inversed_by.as_deref()
    );
}
