//! `Doctrine\ORM\Internal\Hydration`: the hydrators.
//!
//! The persisters hydrate entities with the simple object hydrator; the
//! loads of collections add the hydrated entities to the collection as
//! the object hydrator does (`hydrateAdd()` / `hydrateSet()`), without
//! joined entities. The queries hydrate with [`hydrate`] in their
//! hydration mode. Enum fields' cases are not built from the backing
//! values (PHP's enums), and custom hydration modes are not ported.
//!
//! Upstream: doctrine/orm src/Internal/Hydration/SimpleObjectHydrator.php src/Internal/Hydration/ScalarHydrator.php src/Internal/Hydration/SingleScalarHydrator.php src/Internal/Hydration/ScalarColumnHydrator.php

mod abstract_hydrator;
mod array_hydrator;
mod object_hydrator;

use std::sync::Arc;

use abstract_hydrator::Columns;
pub(crate) use abstract_hydrator::register_classes;
pub use abstract_hydrator::{
    HYDRATION_EXCEPTION, MULTIPLE_SELECTORS_FOUND, NewObjectConstructor, empty_discriminator_value,
    invalid_discriminator_value, missing_discriminator_column,
    missing_discriminator_meta_mapping_column, multiple_selectors_found, non_unique_result,
    parent_object_of_relation_not_found, partial_object_hydration_disallowed,
    register_new_object_class,
};
use array_hydrator::ArrayHydrator;
use object_hydrator::ObjectHydrator;

use php_value::{Key, Object, PhpArray, Thrown, Value};
use php2rust_doctrine_dbal::types;

use crate::entity_manager::EntityManager;
use crate::persistent_collection::PersistentCollection;
use crate::query::ResultSetMapping;
use crate::unit_of_work::Fut;
use crate::unit_of_work::Hints;
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
            let (field_name, type_name) = if let Some(field_name) = rsm.field_mappings.get(&column)
            {
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
                let ty = types::get_type(&type_name)
                    .map_err(|e| Thrown::new(e.class(), e.message(), 0))?;
                value = ty
                    .convert_to_php_value(&value, platform)
                    .map_err(|e| crate::exception::from_conversion(&e))?;
            }

            // Prevent overwrite in case of inherit classes using same
            // property name (See AbstractHydrator)
            let key = Key::from(field_name.as_str());
            if data.get(&key).is_none_or(Value::is_null) || !value_is_null {
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
                    let key =
                        Key::from_value(&key).unwrap_or_else(|| Key::from(to_text(&key).as_str()));
                    target
                        .collection
                        .hydrate_set(key, Value::Object(entity.clone()))?;
                }
                None => target
                    .collection
                    .hydrate_add(Value::Object(entity.clone()))?,
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
pub(crate) fn register_managed(
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
                data.get(&key.as_str().into())
                    .cloned()
                    .unwrap_or(Value::Null),
            );
        }
        id
    };
    em.get_unit_of_work()
        .register_managed(entity, id, data.clone())
}

/// The hints of a hydration besides the unit of work's
/// ([`Hints`]): `$hints[UnitOfWork::HINT_DEFEREAGERLOAD]` as set on the
/// query (`None` when not set) and `$hints['collection']`.
#[derive(Default)]
pub struct HydrationHints {
    pub defer_eager_load: Option<bool>,
    pub collection: Option<Arc<PersistentCollection>>,
}

/// `$em->newHydrator($hydrationMode)->hydrateAll($stmt, $rsm, $hints)`.
pub async fn hydrate(
    em: &Arc<EntityManager>,
    hydration_mode: i64,
    rows: Vec<PhpArray>,
    rsm: &ResultSetMapping,
    hints: &mut Hints,
    extra: HydrationHints,
) -> Result<Value, Thrown> {
    use crate::query::context::{
        HYDRATE_ARRAY, HYDRATE_OBJECT, HYDRATE_SCALAR, HYDRATE_SCALAR_COLUMN, HYDRATE_SIMPLEOBJECT,
        HYDRATE_SINGLE_SCALAR,
    };
    match hydration_mode {
        HYDRATE_OBJECT => {
            let mut hydrator =
                ObjectHydrator::new(em, rsm, hints, extra.defer_eager_load, extra.collection);
            let result = match hydrator.prepare() {
                Ok(()) => hydrator.hydrate_all_data(rows).await,
                Err(e) => Err(e),
            };
            let cleanup = hydrator.cleanup().await;
            let result = result?;
            cleanup?;
            Ok(Value::Array(result))
        }
        HYDRATE_ARRAY => {
            let mut hydrator = ArrayHydrator::new(em, rsm, hints.force_dbal_type_conversion);
            hydrator.prepare();
            Ok(Value::Array(hydrator.hydrate_all_data(&rows)?))
        }
        HYDRATE_SCALAR => {
            let mut columns = Columns::new(em, rsm, hints.force_dbal_type_conversion);
            let mut result = Vec::new();
            for row in &rows {
                result.push(Value::Array(columns.gather_scalar_row_data(row)?));
            }
            Ok(Value::Array(PhpArray::list(result)))
        }
        HYDRATE_SINGLE_SCALAR => {
            let num_rows = rows.len();
            if num_rows == 0 {
                return Err(crate::abstract_query::no_result());
            }
            if num_rows > 1 {
                return Err(crate::abstract_query::non_unique_result(Some(
                    "The query returned multiple rows. Change the query or use a different result function like getScalarResult().",
                )));
            }
            let mut columns = Columns::new(em, rsm, hints.force_dbal_type_conversion);
            let result = columns.gather_scalar_row_data(&rows[0])?;
            if result.len() > 1 {
                return Err(crate::abstract_query::non_unique_result(Some(
                    "The query returned a row containing multiple columns. Change the query or use a different result function like getScalarResult().",
                )));
            }
            Ok(result.values().next().cloned().unwrap_or(Value::Null))
        }
        HYDRATE_SIMPLEOBJECT => {
            let entities = hydrate_all(em, rows, rsm, hints, None).await?;
            Ok(Value::Array(PhpArray::list(
                entities.into_iter().map(Value::Object),
            )))
        }
        HYDRATE_SCALAR_COLUMN => {
            if rsm.field_mappings.len() > 1 {
                let selectors: Vec<String> = rsm.field_mappings.values().cloned().collect();
                return Err(multiple_selectors_found(&selectors));
            }
            Ok(Value::Array(PhpArray::list(rows.into_iter().map(|row| {
                row.values().next().cloned().unwrap_or(Value::Null)
            }))))
        }
        other => Err(crate::exception::invalid_hydration_mode(&other.to_string())),
    }
}

/// `toIterable($stmt, $rsm, $hints)`: the yielded values, row by row
/// (the rows are read whole).
pub async fn to_iterable(
    em: &Arc<EntityManager>,
    hydration_mode: i64,
    rows: Vec<PhpArray>,
    rsm: &ResultSetMapping,
    hints: &mut Hints,
    extra: HydrationHints,
) -> Result<Vec<Value>, Thrown> {
    use crate::query::context::{
        HYDRATE_ARRAY, HYDRATE_OBJECT, HYDRATE_SCALAR, HYDRATE_SIMPLEOBJECT,
    };
    let index_by = !rsm.index_by_map.is_empty();
    let mut yielded = Vec::new();
    let mut yield_row = |result: PhpArray| {
        if result.len() == 1 {
            if index_by {
                yielded.extend(result.values().cloned());
            } else if let Some((_, last)) = result.last() {
                yielded.push(last.clone());
            }
        } else if matches!(result.values().next(), Some(Value::Object(_))) {
            yielded.push(Value::Array(result));
        } else {
            // array_merge(...$result)
            let mut merged = PhpArray::new();
            for value in result.values() {
                if let Value::Array(row) = value {
                    for (k, v) in row.iter() {
                        match k {
                            Key::Int(_) => {
                                let _ = merged.push(v.clone());
                            }
                            Key::Str(_) => merged.insert(k.clone(), v.clone()),
                        }
                    }
                }
            }
            yielded.push(Value::Array(merged));
        }
    };
    match hydration_mode {
        HYDRATE_OBJECT => {
            let mut hydrator =
                ObjectHydrator::new(em, rsm, hints, extra.defer_eager_load, extra.collection);
            let mut outcome = hydrator.prepare();
            if outcome.is_ok() {
                for row in rows {
                    let mut result = PhpArray::new();
                    if let Err(e) = hydrator.hydrate_row_data(&row, &mut result).await {
                        outcome = Err(e);
                        break;
                    }
                    hydrator.cleanup_after_row_iteration();
                    yield_row(result);
                }
            }
            let cleanup = hydrator.cleanup().await;
            outcome?;
            cleanup?;
        }
        HYDRATE_ARRAY => {
            let mut hydrator = ArrayHydrator::new(em, rsm, hints.force_dbal_type_conversion);
            hydrator.prepare();
            for row in &rows {
                let mut result = PhpArray::new();
                hydrator.hydrate_row_data(row, &mut result)?;
                yield_row(result);
            }
        }
        HYDRATE_SCALAR => {
            let mut columns = Columns::new(em, rsm, hints.force_dbal_type_conversion);
            for row in &rows {
                let mut result = PhpArray::new();
                let _ = result.push(Value::Array(columns.gather_scalar_row_data(row)?));
                yield_row(result);
            }
        }
        HYDRATE_SIMPLEOBJECT => {
            for row in rows {
                let entities = hydrate_all(em, vec![row], rsm, hints, None).await?;
                let mut result = PhpArray::new();
                for entity in entities {
                    let _ = result.push(Value::Object(entity));
                }
                yield_row(result);
            }
        }
        _ => {
            return Err(Thrown::new(
                HYDRATION_EXCEPTION,
                "hydrateRowData() not implemented by this hydrator.",
                0,
            ));
        }
    }
    Ok(yielded)
}
