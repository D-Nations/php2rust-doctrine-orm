//! `Doctrine\ORM\Query\QueryException` and `Query\AST\ASTException`: the
//! exceptions of DQL parsing and SQL generation, as [`Thrown`] values of
//! their class (PHP's previous exception is not kept: `Thrown` has none).
//!
//! Upstream: doctrine/orm src/Query/QueryException.php src/Query/AST/ASTException.php src/Exception/NoMatchingPropertyException.php src/Exception/DuplicateFieldException.php

use php_value::Thrown;

use crate::mapping::AssociationMapping;

/// `QueryException::class`.
pub const CLASS: &str = "Doctrine\\ORM\\Query\\QueryException";
/// `ASTException::class`.
pub const AST_EXCEPTION: &str = "Doctrine\\ORM\\Query\\AST\\ASTException";
/// `NoMatchingPropertyException::class`.
pub const NO_MATCHING_PROPERTY: &str = "Doctrine\\ORM\\Exception\\NoMatchingPropertyException";
/// `DuplicateFieldException::class`.
pub const DUPLICATE_FIELD: &str = "Doctrine\\ORM\\Exception\\DuplicateFieldException";

/// `new QueryException($message)`.
pub fn new(message: impl Into<String>) -> Thrown {
    Thrown::new(CLASS, message.into(), 0)
}

pub fn dql_error(dql: &str) -> Thrown {
    new(dql)
}

pub fn syntax_error(message: &str) -> Thrown {
    new(format!("[Syntax Error] {message}"))
}

pub fn semantical_error(message: &str) -> Thrown {
    new(format!("[Semantical Error] {message}"))
}

pub fn invalid_lock_mode() -> Thrown {
    new("Invalid lock mode hint provided.")
}

pub fn invalid_parameter_type(expected: &str, received: &str) -> Thrown {
    new(format!(
        "Invalid parameter type, {received} given, but {expected} expected."
    ))
}

pub fn invalid_parameter_position(pos: &str) -> Thrown {
    new(format!("Invalid parameter position: {pos}"))
}

pub fn too_many_parameters(expected: i64, received: i64) -> Thrown {
    new(format!(
        "Too many parameters: the query defines {expected} parameters and you bound {received}"
    ))
}

pub fn too_few_parameters(expected: i64, received: i64) -> Thrown {
    new(format!(
        "Too few parameters: the query defines {expected} parameters but you only bound {received}"
    ))
}

pub fn invalid_parameter_format(value: &str) -> Thrown {
    new(format!(
        "Invalid parameter format, {value} given, but :<name> or ?<num> expected."
    ))
}

pub fn unknown_parameter(key: &str) -> Thrown {
    new(format!(
        "Invalid parameter: token {key} is not defined in the query."
    ))
}

pub fn parameter_type_mismatch() -> Thrown {
    new("DQL Query parameter and type numbers mismatch, but have to be exactly equal.")
}

/// `invalidPathExpression($pathExpr)`.
pub fn invalid_path_expression(identification_variable: &str, field: Option<&str>) -> Thrown {
    new(format!(
        "Invalid PathExpression '{identification_variable}.{}'.",
        field.unwrap_or("")
    ))
}

/// `invalidLiteral($literal)`: `$literal` as a string.
pub fn invalid_literal(literal: &str) -> Thrown {
    new(format!("Invalid literal '{literal}'"))
}

pub fn iterate_with_fetch_join_collection_not_allowed(assoc: &AssociationMapping) -> Thrown {
    new(format!(
        "Invalid query operation: Not allowed to iterate over fetch join collections in class {} association {}",
        assoc.source_entity, assoc.field_name
    ))
}

pub fn partial_objects_are_dangerous() -> Thrown {
    new(
        "Loading partial objects is dangerous. Fetch full objects or consider using a different fetch mode. If you really want partial objects, set the doctrine.forcePartialLoad query hint to TRUE.",
    )
}

/// `overwritingJoinConditionsNotYetSupported($assoc)`: the
/// `sourceEntityName` and `fieldName` of the array.
pub fn overwriting_join_conditions_not_yet_supported(
    source_entity_name: &str,
    field_name: &str,
) -> Thrown {
    new(format!(
        "Unsupported query operation: It is not yet possible to overwrite the join conditions in class {source_entity_name} association {field_name}. Use WITH to append additional join conditions to the association."
    ))
}

/// `associationPathInverseSideNotSupported($pathExpr)`.
pub fn association_path_inverse_side_not_supported(
    identification_variable: &str,
    field: Option<&str>,
) -> Thrown {
    new(format!(
        "A single-valued association path expression to an inverse side is not supported in DQL queries. Instead of \"{identification_variable}.{}\" use an explicit join.",
        field.unwrap_or("")
    ))
}

pub fn iterate_with_fetch_join_not_allowed(assoc: &AssociationMapping) -> Thrown {
    new(format!(
        "Iterate with fetch join in class {} using association {} not allowed.",
        assoc.source_entity, assoc.field_name
    ))
}

pub fn eager_fetch_join_with_not_allowed(source_entity: &str, field_name: &str) -> Thrown {
    new(format!(
        "Associations with fetch-mode=EAGER may not be using WITH conditions in\n             \"{source_entity}#{field_name}\"."
    ))
}

pub fn iterate_with_mixed_result_not_allowed() -> Thrown {
    new("Iterating a query with mixed results (using scalars) is not supported.")
}

pub fn association_path_composite_key_not_supported() -> Thrown {
    new(
        "A single-valued association path expression to an entity with a composite primary key is not supported. Explicitly name the components of the composite primary key in the query.",
    )
}

pub fn instance_of_unrelated_class(class_name: &str, root_class: &str) -> Thrown {
    new(format!(
        "Cannot check if a child of '{root_class}' is instanceof '{class_name}', inheritance hierarchy does not exists between these two classes."
    ))
}

pub fn invalid_query_component(dql_alias: &str) -> Thrown {
    new(format!(
        "Invalid query component given for DQL alias '{dql_alias}', requires 'metadata', 'parent', 'relation', 'map', 'nestingLevel' and 'token' keys."
    ))
}

/// `ASTException::noDispatchForNode($node)`.
pub fn no_dispatch_for_node(debug_type: &str) -> Thrown {
    Thrown::new(
        AST_EXCEPTION,
        format!("Double-dispatch for node {debug_type} is not supported."),
        0,
    )
}

/// `NoMatchingPropertyException::create($property)`.
pub fn no_matching_property(property: &str) -> Thrown {
    Thrown::new(
        NO_MATCHING_PROPERTY,
        format!(
            "Column name \"{property}\" does not match any property name. Consider aliasing it to the name of an existing property."
        ),
        0,
    )
}

/// `DuplicateFieldException::create($argName, $columnName)`.
pub fn duplicate_field(arg_name: &str, column_name: &str) -> Thrown {
    Thrown::new(
        DUPLICATE_FIELD,
        format!("Name \"{arg_name}\" for \"{column_name}\" already in use."),
        0,
    )
}

/// `new LogicException($message)`.
pub(crate) fn logic(message: impl Into<String>) -> Thrown {
    Thrown::new("LogicException", message.into(), 0)
}

/// `OptimisticLockException::lockFailed($entity)`.
pub(crate) fn optimistic_lock_failed() -> Thrown {
    Thrown::new(
        "Doctrine\\ORM\\OptimisticLockException",
        "The optimistic lock on an entity failed.",
        0,
    )
}

/// Declares the query exception classes in persistence's class registry.
pub(crate) fn register_classes() {
    use php2rust_doctrine_persistence::php_reflection::{ClassInfo, register_class};

    register_class(
        ClassInfo::class(CLASS)
            .extends("Exception")
            .implements("Doctrine\\ORM\\Exception\\ORMException"),
    );
    register_class(ClassInfo::class(AST_EXCEPTION).extends(CLASS));
    for class in [NO_MATCHING_PROPERTY, DUPLICATE_FIELD] {
        register_class(
            ClassInfo::class(class)
                .extends("LogicException")
                .implements("Doctrine\\ORM\\Exception\\ORMException"),
        );
    }
    register_class(
        ClassInfo::class("Doctrine\\ORM\\OptimisticLockException")
            .extends("Exception")
            .implements("Doctrine\\ORM\\Exception\\ORMException"),
    );
}
