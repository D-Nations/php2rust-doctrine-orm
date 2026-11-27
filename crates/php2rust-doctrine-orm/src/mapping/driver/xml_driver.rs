//! `Doctrine\ORM\Mapping\Driver\XmlDriver`: a metadata driver that
//! enables mapping through XML files.
//!
//! The driver is a persistence [`FileDriver`] over [`XmlDriver`] (the
//! abstract methods); [`XmlDriver::create`] is the constructor. The files
//! are read with php2rust-dom, and the tree is read as `SimpleXML` reads it:
//! `$element->name` and iteration see the child elements that have no
//! namespace prefix, `$element['name']` the attributes without one, and
//! `(string) $element` the element's own text and CDATA children.
//!
//! XSD validation (`$isXsdValidationEnabled`) runs when php2rust-dom can
//! read `doctrine-mapping.xsd`. The pinned php2rust-dom refuses it
//! (`xs:complexContent`, `xs:token`), and then the documents are not
//! validated. `parseOptions()`'s `<object class="..."/>` is the class's
//! `newInstanceWithoutConstructor()` (the port runs no PHP constructor);
//! DBAL's `DefaultExpression` classes are its Rust expressions.
//! The `SimpleXML` and DOM extension checks of the constructor always pass.
//!
//! Upstream: doctrine/orm src/Mapping/Driver/XmlDriver.php src/Mapping/Driver/LoadMappingFileImplementation.php src/Mapping/Driver/SimplifiedXmlDriver.php

use std::sync::{Arc, OnceLock};

use php_value::{Key, PhpArray, Thrown, Value};
use php2rust_doctrine_persistence::mapping::ClassMetadata as PersistenceClassMetadata;
use php2rust_doctrine_persistence::mapping::driver::{
    FileDriver, FileDriverImpl, FileDriverLocator, FileLocator, SymfonyFileLocator,
};
use php2rust_doctrine_persistence::php_reflection::{self, ReflectionClass};
use php2rust_dom::{Document, LoadOptions, NodeId, NodeType, Schema};

use crate::events;
use crate::mapping::class_metadata::{self as cm, ClassMetadata, ClassMetadataInfo};
use crate::mapping::driver::attribute_driver::bind_entity_listener;
use crate::mapping::mapping_exception;
use crate::query::SortDirection;

/// `Doctrine\ORM\Mapping\Driver\XmlDriver`.
pub const CLASS: &str = "Doctrine\\ORM\\Mapping\\Driver\\XmlDriver";

/// `XmlDriver::DEFAULT_FILE_EXTENSION`.
pub const DEFAULT_FILE_EXTENSION: &str = ".dcm.xml";

/// `SimplifiedXmlDriver::DEFAULT_FILE_EXTENSION`.
pub const SIMPLIFIED_DEFAULT_FILE_EXTENSION: &str = ".orm.xml";

/// `doctrine-mapping.xsd`, which `validateMapping()` reads.
const MAPPING_XSD: &[u8] = include_bytes!("../../../doctrine-mapping.xsd");

/// An element of a mapping file (`SimpleXMLElement`): a node of its
/// parsed document.
#[derive(Clone, Debug)]
pub struct XmlElement {
    document: Arc<Document>,
    node: NodeId,
}

impl XmlElement {
    fn at(&self, node: NodeId) -> Self {
        Self {
            document: Arc::clone(&self.document),
            node,
        }
    }

    /// `getName()`.
    pub fn get_name(&self) -> &str {
        self.document.local_name(self.node).unwrap_or_default()
    }

    /// `$element[$name]`: the value of the attribute without a prefix.
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.document
            .attributes(self.node)
            .iter()
            .find(|a| a.prefix.is_none() && a.local_name == name)
            .map(|a| a.value.as_str())
    }

    /// `isset($element[$name])`.
    fn has(&self, name: &str) -> bool {
        self.attr(name).is_some()
    }

    /// `(string) $element[$name]`.
    fn string(&self, name: &str) -> String {
        self.attr(name).unwrap_or_default().to_owned()
    }

    /// `isset($element[$name]) ? (string) $element[$name] : null`.
    fn opt_string(&self, name: &str) -> Value {
        self.attr(name).map_or(Value::Null, Value::from)
    }

    /// `$element->children()`: the child elements without a prefix.
    pub fn children(&self) -> Vec<Self> {
        self.document
            .children(self.node)
            .iter()
            .filter(|&&child| {
                self.document.node_type(child) == NodeType::Element
                    && self.document.prefix(child).is_none()
            })
            .map(|&child| self.at(child))
            .collect()
    }

    /// `$element->$name`, iterated: the child elements of that name.
    pub fn children_named(&self, name: &str) -> Vec<Self> {
        self.children()
            .into_iter()
            .filter(|child| child.get_name() == name)
            .collect()
    }

    /// `$element->$name` for one element: the first of them.
    pub fn child(&self, name: &str) -> Option<Self> {
        self.children()
            .into_iter()
            .find(|child| child.get_name() == name)
    }

    /// `$element->$parent->$name ?? []`, iterated.
    fn grandchildren(&self, parent: &str, name: &str) -> Vec<Self> {
        self.child(parent)
            .map(|parent| parent.children_named(name))
            .unwrap_or_default()
    }

    /// `$element->count()`.
    fn count(&self) -> usize {
        self.children().len()
    }

    /// `(string) $element`: its text and CDATA children.
    pub fn text(&self) -> String {
        let mut text = String::new();
        for &child in self.document.children(self.node) {
            if matches!(
                self.document.node_type(child),
                NodeType::Text | NodeType::CData
            ) {
                text.push_str(self.document.node_value(child).unwrap_or_default());
            }
        }
        text
    }

    /// `(bool) $element`: an element is false when it has neither
    /// attributes nor children.
    fn truthy(&self) -> bool {
        !self.document.attributes(self.node).is_empty()
            || !self.document.children(self.node).is_empty()
    }
}

/// The abstract methods of `XmlDriver`, over a persistence
/// [`FileDriver`].
#[derive(Debug)]
pub struct XmlDriver {
    is_xsd_validation_enabled: bool,
}

/// `constant('Doctrine\ORM\Mapping\ClassMetadata::' . $name)`.
fn class_metadata_constant(name: &str) -> Result<i64, Thrown> {
    cm::constant(name).ok_or_else(|| {
        Thrown::new(
            "Error",
            format!("Undefined constant {}::{name}", cm::CLASS),
            0,
        )
    })
}

/// `constant('Doctrine\ORM\Events::' . $name)`.
fn events_constant(name: &str) -> Result<&'static str, Thrown> {
    const EVENTS: [&str; 10] = [
        events::PRE_REMOVE,
        events::POST_REMOVE,
        events::PRE_PERSIST,
        events::POST_PERSIST,
        events::PRE_UPDATE,
        events::POST_UPDATE,
        events::POST_LOAD,
        events::LOAD_CLASS_METADATA,
        events::ON_CLASS_METADATA_NOT_FOUND,
        events::PRE_FLUSH,
    ];
    const OTHERS: [&str; 3] = ["onFlush", "postFlush", "onClear"];
    EVENTS
        .iter()
        .chain(OTHERS.iter())
        .find(|event| **event == name)
        .copied()
        .ok_or_else(|| {
            Thrown::new(
                "Error",
                format!("Undefined constant Doctrine\\ORM\\Events::{name}"),
                0,
            )
        })
}

/// `explode(',', $value)`.
fn explode(value: &str) -> Value {
    Value::Array(PhpArray::list(value.split(',').map(Value::from)))
}

/// `$array[] = $value` (only fails past `PHP_INT_MAX` entries).
fn push(array: &mut PhpArray, value: Value) {
    let _ = array.push(value);
}

/// `$array[$key][] = $value`.
fn push_to(array: &mut PhpArray, key: &str, value: Value) {
    let mut list = match array.get(&Key::from(key)) {
        Some(Value::Array(list)) => list.clone(),
        _ => PhpArray::new(),
    };
    push(&mut list, value);
    array.insert(key, Value::Array(list));
}

/// `(int) $string`.
fn int(value: &str) -> Value {
    Value::Int(php_value::ops::to_int(&Value::from(value)))
}

impl XmlDriver {
    /// `new XmlDriver($locator, $fileExtension, $isXsdValidationEnabled)`.
    pub fn create(
        locator: impl Into<FileDriverLocator>,
        file_extension: &str,
        is_xsd_validation_enabled: bool,
    ) -> FileDriver<Self> {
        FileDriver::new(
            Self {
                is_xsd_validation_enabled,
            },
            locator,
            Some(file_extension),
        )
    }

    /// `new SimplifiedXmlDriver($prefixes, $fileExtension,
    /// $isXsdValidationEnabled)`: a [`SymfonyFileLocator`] over the
    /// prefixes.
    pub fn simplified(
        prefixes: &[(String, String)],
        file_extension: &str,
        is_xsd_validation_enabled: bool,
    ) -> Result<FileDriver<Self>, Thrown> {
        let locator: Arc<dyn FileLocator> = Arc::new(SymfonyFileLocator::new(
            prefixes.to_vec(),
            Some(file_extension),
            ".",
        )?);
        Ok(Self::create(
            locator,
            file_extension,
            is_xsd_validation_enabled,
        ))
    }

    /// `loadMetadataForClass()` on the ORM's class metadata.
    #[allow(
        clippy::too_many_lines,
        reason = "one function upstream; kept in its order"
    )]
    pub fn load(
        driver: &FileDriver<Self>,
        class_name: &str,
        metadata: &mut ClassMetadataInfo,
    ) -> Result<(), Thrown> {
        let xml_root = driver.get_element(class_name)?;

        match xml_root.get_name() {
            "entity" => {
                if let Some(repository_class) = xml_root.attr("repository-class") {
                    metadata.set_custom_repository_class(Some(repository_class));
                }
                if xml_root.attr("read-only").is_some_and(evaluate_boolean) {
                    metadata.mark_read_only();
                }
            }
            "mapped-superclass" => {
                metadata.set_custom_repository_class(xml_root.attr("repository-class"));
                metadata.is_mapped_superclass = true;
            }
            "embeddable" => metadata.is_embedded_class = true,
            _ => {
                return Err(
                    mapping_exception::class_is_not_a_valid_entity_or_mapped_super_class(
                        class_name,
                    ),
                );
            }
        }

        // Evaluate <entity...> attributes
        let mut primary_table = PhpArray::new();
        if let Some(table) = xml_root.attr("table") {
            primary_table.insert("name", Value::from(table));
        }
        if let Some(schema) = xml_root.attr("schema") {
            primary_table.insert("schema", Value::from(schema));
        }
        metadata.set_primary_table(&primary_table);

        // Evaluate second level cache
        if let Some(cache) = xml_root.child("cache") {
            metadata.enable_cache(cache_to_array(&cache)?);
        }

        if let Some(inheritance_type) = xml_root.attr("inheritance-type") {
            metadata.set_inheritance_type(class_metadata_constant(&format!(
                "INHERITANCE_TYPE_{inheritance_type}"
            ))?)?;

            if metadata.inheritance_type != cm::INHERITANCE_TYPE_NONE {
                // Evaluate <discriminator-column...>
                if let Some(discr_column) = xml_root.child("discriminator-column") {
                    let mut column_def = PhpArray::new();
                    column_def.insert("name", discr_column.opt_string("name"));
                    column_def.insert(
                        "type",
                        Value::from(discr_column.attr("type").unwrap_or("string")),
                    );
                    column_def.insert(
                        "length",
                        discr_column.attr("length").map_or(Value::Int(255), int),
                    );
                    column_def.insert(
                        "columnDefinition",
                        discr_column.opt_string("column-definition"),
                    );
                    column_def.insert("enumType", discr_column.opt_string("enum-type"));
                    if discr_column.has("options") {
                        // `$discrColumn['options']->children()`: an
                        // attribute has no child elements.
                        column_def.insert("options", Value::Array(PhpArray::new()));
                    }
                    metadata.set_discriminator_column(Some(column_def))?;
                } else {
                    let mut column_def = PhpArray::new();
                    column_def.insert("name", Value::from("dtype"));
                    column_def.insert("type", Value::from("string"));
                    column_def.insert("length", Value::Int(255));
                    metadata.set_discriminator_column(Some(column_def))?;
                }

                // Evaluate <discriminator-map...>
                if xml_root.child("discriminator-map").is_some() {
                    let mut map = PhpArray::new();
                    for discr_map_element in
                        xml_root.grandchildren("discriminator-map", "discriminator-mapping")
                    {
                        map.insert(
                            Key::from(discr_map_element.string("value")),
                            Value::from(discr_map_element.string("class")),
                        );
                    }
                    metadata.set_discriminator_map(&map)?;
                }
            }
        }

        // Evaluate <change-tracking-policy...>
        if let Some(policy) = xml_root.attr("change-tracking-policy") {
            metadata.set_change_tracking_policy(class_metadata_constant(&format!(
                "CHANGETRACKING_{}",
                policy.to_ascii_uppercase()
            ))?);
        }

        // Evaluate <indexes...>
        if xml_root.child("indexes").is_some() {
            let mut indexes = PhpArray::new();
            for index_xml in xml_root.grandchildren("indexes", "index") {
                let mut index = PhpArray::new();
                if let Some(columns) = index_xml.attr("columns").filter(|c| !is_empty(c)) {
                    index.insert("columns", explode(columns));
                }
                if let Some(fields) = index_xml.attr("fields") {
                    index.insert("fields", explode(fields));
                }
                let has_columns = index.get(&Key::from("columns")).is_some();
                let has_fields = index.get(&Key::from("fields")).is_some();
                if has_columns == has_fields {
                    let name = index_xml
                        .attr("name")
                        .map_or_else(|| indexes.len().to_string(), str::to_owned);
                    return Err(mapping_exception::invalid_index_configuration(
                        class_name, &name,
                    ));
                }
                if let Some(flags) = index_xml.attr("flags") {
                    index.insert("flags", explode(flags));
                }
                if let Some(options) = index_xml.child("options") {
                    index.insert("options", Value::Array(parse_options(&options.children())?));
                }
                match index_xml.attr("name") {
                    Some(name) => indexes.insert(Key::from(name), Value::Array(index)),
                    None => push(&mut indexes, Value::Array(index)),
                }
            }
            metadata.table.insert("indexes", Value::Array(indexes));
        }

        // Evaluate <unique-constraints..>
        if xml_root.child("unique-constraints").is_some() {
            let mut constraints = PhpArray::new();
            for unique_xml in xml_root.grandchildren("unique-constraints", "unique-constraint") {
                let mut unique = PhpArray::new();
                if let Some(columns) = unique_xml.attr("columns").filter(|c| !is_empty(c)) {
                    unique.insert("columns", explode(columns));
                }
                if let Some(fields) = unique_xml.attr("fields") {
                    unique.insert("fields", explode(fields));
                }
                let has_columns = unique.get(&Key::from("columns")).is_some();
                let has_fields = unique.get(&Key::from("fields")).is_some();
                if has_columns == has_fields {
                    let name = unique_xml
                        .attr("name")
                        .map_or_else(|| constraints.len().to_string(), str::to_owned);
                    return Err(mapping_exception::invalid_unique_constraint_configuration(
                        class_name, &name,
                    ));
                }
                if let Some(options) = unique_xml.child("options") {
                    unique.insert("options", Value::Array(parse_options(&options.children())?));
                }
                match unique_xml.attr("name") {
                    Some(name) => {
                        constraints.insert(Key::from(name), Value::Array(unique));
                    }
                    None => push(&mut constraints, Value::Array(unique)),
                }
            }
            metadata
                .table
                .insert("uniqueConstraints", Value::Array(constraints));
        }

        if let Some(options) = xml_root.child("options") {
            metadata
                .table
                .insert("options", Value::Array(parse_options(&options.children())?));
        }

        // Evaluate <field ...> mappings
        for field_mapping in xml_root.children_named("field") {
            let mut mapping = column_to_array(&field_mapping)?;
            if mapping.get(&Key::from("version")).is_some() {
                metadata.set_version_mapping(&mut mapping)?;
                mapping.remove(&Key::from("version"));
            }
            metadata.map_field(mapping)?;
        }

        for embedded_mapping in xml_root.children_named("embedded") {
            let column_prefix = embedded_mapping.opt_string("column-prefix");
            let use_column_prefix = embedded_mapping
                .attr("use-column-prefix")
                .is_none_or(evaluate_boolean);
            let mut mapping = PhpArray::new();
            mapping.insert("fieldName", Value::from(embedded_mapping.string("name")));
            mapping.insert("class", embedded_mapping.opt_string("class"));
            mapping.insert(
                "columnPrefix",
                if use_column_prefix {
                    column_prefix
                } else {
                    Value::Bool(false)
                },
            );
            metadata.map_embedded(mapping)?;
        }

        // Evaluate <id ...> mappings
        let mut association_ids: Vec<(String, Value)> = Vec::new();
        for id_element in xml_root.children_named("id") {
            let position = id_element.attr("position").map_or(Value::Null, int);

            if id_element
                .attr("association-key")
                .is_some_and(evaluate_boolean)
            {
                let name = id_element.string("name");
                match association_ids.iter_mut().find(|(n, _)| *n == name) {
                    Some(entry) => entry.1 = position,
                    None => association_ids.push((name, position)),
                }
                continue;
            }

            let mut mapping = column_to_array(&id_element)?;
            mapping.insert("id", Value::Bool(true));
            mapping.insert("idPosition", position);
            metadata.map_field(mapping)?;

            if let Some(generator) = id_element.child("generator") {
                let strategy = generator.attr("strategy").unwrap_or("AUTO");
                metadata.set_id_generator_type(class_metadata_constant(&format!(
                    "GENERATOR_TYPE_{strategy}"
                ))?);
            }

            // Check for SequenceGenerator/TableGenerator definition
            if let Some(seq_generator) = id_element.child("sequence-generator") {
                let mut definition = PhpArray::new();
                definition.insert(
                    "sequenceName",
                    Value::from(seq_generator.string("sequence-name")),
                );
                definition.insert(
                    "allocationSize",
                    Value::from(seq_generator.string("allocation-size")),
                );
                definition.insert(
                    "initialValue",
                    Value::from(seq_generator.string("initial-value")),
                );
                metadata.set_sequence_generator_definition(definition)?;
            } else if let Some(custom_generator) = id_element.child("custom-id-generator") {
                let mut definition = PhpArray::new();
                definition.insert("class", Value::from(custom_generator.string("class")));
                metadata.set_custom_generator_definition(definition);
            }
        }
        let association_id = |field_name: &str| {
            association_ids
                .iter()
                .find(|(name, _)| name == field_name)
                .map(|(_, position)| position.clone())
        };

        // Evaluate <one-to-one ...> mappings
        for one_to_one_element in xml_root.children_named("one-to-one") {
            let field_name = one_to_one_element.string("field");
            let mut mapping = PhpArray::new();
            mapping.insert("fieldName", Value::from(field_name.as_str()));

            if let Some(target_entity) = one_to_one_element.attr("target-entity") {
                mapping.insert("targetEntity", Value::from(target_entity));
            }
            if let Some(position) = association_id(&field_name) {
                mapping.insert("id", Value::Bool(true));
                mapping.insert("idPosition", position);
            }
            if let Some(fetch) = one_to_one_element.attr("fetch") {
                mapping.insert(
                    "fetch",
                    Value::Int(class_metadata_constant(&format!("FETCH_{fetch}"))?),
                );
            }
            if let Some(mapped_by) = one_to_one_element.attr("mapped-by") {
                mapping.insert("mappedBy", Value::from(mapped_by));
            } else {
                if let Some(inversed_by) = one_to_one_element.attr("inversed-by") {
                    mapping.insert("inversedBy", Value::from(inversed_by));
                }
                mapping.insert(
                    "joinColumns",
                    Value::Array(PhpArray::list(element_join_columns(&one_to_one_element))),
                );
            }
            if let Some(cascade) = one_to_one_element.child("cascade") {
                mapping.insert("cascade", get_cascade_mappings(&cascade));
            }
            if let Some(orphan_removal) = one_to_one_element.attr("orphan-removal") {
                mapping.insert(
                    "orphanRemoval",
                    Value::Bool(evaluate_boolean(orphan_removal)),
                );
            }
            // Evaluate second level cache
            if let Some(cache) = one_to_one_element.child("cache") {
                let defaults =
                    metadata.get_association_cache_defaults(&field_name, cache_to_array(&cache)?);
                mapping.insert("cache", Value::Array(defaults));
            }
            metadata.map_one_to_one(mapping)?;
        }

        // Evaluate <one-to-many ...> mappings
        for one_to_many_element in xml_root.children_named("one-to-many") {
            let field_name = one_to_many_element.string("field");
            let mut mapping = PhpArray::new();
            mapping.insert("fieldName", Value::from(field_name.as_str()));
            mapping.insert(
                "mappedBy",
                Value::from(one_to_many_element.string("mapped-by")),
            );

            if let Some(target_entity) = one_to_many_element.attr("target-entity") {
                mapping.insert("targetEntity", Value::from(target_entity));
            }
            if let Some(fetch) = one_to_many_element.attr("fetch") {
                mapping.insert(
                    "fetch",
                    Value::Int(class_metadata_constant(&format!("FETCH_{fetch}"))?),
                );
            }
            if let Some(cascade) = one_to_many_element.child("cascade") {
                mapping.insert("cascade", get_cascade_mappings(&cascade));
            }
            if let Some(orphan_removal) = one_to_many_element.attr("orphan-removal") {
                mapping.insert(
                    "orphanRemoval",
                    Value::Bool(evaluate_boolean(orphan_removal)),
                );
            }
            if one_to_many_element.child("order-by").is_some() {
                mapping.insert("orderBy", Value::Array(order_by(&one_to_many_element)?));
            }
            if let Some(index_by) = one_to_many_element.attr("index-by") {
                mapping.insert("indexBy", Value::from(index_by));
            } else if one_to_many_element.child("index-by").is_some() {
                return Err(invalid_index_by_tag());
            }
            // Evaluate second level cache
            if let Some(cache) = one_to_many_element.child("cache") {
                let defaults =
                    metadata.get_association_cache_defaults(&field_name, cache_to_array(&cache)?);
                mapping.insert("cache", Value::Array(defaults));
            }
            metadata.map_one_to_many(mapping)?;
        }

        // Evaluate <many-to-one ...> mappings
        for many_to_one_element in xml_root.children_named("many-to-one") {
            let field_name = many_to_one_element.string("field");
            let mut mapping = PhpArray::new();
            mapping.insert("fieldName", Value::from(field_name.as_str()));

            if let Some(target_entity) = many_to_one_element.attr("target-entity") {
                mapping.insert("targetEntity", Value::from(target_entity));
            }
            if let Some(position) = association_id(&field_name) {
                mapping.insert("id", Value::Bool(true));
                mapping.insert("idPosition", position);
            }
            if let Some(fetch) = many_to_one_element.attr("fetch") {
                mapping.insert(
                    "fetch",
                    Value::Int(class_metadata_constant(&format!("FETCH_{fetch}"))?),
                );
            }
            if let Some(inversed_by) = many_to_one_element.attr("inversed-by") {
                mapping.insert("inversedBy", Value::from(inversed_by));
            }
            mapping.insert(
                "joinColumns",
                Value::Array(PhpArray::list(element_join_columns(&many_to_one_element))),
            );
            if let Some(cascade) = many_to_one_element.child("cascade") {
                mapping.insert("cascade", get_cascade_mappings(&cascade));
            }
            // Evaluate second level cache
            if let Some(cache) = many_to_one_element.child("cache") {
                let defaults =
                    metadata.get_association_cache_defaults(&field_name, cache_to_array(&cache)?);
                mapping.insert("cache", Value::Array(defaults));
            }
            metadata.map_many_to_one(mapping)?;
        }

        // Evaluate <many-to-many ...> mappings
        for many_to_many_element in xml_root.children_named("many-to-many") {
            let field_name = many_to_many_element.string("field");
            let mut mapping = PhpArray::new();
            mapping.insert("fieldName", Value::from(field_name.as_str()));

            if let Some(target_entity) = many_to_many_element.attr("target-entity") {
                mapping.insert("targetEntity", Value::from(target_entity));
            }
            if let Some(fetch) = many_to_many_element.attr("fetch") {
                mapping.insert(
                    "fetch",
                    Value::Int(class_metadata_constant(&format!("FETCH_{fetch}"))?),
                );
            }
            if let Some(orphan_removal) = many_to_many_element.attr("orphan-removal") {
                mapping.insert(
                    "orphanRemoval",
                    Value::Bool(evaluate_boolean(orphan_removal)),
                );
            }
            if let Some(mapped_by) = many_to_many_element.attr("mapped-by") {
                mapping.insert("mappedBy", Value::from(mapped_by));
            } else if let Some(join_table_element) = many_to_many_element.child("join-table") {
                if let Some(inversed_by) = many_to_many_element.attr("inversed-by") {
                    mapping.insert("inversedBy", Value::from(inversed_by));
                }

                let mut join_table = PhpArray::new();
                join_table.insert("name", Value::from(join_table_element.string("name")));
                if let Some(schema) = join_table_element.attr("schema") {
                    join_table.insert("schema", Value::from(schema));
                }
                join_table_names_and_columns(&join_table_element, &mut join_table)?;
                mapping.insert("joinTable", Value::Array(join_table));
            }
            if let Some(cascade) = many_to_many_element.child("cascade") {
                mapping.insert("cascade", get_cascade_mappings(&cascade));
            }
            if many_to_many_element.child("order-by").is_some() {
                mapping.insert("orderBy", Value::Array(order_by(&many_to_many_element)?));
            }
            if let Some(index_by) = many_to_many_element.attr("index-by") {
                mapping.insert("indexBy", Value::from(index_by));
            } else if many_to_many_element.child("index-by").is_some() {
                return Err(invalid_index_by_tag());
            }
            // Evaluate second level cache
            if let Some(cache) = many_to_many_element.child("cache") {
                let defaults =
                    metadata.get_association_cache_defaults(&field_name, cache_to_array(&cache)?);
                mapping.insert("cache", Value::Array(defaults));
            }
            metadata.map_many_to_many(mapping)?;
        }

        // Evaluate attribute-overrides
        for override_element in xml_root.grandchildren("attribute-overrides", "attribute-override")
        {
            let field_name = override_element.string("name");
            for field in override_element.children_named("field") {
                let mut mapping = column_to_array(&field)?;
                mapping.insert("fieldName", Value::from(field_name.as_str()));
                metadata.set_attribute_override(&field_name, &mapping)?;
            }
        }

        // Evaluate association-overrides
        for override_element in
            xml_root.grandchildren("association-overrides", "association-override")
        {
            let field_name = override_element.string("name");
            let mut override_mapping = PhpArray::new();

            // Check for join-columns
            if override_element.child("join-columns").is_some() {
                let mut join_columns = Vec::new();
                for join_column_element in
                    override_element.grandchildren("join-columns", "join-column")
                {
                    join_columns.push(Value::Array(join_column_to_array(&join_column_element)));
                }
                override_mapping.insert("joinColumns", Value::Array(PhpArray::list(join_columns)));
            }

            // Check for join-table
            if let Some(join_table_element) = override_element
                .child("join-table")
                .filter(XmlElement::truthy)
            {
                let mut join_table = PhpArray::new();
                join_table.insert("name", Value::from(join_table_element.string("name")));
                join_table.insert("schema", Value::from(join_table_element.string("schema")));
                join_table_names_and_columns(&join_table_element, &mut join_table)?;
                override_mapping.insert("joinTable", Value::Array(join_table));
            }

            // Check for inversed-by
            if let Some(inversed_by) = override_element.child("inversed-by") {
                override_mapping.insert("inversedBy", Value::from(inversed_by.string("name")));
            }

            // Check for `fetch`
            if let Some(fetch) = override_element.attr("fetch") {
                override_mapping.insert(
                    "fetch",
                    Value::Int(class_metadata_constant(&format!("FETCH_{fetch}"))?),
                );
            }

            metadata.set_association_override(&field_name, &override_mapping)?;
        }

        // Evaluate <lifecycle-callbacks...>
        for lifecycle_callback in
            xml_root.grandchildren("lifecycle-callbacks", "lifecycle-callback")
        {
            metadata.add_lifecycle_callback(
                &lifecycle_callback.string("method"),
                events_constant(&lifecycle_callback.string("type"))?,
            )?;
        }

        // Evaluate entity listener
        for listener_element in xml_root.grandchildren("entity-listeners", "entity-listener") {
            let class_name = listener_element.string("class");
            // Evaluate the listener using naming convention.
            if listener_element.count() == 0 {
                bind_entity_listener(metadata, &class_name)?;
                continue;
            }
            for callback_element in listener_element.children() {
                let event_name = callback_element.string("type");
                let method_name = callback_element.string("method");
                metadata.add_entity_listener(&event_name, &class_name, &method_name)?;
            }
        }

        Ok(())
    }

    /// `doLoadMappingFile()`.
    fn do_load_mapping_file(&self, file: &str) -> Result<Vec<(String, XmlElement)>, Thrown> {
        self.validate_mapping(file)?;
        let source = std::fs::read(file).map_err(|e| {
            Thrown::new(
                "ErrorException",
                format!("file_get_contents({file}): Failed to open stream: {e}"),
                0,
            )
        })?;
        // simplexml_load_string() gives false for what does not load.
        let document = php2rust_dom::load_xml(&source, LoadOptions::default())
            .ok()
            .and_then(|loaded| loaded.document)
            .ok_or_else(|| Thrown::new("AssertionError", "assert($xmlElement !== false)", 0))?;
        let document = Arc::new(document);
        let Some(root) = document.document_element() else {
            return Err(Thrown::new(
                "AssertionError",
                "assert($xmlElement !== false)",
                0,
            ));
        };
        let xml_element = XmlElement {
            document: Arc::clone(&document),
            node: root,
        };

        let mut result: Vec<(String, XmlElement)> = Vec::new();
        let mut add = |element: XmlElement| {
            let name = element.string("name");
            match result.iter_mut().find(|(n, _)| *n == name) {
                Some(entry) => entry.1 = element,
                None => result.push((name, element)),
            }
        };
        let entities = xml_element.children_named("entity");
        let mapped_superclasses = xml_element.children_named("mapped-superclass");
        if !entities.is_empty() {
            entities.into_iter().for_each(&mut add);
        } else if !mapped_superclasses.is_empty() {
            mapped_superclasses.into_iter().for_each(&mut add);
        } else {
            xml_element
                .children_named("embeddable")
                .into_iter()
                .for_each(&mut add);
        }
        Ok(result)
    }

    /// `validateMapping()`: the document against `doctrine-mapping.xsd`,
    /// when php2rust-dom can read the schema.
    fn validate_mapping(&self, file: &str) -> Result<(), Thrown> {
        static SCHEMA: OnceLock<Option<Schema>> = OnceLock::new();

        if !self.is_xsd_validation_enabled {
            return Ok(());
        }
        let Some(schema) = SCHEMA
            .get_or_init(|| Schema::parse(MAPPING_XSD, &mut |_| None).ok())
            .as_ref()
        else {
            return Ok(());
        };

        // `$document->load($file)`; what does not load leaves an empty
        // document, which the schema rejects.
        let source = std::fs::read(file).unwrap_or_default();
        let loaded = php2rust_dom::load_xml(&source, LoadOptions::default()).ok();
        let Some(document) = loaded.and_then(|loaded| loaded.document) else {
            return Err(mapping_exception::from_lib_xml_errors(&[]));
        };
        match schema.validate(&document, Some(file)) {
            Ok(validation) if !validation.valid => {
                Err(mapping_exception::from_lib_xml_errors(&validation.errors))
            }
            _ => Ok(()),
        }
    }
}

/// `evaluateBoolean()`.
fn evaluate_boolean(flag: &str) -> bool {
    flag == "true" || flag == "1"
}

/// `empty($string)`.
fn is_empty(value: &str) -> bool {
    value.is_empty() || value == "0"
}

fn invalid_index_by_tag() -> Thrown {
    Thrown::new(
        "InvalidArgumentException",
        "<index-by /> is not a valid tag",
        0,
    )
}

/// `<join-column>` or `<join-columns>` of a to-one association.
fn element_join_columns(element: &XmlElement) -> Vec<Value> {
    let mut join_columns = Vec::new();
    if let Some(join_column) = element.child("join-column") {
        join_columns.push(Value::Array(join_column_to_array(&join_column)));
    } else if element.child("join-columns").is_some() {
        for join_column_element in element.grandchildren("join-columns", "join-column") {
            join_columns.push(Value::Array(join_column_to_array(&join_column_element)));
        }
    }
    join_columns
}

/// The foreign key names, options and join columns of a `<join-table>`
/// (an association override reads the column lists only when their
/// element is there, which gives the same lists).
fn join_table_names_and_columns(
    join_table_element: &XmlElement,
    join_table: &mut PhpArray,
) -> Result<(), Thrown> {
    if let Some(name) = join_table_element.attr("foreign-key-name") {
        join_table.insert("foreignKeyName", Value::from(name));
    }
    if let Some(name) = join_table_element.attr("inverse-foreign-key-name") {
        join_table.insert("inverseForeignKeyName", Value::from(name));
    }
    if let Some(options) = join_table_element.child("options") {
        join_table.insert("options", Value::Array(parse_options(&options.children())?));
    }
    for join_column_element in join_table_element.grandchildren("join-columns", "join-column") {
        push_to(
            join_table,
            "joinColumns",
            Value::Array(join_column_to_array(&join_column_element)),
        );
    }
    for join_column_element in
        join_table_element.grandchildren("inverse-join-columns", "join-column")
    {
        push_to(
            join_table,
            "inverseJoinColumns",
            Value::Array(join_column_to_array(&join_column_element)),
        );
    }
    Ok(())
}

/// The `<order-by>` of a to-many association.
fn order_by(element: &XmlElement) -> Result<PhpArray, Thrown> {
    let mut order_by = PhpArray::new();
    for order_by_field in element.grandchildren("order-by", "order-by-field") {
        order_by.insert(
            Key::from(order_by_field.string("name")),
            get_direction(&order_by_field)?,
        );
    }
    Ok(order_by)
}

/// `parseOptions()`: (nested) option elements.
fn parse_options(options: &[XmlElement]) -> Result<PhpArray, Thrown> {
    let mut array = PhpArray::new();

    for option in options {
        let value = if option.count() > 0 {
            // Check if this option contains an <object> element
            let children = option.children();
            match children.iter().find(|child| child.get_name() == "object") {
                Some(object) => parse_object_element(object)?,
                None => Value::Array(parse_options(&children)?),
            }
        } else {
            Value::from(option.text())
        };

        match option.attr("name") {
            Some(name_attribute) => {
                let value = if name_attribute == "unsigned" || name_attribute == "fixed" {
                    // `(string)` of an array is 'Array'.
                    Value::Bool(match &value {
                        Value::String(s) => evaluate_boolean(&s.to_string_lossy()),
                        _ => false,
                    })
                } else {
                    value
                };
                array.insert(Key::from(name_attribute), value);
            }
            None => push(&mut array, value),
        }
    }

    Ok(array)
}

/// `parseObjectElement()`: the object an `<object class="..."/>` names.
fn parse_object_element(object_element: &XmlElement) -> Result<Value, Thrown> {
    let Some(class_name) = object_element.attr("class") else {
        return Err(mapping_exception::missing_required_option(
            "object", "class", "",
        ));
    };
    // DBAL's default expressions are Rust objects, not registered classes.
    {
        use php2rust_doctrine_dbal::schema::{
            CurrentDate, CurrentTime, CurrentTimestamp, DefaultExpressionObject,
        };
        match class_name.trim_start_matches('\\') {
            "Doctrine\\DBAL\\Schema\\DefaultExpression\\CurrentTimestamp" => {
                return Ok(DefaultExpressionObject::value(CurrentTimestamp));
            }
            "Doctrine\\DBAL\\Schema\\DefaultExpression\\CurrentTime" => {
                return Ok(DefaultExpressionObject::value(CurrentTime));
            }
            "Doctrine\\DBAL\\Schema\\DefaultExpression\\CurrentDate" => {
                return Ok(DefaultExpressionObject::value(CurrentDate));
            }
            _ => {}
        }
    }
    if !php_reflection::class_exists(class_name) {
        return Err(Thrown::new(
            "InvalidArgumentException",
            format!("Class \"{class_name}\" does not exist"),
            0,
        ));
    }
    Ok(Value::Object(
        ReflectionClass::new(class_name)?.new_instance_without_constructor()?,
    ))
}

/// `joinColumnToArray()`.
fn join_column_to_array(join_column_element: &XmlElement) -> PhpArray {
    let mut join_column = PhpArray::new();
    join_column.insert("name", Value::from(join_column_element.string("name")));
    join_column.insert(
        "referencedColumnName",
        Value::from(join_column_element.string("referenced-column-name")),
    );
    if let Some(unique) = join_column_element.attr("unique") {
        join_column.insert("unique", Value::Bool(evaluate_boolean(unique)));
    }
    if let Some(nullable) = join_column_element.attr("nullable") {
        join_column.insert("nullable", Value::Bool(evaluate_boolean(nullable)));
    }
    if let Some(on_delete) = join_column_element.attr("on-delete") {
        join_column.insert("onDelete", Value::from(on_delete));
    }
    if let Some(definition) = join_column_element.attr("column-definition") {
        join_column.insert("columnDefinition", Value::from(definition));
    }
    if let Some(name) = join_column_element.attr("foreign-key-name") {
        join_column.insert("foreignKeyName", Value::from(name));
    }
    if join_column_element.has("options") {
        // `$joinColumnElement['options']->children()`: an attribute has
        // no child elements.
        join_column.insert("options", Value::Array(PhpArray::new()));
    }
    join_column
}

/// `columnToArray()`: the given field as an array.
fn column_to_array(field_mapping: &XmlElement) -> Result<PhpArray, Thrown> {
    let mut mapping = PhpArray::new();
    mapping.insert("fieldName", Value::from(field_mapping.string("name")));

    if let Some(ty) = field_mapping.attr("type") {
        mapping.insert("type", Value::from(ty));
    }
    if let Some(column) = field_mapping.attr("column") {
        mapping.insert("columnName", Value::from(column));
    }
    if let Some(length) = field_mapping.attr("length") {
        mapping.insert("length", int(length));
    }
    if let Some(precision) = field_mapping.attr("precision") {
        mapping.insert("precision", int(precision));
    }
    if let Some(scale) = field_mapping.attr("scale") {
        mapping.insert("scale", int(scale));
    }
    if let Some(unique) = field_mapping.attr("unique") {
        mapping.insert("unique", Value::Bool(evaluate_boolean(unique)));
    }
    if let Some(index) = field_mapping.attr("index") {
        mapping.insert("index", Value::Bool(evaluate_boolean(index)));
    }
    if let Some(nullable) = field_mapping.attr("nullable") {
        mapping.insert("nullable", Value::Bool(evaluate_boolean(nullable)));
    }
    if field_mapping
        .attr("insertable")
        .is_some_and(|v| !evaluate_boolean(v))
    {
        mapping.insert("notInsertable", Value::Bool(true));
    }
    if field_mapping
        .attr("updatable")
        .is_some_and(|v| !evaluate_boolean(v))
    {
        mapping.insert("notUpdatable", Value::Bool(true));
    }
    if let Some(generated) = field_mapping.attr("generated") {
        mapping.insert(
            "generated",
            Value::Int(class_metadata_constant(&format!("GENERATED_{generated}"))?),
        );
    }
    // `isset($fieldMapping['version']) && $fieldMapping['version']`: an
    // attribute is truthy.
    if let Some(version) = field_mapping.attr("version") {
        mapping.insert("version", Value::Bool(evaluate_boolean(version)));
    }
    if let Some(definition) = field_mapping.attr("column-definition") {
        mapping.insert("columnDefinition", Value::from(definition));
    }
    if let Some(enum_type) = field_mapping.attr("enum-type") {
        mapping.insert("enumType", Value::from(enum_type));
    }
    if let Some(options) = field_mapping.child("options") {
        mapping.insert("options", Value::Array(parse_options(&options.children())?));
    }

    Ok(mapping)
}

/// `cacheToArray()`: the normalized cache configuration.
fn cache_to_array(cache_mapping: &XmlElement) -> Result<PhpArray, Thrown> {
    let region = cache_mapping.opt_string("region");
    let usage = cache_mapping.attr("usage").map(str::to_ascii_uppercase);

    let usage = match usage.filter(|u| !is_empty(u)) {
        Some(usage) => Value::Int(cm::constant(&format!("CACHE_USAGE_{usage}")).ok_or_else(
            || {
                Thrown::new(
                    "InvalidArgumentException",
                    format!("Invalid cache usage \"{usage}\""),
                    0,
                )
            },
        )?),
        None => Value::Null,
    };

    let mut cache = PhpArray::new();
    cache.insert("usage", usage);
    cache.insert("region", region);
    Ok(cache)
}

/// `getCascadeMappings()`: the cascade options of a cascade element.
fn get_cascade_mappings(cascade_element: &XmlElement) -> Value {
    // According to the JPA specifications, XML uses "cascade-persist"
    // instead of "persist". Here, both variations are supported because
    // Attribute uses "persist" and we want to make sure that this driver
    // doesn't need to know anything about the supported cascading actions
    Value::Array(PhpArray::list(cascade_element.children().iter().map(
        |action| Value::from(action.get_name().replace("cascade-", "")),
    )))
}

/// `getDirection()`.
fn get_direction(order_by_field: &XmlElement) -> Result<Value, Thrown> {
    let direction = order_by_field.attr("direction").unwrap_or("ASC");
    match direction {
        "ASC" => Ok(cm::sort_direction(SortDirection::Ascending)),
        "DESC" => Ok(cm::sort_direction(SortDirection::Descending)),
        _ => Err(Thrown::new(
            "AssertionError",
            "assert(in_array($direction, ['ASC', 'DESC'], true))",
            0,
        )),
    }
}

impl FileDriverImpl for XmlDriver {
    type Element = XmlElement;

    /// `loadMappingFile()` (`LoadMappingFileImplementation`).
    fn load_mapping_file(&self, file: &str) -> Result<Vec<(String, XmlElement)>, Thrown> {
        self.do_load_mapping_file(file)
    }

    fn load_metadata_for_class(
        &self,
        driver: &FileDriver<Self>,
        class_name: &str,
        metadata: &dyn PersistenceClassMetadata,
    ) -> Result<(), Thrown> {
        let Some(metadata) = metadata.downcast_ref::<ClassMetadata>() else {
            return Err(Thrown::new(
                "TypeError",
                format!(
                    "{CLASS}::loadMetadataForClass(): Argument #2 ($metadata) must be of type {}",
                    cm::CLASS
                ),
                0,
            ));
        };
        let mut info = metadata.write();
        Self::load(driver, class_name, &mut info)
    }
}
