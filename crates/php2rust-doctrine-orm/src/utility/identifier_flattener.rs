//! The `IdentifierFlattener` utility now houses some of the identifier
//! manipulation logic from the `UnitOfWork`.
//!
//! Upstream: doctrine/orm src/Utility/IdentifierFlattener.php

use php_value::{PhpArray, Thrown, Value};

use crate::entity_manager::EntityManager;
use crate::mapping::ClassMetadataInfo;

use super::{implode_values, to_text};

/// `flattenIdentifier()`: the identifier with the associated entities of
/// a derived identifier replaced by their (flattened) identifiers.
pub fn flatten_identifier(
    em: &EntityManager,
    class: &ClassMetadataInfo,
    id: &PhpArray,
) -> Result<PhpArray, Thrown> {
    let mut flat_id = PhpArray::new();
    for field in &class.identifier {
        let value = id.get(&field.as_str().into()).cloned();
        if let Some(assoc) = class.association_mappings.get(field) {
            if let Some(Value::Object(object)) = &value
                && object.instance_of(&assoc.target_entity)
            {
                let target = em.get_class_metadata(&assoc.target_entity)?;
                let uow = em.get_unit_of_work();
                let target_info = target.read();
                let associated = if uow.is_in_identity_map(object)? {
                    flatten_identifier(em, &target_info, &uow.get_entity_identifier(object)?)?
                } else {
                    let values = target_info.get_identifier_values(object)?;
                    flatten_identifier(em, &target_info, &values)?
                };
                flat_id.insert(
                    field.as_str(),
                    Value::from(implode_values(&associated).as_str()),
                );
                continue;
            }
            let mut associated = Vec::new();
            for join_column in &assoc.join_columns {
                associated.push(to_text(
                    id.get(&join_column.name.as_str().into())
                        .unwrap_or(&Value::Null),
                ));
            }
            flat_id.insert(field.as_str(), Value::from(associated.join(" ").as_str()));
            continue;
        }
        flat_id.insert(field.as_str(), backed_value(value.unwrap_or(Value::Null)));
    }
    Ok(flat_id)
}

/// `$value instanceof BackedEnum ? $value->value : $value`.
pub(crate) fn backed_value(value: Value) -> Value {
    if let Value::Object(object) = &value
        && object.instance_of("BackedEnum")
        && let Some(v) = object.property(b"value")
    {
        return v;
    }
    value
}
