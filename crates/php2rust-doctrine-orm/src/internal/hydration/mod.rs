//! `Doctrine\ORM\Internal\Hydration`: the hydrators.
//!
//! The persisters hydrate entities with the simple object hydrator; the
//! loads of collections add the hydrated entities to the collection as
//! the object hydrator does (`hydrateAdd()` / `hydrateSet()`), without
//! joined entities. The queries hydrate with [`hydrate`] in their
//! hydration mode. Enum fields' cases are not built from the backing
//! values (the simple object hydrator checks them), and custom hydration
//! modes are not ported.
//!
//! Upstream: doctrine/orm src/Internal/Hydration/SimpleObjectHydrator.php src/Internal/Hydration/ScalarHydrator.php src/Internal/Hydration/SingleScalarHydrator.php src/Internal/Hydration/ScalarColumnHydrator.php

mod abstract_hydrator;
mod array_hydrator;
mod custom_hydrator;
mod iterable;
mod object_hydrator;

use std::sync::Arc;

use abstract_hydrator::Columns;
pub use abstract_hydrator::build_enum;
pub(crate) use abstract_hydrator::register_classes;
pub use abstract_hydrator::{
    HYDRATION_EXCEPTION, MULTIPLE_SELECTORS_FOUND, NewObjectArgsConstructor, NewObjectConstructor,
    empty_discriminator_value, invalid_discriminator_value, missing_discriminator_column,
    missing_discriminator_meta_mapping_column, multiple_selectors_found, non_unique_result,
    parent_object_of_relation_not_found, partial_object_hydration_disallowed,
    register_new_object_class, register_new_object_class_with_arguments,
};
use array_hydrator::ArrayHydrator;
pub(crate) use custom_hydrator::new_hydrator_of_class;
pub use custom_hydrator::{
    AbstractHydrator, HydrationContext, HydratorConstructor, ToIterable,
    hydrate_all as hydrate_all_with, register_hydrator_class, to_iterable as to_iterable_with,
};
pub use iterable::Iterable;
use object_hydrator::ObjectHydrator;

use php_value::{Key, Object, PhpArray, Thrown, Value};

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

#[allow(
    clippy::too_many_lines,
    reason = "prepare() and hydrateRowData(), in their order"
)]
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
    let dql_alias = rsm.alias_map.keys().next().unwrap_or_default().to_owned();
    let mut columns = Columns::new(em, rsm, false);
    let class = columns.get_class_metadata(&class_name)?;
    let uow = em.get_unit_of_work();
    let platform = em.get_platform();

    let mut result = Vec::new();
    for mut row in rows {
        // hydrateRowData()
        let (mut entity_name, inheritance_type) = {
            let info = class.read();
            (info.name.clone(), info.inheritance_type)
        };
        let mut data = PhpArray::new();
        let mut discr_column_value = Value::Null;

        // We need to find the correct entity class name if we have
        // inheritance in resultset
        if inheritance_type != crate::mapping::class_metadata::INHERITANCE_TYPE_NONE {
            let (mut discr_column_name, discr_map) = {
                let info = class.read();
                (
                    info.get_discriminator_column()?.name.clone(),
                    info.discriminator_map.clone(),
                )
            };

            // Find mapped discriminator column from the result set.
            if let Some((column, _)) = rsm
                .meta_mappings
                .iter()
                .find(|(_, field)| **field == discr_column_name)
                && !column.is_empty()
                && column != "0"
            {
                discr_column_name = column.to_owned();
            }

            let discr_key = Key::from_bytes(discr_column_name.as_bytes());
            let discr_value = match row.get(&discr_key) {
                None | Some(Value::Null) => {
                    return Err(missing_discriminator_column(
                        &entity_name,
                        &discr_column_name,
                        &dql_alias,
                    ));
                }
                Some(value) => value.clone(),
            };
            if matches!(&discr_value, Value::String(s) if s.is_empty()) {
                return Err(empty_discriminator_value(&dql_alias));
            }
            let discr_text = to_text(&discr_value);
            let Some(sub_class) = discr_map.get(&Key::from_bytes(discr_text.as_bytes())) else {
                let values: Vec<String> = discr_map.keys().map(key_text).collect();
                return Err(invalid_discriminator_value(&discr_text, &values));
            };
            entity_name = to_text(sub_class);
            discr_column_value = discr_value;
            row.remove(&discr_key);
        }

        for (column, value) in row.iter() {
            let column = key_text(column);
            // An ObjectHydrator should be used instead of
            // SimpleObjectHydrator
            if rsm.relation_map.contains_key(&column) {
                return Err(Thrown::new(
                    "Exception",
                    format!("Unable to retrieve association information for column \"{column}\""),
                    0,
                ));
            }

            let Some(cache_key_info) = columns.hydrate_column_info(&column)? else {
                continue;
            };

            // If we have inheritance in resultset, make sure the field
            // belongs to the correct class
            if let Some((_, discriminator_values)) = &cache_key_info.discriminator
                && !discriminator_values.contains(&to_text(&discr_column_value))
            {
                continue;
            }

            // Check if value is null before conversion (because some
            // types convert null to something else)
            let value_is_null = value.is_null();

            // Convert field to a valid PHP value
            let mut value = value.clone();
            if let Some(ty) = &cache_key_info.ty {
                value = ty
                    .convert_to_php_value(&value, platform)
                    .map_err(|e| crate::exception::from_conversion(&e))?;
            }

            let field_name = key_text(&cache_key_info.field_name);
            if !value.is_null()
                && let Some(enum_type) = rsm.enum_mappings.get(&column)
            {
                // The entity keeps the backing values: the cases are
                // only checked here.
                let failed = match &value {
                    Value::Array(values) => values
                        .values()
                        .find(|v| build_enum(v, enum_type).is_err())
                        .cloned(),
                    single => build_enum(single, enum_type).err().map(|_| single.clone()),
                };
                if let Some(current_value) = failed
                    && let Err(error) = build_enum(&current_value, enum_type)
                {
                    if error.class != "ValueError" {
                        return Err(error);
                    }
                    return Err(crate::mapping::mapping_exception::invalid_enum_value(
                        &entity_name,
                        &field_name,
                        &to_text(&current_value),
                        enum_type,
                    ));
                }
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
                    let key = index_by_value(em, &entity, index_by)?;
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
/// `$row[$rsm->indexByMap[$alias]]` for an entity loaded into an indexed
/// collection: the value of the `indexBy` field, or, for the column of an
/// association (`addIndexBy()` finds its meta mapping), the referenced
/// identifier of the associated entity.
pub(crate) fn index_by_value(
    em: &EntityManager,
    entity: &Object,
    index_by: &str,
) -> Result<Value, Thrown> {
    let class = em.get_class_metadata(entity.class())?;
    let join_column = {
        let class = class.read();
        if class.has_field(index_by) || class.association_mappings.contains_key(index_by) {
            None
        } else {
            class
                .association_mappings
                .iter()
                .find_map(|(field, assoc)| {
                    assoc
                        .join_columns
                        .iter()
                        .find(|jc| jc.name == index_by)
                        .map(|jc| {
                            (
                                field.to_owned(),
                                assoc.target_entity.clone(),
                                jc.referenced_column_name.clone(),
                            )
                        })
                })
        }
    };
    let Some((field, target_entity, referenced_column)) = join_column else {
        return class.read().get_field_value(entity, index_by);
    };
    let Value::Object(target) = class.read().get_field_value(entity, &field)? else {
        return Ok(Value::Null);
    };
    let target_class = em.get_class_metadata(&target_entity)?;
    let target_class = target_class.read();
    let target_field = target_class.get_field_for_column(&referenced_column)?;
    let mut value = target_class.get_field_value(&target, &target_field)?;
    if value.is_null() {
        // A reference: its identifier.
        value = target_class
            .get_identifier_values(&target)?
            .get(&Key::from(target_field.as_str()))
            .cloned()
            .unwrap_or(Value::Null);
    }
    Ok(value)
}

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

/// `iterator_to_array($hydrator->toIterable($stmt, $rsm, $hints),
/// false)`: the values the generator yields, to the end.
pub async fn to_iterable(
    em: &Arc<EntityManager>,
    hydration_mode: i64,
    rows: Vec<PhpArray>,
    rsm: &ResultSetMapping,
    hints: &Hints,
    extra: HydrationHints,
) -> Result<Vec<Value>, Thrown> {
    Iterable::new(
        em.clone(),
        hydration_mode,
        rows,
        rsm.clone(),
        hints.clone(),
        extra,
    )
    .to_vec()
    .await
}

/// [`to_iterable`] with the generator's keys: the index-by keys of
/// `yield from $result`, and the generator's own integer keys (from 0)
/// for the other values.
pub async fn to_iterable_with_keys(
    em: &Arc<EntityManager>,
    hydration_mode: i64,
    rows: Vec<PhpArray>,
    rsm: &ResultSetMapping,
    hints: &Hints,
    extra: HydrationHints,
) -> Result<Vec<(Key, Value)>, Thrown> {
    Iterable::new(
        em.clone(),
        hydration_mode,
        rows,
        rsm.clone(),
        hints.clone(),
        extra,
    )
    .to_vec_with_keys()
    .await
}
