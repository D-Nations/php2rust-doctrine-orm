//! The ORM's exceptions the entity manager, the unit of work and the
//! persisters throw, as `php_value::Thrown` (class and message), with
//! upstream's message texts.
//!
//! `objToStr()` gives `get_debug_type($obj) . '@' . spl_object_id($obj)`:
//! the object id here is the object's address, not PHP's small integer.
//!
//! Upstream: doctrine/orm src/ORMInvalidArgumentException.php src/EntityNotFoundException.php
//! Upstream: doctrine/orm src/Exception/EntityManagerClosed.php src/Exception/MissingIdentifierField.php
//! Upstream: doctrine/orm src/Exception/UnrecognizedIdentifierFields.php src/Exception/EntityIdentityCollisionException.php
//! Upstream: doctrine/orm src/Exception/MissingMappingDriverImplementation.php src/Exception/UnexpectedAssociationValue.php
//! Upstream: doctrine/orm src/Exception/InvalidEntityRepository.php src/Exception/EntityMissingAssignedId.php
//! Upstream: doctrine/orm src/Exception/NotSupported.php src/Exception/ORMException.php
//! Upstream: doctrine/orm src/Persisters/Exception/CantUseInOperatorOnCompositeKeys.php
//! Upstream: doctrine/orm src/Persisters/Exception/InvalidOrientation.php src/Persisters/Exception/UnrecognizedField.php
//! Upstream: doctrine/orm src/Repository/Exception/InvalidFindByCall.php src/OptimisticLockException.php
//! Upstream: doctrine/orm src/TransactionRequiredException.php
//! Upstream: doctrine/orm src/Repository/Exception/InvalidMagicMethodCall.php src/Persisters/MatchingAssociationFieldRequiresObject.php
//! Upstream: doctrine/orm src/Exception/PersisterException.php

use php_value::{Object, Thrown};

use crate::internal::object_id;
use crate::mapping::AssociationMapping;

pub const ORM_INVALID_ARGUMENT_EXCEPTION: &str = "Doctrine\\ORM\\ORMInvalidArgumentException";
pub const ENTITY_NOT_FOUND_EXCEPTION: &str = "Doctrine\\ORM\\EntityNotFoundException";
pub const ENTITY_MANAGER_CLOSED: &str = "Doctrine\\ORM\\Exception\\EntityManagerClosed";
pub const MISSING_IDENTIFIER_FIELD: &str = "Doctrine\\ORM\\Exception\\MissingIdentifierField";
pub const UNRECOGNIZED_IDENTIFIER_FIELDS: &str =
    "Doctrine\\ORM\\Exception\\UnrecognizedIdentifierFields";
pub const ENTITY_IDENTITY_COLLISION_EXCEPTION: &str =
    "Doctrine\\ORM\\Exception\\EntityIdentityCollisionException";
pub const MISSING_MAPPING_DRIVER_IMPLEMENTATION: &str =
    "Doctrine\\ORM\\Exception\\MissingMappingDriverImplementation";
pub const UNEXPECTED_ASSOCIATION_VALUE: &str =
    "Doctrine\\ORM\\Exception\\UnexpectedAssociationValue";
pub const INVALID_ENTITY_REPOSITORY: &str = "Doctrine\\ORM\\Exception\\InvalidEntityRepository";
pub const ENTITY_MISSING_ASSIGNED_ID: &str = "Doctrine\\ORM\\Exception\\EntityMissingAssignedId";
pub const NOT_SUPPORTED: &str = "Doctrine\\ORM\\Exception\\NotSupported";
pub const CANT_USE_IN_OPERATOR_ON_COMPOSITE_KEYS: &str =
    "Doctrine\\ORM\\Persisters\\Exception\\CantUseInOperatorOnCompositeKeys";
pub const INVALID_ORIENTATION: &str = "Doctrine\\ORM\\Persisters\\Exception\\InvalidOrientation";
pub const UNRECOGNIZED_FIELD: &str = "Doctrine\\ORM\\Persisters\\Exception\\UnrecognizedField";
pub const MATCHING_ASSOCIATION_FIELD_REQUIRES_OBJECT: &str =
    "Doctrine\\ORM\\Persisters\\MatchingAssociationFieldRequiresObject";
pub const INVALID_FIND_BY_CALL: &str = "Doctrine\\ORM\\Repository\\Exception\\InvalidFindByCall";
pub const OPTIMISTIC_LOCK_EXCEPTION: &str = "Doctrine\\ORM\\OptimisticLockException";
pub const TRANSACTION_REQUIRED_EXCEPTION: &str = "Doctrine\\ORM\\TransactionRequiredException";
pub const INVALID_MAGIC_METHOD_CALL: &str =
    "Doctrine\\ORM\\Repository\\Exception\\InvalidMagicMethodCall";

fn new(class: &str, message: String) -> Thrown {
    Thrown::new(class, message, 0)
}

/// `get_debug_type($obj) . '@' . spl_object_id($obj)` (or `(string) $obj`
/// for a `Stringable` object).
pub fn obj_to_str(obj: &Object) -> String {
    match obj.to_php_string() {
        Some(s) => s.to_string_lossy().into_owned(),
        None => format!("{}@{}", obj.class(), object_id(obj)),
    }
}

// ORMInvalidArgumentException

pub fn invalid_auto_generate_mode(value: i64) -> Thrown {
    new(
        ORM_INVALID_ARGUMENT_EXCEPTION,
        format!("Invalid auto generate mode \"{value}\" given."),
    )
}

pub fn schedule_insert_for_managed_entity(entity: &Object) -> Thrown {
    new(
        ORM_INVALID_ARGUMENT_EXCEPTION,
        format!(
            "A managed+dirty entity {} can not be scheduled for insertion.",
            obj_to_str(entity)
        ),
    )
}

pub fn schedule_insert_for_removed_entity(entity: &Object) -> Thrown {
    new(
        ORM_INVALID_ARGUMENT_EXCEPTION,
        format!(
            "Removed entity {} can not be scheduled for insertion.",
            obj_to_str(entity)
        ),
    )
}

pub fn schedule_insert_twice(entity: &Object) -> Thrown {
    new(
        ORM_INVALID_ARGUMENT_EXCEPTION,
        format!(
            "Entity {} can not be scheduled for insertion twice.",
            obj_to_str(entity)
        ),
    )
}

pub fn entity_without_identity(class_name: &str, entity: &Object) -> Thrown {
    new(
        ORM_INVALID_ARGUMENT_EXCEPTION,
        format!(
            "The given entity of type '{class_name}' ({}) has no identity/no id values set. It cannot be added to the identity map.",
            obj_to_str(entity)
        ),
    )
}

pub fn read_only_requires_managed_entity(entity: &Object) -> Thrown {
    new(
        ORM_INVALID_ARGUMENT_EXCEPTION,
        format!(
            "Only managed entities can be marked or checked as read only. But {} is not",
            obj_to_str(entity)
        ),
    )
}

fn new_entity_found_through_relationship_message(
    association: &AssociationMapping,
    entity: &Object,
) -> String {
    let mut message = format!(
        "A new entity was found through the relationship '{}#{}' that was not configured to cascade persist operations for entity: {}. To solve this issue: Either explicitly call EntityManager#persist() on this unknown entity or configure cascade persist this association in the mapping for example #[ORM\\ManyToOne(..., cascade: ['persist'])].",
        association.source_entity,
        association.field_name,
        obj_to_str(entity)
    );
    if !entity.is_stringable() {
        message.push_str(" If you cannot find out which entity causes the problem implement '");
        message.push_str(&association.target_entity);
        message.push_str("#__toString()' to get a clue.");
    }
    message
}

pub fn new_entities_found_through_relationships(
    entities: &[(AssociationMapping, Object)],
) -> Thrown {
    let messages: Vec<String> = entities
        .iter()
        .map(|(assoc, entity)| new_entity_found_through_relationship_message(assoc, entity))
        .collect();
    if messages.len() == 1 {
        return new(ORM_INVALID_ARGUMENT_EXCEPTION, messages[0].clone());
    }
    new(
        ORM_INVALID_ARGUMENT_EXCEPTION,
        format!(
            "Multiple non-persisted new entities were found through the given association graph:\n\n * {}",
            messages.join("\n * ")
        ),
    )
}

pub fn new_entity_found_through_relationship(
    association: &AssociationMapping,
    entry: &Object,
) -> Thrown {
    new(
        ORM_INVALID_ARGUMENT_EXCEPTION,
        new_entity_found_through_relationship_message(association, entry),
    )
}

pub fn detached_entity_found_through_relationship(
    assoc: &AssociationMapping,
    entry: &Object,
) -> Thrown {
    new(
        ORM_INVALID_ARGUMENT_EXCEPTION,
        format!(
            "A detached entity of type {} ({})  was found through the relationship '{}#{}' during cascading a persist operation.",
            assoc.target_entity,
            obj_to_str(entry),
            assoc.source_entity,
            assoc.field_name
        ),
    )
}

pub fn entity_not_managed(entity: &Object) -> Thrown {
    new(
        ORM_INVALID_ARGUMENT_EXCEPTION,
        format!(
            "Entity {} is not managed. An entity is managed if its fetched from the database or registered as new through EntityManager#persist",
            obj_to_str(entity)
        ),
    )
}

pub fn entity_has_no_identity(entity: &Object, operation: &str) -> Thrown {
    new(
        ORM_INVALID_ARGUMENT_EXCEPTION,
        format!(
            "Entity has no identity, therefore {operation} cannot be performed. {}",
            obj_to_str(entity)
        ),
    )
}

pub fn entity_is_removed(entity: &Object, operation: &str) -> Thrown {
    new(
        ORM_INVALID_ARGUMENT_EXCEPTION,
        format!(
            "Entity is removed, therefore {operation} cannot be performed. {}",
            obj_to_str(entity)
        ),
    )
}

pub fn detached_entity_cannot(entity: &Object, operation: &str) -> Thrown {
    new(
        ORM_INVALID_ARGUMENT_EXCEPTION,
        format!(
            "Detached entity {} cannot be {operation}",
            obj_to_str(entity)
        ),
    )
}

pub fn invalid_composite_identifier() -> Thrown {
    new(
        ORM_INVALID_ARGUMENT_EXCEPTION,
        "Binding an entity with a composite primary key to a query is not supported. You should split the parameter into the explicit fields and bind them separately.".to_owned(),
    )
}

pub fn invalid_identifier_binding_entity(class: &str) -> Thrown {
    new(
        ORM_INVALID_ARGUMENT_EXCEPTION,
        format!(
            "Binding entities to query parameters only allowed for entities that have an identifier.\nClass \"{class}\" does not have an identifier."
        ),
    )
}

/// `invalidAssociation()`: `debug_type` is `get_debug_type($actualValue)`.
pub fn invalid_association(
    target_class_name: &str,
    assoc: &AssociationMapping,
    debug_type: &str,
) -> Thrown {
    new(
        ORM_INVALID_ARGUMENT_EXCEPTION,
        format!(
            "Expected value of type \"{target_class_name}\" for association field \"{}#${}\", got \"{debug_type}\" instead.",
            assoc.source_entity, assoc.field_name
        ),
    )
}

pub fn missing_primary_key_value(class_name: &str, id_field: &str) -> Thrown {
    new(
        ORM_INVALID_ARGUMENT_EXCEPTION,
        format!("Missing value for primary key {id_field} on {class_name}"),
    )
}

// EntityNotFoundException

pub fn entity_not_found_from_class_name_and_identifier(
    class_name: &str,
    id: &[(String, String)],
) -> Thrown {
    let ids: Vec<String> = id.iter().map(|(k, v)| format!("{k}({v})")).collect();
    let suffix = if ids.is_empty() {
        String::new()
    } else {
        format!(" for IDs {}", ids.join(", "))
    };
    new(
        ENTITY_NOT_FOUND_EXCEPTION,
        format!("Entity of type '{class_name}'{suffix} was not found"),
    )
}

pub fn no_identifier_found(class_name: &str) -> Thrown {
    new(
        ENTITY_NOT_FOUND_EXCEPTION,
        format!("Unable to find \"{class_name}\" entity identifier associated with the UnitOfWork"),
    )
}

// Exception/*

pub fn entity_manager_closed() -> Thrown {
    new(
        ENTITY_MANAGER_CLOSED,
        "The EntityManager is closed.".to_owned(),
    )
}

pub fn missing_identifier_field(field_name: &str, class_name: &str) -> Thrown {
    new(
        MISSING_IDENTIFIER_FIELD,
        format!("The identifier {field_name} is missing for a query of {class_name}"),
    )
}

pub fn unrecognized_identifier_fields(class_name: &str, field_names: &[String]) -> Thrown {
    new(
        UNRECOGNIZED_IDENTIFIER_FIELDS,
        format!(
            "Unrecognized identifier fields: \"{}\" are not present on class \"{class_name}\".",
            field_names.join("', '")
        ),
    )
}

pub fn entity_identity_collision(existing: &Object, new_entity: &Object, id_hash: &str) -> Thrown {
    new(
        ENTITY_IDENTITY_COLLISION_EXCEPTION,
        format!(
            "While adding an entity of class {} with an ID hash of \"{id_hash}\" to the identity map,\nanother object of class {} was already present for the same ID. This exception\nis a safeguard against an internal inconsistency - IDs should uniquely map to\nentity object instances. This problem may occur if:\n\n- you use application-provided IDs and reuse ID values;\n- database-provided IDs are reassigned after truncating the database without\nclearing the EntityManager;\n- you might have been using EntityManager#getReference() to create a reference\nfor a nonexistent ID that was subsequently (by the RDBMS) assigned to another\nentity.\n\nOtherwise, it might be an ORM-internal inconsistency, please report it.",
            new_entity.class(),
            existing.class()
        ),
    )
}

pub fn missing_mapping_driver_implementation() -> Thrown {
    new(
        MISSING_MAPPING_DRIVER_IMPLEMENTATION,
        "It's a requirement to specify a Metadata Driver and pass it to Doctrine\\ORM\\Configuration::setMetadataDriverImpl().".to_owned(),
    )
}

pub fn unexpected_association_value(
    class: &str,
    association: &str,
    given: &str,
    expected: &str,
) -> Thrown {
    new(
        UNEXPECTED_ASSOCIATION_VALUE,
        format!(
            "Found entity of type {given} on association {class}#{association}, but expecting {expected}"
        ),
    )
}

pub fn invalid_entity_repository(class_name: &str) -> Thrown {
    new(
        INVALID_ENTITY_REPOSITORY,
        format!(
            "Invalid repository class '{class_name}'. It must be a Doctrine\\ORM\\EntityRepository."
        ),
    )
}

pub fn entity_missing_assigned_id(entity: &Object, field: &str) -> Thrown {
    new(
        ENTITY_MISSING_ASSIGNED_ID,
        format!(
            "Entity of type {} is missing an assigned ID for field  '{field}'. The identifier generation strategy for this entity requires the ID field to be populated before EntityManager#persist() is called. If you want automatically generated identifiers instead you need to adjust the metadata mapping accordingly.",
            entity.class()
        ),
    )
}

pub fn not_supported() -> Thrown {
    new(
        NOT_SUPPORTED,
        "This behaviour is (currently) not supported by Doctrine 2".to_owned(),
    )
}

// Persisters\Exception

pub fn cant_use_in_operator_on_composite_keys() -> Thrown {
    new(
        CANT_USE_IN_OPERATOR_ON_COMPOSITE_KEYS,
        "Can't use IN operator on entities that have composite keys.".to_owned(),
    )
}

pub fn invalid_orientation(class_name: &str, field: &str) -> Thrown {
    new(
        INVALID_ORIENTATION,
        format!("Invalid order by orientation specified for {class_name}#{field}"),
    )
}

pub fn unrecognized_field(class_name: &str, field: &str) -> Thrown {
    new(
        UNRECOGNIZED_FIELD,
        format!("Unrecognized field: {class_name}::${field}"),
    )
}

// Persisters\MatchingAssociationFieldRequiresObject

pub fn matching_association_field_requires_object(class: &str, association_name: &str) -> Thrown {
    new(
        MATCHING_ASSOCIATION_FIELD_REQUIRES_OBJECT,
        format!(
            "Cannot match on {class}::{association_name} with a non-object value. Matching objects by id is not compatible with matching on an in-memory collection, which compares objects by reference."
        ),
    )
}

// Repository\Exception\InvalidFindByCall

pub fn invalid_find_by_call_from_inverse_side_usage(
    entity_name: &str,
    association: &str,
) -> Thrown {
    new(
        INVALID_FIND_BY_CALL,
        format!(
            "You cannot search for the association field '{entity_name}#{association}', because it is the inverse side of an association. Find methods only work on owning side associations."
        ),
    )
}

// OptimisticLockException

pub fn optimistic_lock_failed(entity: &Object) -> Thrown {
    let _ = entity;
    new(
        OPTIMISTIC_LOCK_EXCEPTION,
        "The optimistic lock on an entity failed.".to_owned(),
    )
}

pub fn lock_failed_version_mismatch(expected: &str, actual: &str) -> Thrown {
    new(
        OPTIMISTIC_LOCK_EXCEPTION,
        format!(
            "The optimistic lock failed, version {expected} was expected, but is actually {actual}"
        ),
    )
}

pub fn optimistic_lock_commit_failed() -> Thrown {
    new(OPTIMISTIC_LOCK_EXCEPTION, "Commit failed".to_owned())
}

pub fn not_versioned(entity_name: &str) -> Thrown {
    new(
        OPTIMISTIC_LOCK_EXCEPTION,
        format!("Cannot obtain optimistic lock on unversioned entity {entity_name}"),
    )
}

pub fn transaction_required() -> Thrown {
    new(
        TRANSACTION_REQUIRED_EXCEPTION,
        "An open transaction is required for this operation.".to_owned(),
    )
}

pub fn because_field_not_found_in(entity_name: &str, field_name: &str, method: &str) -> Thrown {
    new(
        INVALID_MAGIC_METHOD_CALL,
        format!(
            "Entity '{entity_name}' has no field '{field_name}'. You can therefore not call '{method}' on the entities' repository."
        ),
    )
}

pub fn on_missing_parameter(method_name: &str) -> Thrown {
    new(
        INVALID_MAGIC_METHOD_CALL,
        format!("You need to pass a parameter to '{method_name}'"),
    )
}

/// A DBAL exception as a `Thrown` (its class and message).
pub fn from_dbal(e: &php2rust_doctrine_dbal::Exception) -> Thrown {
    Thrown::new(e.class(), e.message().clone(), 0)
}

/// A DBAL type conversion error as a `Thrown`.
pub fn from_conversion(e: &php2rust_doctrine_dbal::types::ConversionError) -> Thrown {
    match e {
        php2rust_doctrine_dbal::types::ConversionError::Thrown(t) => t.clone(),
        other => Thrown::new(
            other
                .class()
                .unwrap_or("Doctrine\\DBAL\\Types\\ConversionException"),
            other.to_string(),
            0,
        ),
    }
}

/// `InvalidHydrationMode::class`.
pub const INVALID_HYDRATION_MODE: &str = "Doctrine\\ORM\\Exception\\InvalidHydrationMode";

/// `InvalidHydrationMode::fromMode($mode)`.
pub fn invalid_hydration_mode(mode: &str) -> Thrown {
    new(
        INVALID_HYDRATION_MODE,
        format!("\"{mode}\" is an invalid hydration mode."),
    )
}
