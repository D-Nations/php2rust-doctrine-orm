//! `ObjectHydrator`: hydrates an object graph out of a result set mapping,
//! the entities of the root aliases and the fetch-joined associations,
//! with scalars and `NEW` objects for a mixed result.
//!
//! Upstream: doctrine/orm src/Internal/Hydration/ObjectHydrator.php

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use php_value::{Key, Object, PhpArray, Thrown, Value};
use php2rust_doctrine_collections::ArrayCollection;

use super::abstract_hydrator::{
    Columns, RowData, empty_discriminator_value, invalid_discriminator_value,
    missing_discriminator_column, missing_discriminator_meta_mapping_column,
    parent_object_of_relation_not_found, value_key,
};
use crate::collection_object::ArrayCollectionObject;
use crate::entity_manager::EntityManager;
use crate::internal::object_id;
use crate::mapping::ClassMetadata;
use crate::persistent_collection::PersistentCollection;
use crate::query::ResultSetMapping;
use crate::unit_of_work::{Hints, UnitOfWork};
use crate::utility::to_text;

/// `ObjectHydrator`.
pub(crate) struct ObjectHydrator<'a> {
    columns: Columns<'a>,
    hints: &'a mut Hints,
    /// `$hints['collection']`: the collection the root entities go to.
    collection: Option<Arc<PersistentCollection>>,
    /// `HINT_DEFEREAGERLOAD === true` at cleanup.
    trigger_eager_loads: bool,
    root_identifier_map: HashMap<String, HashMap<String, Key>>,
    identifier_map: HashMap<String, HashMap<String, HashMap<String, Key>>>,
    result_pointers: HashMap<String, Value>,
    id_template: HashMap<String, String>,
    result_counter: i64,
    root_aliases: HashSet<String>,
    initialized_collections: Vec<(String, Arc<PersistentCollection>)>,
    uninitialized_collections: Vec<(String, Arc<PersistentCollection>)>,
    existing_collections: HashSet<String>,
}

fn rsm_get<'r>(map: &'r crate::mapping::ordered_map::OrderedMap<String>, key: &str) -> &'r str {
    map.get(key).map_or("", String::as_str)
}

impl<'a> ObjectHydrator<'a> {
    /// `new ObjectHydrator($em)` with `hydrateAll()`'s result set mapping
    /// and hints. `defer_eager_load_hint` is `$hints[HINT_DEFEREAGERLOAD]`
    /// (`None` when not set).
    pub fn new(
        em: &'a Arc<EntityManager>,
        rsm: &'a ResultSetMapping,
        hints: &'a mut Hints,
        defer_eager_load_hint: Option<bool>,
        collection: Option<Arc<PersistentCollection>>,
    ) -> Self {
        // prepare(): an unset HINT_DEFEREAGERLOAD becomes true
        hints.defer_eager_load = defer_eager_load_hint.is_none_or(|_| hints.defer_eager_load);
        let trigger_eager_loads = defer_eager_load_hint.is_none_or(|v| v);
        let force = hints.force_dbal_type_conversion;
        Self {
            columns: Columns::new(em, rsm, force),
            hints,
            collection,
            trigger_eager_loads,
            root_identifier_map: HashMap::new(),
            identifier_map: HashMap::new(),
            result_pointers: HashMap::new(),
            id_template: HashMap::new(),
            result_counter: 0,
            root_aliases: HashSet::new(),
            initialized_collections: Vec::new(),
            uninitialized_collections: Vec::new(),
            existing_collections: HashSet::new(),
        }
    }

    fn rsm(&self) -> &'a ResultSetMapping {
        self.columns.rsm
    }

    fn em(&self) -> &'a Arc<EntityManager> {
        self.columns.em
    }

    fn uow(&self) -> &'a UnitOfWork {
        self.columns.em.get_unit_of_work()
    }

    fn mark_fetched(&mut self, alias: &str, field: &str) {
        if !self
            .hints
            .fetched
            .iter()
            .any(|(a, f)| a == alias && f == field)
        {
            self.hints
                .fetched
                .push((alias.to_owned(), field.to_owned()));
        }
    }

    fn is_fetched(&self, alias: &str, field: &str) -> bool {
        self.hints
            .fetched
            .iter()
            .any(|(a, f)| a == alias && f == field)
    }

    /// `prepare()`.
    pub fn prepare(&mut self) -> Result<(), Thrown> {
        let rsm = self.rsm();
        for (dql_alias, class_name) in rsm.alias_map.iter() {
            self.id_template.insert(dql_alias.to_owned(), String::new());

            // Remember which associations are "fetch joined", so that we
            // know where to inject collection stubs or proxies and where
            // not.
            let Some(relation) = rsm.relation_map.get(dql_alias) else {
                continue;
            };
            let parent = rsm_get(&rsm.parent_alias_map, dql_alias);
            let Some(source_class_name) = rsm.alias_map.get(parent) else {
                if rsm.nested_entities.contains_key(&Key::from(dql_alias)) {
                    continue;
                }
                return Err(parent_object_of_relation_not_found(dql_alias, parent));
            };
            let source_class = self.columns.get_class_metadata(source_class_name)?;
            let assoc = source_class
                .read()
                .association_mappings
                .get(relation)
                .cloned()
                .ok_or_else(|| undefined_key(relation))?;

            self.mark_fetched(parent, &assoc.field_name);

            if assoc.is_many_to_many() {
                continue;
            }

            // Mark any non-collection opposite sides as fetched, too.
            if !assoc.is_owning_side() {
                if let Some(mapped_by) = &assoc.mapped_by {
                    self.mark_fetched(dql_alias, mapped_by);
                }
                continue;
            }

            // handle fetch-joined owning side bi-directional one-to-one
            // associations
            if let Some(inversed_by) = &assoc.inversed_by {
                let class = self.columns.get_class_metadata(class_name)?;
                let inverse = class
                    .read()
                    .association_mappings
                    .get(inversed_by)
                    .cloned()
                    .ok_or_else(|| undefined_key(inversed_by))?;
                if !inverse.is_to_one() {
                    continue;
                }
                self.mark_fetched(dql_alias, &inverse.field_name);
            }
        }
        Ok(())
    }

    /// `cleanup()`.
    pub async fn cleanup(&mut self) -> Result<(), Thrown> {
        self.cleanup_after_row_iteration();
        if self.trigger_eager_loads {
            self.uow().trigger_eager_loads().await?;
        }
        self.uow().hydration_complete()
    }

    /// `cleanupAfterRowIteration()`.
    pub fn cleanup_after_row_iteration(&mut self) {
        self.identifier_map.clear();
        self.root_identifier_map.clear();
        self.initialized_collections.clear();
        self.uninitialized_collections.clear();
        self.existing_collections.clear();
        self.result_pointers.clear();
    }

    /// `hydrateAllData()`.
    pub async fn hydrate_all_data(&mut self, rows: Vec<PhpArray>) -> Result<PhpArray, Thrown> {
        let mut result = PhpArray::new();
        for row in rows {
            self.hydrate_row_data(&row, &mut result).await?;
        }
        self.finish_collections();
        Ok(result)
    }

    /// Take snapshots from all newly initialized collections.
    pub fn finish_collections(&mut self) {
        for (_, coll) in &self.initialized_collections {
            coll.take_snapshot();
        }
        for (_, coll) in &self.uninitialized_collections {
            if !coll.is_initialized() {
                coll.set_initialized(true);
            }
        }
    }

    /// `initRelatedCollection()`.
    fn init_related_collection(
        &mut self,
        entity: &Object,
        class: &Arc<ClassMetadata>,
        field_name: &str,
        parent_dql_alias: &str,
    ) -> Result<Arc<PersistentCollection>, Thrown> {
        let oid = object_id(entity);
        let key = format!("{oid}{field_name}");
        let relation = class
            .read()
            .association_mappings
            .get(field_name)
            .cloned()
            .ok_or_else(|| undefined_key(field_name))?;
        let value = class.read().get_field_value(entity, field_name)?;

        if let Some(pc) = PersistentCollection::from_value(&value) {
            if self.hints.refresh
                || (self.is_fetched(parent_dql_alias, field_name) && !pc.is_initialized())
            {
                // Is already PersistentCollection, but either REFRESH or
                // FETCH-JOIN and UNINITIALIZED!
                pc.set_dirty(false);
                pc.set_initialized(true);
                pc.with_unwrapped(ArrayCollection::clear);
                self.initialized_collections.push((key, pc.clone()));
            } else {
                // Is already PersistentCollection, and DON'T REFRESH or
                // FETCH-JOIN!
                self.existing_collections.insert(key);
            }
            return Ok(pc);
        }

        let elements = match &value {
            Value::Null => PhpArray::new(),
            Value::Array(array) => array.clone(),
            other => match ArrayCollectionObject::from_value(other) {
                Some(collection) => collection.lock().to_array(),
                None => PhpArray::list([other.clone()]),
            },
        };
        let target_class = self.columns.get_class_metadata(&relation.target_entity)?;
        let pc = PersistentCollection::new(
            Some(self.em()),
            Some(target_class),
            ArrayCollection::new(elements),
        );
        pc.set_owner(entity, &relation);
        class
            .read()
            .set_field_value(entity, field_name, pc.value())?;
        self.uow()
            .set_original_entity_property(oid, field_name, pc.value());
        self.initialized_collections.push((key, pc.clone()));
        Ok(pc)
    }

    /// `getEntity($data, $dqlAlias)`.
    async fn get_entity(&mut self, mut data: PhpArray, dql_alias: &str) -> Result<Object, Thrown> {
        let rsm = self.rsm();
        let mut class_name = rsm_get(&rsm.alias_map, dql_alias).to_owned();

        if let Some(field_name) = rsm.discriminator_columns.get(dql_alias) {
            let Some(discr_column) = rsm.meta_mappings.get(field_name) else {
                return Err(missing_discriminator_meta_mapping_column(
                    &class_name,
                    field_name,
                    dql_alias,
                ));
            };
            let discr_key = Key::from(discr_column.as_str());
            let discriminator_value = match data.get(&discr_key) {
                None | Some(Value::Null) => {
                    return Err(missing_discriminator_column(
                        &class_name,
                        discr_column,
                        dql_alias,
                    ));
                }
                // `$discriminatorValue instanceof BackedEnum`: its value.
                Some(Value::Object(object))
                    if let Some(backing) = object
                        .downcast_ref::<php_value::object::EnumCase>()
                        .and_then(php_value::object::EnumCase::value) =>
                {
                    to_text(backing)
                }
                Some(value) => to_text(value),
            };
            if discriminator_value.is_empty()
                && matches!(data.get(&discr_key), Some(Value::String(_)))
            {
                return Err(empty_discriminator_value(dql_alias));
            }
            let class = self.columns.get_class_metadata(&class_name)?;
            let discr_map = class.read().discriminator_map.clone();
            let Some(sub_class) = discr_map.get(&Key::from_bytes(discriminator_value.as_bytes()))
            else {
                let values: Vec<String> = discr_map.keys().map(crate::utility::key_text).collect();
                return Err(invalid_discriminator_value(&discriminator_value, &values));
            };
            class_name = to_text(sub_class);
            data.remove(&discr_key);
        }

        if let Some(entity) = self.hints.refresh_entity.clone()
            && self.root_aliases.contains(dql_alias)
        {
            super::register_managed(self.em(), &class_name, &entity, &data)?;
        }

        self.hints.fetch_alias = Some(dql_alias.to_owned());
        self.uow()
            .create_entity(&class_name, data, self.hints)
            .await
    }

    /// `getEntityFromIdentityMap($className, $data)`.
    fn get_entity_from_identity_map(
        &mut self,
        class_name: &str,
        data: &PhpArray,
    ) -> Result<Option<Object>, Thrown> {
        let class = self.columns.get_class_metadata(class_name)?;
        let info = class.read();
        let column_of = |field_name: &str| -> Value {
            let key = match info.association_mappings.get(field_name) {
                Some(assoc) => assoc
                    .join_columns
                    .first()
                    .map(|jc| jc.name.clone())
                    .unwrap_or_default(),
                None => field_name.to_owned(),
            };
            data.get(&Key::from(key.as_str()))
                .cloned()
                .unwrap_or(Value::Null)
        };
        let id_hash = if info.is_identifier_composite {
            let ids = PhpArray::list(info.identifier.iter().map(|f| column_of(f)));
            UnitOfWork::get_id_hash_by_identifier(&ids)
                .trim_start()
                .to_owned()
        } else {
            let first = info.identifier.first().cloned().unwrap_or_default();
            to_text(&column_of(&first))
        };
        Ok(self
            .uow()
            .try_get_by_id_hash(&id_hash, &info.root_entity_name))
    }

    /// `hydrateRowData($row, &$result)`.
    #[allow(clippy::too_many_lines, reason = "hydrateRowData(), in its order")]
    pub async fn hydrate_row_data(
        &mut self,
        row: &PhpArray,
        result: &mut PhpArray,
    ) -> Result<(), Thrown> {
        let rsm = self.rsm();
        // Initialize
        let mut id = self.id_template.clone(); // initialize the id-memory
        let mut nonempty_components = HashSet::new();
        // Split the row data into chunks of class data.
        let (row_data, pending) =
            self.columns
                .gather_row_data_parts(row, &mut id, &mut nonempty_components)?;
        // hydrateNestedEntity(): the entities that are NEW arguments.
        let mut nested_entities = Vec::new();
        for dql_alias in self.columns.nested_entity_aliases(&row_data, &pending) {
            let Some((_, data)) = row_data.data.iter().find(|(a, _)| *a == dql_alias) else {
                continue;
            };
            let data = data.clone();
            let entity = if rsm
                .nested_entities
                .contains_key(&Key::from(dql_alias.as_str()))
            {
                Value::Object(self.get_entity(data, &dql_alias).await?)
            } else {
                Value::Array(data)
            };
            nested_entities.push((dql_alias, entity));
        }
        let row_data: RowData =
            self.columns
                .finish_row_data(row_data, pending, &nested_entities)?;

        // reset result pointers for each data row
        self.result_pointers.clear();

        let mut result_key: Option<Key> = None;

        // Hydrate the data chunks
        for (dql_alias, data) in row_data.data {
            let dql_alias = dql_alias.as_str();
            let entity_name = rsm_get(&rsm.alias_map, dql_alias).to_owned();

            if let Some(parent_alias) = rsm.parent_alias_map.get(dql_alias) {
                // It's a joined result

                // we need the $path to save into the identifier map which
                // entities were already seen for this parent-child
                // relationship
                let path = format!("{parent_alias}.{dql_alias}");

                // We have a RIGHT JOIN result here. Doctrine cannot
                // hydrate RIGHT JOIN Object-Graphs
                if !nonempty_components.contains(parent_alias) {
                    // TODO: Add special case code where we hydrate the
                    // right join objects into identity map at least
                    continue;
                }

                let parent_class = self
                    .columns
                    .get_class_metadata(rsm_get(&rsm.alias_map, parent_alias))?;
                let relation_field = rsm_get(&rsm.relation_map, dql_alias).to_owned();
                let relation = parent_class
                    .read()
                    .association_mappings
                    .get(&relation_field)
                    .cloned()
                    .ok_or_else(|| undefined_key(&relation_field))?;

                // Get a reference to the parent object to which the joined
                // element belongs.
                let parent_object = if rsm.is_mixed && self.root_aliases.contains(parent_alias) {
                    match self.result_pointers.get(parent_alias) {
                        Some(Value::Array(object_class)) => object_class.values().next().cloned(),
                        other => other.cloned(),
                    }
                } else {
                    self.result_pointers.get(parent_alias).cloned()
                };
                let Some(Value::Object(parent_object)) = parent_object else {
                    // Parent object of relation not found, mark as
                    // not-fetched again
                    if nonempty_components.contains(dql_alias) {
                        let element = self.get_entity(data, dql_alias).await?;
                        // Update result pointer and provide initial fetch
                        // data for parent
                        self.result_pointers
                            .insert(dql_alias.to_owned(), Value::Object(element));
                    }
                    // Mark as not-fetched again
                    self.hints
                        .fetched
                        .retain(|(a, f)| !(a == parent_alias && *f == relation_field));
                    continue;
                };

                let oid = object_id(&parent_object);
                let parent_id = id.get(parent_alias).cloned().unwrap_or_default();
                let own_id = id.get(dql_alias).cloned().unwrap_or_default();

                // Check the type of the relation (many or single-valued)
                #[allow(clippy::if_not_else, reason = "hydrateRowData()'s order")]
                if !relation.is_to_one() {
                    // PATH A: Collection-valued association
                    let refl_field_value = parent_class
                        .read()
                        .get_field_value(&parent_object, &relation_field)?;

                    if nonempty_components.contains(dql_alias) {
                        let coll_key = format!("{oid}{relation_field}");
                        let coll = if let Some((_, coll)) = self
                            .initialized_collections
                            .iter()
                            .find(|(k, _)| *k == coll_key)
                        {
                            Some(coll.clone())
                        } else if self.existing_collections.contains(&coll_key) {
                            PersistentCollection::from_value(&refl_field_value)
                        } else {
                            Some(self.init_related_collection(
                                &parent_object,
                                &parent_class,
                                &relation_field,
                                parent_alias,
                            )?)
                        };
                        let Some(coll) = coll else {
                            continue;
                        };

                        let index = self
                            .identifier_map
                            .get(&path)
                            .and_then(|m| m.get(&parent_id))
                            .and_then(|m| m.get(&own_id))
                            .cloned();
                        let index_is_valid = index
                            .as_ref()
                            .is_some_and(|index| coll.with_unwrapped(|c| !c.get(index).is_null()));

                        if index_is_valid {
                            // Update result pointer
                            let index = index.unwrap_or(Key::Int(0));
                            let element = coll.with_unwrapped(|c| c.get(&index));
                            self.result_pointers.insert(dql_alias.to_owned(), element);
                        } else if self.existing_collections.contains(&coll_key) {
                            // Collection exists, only look for the element
                            // in the identity map.
                            match self.get_entity_from_identity_map(&entity_name, &data)? {
                                Some(element) => {
                                    self.result_pointers
                                        .insert(dql_alias.to_owned(), Value::Object(element));
                                }
                                None => {
                                    self.result_pointers.remove(dql_alias);
                                }
                            }
                        } else {
                            let element = Value::Object(self.get_entity(data, dql_alias).await?);
                            let index_value =
                                if let Some(index_by) = rsm.index_by_map.get(dql_alias) {
                                    let index_value = value_key(
                                        &row.get(&Key::from(index_by.as_str()))
                                            .cloned()
                                            .unwrap_or(Value::Null),
                                    );
                                    coll.hydrate_set(index_value.clone(), element.clone())?;
                                    index_value
                                } else {
                                    if !coll.with_unwrapped(|c| c.contains(&element)) {
                                        coll.hydrate_add(element.clone())?;
                                    }
                                    coll.with_unwrapped(|c| {
                                        c.last();
                                        c.key()
                                    })
                                    .unwrap_or(Key::Int(0))
                                };
                            self.identifier_map
                                .entry(path.clone())
                                .or_default()
                                .entry(parent_id.clone())
                                .or_default()
                                .insert(own_id.clone(), index_value);
                            // Update result pointer
                            self.result_pointers.insert(dql_alias.to_owned(), element);
                        }
                    } else if !refl_field_value.is_truthy() {
                        self.init_related_collection(
                            &parent_object,
                            &parent_class,
                            &relation_field,
                            parent_alias,
                        )?;
                    } else if let Some(pc) = PersistentCollection::from_value(&refl_field_value) {
                        let key = format!("{oid}{relation_field}");
                        if !pc.is_initialized()
                            && !self
                                .uninitialized_collections
                                .iter()
                                .any(|(k, _)| *k == key)
                        {
                            self.uninitialized_collections.push((key, pc));
                        }
                    }
                } else {
                    // PATH B: Single-valued association
                    let refl_field_value = parent_class
                        .read()
                        .get_field_value(&parent_object, &relation_field)?;

                    if !refl_field_value.is_truthy()
                        || self.hints.refresh
                        || self.uow().is_uninitialized_object(&refl_field_value)
                    {
                        // we only need to take action if this value is
                        // null, we refresh the entity or its an
                        // uninitialized proxy.
                        if nonempty_components.contains(dql_alias) {
                            let element = self.get_entity(data, dql_alias).await?;
                            parent_class.read().set_field_value(
                                &parent_object,
                                &relation_field,
                                Value::Object(element.clone()),
                            )?;
                            self.uow().set_original_entity_property(
                                oid,
                                &relation_field,
                                Value::Object(element.clone()),
                            );
                            let target_class =
                                self.columns.get_class_metadata(&relation.target_entity)?;

                            if relation.is_owning_side() {
                                // TODO: Just check hints['fetched'] here?
                                // If there is an inverse mapping on the
                                // target class its bidirectional
                                if let Some(inversed_by) = &relation.inversed_by {
                                    let inverse = target_class
                                        .read()
                                        .association_mappings
                                        .get(inversed_by)
                                        .cloned()
                                        .ok_or_else(|| undefined_key(inversed_by))?;
                                    if inverse.is_to_one() {
                                        target_class.read().set_field_value(
                                            &element,
                                            &inverse.field_name,
                                            Value::Object(parent_object.clone()),
                                        )?;
                                        self.uow().set_original_entity_property(
                                            object_id(&element),
                                            &inverse.field_name,
                                            Value::Object(parent_object.clone()),
                                        );
                                    }
                                }
                            } else if let Some(mapped_by) = &relation.mapped_by {
                                // For sure bidirectional, as there is no
                                // inverse side in unidirectional mappings
                                target_class.read().set_field_value(
                                    &element,
                                    mapped_by,
                                    Value::Object(parent_object.clone()),
                                )?;
                                self.uow().set_original_entity_property(
                                    object_id(&element),
                                    mapped_by,
                                    Value::Object(parent_object.clone()),
                                );
                            }

                            // Update result pointer
                            self.result_pointers
                                .insert(dql_alias.to_owned(), Value::Object(element));
                        } else {
                            self.uow().set_original_entity_property(
                                oid,
                                &relation_field,
                                Value::Null,
                            );
                            parent_class.read().set_field_value(
                                &parent_object,
                                &relation_field,
                                Value::Null,
                            )?;
                        }
                        // else leave $reflFieldValue null for single-valued
                        // associations
                    } else {
                        // Update result pointer
                        self.result_pointers
                            .insert(dql_alias.to_owned(), refl_field_value);
                    }
                }
            } else {
                // PATH C: Its a root result element
                self.root_aliases.insert(dql_alias.to_owned()); // Mark as root alias
                let entity_key = match rsm.entity_mappings.get(dql_alias) {
                    Some(Some(key)) if !key.is_empty() && key != "0" => {
                        Key::from_bytes(key.as_bytes())
                    }
                    _ => Key::Int(0),
                };

                // if this row has a NULL value for the root result id then
                // make it a null result.
                if !nonempty_components.contains(dql_alias) {
                    let element = if rsm.is_mixed {
                        let mut element = PhpArray::new();
                        element.insert(entity_key, Value::Null);
                        Value::Array(element)
                    } else {
                        Value::Null
                    };
                    push(result, element)?;
                    result_key = Some(Key::Int(self.result_counter));
                    self.result_counter += 1;
                    continue;
                }

                let own_id = id.get(dql_alias).cloned().unwrap_or_default();
                // check for existing result from the iterations before
                let existing = self
                    .root_identifier_map
                    .get(dql_alias)
                    .and_then(|m| m.get(&own_id))
                    .cloned();
                if let Some(index) = existing {
                    // Update result pointer
                    let element = result.get(&index).cloned().unwrap_or(Value::Null);
                    self.result_pointers.insert(dql_alias.to_owned(), element);
                    result_key = Some(index);
                } else {
                    let entity = self.get_entity(data, dql_alias).await?;
                    let element = if rsm.is_mixed {
                        let mut element = PhpArray::new();
                        element.insert(entity_key, Value::Object(entity));
                        Value::Array(element)
                    } else {
                        Value::Object(entity)
                    };

                    let key = if let Some(index_by) = rsm.index_by_map.get(dql_alias) {
                        let key = value_key(
                            &row.get(&Key::from(index_by.as_str()))
                                .cloned()
                                .unwrap_or(Value::Null),
                        );
                        if let Some(collection) = &self.collection {
                            collection.hydrate_set(key.clone(), element.clone())?;
                        }
                        result.insert(key.clone(), element.clone());
                        key
                    } else {
                        let key = Key::Int(self.result_counter);
                        self.result_counter += 1;
                        if let Some(collection) = &self.collection {
                            collection.hydrate_add(element.clone())?;
                        }
                        push(result, element.clone())?;
                        key
                    };
                    self.root_identifier_map
                        .entry(dql_alias.to_owned())
                        .or_default()
                        .insert(own_id, key.clone());
                    result_key = Some(key);

                    // Update result pointer
                    self.result_pointers.insert(dql_alias.to_owned(), element);
                }
            }

            if self.hints.internal_iteration {
                self.uow().hydration_complete()?;
            }
        }

        if result_key.is_none() {
            self.result_counter += 1;
        }

        let scalar_count = row_data.scalars.as_ref().map_or(0, PhpArray::len);
        // Append scalar values to mixed result sets
        if let Some(scalars) = row_data.scalars {
            let key = match &result_key {
                Some(key) => key.clone(),
                None => match rsm.index_by_map.get("scalars") {
                    Some(column) => value_key(
                        &row.get(&Key::from(column.as_str()))
                            .cloned()
                            .unwrap_or(Value::Null),
                    ),
                    None => Key::Int(self.result_counter - 1),
                },
            };
            for (name, value) in scalars.iter() {
                set_in_row(result, &key, name.clone(), value.clone());
            }
            result_key = Some(key);
        }

        // Append new object to mixed result sets
        if !row_data.new_objects.is_empty() {
            let key = result_key.unwrap_or(Key::Int(self.result_counter - 1));
            let count = row_data.new_objects.len();
            for (obj_index, obj, _) in row_data.new_objects {
                if scalar_count == 0 && count == 1 {
                    result.insert(key.clone(), obj);
                    continue;
                }
                set_in_row(result, &key, obj_index, obj);
            }
        }

        Ok(())
    }
}

fn undefined_key(key: &str) -> Thrown {
    Thrown::new("Error", format!("Undefined array key \"{key}\""), 0)
}

/// `$result[] = $element`.
pub(crate) fn push(result: &mut PhpArray, element: Value) -> Result<(), Thrown> {
    result.push(element).map(|_| ()).map_err(|_| {
        Thrown::new(
            "Error",
            "Cannot add element to the array as the next element is already occupied",
            0,
        )
    })
}

/// `$result[$resultKey][$name] = $value`.
pub(crate) fn set_in_row(result: &mut PhpArray, key: &Key, name: Key, value: Value) {
    match result.get_mut(key) {
        Some(Value::Array(row)) => row.insert(name, value),
        Some(Value::Null) | None => {
            let mut row = PhpArray::new();
            row.insert(name, value);
            result.insert(key.clone(), Value::Array(row));
        }
        Some(_) => {}
    }
}
