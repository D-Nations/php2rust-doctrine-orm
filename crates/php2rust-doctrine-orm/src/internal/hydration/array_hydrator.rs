//! `ArrayHydrator`: hydrates the result as nested arrays. PHP keeps
//! references into the result tree (`$this->resultPointers`); here a
//! pointer is the path of keys from the result's root.
//!
//! Upstream: doctrine/orm src/Internal/Hydration/ArrayHydrator.php

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use php_value::{Key, PhpArray, Thrown, Value};

use super::abstract_hydrator::{Columns, value_key};
use super::object_hydrator::{push, set_in_row};
use crate::entity_manager::EntityManager;
use crate::query::ResultSetMapping;

/// A result pointer: `None` is the initial `[]` of `prepare()`.
type Pointer = Option<Vec<Key>>;

/// `ArrayHydrator`.
pub(crate) struct ArrayHydrator<'a> {
    columns: Columns<'a>,
    root_aliases: HashSet<String>,
    is_simple_query: bool,
    root_identifier_map: HashMap<String, HashMap<String, Key>>,
    identifier_map: HashMap<String, HashMap<String, HashMap<String, Key>>>,
    /// The pointers in their PHP array order.
    result_pointers: Vec<(String, Pointer)>,
    id_template: HashMap<String, String>,
    result_counter: i64,
}

/// The value at a path of the result tree.
fn at<'r>(root: &'r PhpArray, path: &[Key]) -> Option<&'r Value> {
    let (first, rest) = path.split_first()?;
    let mut value = root.get(first)?;
    for key in rest {
        match value {
            Value::Array(array) => value = array.get(key)?,
            _ => return None,
        }
    }
    Some(value)
}

/// The array at a path of the result tree (the root for an empty path),
/// made an array when it is `null` or missing (PHP's auto-vivification).
fn array_at<'r>(root: &'r mut PhpArray, path: &[Key]) -> Option<&'r mut PhpArray> {
    let mut current = root;
    for key in path {
        if !matches!(current.get(key), Some(Value::Array(_))) {
            if current.get(key).is_some_and(|v| !v.is_null()) {
                return None;
            }
            current.insert(key.clone(), Value::Array(PhpArray::new()));
        }
        match current.get_mut(key) {
            Some(Value::Array(array)) => current = array,
            _ => return None,
        }
    }
    Some(current)
}

impl<'a> ArrayHydrator<'a> {
    pub fn new(
        em: &'a Arc<EntityManager>,
        rsm: &'a ResultSetMapping,
        force_dbal_type_conversion: bool,
    ) -> Self {
        let mut columns = Columns::new(em, rsm, force_dbal_type_conversion);
        // The array result holds the enum cases.
        columns.set_build_field_enums(true);
        Self {
            columns,
            root_aliases: HashSet::new(),
            is_simple_query: false,
            root_identifier_map: HashMap::new(),
            identifier_map: HashMap::new(),
            result_pointers: Vec::new(),
            id_template: HashMap::new(),
            result_counter: 0,
        }
    }

    /// `prepare()`.
    pub fn prepare(&mut self) {
        let rsm = self.columns.rsm;
        self.is_simple_query = rsm.alias_map.len() <= 1;
        for (dql_alias, _) in rsm.alias_map.iter() {
            self.result_pointers.push((dql_alias.to_owned(), None));
            self.id_template.insert(dql_alias.to_owned(), String::new());
        }
    }

    fn pointer(&self, alias: &str) -> Option<&Pointer> {
        self.result_pointers
            .iter()
            .find(|(a, _)| a == alias)
            .map(|(_, p)| p)
    }

    fn set_pointer(&mut self, alias: &str, path: Vec<Key>) {
        if let Some((_, pointer)) = self.result_pointers.iter_mut().find(|(a, _)| a == alias) {
            *pointer = Some(path);
        } else {
            self.result_pointers.push((alias.to_owned(), Some(path)));
        }
    }

    fn unset_pointer(&mut self, alias: &str) {
        self.result_pointers.retain(|(a, _)| a != alias);
    }

    /// `hydrateAllData()`.
    pub fn hydrate_all_data(&mut self, rows: &[PhpArray]) -> Result<PhpArray, Thrown> {
        let mut result = PhpArray::new();
        for row in rows {
            self.hydrate_row_data(row, &mut result)?;
        }
        Ok(result)
    }

    /// `hydrateRowData($row, &$result)`.
    #[allow(clippy::too_many_lines, reason = "hydrateRowData(), in its order")]
    pub fn hydrate_row_data(
        &mut self,
        row: &PhpArray,
        result: &mut PhpArray,
    ) -> Result<(), Thrown> {
        let rsm = self.columns.rsm;
        // 1) Initialize
        let mut id = self.id_template.clone(); // initialize the id-memory
        let mut nonempty_components = HashSet::new();
        let row_data = self
            .columns
            .gather_row_data(row, &mut id, &mut nonempty_components)?;

        let mut result_key: Option<Key> = None;

        // 2) Now hydrate the data found in the current row.
        for (dql_alias, data) in row_data.data {
            let dql_alias = dql_alias.as_str();
            let mut index: Option<Key> = None;

            if let Some(parent) = rsm.parent_alias_map.get(dql_alias) {
                // It's a joined result
                let path = format!("{parent}.{dql_alias}");

                // missing parent data, skipping as RIGHT JOIN hydration is
                // not supported.
                if !nonempty_components.contains(parent) {
                    continue;
                }

                // Get a reference to the right element in the result tree.
                // This element will get the associated element attached.
                let base_path: Vec<Key> = if rsm.is_mixed && self.root_aliases.contains(parent) {
                    // reset($this->resultPointers) and its first key
                    let first_key = match self.result_pointers.first() {
                        Some((_, Some(first))) => match at(result, first) {
                            Some(Value::Array(first)) => first.keys().next().cloned(),
                            _ => None,
                        },
                        _ => None,
                    }
                    .unwrap_or_else(|| Key::from(""));
                    if let Some(Some(p)) = self.pointer(parent) {
                        let mut p = p.clone();
                        p.push(first_key);
                        p
                    } else {
                        self.unset_pointer(dql_alias);
                        continue;
                    }
                } else if let Some(Some(p)) = self.pointer(parent) {
                    p.clone()
                } else {
                    self.unset_pointer(dql_alias); // Ticket #1228
                    continue;
                };

                let relation_alias = rsm.relation_map.get(dql_alias).cloned().unwrap_or_default();
                let parent_class = self
                    .columns
                    .get_class_metadata(rsm.alias_map.get(parent).map_or("", String::as_str))?;
                let is_to_one = parent_class
                    .read()
                    .association_mappings
                    .get(&relation_alias)
                    .ok_or_else(|| {
                        Thrown::new(
                            "Error",
                            format!("Undefined array key \"{relation_alias}\""),
                            0,
                        )
                    })?
                    .is_to_one();
                let relation_key = Key::from(relation_alias.as_str());
                let Some(base_element) = array_at(result, &base_path) else {
                    continue;
                };

                let one_to_one;
                // Check the type of the relation (many or single-valued)
                if is_to_one {
                    one_to_one = true;
                    if base_element.get(&relation_key).is_none_or(Value::is_null) {
                        if nonempty_components.contains(dql_alias) {
                            base_element.insert(relation_key.clone(), Value::Array(data));
                        } else {
                            base_element.insert(relation_key.clone(), Value::Null);
                        }
                    }
                } else {
                    one_to_one = false;
                    if base_element.get(&relation_key).is_none_or(Value::is_null) {
                        base_element.insert(relation_key.clone(), Value::Array(PhpArray::new()));
                    }

                    if nonempty_components.contains(dql_alias) {
                        let parent_id = id.get(parent).cloned().unwrap_or_default();
                        let own_id = id.get(dql_alias).cloned().unwrap_or_default();
                        index = self
                            .identifier_map
                            .get(&path)
                            .and_then(|m| m.get(&parent_id))
                            .and_then(|m| m.get(&own_id))
                            .cloned();
                        let index_is_valid = index.as_ref().is_some_and(|index| {
                            matches!(base_element.get(&relation_key), Some(Value::Array(coll)) if coll.get(index).is_some_and(|v| !v.is_null()))
                        });

                        if !index_is_valid {
                            let Some(Value::Array(coll)) = base_element.get_mut(&relation_key)
                            else {
                                continue;
                            };
                            if let Some(index_by) = rsm.index_by_map.get(dql_alias) {
                                let key = value_key(
                                    &row.get(&Key::from(index_by.as_str()))
                                        .cloned()
                                        .unwrap_or(Value::Null),
                                );
                                coll.insert(key, Value::Array(data));
                            } else {
                                push(coll, Value::Array(data))?;
                            }
                            let last = coll.last().map_or(Key::Int(0), |(k, _)| k.clone());
                            self.identifier_map
                                .entry(path)
                                .or_default()
                                .entry(parent_id)
                                .or_default()
                                .insert(own_id, last);
                        }
                    }
                }

                let mut coll_path = base_path;
                coll_path.push(relation_key);
                if matches!(at(result, &coll_path), Some(Value::Array(_))) {
                    self.update_result_pointer(result, coll_path, index, dql_alias, one_to_one);
                }
            } else {
                // It's a root result element

                self.root_aliases.insert(dql_alias.to_owned()); // Mark as root
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
                // Check for an existing element
                let existing = self
                    .root_identifier_map
                    .get(dql_alias)
                    .and_then(|m| m.get(&own_id))
                    .cloned();
                match existing {
                    Some(existing) if !self.is_simple_query => {
                        result_key = Some(existing.clone());
                        index = Some(existing);
                    }
                    _ => {
                        let element = if rsm.is_mixed {
                            let mut element = PhpArray::new();
                            element.insert(entity_key, Value::Array(data));
                            Value::Array(element)
                        } else {
                            Value::Array(data)
                        };

                        let key = if let Some(index_by) = rsm.index_by_map.get(dql_alias) {
                            let key = value_key(
                                &row.get(&Key::from(index_by.as_str()))
                                    .cloned()
                                    .unwrap_or(Value::Null),
                            );
                            result.insert(key.clone(), element);
                            key
                        } else {
                            let key = Key::Int(self.result_counter);
                            push(result, element)?;
                            self.result_counter += 1;
                            key
                        };
                        self.root_identifier_map
                            .entry(dql_alias.to_owned())
                            .or_default()
                            .insert(own_id, key.clone());
                        result_key = Some(key);
                    }
                }

                self.update_result_pointer(result, Vec::new(), index, dql_alias, false);
            }
        }

        if result_key.is_none() {
            self.result_counter += 1;
        }

        let scalar_count = row_data.scalars.as_ref().map_or(0, PhpArray::len);
        // Append scalar values to mixed result sets
        if let Some(scalars) = row_data.scalars {
            // this only ever happens when no object is fetched (scalar
            // result only)
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
            for (obj_index, obj, args) in row_data.new_objects {
                if args == scalar_count || (scalar_count == 0 && count == 1) {
                    result.insert(key.clone(), obj);
                    continue;
                }
                set_in_row(result, &key, obj_index, obj);
            }
        }

        Ok(())
    }

    /// `updateResultPointer(&$coll, $index, $dqlAlias, $oneToOne)`.
    fn update_result_pointer(
        &mut self,
        result: &PhpArray,
        coll_path: Vec<Key>,
        index: Option<Key>,
        dql_alias: &str,
        one_to_one: bool,
    ) {
        let coll = if coll_path.is_empty() {
            Some(result)
        } else {
            match at(result, &coll_path) {
                Some(Value::Array(coll)) => Some(coll),
                _ => None,
            }
        };
        let Some(coll) = coll else {
            self.unset_pointer(dql_alias); // Ticket #1228
            return;
        };

        if one_to_one {
            self.set_pointer(dql_alias, coll_path);
            return;
        }

        if let Some(index) = index {
            let mut path = coll_path;
            path.push(index);
            self.set_pointer(dql_alias, path);
            return;
        }

        let Some((last, _)) = coll.last() else {
            return;
        };
        let mut path = coll_path;
        path.push(last.clone());
        self.set_pointer(dql_alias, path);
    }
}
