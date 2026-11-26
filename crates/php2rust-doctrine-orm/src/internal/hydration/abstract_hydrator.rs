//! `AbstractHydrator`: what the hydrators share, the column information of
//! a result set mapping and the split of a row into per-alias data,
//! scalars and new objects.
//!
//! A result is read whole (the DBAL port fetches all rows), so the hydrators
//! take the rows instead of a statement. The `onClear` listener is not
//! registered: nothing can clear the entity manager between two rows of
//! one hydration. Enum fields keep their backing values (`buildEnum()`
//! needs PHP's enums).
//!
//! Upstream: doctrine/orm src/Internal/Hydration/AbstractHydrator.php src/Internal/Hydration/HydrationException.php src/Exception/MultipleSelectorsFoundException.php

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, OnceLock, PoisonError, RwLock};

use php_value::{Key, PhpArray, Thrown, Value};
use php2rust_doctrine_dbal::types::{self, Type};

use crate::entity_manager::EntityManager;
use crate::mapping::ClassMetadata;
use crate::query::ResultSetMapping;
use crate::utility::{key_text, to_text};

/// `HydrationException::class`.
pub const HYDRATION_EXCEPTION: &str = "Doctrine\\ORM\\Internal\\Hydration\\HydrationException";
/// `MultipleSelectorsFoundException::class`.
pub const MULTIPLE_SELECTORS_FOUND: &str =
    "Doctrine\\ORM\\Exception\\MultipleSelectorsFoundException";

fn hydration_exception(message: String) -> Thrown {
    Thrown::new(HYDRATION_EXCEPTION, message, 0)
}

/// `HydrationException::nonUniqueResult()`.
pub fn non_unique_result() -> Thrown {
    hydration_exception("The result returned by the query was not unique.".to_owned())
}

/// `HydrationException::parentObjectOfRelationNotFound()`.
pub fn parent_object_of_relation_not_found(alias: &str, parent_alias: &str) -> Thrown {
    hydration_exception(format!(
        "The parent object of entity result with alias '{alias}' was not found. The parent alias is '{parent_alias}'."
    ))
}

/// `HydrationException::emptyDiscriminatorValue()`.
pub fn empty_discriminator_value(dql_alias: &str) -> Thrown {
    hydration_exception(format!(
        "The DQL alias '{dql_alias}' contains an entity of an inheritance hierarchy with an empty discriminator value. This means that the database contains inconsistent data with an empty discriminator value in a table row."
    ))
}

/// `HydrationException::missingDiscriminatorColumn()`.
pub fn missing_discriminator_column(
    entity_name: &str,
    discr_column_name: &str,
    dql_alias: &str,
) -> Thrown {
    hydration_exception(format!(
        "The discriminator column \"{discr_column_name}\" is missing for \"{entity_name}\" using the DQL alias \"{dql_alias}\"."
    ))
}

/// `HydrationException::missingDiscriminatorMetaMappingColumn()`.
pub fn missing_discriminator_meta_mapping_column(
    entity_name: &str,
    discr_column_name: &str,
    dql_alias: &str,
) -> Thrown {
    hydration_exception(format!(
        "The meta mapping for the discriminator column \"{discr_column_name}\" is missing for \"{entity_name}\" using the DQL alias \"{dql_alias}\"."
    ))
}

/// `HydrationException::invalidDiscriminatorValue()`.
pub fn invalid_discriminator_value(discr_value: &str, discr_values: &[String]) -> Thrown {
    hydration_exception(format!(
        "The discriminator value \"{discr_value}\" is invalid. It must be one of \"{}\".",
        discr_values.join("\", \"")
    ))
}

/// `HydrationException::partialObjectHydrationDisallowed()`.
pub fn partial_object_hydration_disallowed() -> Thrown {
    hydration_exception(
        "Hydration of entity objects is not allowed when DQL PARTIAL keyword is used.".to_owned(),
    )
}

/// `MultipleSelectorsFoundException::create($selectors)`.
pub fn multiple_selectors_found(selectors: &[String]) -> Thrown {
    Thrown::new(
        MULTIPLE_SELECTORS_FOUND,
        format!(
            "Multiple selectors found: {}. Please select only one.",
            selectors.join(", ")
        ),
        0,
    )
}

/// Declares the hydration exception classes in persistence's class
/// registry.
pub(crate) fn register_classes() {
    use php2rust_doctrine_persistence::php_reflection::{ClassInfo, register_class};

    register_class(
        ClassInfo::class(HYDRATION_EXCEPTION)
            .extends("Exception")
            .implements("Doctrine\\ORM\\Exception\\ORMException"),
    );
    register_class(
        ClassInfo::class(MULTIPLE_SELECTORS_FOUND)
            .extends("LogicException")
            .implements("Doctrine\\ORM\\Exception\\ORMException"),
    );
}

/// `new $className(...$args)` for the class of a DQL `NEW` expression.
pub type NewObjectConstructor = Arc<dyn Fn(Vec<Value>) -> Result<Value, Thrown> + Send + Sync>;

fn new_object_classes() -> &'static RwLock<HashMap<String, NewObjectConstructor>> {
    static CLASSES: OnceLock<RwLock<HashMap<String, NewObjectConstructor>>> = OnceLock::new();
    CLASSES.get_or_init(Default::default)
}

/// Registers the constructor of a class a DQL `NEW` expression names: Rust
/// has no classes to instantiate by name (`ReflectionClass::newInstanceArgs()`).
pub fn register_new_object_class(class_name: &str, constructor: NewObjectConstructor) {
    let mut classes = new_object_classes()
        .write()
        .unwrap_or_else(PoisonError::into_inner);
    classes.insert(class_name.trim_start_matches('\\').to_owned(), constructor);
}

fn new_instance_args(class_name: &str, args: Vec<Value>) -> Result<Value, Thrown> {
    let constructor = new_object_classes()
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .get(class_name.trim_start_matches('\\'))
        .cloned();
    match constructor {
        Some(constructor) => constructor(args),
        None => Err(Thrown::new(
            "ReflectionException",
            format!("Class \"{class_name}\" does not exist"),
            0,
        )),
    }
}

/// The kind of a result column (`hydrateColumnInfo()`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ColumnKind {
    /// A field of an entity.
    Field,
    /// An argument of a `NEW` object (also a scalar).
    NewObjectParameter,
    /// A scalar, type converted for the paginator
    /// (`FORCE_DBAL_TYPE_CONVERSION`), hydrated like a field of alias `''`.
    ForcedScalar,
    /// A scalar.
    Scalar,
    /// A meta column (a foreign key or a discriminator column).
    MetaColumn,
}

/// The cached information of a result column.
#[derive(Clone)]
pub(crate) struct ColumnInfo {
    pub kind: ColumnKind,
    pub field_name: Key,
    pub ty: Option<Arc<dyn Type>>,
    pub dql_alias: String,
    pub is_identifier: bool,
    pub arg_index: Option<Key>,
    pub obj_index: Option<Key>,
    /// The discriminator column of the owner and the discriminator values
    /// of the declaring class and its subclasses.
    pub discriminator: Option<(String, Vec<String>)>,
}

/// A row split into the data of each alias, the scalars and the new
/// objects (`gatherRowData()`).
#[derive(Debug, Default)]
pub(crate) struct RowData {
    /// The data of each DQL alias, in the order the row names them.
    pub data: Vec<(String, PhpArray)>,
    /// `$rowData['scalars']`.
    pub scalars: Option<PhpArray>,
    /// `$rowData['newObjects']`: the objects and the count of their
    /// arguments.
    pub new_objects: Vec<(Key, Value, usize)>,
}

impl RowData {
    pub fn data_mut(&mut self, alias: &str) -> &mut PhpArray {
        let position = if let Some(position) = self.data.iter().position(|(a, _)| a == alias) {
            position
        } else {
            self.data.push((alias.to_owned(), PhpArray::new()));
            self.data.len() - 1
        };
        &mut self.data[position].1
    }
}

fn get_type(name: &str) -> Result<Arc<dyn Type>, Thrown> {
    types::get_type(name).map_err(|e| Thrown::new(e.class(), e.message(), 0))
}

/// The state every hydrator keeps: the entity manager, the result set
/// mapping, and the caches of column information and class metadata.
pub(crate) struct Columns<'a> {
    pub em: &'a Arc<EntityManager>,
    pub rsm: &'a ResultSetMapping,
    force_dbal_type_conversion: bool,
    cache: HashMap<String, Option<ColumnInfo>>,
    metadata_cache: HashMap<String, Arc<ClassMetadata>>,
}

impl<'a> Columns<'a> {
    pub fn new(
        em: &'a Arc<EntityManager>,
        rsm: &'a ResultSetMapping,
        force_dbal_type_conversion: bool,
    ) -> Self {
        Self {
            em,
            rsm,
            force_dbal_type_conversion,
            cache: HashMap::new(),
            metadata_cache: HashMap::new(),
        }
    }

    /// `getClassMetadata($className)`.
    pub fn get_class_metadata(&mut self, class_name: &str) -> Result<Arc<ClassMetadata>, Thrown> {
        if let Some(class) = self.metadata_cache.get(class_name) {
            return Ok(class.clone());
        }
        let class = self.em.get_class_metadata(class_name)?;
        self.metadata_cache
            .insert(class_name.to_owned(), class.clone());
        Ok(class)
    }

    fn convert(&self, ty: Option<&Arc<dyn Type>>, value: &Value) -> Result<Value, Thrown> {
        match ty {
            Some(ty) => ty
                .convert_to_php_value(value, self.em.get_platform())
                .map_err(|e| crate::exception::from_conversion(&e)),
            None => Ok(value.clone()),
        }
    }

    /// `hydrateColumnInfo($key)`.
    pub fn hydrate_column_info(&mut self, key: &str) -> Result<Option<ColumnInfo>, Thrown> {
        if let Some(info) = self.cache.get(key) {
            return Ok(info.clone());
        }
        let info = self.column_info(key)?;
        self.cache.insert(key.to_owned(), info.clone());
        Ok(info)
    }

    fn column_info(&mut self, key: &str) -> Result<Option<ColumnInfo>, Thrown> {
        let rsm = self.rsm;
        let type_of = |key: &str| -> Result<Option<Arc<dyn Type>>, Thrown> {
            rsm.type_mappings.get(key).map(|t| get_type(t)).transpose()
        };
        // NOTE: Most of the times it's a field mapping, so keep it first!!!
        if let Some(field_name) = rsm.field_mappings.get(key) {
            let declaring = rsm.declaring_classes.get(key).cloned().unwrap_or_default();
            let class = self.get_class_metadata(&declaring)?;
            let owner = rsm.column_owner_map.get(key).cloned().unwrap_or_default();
            let (is_identifier, type_name, has_parents, discriminator_value, sub_classes) = {
                let info = class.read();
                let mapping = info.field_mappings.get(field_name).ok_or_else(|| {
                    Thrown::new(
                        "Error",
                        format!("Undefined array key \"{field_name}\""),
                        0,
                    )
                })?;
                (
                    info.identifier.contains(field_name),
                    mapping.r#type.clone(),
                    !info.parent_classes.is_empty(),
                    to_text(&info.discriminator_value),
                    info.sub_classes.clone(),
                )
            };
            let mut column = ColumnInfo {
                kind: ColumnKind::Field,
                field_name: Key::from(field_name.as_str()),
                ty: Some(get_type(&type_name)?),
                dql_alias: owner.clone(),
                is_identifier,
                arg_index: None,
                obj_index: None,
                discriminator: None,
            };
            // the current discriminator value must be saved in order to
            // disambiguate fields hydration, should there be field name
            // collisions
            if has_parents && let Some(discr_column) = rsm.discriminator_columns.get(&owner) {
                let mut values = Vec::new();
                for sub_class in sub_classes {
                    let sub = self.get_class_metadata(&sub_class)?;
                    values.push(to_text(&sub.read().discriminator_value));
                }
                values.push(discriminator_value);
                column.discriminator = Some((discr_column.clone(), values));
            }
            return Ok(Some(column));
        }

        let key_k = Key::from(key);
        if let Some(Value::Array(mapping)) = rsm.new_object_mappings.get(&key_k) {
            // WARNING: A NEW object is also a scalar, so it must be
            // declared before!
            let index = |name: &str| mapping.get(&Key::from(name)).and_then(Key::from_value);
            return Ok(Some(ColumnInfo {
                kind: ColumnKind::NewObjectParameter,
                field_name: rsm.scalar_mappings.get(key).cloned().unwrap_or(Key::Int(0)),
                ty: type_of(key)?,
                dql_alias: String::new(),
                is_identifier: false,
                arg_index: index("argIndex"),
                obj_index: index("objIndex"),
                discriminator: None,
            }));
        }

        if let Some(field_name) = rsm.scalar_mappings.get(key) {
            return Ok(Some(ColumnInfo {
                kind: if self.force_dbal_type_conversion {
                    ColumnKind::ForcedScalar
                } else {
                    ColumnKind::Scalar
                },
                field_name: field_name.clone(),
                ty: type_of(key)?,
                dql_alias: String::new(),
                is_identifier: false,
                arg_index: None,
                obj_index: None,
                discriminator: None,
            }));
        }

        if let Some(field_name) = rsm.meta_mappings.get(key) {
            // Meta column (has meaning in relational schema only, i.e.
            // foreign keys or discriminator columns).
            let dql_alias = rsm.column_owner_map.get(key).cloned().unwrap_or_default();
            // Cache metadata fetch
            if let Some(class_name) = rsm.alias_map.get(&dql_alias) {
                self.get_class_metadata(class_name)?;
            }
            return Ok(Some(ColumnInfo {
                kind: ColumnKind::MetaColumn,
                field_name: Key::from(field_name.as_str()),
                ty: type_of(key)?,
                dql_alias: dql_alias.clone(),
                is_identifier: rsm
                    .is_identifier_column
                    .get(&dql_alias)
                    .is_some_and(|columns| columns.contains_key(key)),
                arg_index: None,
                obj_index: None,
                discriminator: None,
            }));
        }

        // this column is a left over, maybe from a LIMIT query hack for
        // example in Oracle or DB2 maybe from an additional column that
        // has not been defined in a NativeQuery ResultSetMapping.
        Ok(None)
    }

    /// `gatherRowData($data, &$id, &$nonemptyComponents)`.
    #[allow(clippy::too_many_lines, reason = "gatherRowData(), in its order")]
    pub fn gather_row_data(
        &mut self,
        row: &PhpArray,
        id: &mut HashMap<String, String>,
        nonempty_components: &mut HashSet<String>,
    ) -> Result<RowData, Thrown> {
        let rsm = self.rsm;
        let mut row_data = RowData::default();

        // The classes of the new objects, by object index.
        let mut new_object_classes: Vec<(Key, String)> = rsm
            .new_object
            .iter()
            .map(|(k, v)| (k.clone(), to_text(v)))
            .collect();
        for mapping in rsm.new_object_mappings.values() {
            if let Value::Array(mapping) = mapping
                && let Some(obj_index) = mapping.get(&Key::from("objIndex")).and_then(Key::from_value)
                && !new_object_classes.iter().any(|(k, _)| *k == obj_index)
            {
                let class_name = mapping
                    .get(&Key::from("className"))
                    .map(to_text)
                    .unwrap_or_default();
                new_object_classes.push((obj_index, class_name));
            }
        }
        let mut new_object_args: Vec<(Key, PhpArray)> = new_object_classes
            .iter()
            .map(|(k, _)| (k.clone(), PhpArray::new()))
            .collect();

        for (key, value) in row.iter() {
            let key = key_text(key);
            let Some(info) = self.hydrate_column_info(&key)? else {
                continue;
            };

            match info.kind {
                ColumnKind::NewObjectParameter => {
                    let value = self.convert(info.ty.as_ref(), value)?;
                    let obj_index = info.obj_index.clone().unwrap_or(Key::Int(0));
                    let arg_index = info.arg_index.clone().unwrap_or(Key::Int(0));
                    if let Some((_, args)) =
                        new_object_args.iter_mut().find(|(k, _)| *k == obj_index)
                    {
                        args.insert(arg_index, value);
                    } else {
                        let mut args = PhpArray::new();
                        args.insert(arg_index, value);
                        new_object_args.push((obj_index, args));
                    }
                }
                ColumnKind::Scalar => {
                    let value = self.convert(info.ty.as_ref(), value)?;
                    row_data
                        .scalars
                        .get_or_insert_with(PhpArray::new)
                        .insert(info.field_name.clone(), value);
                }
                ColumnKind::Field | ColumnKind::MetaColumn | ColumnKind::ForcedScalar => {
                    let dql_alias = info.dql_alias.clone();

                    // If there are field name collisions in the child
                    // class, then we need to only hydrate if we are
                    // looking at the correct discriminator value
                    if let Some((discr_column, values)) = &info.discriminator
                        && let Some(discr) = row.get(&Key::from(discr_column.as_str()))
                        && !discr.is_null()
                        && !values.contains(&to_text(discr))
                    {
                        continue;
                    }

                    // in an inheritance hierarchy the same field could be
                    // defined several times. We overwrite this value so
                    // long we don't have a non-null value, that value we
                    // keep. Per definition it cannot be that a field is
                    // defined several times and has several values.
                    let data = row_data.data_mut(&dql_alias);
                    if data.get(&info.field_name).is_some_and(|v| !v.is_null()) {
                        continue;
                    }
                    let converted = self.convert(info.ty.as_ref(), value)?;
                    row_data
                        .data_mut(&dql_alias)
                        .insert(info.field_name.clone(), converted);

                    if info.is_identifier && !value.is_null() {
                        let entry = id.entry(dql_alias.clone()).or_default();
                        entry.push('|');
                        entry.push_str(&to_text(value));
                        nonempty_components.insert(dql_alias);
                    }
                }
            }
        }

        // The nested new objects and entities of the `NEW` arguments.
        let mut nested_entities: Vec<String> = Vec::new();
        for argument in rsm.nested_new_object_arguments.values() {
            let Value::Array(argument) = argument else {
                continue;
            };
            let field = |name: &str| argument.get(&Key::from(name)).cloned().unwrap_or(Value::Null);
            let owner_index = Key::from_value(&field("ownerIndex")).unwrap_or(Key::Int(0));
            let arg_index = Key::from_value(&field("argIndex")).unwrap_or(Key::Int(0));
            let arg_alias = to_text(&field("argAlias"));
            let arg_alias_key = Key::from_bytes(arg_alias.as_bytes());

            let nested_value = if let Some(position) =
                new_object_args.iter().position(|(k, _)| *k == arg_alias_key)
            {
                let (_, mut args) = new_object_args.remove(position);
                let class_name = new_object_classes
                    .iter()
                    .find(|(k, _)| *k == arg_alias_key)
                    .map(|(_, c)| c.clone())
                    .unwrap_or_default();
                args.ksort();
                new_instance_args(&class_name, args.values().cloned().collect())?
            } else if let Some(position) = row_data.data.iter().position(|(a, _)| *a == arg_alias) {
                if !nested_entities.contains(&arg_alias) {
                    nested_entities.push(arg_alias.clone());
                }
                Value::Array(row_data.data[position].1.clone())
            } else {
                return Err(Thrown::new(
                    "LogicException",
                    format!("{arg_alias} does not exist"),
                    0,
                ));
            };
            if let Some((_, args)) = new_object_args.iter_mut().find(|(k, _)| *k == owner_index) {
                args.insert(arg_index, nested_value);
            } else {
                let mut args = PhpArray::new();
                args.insert(arg_index, nested_value);
                new_object_args.push((owner_index, args));
            }
        }
        if !nested_entities.is_empty() {
            return Err(Thrown::new(
                "LogicException",
                "Entities as arguments of a NEW expression are not supported",
                0,
            ));
        }

        for (obj_index, mut args) in new_object_args {
            args.ksort();
            let count = args.len();
            let class_name = new_object_classes
                .iter()
                .find(|(k, _)| *k == obj_index)
                .map(|(_, c)| c.clone())
                .unwrap_or_default();
            let obj = new_instance_args(&class_name, args.values().cloned().collect())?;
            row_data.new_objects.push((obj_index, obj, count));
        }

        Ok(row_data)
    }

    /// `gatherScalarRowData(&$data)`.
    pub fn gather_scalar_row_data(&mut self, row: &PhpArray) -> Result<PhpArray, Thrown> {
        let mut row_data = PhpArray::new();
        for (key, value) in row.iter() {
            let key = key_text(key);
            let Some(info) = self.hydrate_column_info(&key)? else {
                continue;
            };
            let mut field_name = info.field_name.clone();
            let mut value = value.clone();

            // WARNING: BC break! We know this is the desired behavior to
            // type convert values, but this erroneous behavior exists
            // since 2.0 and we're forced to keep compatibility.
            if !matches!(
                info.kind,
                ColumnKind::Scalar | ColumnKind::NewObjectParameter
            ) {
                value = self.convert(info.ty.as_ref(), &value)?;
                let name = format!("{}_{}", info.dql_alias, key_text(&field_name));
                field_name = Key::from_bytes(name.as_bytes());
            }
            row_data.insert(field_name, value);
        }
        Ok(row_data)
    }
}

/// A result value as an array key (PHP's key coercion of
/// `$result[$value]`).
pub(crate) fn value_key(value: &Value) -> Key {
    match value {
        Value::Int(i) => Key::Int(*i),
        Value::Bool(b) => Key::Int(i64::from(*b)),
        #[allow(clippy::cast_possible_truncation, reason = "PHP truncates a float key")]
        Value::Float(f) => Key::Int(*f as i64),
        other => Key::from_bytes(to_text(other).as_bytes()),
    }
}
