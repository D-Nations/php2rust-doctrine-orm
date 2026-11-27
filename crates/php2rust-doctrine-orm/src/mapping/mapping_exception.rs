//! `Doctrine\ORM\Mapping\MappingException`: something is wrong with the
//! mapping setup. The exceptions are `php_value::Thrown`s of the class
//! [`CLASS`], which is registered as a subclass of persistence's
//! `MappingException` (so `is_mapping_exception()` holds for them).
//!
//! Upstream: doctrine/orm src/Mapping/MappingException.php src/Mapping/Exception/InvalidCustomGenerator.php src/Mapping/Exception/UnknownGeneratorType.php

use php_value::Thrown;
use php2rust_doctrine_persistence::php_reflection;

/// The class name.
pub const CLASS: &str = "Doctrine\\ORM\\Mapping\\MappingException";
/// `Doctrine\ORM\Mapping\Exception\InvalidCustomGenerator`.
pub const INVALID_CUSTOM_GENERATOR: &str =
    "Doctrine\\ORM\\Mapping\\Exception\\InvalidCustomGenerator";
/// `Doctrine\ORM\Mapping\Exception\UnknownGeneratorType`.
pub const UNKNOWN_GENERATOR_TYPE: &str = "Doctrine\\ORM\\Mapping\\Exception\\UnknownGeneratorType";

fn new(message: String) -> Thrown {
    crate::mapping::register_orm_classes();
    Thrown::new(CLASS, message, 0)
}

/// `$e instanceof Doctrine\ORM\Mapping\MappingException`.
pub fn is_mapping_exception(thrown: &Thrown) -> bool {
    crate::mapping::register_orm_classes();
    php_reflection::thrown_is_a(thrown, CLASS)
}

/// `get_parent_class($class)`.
fn parent_class(class: &str) -> Option<String> {
    php_reflection::class_info(class).and_then(|info| info.parent().map(str::to_owned))
}

/// `MappingException::identifierRequired()`.
pub fn identifier_required(entity_name: &str) -> Thrown {
    match parent_class(entity_name) {
        Some(parent) => new(format!(
            "No identifier/primary key specified for Entity \"{entity_name}\" sub class of \"{parent}\". Every Entity must have an identifier/primary key."
        )),
        None => new(format!(
            "No identifier/primary key specified for Entity \"{entity_name}\". Every Entity must have an identifier/primary key."
        )),
    }
}

/// `MappingException::classIsNotAValidEntityOrMappedSuperClass()`.
pub fn class_is_not_a_valid_entity_or_mapped_super_class(class_name: &str) -> Thrown {
    match parent_class(class_name) {
        Some(parent) => new(format!(
            "Class \"{class_name}\" sub class of \"{parent}\" is not a valid entity or mapped super class."
        )),
        None => new(format!(
            "Class \"{class_name}\" is not a valid entity or mapped super class."
        )),
    }
}

/// `MappingException::missingRequiredOption()`.
pub fn missing_required_option(field: &str, expected_option: &str, hint: &str) -> Thrown {
    let mut message = format!(
        "The mapping of field '{field}' is invalid: The option '{expected_option}' is required."
    );
    if !hint.is_empty() && hint != "0" {
        message.push_str(" (Hint: ");
        message.push_str(hint);
        message.push(')');
    }
    new(message)
}

/// `MappingException::fileMappingDriversRequireConfiguredDirectoryPath()`.
pub fn file_mapping_drivers_require_configured_directory_path(path: Option<&str>) -> Thrown {
    let path = match path {
        Some(path) if !path.is_empty() && path != "0" => format!("[{path}]"),
        Some(path) => path.to_owned(),
        None => String::new(),
    };
    new(format!(
        "File mapping drivers must have a valid directory path, however the given path {path} seems to be incorrect!"
    ))
}

/// `MappingException::invalidEntriesInDiscriminatorMap()`.
pub fn invalid_entries_in_discriminator_map(
    entries: &[String],
    owning_class: &str,
    enum_type: &str,
) -> Thrown {
    let entries: Vec<String> = entries.iter().map(|entry| format!("'{entry}'")).collect();
    new(format!(
        "The entries {} in the discriminator map of class '{owning_class}' do not correspond to enum cases of '{enum_type}'.",
        entries.join(", ")
    ))
}

/// `MappingException::duplicateDiscriminatorEntry()`: `$map` as
/// (value, class) pairs.
pub fn duplicate_discriminator_entry(
    class_name: &str,
    entries: &[String],
    map: &[(String, String)],
) -> Thrown {
    let pairs: Vec<String> = map.iter().map(|(a, b)| format!("'{a}': '{b}'")).collect();
    new(format!(
        "The entries {} in discriminator map of class '{class_name}' is duplicated. If the discriminator map is automatically generated you have to convert it to an explicit discriminator map now. The entries of the current map are: @DiscriminatorMap({{{}}})",
        entries.join(", "),
        pairs.join(", ")
    ))
}

/// `MappingException::invalidCascadeOption()`.
pub fn invalid_cascade_option(
    cascades: &[String],
    class_name: &str,
    property_name: &str,
) -> Thrown {
    let cascades: Vec<String> = cascades.iter().map(|e| format!("'{e}'")).collect();
    new(format!(
        "You have specified invalid cascade options for {class_name}::${property_name}: {}; available options: 'remove', 'persist', 'refresh', and 'detach'",
        cascades.join(", ")
    ))
}

/// `MappingException::invalidOverrideType()`: the given value's
/// `get_debug_type()`.
pub fn invalid_override_type(expected_type: &str, given_type: &str) -> Thrown {
    new(format!(
        "Expected {expected_type}, but {given_type} was given."
    ))
}

// `MappingException::fromLibXmlErrors()` is not ported (no XML driver).

/// `InvalidCustomGenerator::onClassNotConfigured()`.
pub fn invalid_custom_generator_on_class_not_configured() -> Thrown {
    crate::mapping::register_orm_classes();
    Thrown::new(
        INVALID_CUSTOM_GENERATOR,
        "Cannot instantiate custom generator, no class has been defined",
        0,
    )
}

/// `InvalidCustomGenerator::onMissingClass()`: the definition as PHP's
/// `var_export()` gives it.
pub fn invalid_custom_generator_on_missing_class(definition_export: &str) -> Thrown {
    crate::mapping::register_orm_classes();
    Thrown::new(
        INVALID_CUSTOM_GENERATOR,
        format!("Cannot instantiate custom generator : {definition_export}"),
        0,
    )
}

/// `UnknownGeneratorType::create()`.
pub fn unknown_generator_type(generator_type: i64) -> Thrown {
    crate::mapping::register_orm_classes();
    Thrown::new(
        UNKNOWN_GENERATOR_TYPE,
        format!("Unknown generator type: {generator_type}"),
        0,
    )
}

/// `MappingException::invalidAssociationType()`.
pub fn invalid_association_type(entity_name: &str, field_name: &str, ty: i64) -> Thrown {
    new(format!(
        "The association \"{entity_name}#{field_name}\" must be of type \"ClassMetadata::ONE_TO_MANY\", \"ClassMetadata::MANY_TO_MANY\" or \"ClassMetadata::MANY_TO_ONE\", \"{ty}\" given."
    ))
}

/// `MappingException::invalidInheritanceType()`.
pub fn invalid_inheritance_type(entity_name: &str, ty: i64) -> Thrown {
    new(format!(
        "The inheritance type '{ty}' specified for '{entity_name}' does not exist."
    ))
}

/// `MappingException::generatorNotAllowedWithCompositeId()`.
pub fn generator_not_allowed_with_composite_id() -> Thrown {
    new("Id generators can't be used with a composite id.".to_owned())
}

/// `MappingException::missingFieldName()`.
pub fn missing_field_name(entity: &str) -> Thrown {
    new(format!(
        "The field or association mapping misses the 'fieldName' attribute in entity '{entity}'."
    ))
}

/// `MappingException::missingTargetEntity()`.
pub fn missing_target_entity(field_name: &str) -> Thrown {
    new(format!(
        "The association mapping '{field_name}' misses the 'targetEntity' attribute."
    ))
}

/// `MappingException::missingSourceEntity()`.
pub fn missing_source_entity(field_name: &str) -> Thrown {
    new(format!(
        "The association mapping '{field_name}' misses the 'sourceEntity' attribute."
    ))
}

/// `MappingException::missingEmbeddedClass()`.
pub fn missing_embedded_class(field_name: &str) -> Thrown {
    new(format!(
        "The embed mapping '{field_name}' misses the 'class' attribute."
    ))
}

/// `MappingException::mappingFileNotFound()`.
pub fn mapping_file_not_found(entity_name: &str, file_name: &str) -> Thrown {
    new(format!(
        "No mapping file found named '{file_name}' for class '{entity_name}'."
    ))
}

/// `MappingException::invalidOverrideFieldName()`.
pub fn invalid_override_field_name(class_name: &str, field_name: &str) -> Thrown {
    new(format!(
        "Invalid field override named '{field_name}' for class '{class_name}'."
    ))
}

/// `MappingException::invalidOverrideFieldType()`.
pub fn invalid_override_field_type(class_name: &str, field_name: &str) -> Thrown {
    new(format!(
        "The column type of attribute '{field_name}' on class '{class_name}' could not be changed."
    ))
}

/// `MappingException::mappingNotFound()`.
pub fn mapping_not_found(class_name: &str, field_name: &str) -> Thrown {
    new(format!(
        "No mapping found for field '{field_name}' on class '{class_name}'."
    ))
}

/// `MappingException::queryNotFound()`.
pub fn query_not_found(class_name: &str, query_name: &str) -> Thrown {
    new(format!(
        "No query found named '{query_name}' on class '{class_name}'."
    ))
}

/// `MappingException::resultMappingNotFound()`.
pub fn result_mapping_not_found(class_name: &str, result_name: &str) -> Thrown {
    new(format!(
        "No result set mapping found named '{result_name}' on class '{class_name}'."
    ))
}

/// `MappingException::emptyQueryMapping()`.
pub fn empty_query_mapping(entity: &str, query_name: &str) -> Thrown {
    new(format!(
        "Query named \"{query_name}\" in \"{entity}\" could not be empty."
    ))
}

/// `MappingException::nameIsMandatoryForQueryMapping()`.
pub fn name_is_mandatory_for_query_mapping(class_name: &str) -> Thrown {
    new(format!(
        "Query name on entity class '{class_name}' is not defined."
    ))
}

/// `MappingException::missingQueryMapping()`.
pub fn missing_query_mapping(entity: &str, query_name: &str) -> Thrown {
    new(format!(
        "Query named \"{query_name}\" in \"{entity} requires a result class or result set mapping."
    ))
}

/// `MappingException::missingResultSetMappingEntity()`.
pub fn missing_result_set_mapping_entity(entity: &str, result_name: &str) -> Thrown {
    new(format!(
        "Result set mapping named \"{result_name}\" in \"{entity} requires a entity class name."
    ))
}

/// `MappingException::missingResultSetMappingFieldName()`.
pub fn missing_result_set_mapping_field_name(entity: &str, result_name: &str) -> Thrown {
    new(format!(
        "Result set mapping named \"{result_name}\" in \"{entity} requires a field name."
    ))
}

/// `MappingException::oneToManyRequiresMappedBy()`.
pub fn one_to_many_requires_mapped_by(entity_name: &str, field_name: &str) -> Thrown {
    new(format!(
        "OneToMany mapping on entity '{entity_name}' field '{field_name}' requires the 'mappedBy' attribute."
    ))
}

/// `MappingException::joinTableRequired()`.
pub fn join_table_required(field_name: &str) -> Thrown {
    new(format!(
        "The mapping of field '{field_name}' requires the 'joinTable' attribute."
    ))
}

/// `MappingException::invalidMapping()`.
pub fn invalid_mapping(field_name: &str) -> Thrown {
    new(format!("The mapping of field '{field_name}' is invalid."))
}

/// `MappingException::reflectionFailure()`.
pub fn reflection_failure(entity: &str) -> Thrown {
    new(format!("An error occurred in {entity}"))
}

/// `MappingException::joinColumnMustPointToMappedField()`.
pub fn join_column_must_point_to_mapped_field(class_name: &str, join_column: &str) -> Thrown {
    new(format!(
        "The column {join_column} must be mapped to a field in class {class_name} since it is referenced by a join column of another class."
    ))
}

/// `MappingException::joinColumnNotAllowedOnOneToOneInverseSide()`.
pub fn join_column_not_allowed_on_one_to_one_inverse_side(
    class_name: &str,
    field_name: &str,
) -> Thrown {
    new(format!(
        "{class_name}#{field_name} is a OneToOne inverse side, which does not allow join columns."
    ))
}

/// `MappingException::duplicateFieldMapping()`.
pub fn duplicate_field_mapping(entity: &str, field_name: &str) -> Thrown {
    new(format!(
        "Property \"{field_name}\" in \"{entity}\" was already declared, but it must be declared only once"
    ))
}

/// `MappingException::duplicateAssociationMapping()`.
pub fn duplicate_association_mapping(entity: &str, field_name: &str) -> Thrown {
    new(format!(
        "Property \"{field_name}\" in \"{entity}\" was already declared, but it must be declared only once"
    ))
}

/// `MappingException::duplicateQueryMapping()`.
pub fn duplicate_query_mapping(entity: &str, query_name: &str) -> Thrown {
    new(format!(
        "Query named \"{query_name}\" in \"{entity}\" was already declared, but it must be declared only once"
    ))
}

/// `MappingException::duplicateResultSetMapping()`.
pub fn duplicate_result_set_mapping(entity: &str, result_name: &str) -> Thrown {
    new(format!(
        "Result set mapping named \"{result_name}\" in \"{entity}\" was already declared, but it must be declared only once"
    ))
}

/// `MappingException::singleIdNotAllowedOnCompositePrimaryKey()`.
pub fn single_id_not_allowed_on_composite_primary_key(entity: &str) -> Thrown {
    new(format!(
        "Single id is not allowed on composite primary key in entity {entity}"
    ))
}

/// `MappingException::noIdDefined()`.
pub fn no_id_defined(entity: &str) -> Thrown {
    new(format!("No ID defined for entity {entity}"))
}

/// `MappingException::unsupportedOptimisticLockingType()`.
pub fn unsupported_optimistic_locking_type(
    entity: &str,
    field_name: &str,
    unsupported_type: &str,
) -> Thrown {
    new(format!(
        "Locking type \"{unsupported_type}\" (specified in \"{entity}\", field \"{field_name}\") is not supported by Doctrine."
    ))
}

/// `MappingException::invalidClassInDiscriminatorMap()`.
pub fn invalid_class_in_discriminator_map(class_name: &str, owning_class: &str) -> Thrown {
    new(format!(
        "Entity class '{class_name}' used in the discriminator map of class '{owning_class}' does not exist."
    ))
}

/// `MappingException::missingInheritanceTypeDeclaration()`.
pub fn missing_inheritance_type_declaration(
    root_entity_class: &str,
    child_entity_class: &str,
) -> Thrown {
    new(format!(
        "Entity class '{child_entity_class}' is a subclass of the root entity class '{root_entity_class}', but no inheritance mapping type was declared."
    ))
}

/// `MappingException::missingDiscriminatorMap()`.
pub fn missing_discriminator_map(class_name: &str) -> Thrown {
    new(format!(
        "Entity class '{class_name}' is using inheritance but no discriminator map was defined."
    ))
}

/// `MappingException::missingDiscriminatorColumn()`.
pub fn missing_discriminator_column(class_name: &str) -> Thrown {
    new(format!(
        "Entity class '{class_name}' is using inheritance but no discriminator column was defined."
    ))
}

/// `MappingException::invalidDiscriminatorColumnType()`.
pub fn invalid_discriminator_column_type(class_name: &str, ty: &str) -> Thrown {
    new(format!(
        "Discriminator column type on entity class '{class_name}' is not allowed to be '{ty}'. 'string' or 'integer' type variables are suggested!"
    ))
}

/// `MappingException::nameIsMandatoryForDiscriminatorColumns()`.
pub fn name_is_mandatory_for_discriminator_columns(class_name: &str) -> Thrown {
    new(format!(
        "Discriminator column name on entity class '{class_name}' is not defined."
    ))
}

/// `MappingException::cannotVersionIdField()`.
pub fn cannot_version_id_field(class_name: &str, field_name: &str) -> Thrown {
    new(format!(
        "Setting Id field '{field_name}' as versionable in entity class '{class_name}' is not supported."
    ))
}

/// `MappingException::duplicateColumnName()`.
pub fn duplicate_column_name(class_name: &str, column_name: &str) -> Thrown {
    new(format!(
        "Duplicate definition of column '{column_name}' on entity '{class_name}' in a field or discriminator column mapping."
    ))
}

/// `MappingException::illegalToManyAssociationOnMappedSuperclass()`.
pub fn illegal_to_many_association_on_mapped_superclass(class_name: &str, field: &str) -> Thrown {
    new(format!(
        "It is illegal to put an inverse side one-to-many or many-to-many association on mapped superclass '{class_name}#{field}'."
    ))
}

/// `MappingException::cannotMapCompositePrimaryKeyEntitiesAsForeignId()`.
pub fn cannot_map_composite_primary_key_entities_as_foreign_id(
    class_name: &str,
    target_entity: &str,
    target_field: &str,
) -> Thrown {
    new(format!(
        "It is not possible to map entity '{class_name}' with a composite primary key as part of the primary key of another entity '{target_entity}#{target_field}'."
    ))
}

/// `MappingException::noSingleAssociationJoinColumnFound()`.
pub fn no_single_association_join_column_found(class_name: &str, field: &str) -> Thrown {
    new(format!(
        "'{class_name}#{field}' is not an association with a single join column."
    ))
}

/// `MappingException::noFieldNameFoundForColumn()`.
pub fn no_field_name_found_for_column(class_name: &str, column: &str) -> Thrown {
    new(format!(
        "Cannot find a field on '{class_name}' that is mapped to column '{column}'. Either the field does not exist or an association exists but it has multiple join columns."
    ))
}

/// `MappingException::illegalOrphanRemovalOnIdentifierAssociation()`.
pub fn illegal_orphan_removal_on_identifier_association(class_name: &str, field: &str) -> Thrown {
    new(format!(
        "The orphan removal option is not allowed on an association that is part of the identifier in '{class_name}#{field}'."
    ))
}

/// `MappingException::illegalOrphanRemoval()`.
pub fn illegal_orphan_removal(class_name: &str, field: &str) -> Thrown {
    new(format!(
        "Orphan removal is only allowed on one-to-one and one-to-many associations, but {class_name}#{field} is not."
    ))
}

/// `MappingException::illegalInverseIdentifierAssociation()`.
pub fn illegal_inverse_identifier_association(class_name: &str, field: &str) -> Thrown {
    new(format!(
        "An inverse association is not allowed to be identifier in '{class_name}#{field}'."
    ))
}

/// `MappingException::illegalToManyIdentifierAssociation()`.
pub fn illegal_to_many_identifier_association(class_name: &str, field: &str) -> Thrown {
    new(format!(
        "Many-to-many or one-to-many associations are not allowed to be identifier in '{class_name}#{field}'."
    ))
}

/// `MappingException::noInheritanceOnMappedSuperClass()`.
pub fn no_inheritance_on_mapped_super_class(class_name: &str) -> Thrown {
    new(format!(
        "It is not supported to define inheritance information on a mapped superclass '{class_name}'."
    ))
}

/// `MappingException::mappedClassNotPartOfDiscriminatorMap()`.
pub fn mapped_class_not_part_of_discriminator_map(
    class_name: &str,
    root_class_name: &str,
) -> Thrown {
    new(format!(
        "Entity '{class_name}' has to be part of the discriminator map of '{root_class_name}' to be properly mapped in the inheritance hierarchy. Alternatively you can make '{class_name}' an abstract class to avoid this exception from occurring."
    ))
}

/// `MappingException::lifecycleCallbackMethodNotFound()`.
pub fn lifecycle_callback_method_not_found(class_name: &str, method_name: &str) -> Thrown {
    new(format!(
        "Entity '{class_name}' has no method '{method_name}' to be registered as lifecycle callback."
    ))
}

/// `MappingException::illegalLifecycleCallbackOnEmbeddedClass()`.
pub fn illegal_lifecycle_callback_on_embedded_class(event: &str, class_name: &str) -> Thrown {
    new(format!(
        "Context: Attempt to register lifecycle callback \"{event}\" on embedded class \"{class_name}\".\nProblem: Registering lifecycle callbacks on embedded classes is not allowed."
    ))
}

/// `MappingException::entityListenerClassNotFound()`.
pub fn entity_listener_class_not_found(listener_name: &str, class_name: &str) -> Thrown {
    new(format!(
        "Entity Listener \"{listener_name}\" declared on \"{class_name}\" not found."
    ))
}

/// `MappingException::entityListenerMethodNotFound()`.
pub fn entity_listener_method_not_found(
    listener_name: &str,
    method_name: &str,
    class_name: &str,
) -> Thrown {
    new(format!(
        "Entity Listener \"{listener_name}\" declared on \"{class_name}\" has no method \"{method_name}\"."
    ))
}

/// `MappingException::duplicateEntityListener()`.
pub fn duplicate_entity_listener(
    listener_name: &str,
    method_name: &str,
    class_name: &str,
) -> Thrown {
    new(format!(
        "Entity Listener \"{listener_name}#{method_name}()\" in \"{class_name}\" was already declared, but it must be declared only once."
    ))
}

/// `MappingException::invalidFetchMode()`.
pub fn invalid_fetch_mode(class_name: &str, fetch_mode: &str) -> Thrown {
    new(format!(
        "Entity '{class_name}' has a mapping with invalid fetch mode '{fetch_mode}'"
    ))
}

/// `MappingException::invalidGeneratedMode()`.
pub fn invalid_generated_mode(generated_mode: &str) -> Thrown {
    new(format!("Invalid generated mode '{generated_mode}'"))
}

/// `MappingException::compositeKeyAssignedIdGeneratorRequired()`.
pub fn composite_key_assigned_id_generator_required(class_name: &str) -> Thrown {
    new(format!(
        "Entity '{class_name}' has a composite identifier but uses an ID generator other than manually assigning (Identity, Sequence). This is not supported."
    ))
}

/// `MappingException::invalidTargetEntityClass()`.
pub fn invalid_target_entity_class(
    target_entity: &str,
    source_entity: &str,
    association_name: &str,
) -> Thrown {
    new(format!(
        "The target-entity {target_entity} cannot be found in '{source_entity}#{association_name}'."
    ))
}

/// `MappingException::missingSequenceName()`.
pub fn missing_sequence_name(class_name: &str) -> Thrown {
    new(format!(
        "Missing \"sequenceName\" attribute for sequence id generator definition on class \"{class_name}\"."
    ))
}

/// `MappingException::infiniteEmbeddableNesting()`.
pub fn infinite_embeddable_nesting(class_name: &str, property_name: &str) -> Thrown {
    new(format!(
        "Infinite nesting detected for embedded property {class_name}::{property_name}. You cannot embed an embeddable from the same type inside an embeddable."
    ))
}

/// `MappingException::illegalOverrideOfInheritedProperty()`.
pub fn illegal_override_of_inherited_property(
    class_name: &str,
    property_name: &str,
    inherit_from_class: &str,
) -> Thrown {
    new(format!(
        "Overrides are only allowed for fields or associations declared in mapped superclasses or traits. This is not the case for {class_name}::{property_name}, which was inherited from {inherit_from_class}."
    ))
}

/// `MappingException::invalidIndexConfiguration()`.
pub fn invalid_index_configuration(class_name: &str, index_name: &str) -> Thrown {
    new(format!(
        "Index {index_name} for entity {class_name} should contain columns or fields values, but not both."
    ))
}

/// `MappingException::invalidUniqueConstraintConfiguration()`.
pub fn invalid_unique_constraint_configuration(class_name: &str, index_name: &str) -> Thrown {
    new(format!(
        "Unique constraint {index_name} for entity {class_name} should contain columns or fields values, but not both."
    ))
}

/// `MappingException::backedEnumTypeRequired()`.
pub fn backed_enum_type_required(class_name: &str, field_name: &str, enum_type: &str) -> Thrown {
    new(format!(
        "Attempting to map a non-backed enum type {enum_type} in entity {class_name}::${field_name}. Please use backed enums only"
    ))
}

/// `MappingException::nonEnumTypeMapped()`.
pub fn non_enum_type_mapped(class_name: &str, field_name: &str, enum_type: &str) -> Thrown {
    new(format!(
        "Attempting to map non-enum type {enum_type} as enum in entity {class_name}::${field_name}"
    ))
}

/// `MappingException::invalidEnumValue()`.
pub fn invalid_enum_value(
    class_name: &str,
    field_name: &str,
    value: &str,
    enum_type: &str,
) -> Thrown {
    new(format!(
        "Context: Trying to hydrate enum property \"{class_name}::${field_name}\"\nProblem: Case \"{value}\" is not listed in enum \"{enum_type}\"\nSolution: Either add the case to the enum type or migrate the database column to use another case of the enum"
    ))
}

/// `MappingException::fromLibXmlErrors()`: one line per error (`PHP_EOL`
/// between them), with the error's file (`''` for none) and line.
pub fn from_lib_xml_errors(errors: &[php2rust_dom::LibxmlError]) -> Thrown {
    let lines: Vec<String> = errors
        .iter()
        .map(|error| {
            format!(
                "libxml error: {} in {} at line {}",
                error.message,
                error.file.as_deref().unwrap_or_default(),
                error.line
            )
        })
        .collect();
    new(lines.join("\n"))
}

/// `MappingException::invalidAttributeOnEmbeddable()`.
pub fn invalid_attribute_on_embeddable(entity_name: &str, attribute_name: &str) -> Thrown {
    new(format!(
        "Attribute \"{attribute_name}\" on embeddable \"{entity_name}\" is not allowed."
    ))
}

/// `MappingException::mappingVirtualPropertyNotAllowed()`.
pub fn mapping_virtual_property_not_allowed(entity_name: &str, property_name: &str) -> Thrown {
    new(format!(
        "Mapping virtual property \"{property_name}\" on entity \"{entity_name}\" is not allowed."
    ))
}

/// `MappingException::invalidOrderByDirection()`.
pub fn invalid_order_by_direction(
    direction: &str,
    order_by_field: &str,
    field_name: &str,
    class_name: &str,
) -> Thrown {
    new(format!(
        "Invalid order direction \"{direction}\" for field \"{order_by_field}\" in association \"{field_name}\" of class \"{class_name}\". Valid directions are \"ASC\" and \"DESC\"."
    ))
}
