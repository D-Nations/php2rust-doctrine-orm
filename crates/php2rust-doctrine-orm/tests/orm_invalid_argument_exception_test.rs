//! Port of `ORMInvalidArgumentExceptionTest`. `spl_object_id()` is the
//! port's object id (`internal::object_id()`, the object's address).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/ORMInvalidArgumentExceptionTest.php

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

use std::sync::Arc;

use php_value::object::StdClass;
use php_value::{Object, PhpArray, PhpObject, PhpString, php_array};
use php2rust_doctrine_orm::exception::{
    ORM_INVALID_ARGUMENT_EXCEPTION, new_entities_found_through_relationships,
};
use php2rust_doctrine_orm::internal::object_id;
use php2rust_doctrine_orm::mapping::AssociationMapping;
use php2rust_doctrine_orm::mapping::association_mapping::{AssociationClass, AssociationKind};

/// `new class { public function __toString(): string { ... } }`.
#[derive(Debug)]
struct Entity3;

impl PhpObject for Entity3 {
    fn class(&self) -> &'static str {
        "class@anonymous"
    }

    fn properties(&self) -> PhpArray {
        PhpArray::new()
    }

    fn to_php_string(&self) -> Option<PhpString> {
        Some(PhpString::from("ThisIsAStringRepresentationOfEntity3"))
    }
}

fn one_to_many(source: &str, field: &str, target: &str) -> AssociationMapping {
    AssociationMapping::from_mapping_array(
        AssociationClass::Final(AssociationKind::OneToMany),
        &php_array! {
            "sourceEntity" => source,
            "fieldName" => field,
            "targetEntity" => target,
        },
    )
    .unwrap()
}

/// A data set: its name, the new entities and the expected message.
type DataSet = (&'static str, Vec<(AssociationMapping, Object)>, String);

/// `newEntitiesFoundThroughRelationshipsErrorMessages()`.
fn new_entities_found_through_relationships_error_messages() -> Vec<DataSet> {
    let entity1: Object = Arc::new(StdClass::default());
    let entity2: Object = Arc::new(StdClass::default());
    let entity3: Object = Arc::new(Entity3);
    let association1 = one_to_many("foo1", "bar1", "baz1");
    let association2 = one_to_many("foo2", "bar2", "baz2");
    let association3 = one_to_many("foo3", "bar3", "baz3");

    vec![
        (
            "one entity found",
            vec![(association1.clone(), entity1.clone())],
            format!(
                "A new entity was found through the relationship 'foo1#bar1' that was not configured to cascade \
                 persist operations for entity: stdClass@{}\
                 . To solve this issue: Either explicitly call EntityManager#persist() on this unknown entity \
                 or configure cascade persist this association in the mapping for example \
                 #[ORM\\ManyToOne(..., cascade: ['persist'])]. If you cannot find out which entity causes the problem \
                 implement 'baz1#__toString()' to get a clue.",
                object_id(&entity1)
            ),
        ),
        (
            "two entities found",
            vec![
                (association1, entity1.clone()),
                (association2, entity2.clone()),
            ],
            format!(
                "Multiple non-persisted new entities were found through the given association graph:\n\n \
                 * A new entity was found through the relationship 'foo1#bar1' that was not configured to \
                 cascade persist operations for entity: stdClass@{}. \
                 To solve this issue: Either explicitly call EntityManager#persist() on this unknown entity \
                 or configure cascade persist this association in the mapping for example \
                 #[ORM\\ManyToOne(..., cascade: ['persist'])]. If you cannot find out which entity causes the problem \
                 implement 'baz1#__toString()' to get a clue.\n \
                 * A new entity was found through the relationship 'foo2#bar2' that was not configured to \
                 cascade persist operations for entity: stdClass@{}. To solve \
                 this issue: Either explicitly call EntityManager#persist() on this unknown entity or \
                 configure cascade persist this association in the mapping for example \
                 #[ORM\\ManyToOne(..., cascade: ['persist'])]. If you cannot find out which entity causes the problem \
                 implement 'baz2#__toString()' to get a clue.",
                object_id(&entity1),
                object_id(&entity2)
            ),
        ),
        (
            "two entities found, one is stringable",
            vec![(association3, entity3)],
            "A new entity was found through the relationship 'foo3#bar3' that was not configured to cascade \
             persist operations for entity: ThisIsAStringRepresentationOfEntity3\
             . To solve this issue: Either explicitly call EntityManager#persist() on this unknown entity \
             or configure cascade persist this association in the mapping for example \
             #[ORM\\ManyToOne(..., cascade: ['persist'])]."
                .to_owned(),
        ),
    ]
}

#[test]
fn testNewEntitiesFoundThroughRelationships() {
    for (name, new_entities, expected_message) in
        new_entities_found_through_relationships_error_messages()
    {
        let exception = new_entities_found_through_relationships(&new_entities);

        assert_eq!(exception.class, ORM_INVALID_ARGUMENT_EXCEPTION, "{name}");
        assert_eq!(
            exception.message.to_string_lossy(),
            expected_message,
            "{name}"
        );
    }
}
