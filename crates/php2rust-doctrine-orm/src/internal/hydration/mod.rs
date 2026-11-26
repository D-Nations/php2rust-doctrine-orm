//! `Doctrine\ORM\Internal\Hydration`: the hydration the persisters use.
//!
//! The persisters hydrate entities with the simple object hydrator; the
//! loads of collections add the hydrated entities to the collection as
//! the object hydrator does (`hydrateAdd()` / `hydrateSet()`), without
//! joined entities. The object and array hydrators of DQL queries are
//! not ported, nor are enum fields' cases built from the backing values.
//!
//! Upstream: doctrine/orm src/Internal/Hydration/AbstractHydrator.php src/Internal/Hydration/SimpleObjectHydrator.php

use std::sync::Arc;

use php_value::{Key, Object, PhpArray, Thrown, Value};
use php2rust_doctrine_dbal::types;

use crate::entity_manager::EntityManager;
use crate::persistent_collection::PersistentCollection;
use crate::query::ResultSetMapping;
use crate::unit_of_work::Hints;
use crate::unit_of_work::Fut;
use crate::utility::{key_text, to_text};

/// Where the hydrated entities go besides the result: the collection
/// being loaded (the `collection` hint), indexed by a field.
pub struct CollectionTarget<'a> {
    pub collection: &'a Arc<PersistentCollection>,
    pub index_by: Option<String>,
}

/// `SimpleObjectHydrator::hydrateAll()`.
pub fn hydrate_all<'a>(
    em: &'a Arc<EntityManager>,
    rows: Vec<PhpArray>,
    rsm: &'a ResultSetMapping,
    hints: &'a mut Hints,
    collection: Option<CollectionTarget<'a>>,
) -> Fut<'a, Vec<Object>> {
    Box::pin(do_hydrate_all(em, rows, rsm, hints, collection))
}

async fn do_hydrate_all(
    em: &Arc<EntityManager>,
    rows: Vec<PhpArray>,
    rsm: &ResultSetMapping,
    hints: &mut Hints,
    collection: Option<CollectionTarget<'_>>,
) -> Result<Vec<Object>, Thrown> {
    // prepare()
    if rsm.alias_map.len() != 1 {
        return Err(Thrown::new(
            "RuntimeException",
            "Cannot use SimpleObjectHydrator with a ResultSetMapping that contains more than one object result.",
            0,
        ));
    }
    if !rsm.scalar_mappings.is_empty() {
        return Err(Thrown::new(
            "RuntimeException",
            "Cannot use SimpleObjectHydrator with a ResultSetMapping that contains scalar mappings.",
            0,
        ));
    }
    let class_name = rsm.alias_map.values().next().cloned().unwrap_or_default();
    let class = em.get_class_metadata(&class_name)?;
    let uow = em.get_unit_of_work();
    let platform = em.get_platform();

    let mut result = Vec::new();
    for row in rows {
        // hydrateRowData()
        let entity_name = class.read().name.clone();
        let mut data = PhpArray::new();
        for (column, value) in row.iter() {
            let column = key_text(column);
            if rsm.relation_map.contains_key(&column) {
                return Err(Thrown::new(
                    "Exception",
                    format!("Unable to retrieve association information for column \"{column}\""),
                    0,
                ));
            }
            // hydrateColumnInfo()
            let (field_name, type_name) = if let Some(field_name) = rsm.field_mappings.get(&column) {
                let declaring = rsm
                    .declaring_classes
                    .get(&column)
                    .cloned()
                    .unwrap_or_else(|| class_name.clone());
                let declaring = em.get_class_metadata(&declaring)?;
                let ty = declaring
                    .read()
                    .field_mappings
                    .get(field_name)
                    .map(|m| m.r#type.clone());
                (field_name.clone(), ty)
            } else if let Some(field_name) = rsm.meta_mappings.get(&column) {
                (field_name.clone(), rsm.type_mappings.get(&column).cloned())
            } else {
                continue;
            };

            // Check if value is null before conversion (because some
            // types convert null to something else)
            let value_is_null = value.is_null();
            let mut value = value.clone();
            if let Some(type_name) = type_name {
                let ty = types::get_type(&type_name).map_err(|e| Thrown::new(e.class(), e.message(), 0))?;
                value = ty
                    .convert_to_php_value(&value, platform)
                    .map_err(|e| crate::exception::from_conversion(&e))?;
            }

            // Prevent overwrite in case of inherit classes using same
            // property name (See AbstractHydrator)
            let key = Key::from(field_name.as_str());
            if !data.get(&key).is_some_and(|v| !v.is_null()) || !value_is_null {
                data.insert(key, value);
            }
        }

        if let Some(entity) = hints.refresh_entity.clone() {
            register_managed(em, &class_name, &entity, &data)?;
        }

        let entity = uow.create_entity(&entity_name, data, hints).await?;

        if let Some(target) = &collection {
            match &target.index_by {
                Some(index_by) => {
                    let entity_class = em.get_class_metadata(entity.class())?;
                    let key = entity_class.read().get_field_value(&entity, index_by)?;
                    let key = Key::from_value(&key).unwrap_or_else(|| Key::from(to_text(&key).as_str()));
                    target.collection.hydrate_set(key, Value::Object(entity.clone()))?;
                }
                None => target.collection.hydrate_add(Value::Object(entity.clone()))?,
            }
        }
        result.push(entity);
    }

    if hints.defer_eager_load || collection.is_none() {
        uow.trigger_eager_loads().await?;
    }
    uow.hydration_complete()?;
    Ok(result)
}

/// `AbstractHydrator::registerManaged()`.
fn register_managed(
    em: &EntityManager,
    class_name: &str,
    entity: &Object,
    data: &PhpArray,
) -> Result<(), Thrown> {
    let class = em.get_class_metadata(class_name)?;
    let id = {
        let info = class.read();
        let mut id = PhpArray::new();
        let fields: Vec<String> = if info.is_identifier_composite {
            info.identifier.clone()
        } else {
            info.identifier.first().cloned().into_iter().collect()
        };
        for field_name in fields {
            let key = match info.association_mappings.get(&field_name) {
                Some(assoc) if assoc.is_to_one_owning_side() => assoc
                    .join_columns
                    .first()
                    .map(|jc| jc.name.clone())
                    .unwrap_or_default(),
                _ => field_name.clone(),
            };
            id.insert(
                field_name.as_str(),
                data.get(&key.as_str().into()).cloned().unwrap_or(Value::Null),
            );
        }
        id
    };
    em.get_unit_of_work().register_managed(entity, id, data.clone())
}
