//! The doctrine/orm test models (`Doctrine\Tests\Models\*` and the classes
//! the mapping tests declare next to them), declared from
//! `tests/data/models.json`: `oracle/recorders/orm_models.php` describes
//! them through PHP's reflection. Each class goes to persistence's class
//! registry and its mapping attributes to the ORM's attribute registry,
//! the registrations a derive macro would emit for them.
//!
//! Upstream: doctrine/orm tests/Tests/Models/CMS/CmsUser.php tests/Tests/Models/CMS/CmsAddress.php tests/Tests/Models/Global/GlobalNamespaceModel.php

#![allow(dead_code, reason = "each test binary uses a part of the models")]

use std::collections::HashMap;
use std::sync::{Arc, Once};

use php_value::{Object, PhpArray, Value, vectors};
use php2rust_doctrine_orm::mapping::attributes::{
    self as attr, ClassAttributes, JoinColumnProperties, MappingAttribute,
};
use php2rust_doctrine_orm::object::EntityObject;
use php2rust_doctrine_persistence::php_reflection::{
    ClassInfo, LoadMetadataFn, MethodInfo, PropertyInfo, Visibility, class_info, register_class,
};
use serde_json::Value as Json;

const MODELS: &str = include_str!("../data/models.json");
const QUERY_MODELS: &str = include_str!("../data/query_models.json");

/// Declares the models of the DQL tests (`tests/data/query_models.json`,
/// recorded by `oracle/recorders/orm_query_models.php`), once per test
/// binary.
pub fn register_query_models() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        php2rust_doctrine_orm::mapping::register_orm_classes();
        let json: Json = serde_json::from_str(QUERY_MODELS).unwrap();
        for class in json["classes"].as_array().unwrap() {
            declare(class);
        }
    });
}

/// Declares the models, once per test binary.
pub fn register() {
    register_with_load_metadata(Vec::new);
}

/// Declares the models, once per test binary, with the static
/// `loadMetadata()` of the classes `loaders` names: PHP code, written as
/// Rust fns in the test that calls the `StaticPHPDriver`.
pub fn register_with_load_metadata(loaders: impl FnOnce() -> Vec<(String, LoadMetadataFn)>) {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        php2rust_doctrine_orm::mapping::register_orm_classes();
        let loaders: HashMap<String, LoadMetadataFn> = loaders()
            .into_iter()
            .map(|(class, load)| (class.to_ascii_lowercase(), load))
            .collect();
        let json: Json = serde_json::from_str(MODELS).unwrap();
        for class in json["classes"].as_array().unwrap() {
            declare_with(class, &loaders);
        }
    });
}

fn text(json: &Json) -> String {
    json.as_str().unwrap().to_owned()
}

fn visibility(json: &Json) -> Visibility {
    match json.as_str().unwrap() {
        "private" => Visibility::Private,
        "protected" => Visibility::Protected,
        _ => Visibility::Public,
    }
}

fn declare(class: &Json) {
    declare_with(class, &HashMap::new());
}

fn declare_with(class: &Json, loaders: &HashMap<String, LoadMetadataFn>) {
    let name = text(&class["name"]);
    if class_info(&name).is_some() {
        return;
    }
    let mut info = match class["kind"].as_str().unwrap() {
        "interface" => ClassInfo::interface(name.as_str()),
        "trait" => ClassInfo::trait_(name.as_str()),
        "enum" => {
            let cases: Vec<Object> = class["cases"]
                .as_array()
                .unwrap()
                .iter()
                .map(|case| match vectors::decode(case).unwrap() {
                    Value::Object(object) => object,
                    other => panic!("an enum case: {other:?}"),
                })
                .collect();
            ClassInfo::enumeration(name.as_str(), cases)
        }
        _ => ClassInfo::class(name.as_str()),
    };
    if class["abstract"].as_bool().unwrap() {
        info = info.abstract_class();
    }
    if class["final"].as_bool().unwrap() {
        info = info.final_class();
    }
    if let Some(parent) = class["parent"].as_str() {
        info = info.extends(parent);
    }
    for interface in class["interfaces"].as_array().unwrap() {
        info = info.implements(text(interface));
    }
    let mut attributes = ClassAttributes::new(name.as_str());
    for attribute in class["attributes"].as_array().unwrap() {
        attributes = attributes.class(mapping_attribute(attribute));
    }
    for property in class["properties"].as_array().unwrap() {
        let property_name = text(&property["name"]);
        let mut p = match property["type"].as_array() {
            Some(ty) => PropertyInfo::typed(
                property_name.as_str(),
                ty[0].as_str().unwrap(),
                ty[1].as_bool().unwrap(),
            ),
            None => PropertyInfo::new(property_name.as_str()),
        };
        p = p.visibility(visibility(&property["visibility"]));
        if property["static"].as_bool().unwrap() {
            p = p.static_property();
        }
        if property["readonly"].as_bool().unwrap() {
            p = p.readonly();
        }
        p = match &property["default"] {
            Json::Null => p.no_default(),
            default => p.default_value(vectors::decode(default).unwrap()),
        };
        info = info.property(p);
        let property_attributes: Vec<MappingAttribute> = property["attributes"]
            .as_array()
            .unwrap()
            .iter()
            .map(mapping_attribute)
            .collect();
        if !property_attributes.is_empty() {
            attributes = attributes.property(&property_name, property_attributes);
        }
    }
    let load_metadata = loaders.get(&name.to_ascii_lowercase());
    for method in class["methods"].as_array().unwrap() {
        let method_name = text(&method["name"]);
        if load_metadata.is_some() && method_name == "loadMetadata" {
            continue;
        }
        info = info.method(method_info(method, &method_name));
        let method_attributes: Vec<MappingAttribute> = method["attributes"]
            .as_array()
            .unwrap()
            .iter()
            .map(mapping_attribute)
            .collect();
        if !method_attributes.is_empty() {
            attributes = attributes.method(&method_name, method_attributes);
        }
    }
    if let Some(load) = load_metadata {
        info = info.load_metadata(Arc::clone(load));
    }
    if class["kind"].as_str() == Some("class") {
        let instance_class = name.clone();
        info = info.new_instance(Arc::new(move || {
            EntityObject::object(&instance_class).ok_or_else(|| {
                php_value::Thrown::new("Error", format!("Class \"{instance_class}\" not found"), 0)
            })
        }));
    }
    register_class(info);
    attr::register_attributes(attributes);
}

/// A recorded method.
fn method_info(method: &Json, method_name: &str) -> MethodInfo {
    let mut m = MethodInfo::new(method_name).visibility(visibility(&method["visibility"]));
    if method["static"].as_bool().unwrap() {
        m = m.static_method();
    }
    if let Some(required) = method["required"].as_u64() {
        m = m.required_parameters(usize::try_from(required).unwrap());
    }
    m
}

/// A recorded attribute (`["object", class, properties]`).
fn mapping_attribute(json: &Json) -> MappingAttribute {
    if json[0] == "custom" {
        let properties = match vectors::decode(&json[3]).unwrap() {
            Value::Object(object) => object.properties(),
            other => panic!("an attribute: {other:?}"),
        };
        return attr::CustomAttribute {
            class: text(&json[1]),
            repeatable: json[2].as_bool().unwrap(),
            properties,
        }
        .into();
    }
    match vectors::decode(json).unwrap() {
        Value::Object(object) => from_object(object.class(), &object.properties()),
        other => panic!("an attribute: {other:?}"),
    }
}

fn get<'a>(props: &'a PhpArray, key: &str) -> &'a Value {
    props
        .get(&php_value::Key::from(key))
        .unwrap_or(&Value::Null)
}

fn opt_string(props: &PhpArray, key: &str) -> Option<String> {
    match get(props, key) {
        Value::Null => None,
        value => {
            Some(String::from_utf8(value.to_php_string().unwrap().as_bytes().to_vec()).unwrap())
        }
    }
}

fn string(props: &PhpArray, key: &str) -> String {
    opt_string(props, key).unwrap()
}

fn boolean(props: &PhpArray, key: &str) -> bool {
    matches!(get(props, key), Value::Bool(true))
}

fn opt_int(props: &PhpArray, key: &str) -> Option<i64> {
    match get(props, key) {
        Value::Int(i) => Some(*i),
        _ => None,
    }
}

fn opt_array(props: &PhpArray, key: &str) -> Option<PhpArray> {
    match get(props, key) {
        Value::Array(array) => Some(array.clone()),
        _ => None,
    }
}

fn array(props: &PhpArray, key: &str) -> PhpArray {
    opt_array(props, key).unwrap_or_default()
}

fn opt_strings(props: &PhpArray, key: &str) -> Option<Vec<String>> {
    opt_array(props, key).map(|array| {
        array
            .values()
            .map(|v| String::from_utf8(v.to_php_string().unwrap().as_bytes().to_vec()).unwrap())
            .collect()
    })
}

fn objects(props: &PhpArray, key: &str) -> Option<Vec<PhpArray>> {
    opt_array(props, key).map(|array| {
        array
            .values()
            .map(|v| match v {
                Value::Object(object) => object.properties(),
                other => panic!("an object: {other:?}"),
            })
            .collect()
    })
}

fn join_column(props: &PhpArray) -> JoinColumnProperties {
    JoinColumnProperties {
        name: opt_string(props, "name"),
        referenced_column_name: opt_string(props, "referencedColumnName"),
        deferrable: boolean(props, "deferrable"),
        unique: boolean(props, "unique"),
        nullable: match get(props, "nullable") {
            Value::Bool(b) => Some(*b),
            _ => None,
        },
        on_delete: get(props, "onDelete").clone(),
        column_definition: opt_string(props, "columnDefinition"),
        field_name: opt_string(props, "fieldName"),
        foreign_key_name: opt_string(props, "foreignKeyName"),
        options: array(props, "options"),
    }
}

fn join_columns(props: &PhpArray, key: &str) -> Option<Vec<attr::JoinColumn>> {
    objects(props, key).map(|list| {
        list.iter()
            .map(|p| attr::JoinColumn(join_column(p)))
            .collect()
    })
}

fn column(props: &PhpArray) -> attr::Column {
    attr::Column {
        name: opt_string(props, "name"),
        r#type: opt_string(props, "type"),
        length: opt_int(props, "length"),
        precision: opt_int(props, "precision"),
        scale: opt_int(props, "scale"),
        unique: boolean(props, "unique"),
        nullable: boolean(props, "nullable"),
        insertable: boolean(props, "insertable"),
        updatable: boolean(props, "updatable"),
        enum_type: opt_string(props, "enumType"),
        options: array(props, "options"),
        column_definition: opt_string(props, "columnDefinition"),
        generated: opt_string(props, "generated"),
        index: boolean(props, "index"),
    }
}

fn join_table(props: &PhpArray) -> attr::JoinTable {
    attr::JoinTable {
        name: opt_string(props, "name"),
        schema: opt_string(props, "schema"),
        join_columns: join_columns(props, "joinColumns").unwrap_or_default(),
        inverse_join_columns: join_columns(props, "inverseJoinColumns").unwrap_or_default(),
        foreign_key_name: opt_string(props, "foreignKeyName"),
        inverse_foreign_key_name: opt_string(props, "inverseForeignKeyName"),
        options: array(props, "options"),
    }
}

/// The attribute of class `class` with the public properties `props`.
#[allow(clippy::too_many_lines, reason = "one arm per attribute class")]
pub fn from_object(class: &str, props: &PhpArray) -> MappingAttribute {
    let short = class.rsplit('\\').next().unwrap();
    match short {
        "Entity" => attr::Entity {
            repository_class: opt_string(props, "repositoryClass"),
            read_only: boolean(props, "readOnly"),
        }
        .into(),
        "MappedSuperclass" => attr::MappedSuperclass {
            repository_class: opt_string(props, "repositoryClass"),
        }
        .into(),
        "Embeddable" => attr::Embeddable.into(),
        "Table" => attr::Table {
            name: opt_string(props, "name"),
            schema: opt_string(props, "schema"),
            indexes: opt_array(props, "indexes"),
            unique_constraints: opt_array(props, "uniqueConstraints"),
            options: array(props, "options"),
        }
        .into(),
        "Index" => attr::Index {
            name: opt_string(props, "name"),
            columns: opt_strings(props, "columns"),
            fields: opt_strings(props, "fields"),
            flags: opt_strings(props, "flags"),
            options: opt_array(props, "options"),
        }
        .into(),
        "UniqueConstraint" => attr::UniqueConstraint {
            name: opt_string(props, "name"),
            columns: opt_strings(props, "columns"),
            fields: opt_strings(props, "fields"),
            options: opt_array(props, "options"),
        }
        .into(),
        "Column" => column(props).into(),
        "Id" => attr::Id {
            position: opt_int(props, "position").unwrap_or(0),
        }
        .into(),
        "GeneratedValue" => attr::GeneratedValue {
            strategy: string(props, "strategy"),
        }
        .into(),
        "SequenceGenerator" => attr::SequenceGenerator {
            sequence_name: opt_string(props, "sequenceName"),
            allocation_size: opt_int(props, "allocationSize").unwrap(),
            initial_value: opt_int(props, "initialValue").unwrap(),
        }
        .into(),
        "CustomIdGenerator" => attr::CustomIdGenerator {
            class: opt_string(props, "class"),
        }
        .into(),
        "Version" => attr::Version.into(),
        "OneToOne" => attr::OneToOne {
            target_entity: opt_string(props, "targetEntity"),
            mapped_by: opt_string(props, "mappedBy"),
            inversed_by: opt_string(props, "inversedBy"),
            cascade: opt_strings(props, "cascade"),
            fetch: string(props, "fetch"),
            orphan_removal: boolean(props, "orphanRemoval"),
        }
        .into(),
        "ManyToOne" => attr::ManyToOne {
            target_entity: opt_string(props, "targetEntity"),
            cascade: opt_strings(props, "cascade"),
            fetch: string(props, "fetch"),
            inversed_by: opt_string(props, "inversedBy"),
        }
        .into(),
        "OneToMany" => attr::OneToMany {
            target_entity: opt_string(props, "targetEntity"),
            mapped_by: opt_string(props, "mappedBy"),
            cascade: opt_strings(props, "cascade"),
            fetch: string(props, "fetch"),
            orphan_removal: boolean(props, "orphanRemoval"),
            index_by: opt_string(props, "indexBy"),
        }
        .into(),
        "ManyToMany" => attr::ManyToMany {
            target_entity: string(props, "targetEntity"),
            mapped_by: opt_string(props, "mappedBy"),
            inversed_by: opt_string(props, "inversedBy"),
            cascade: opt_strings(props, "cascade"),
            fetch: string(props, "fetch"),
            orphan_removal: boolean(props, "orphanRemoval"),
            index_by: opt_string(props, "indexBy"),
        }
        .into(),
        "JoinColumn" => attr::JoinColumn(join_column(props)).into(),
        "InverseJoinColumn" => attr::InverseJoinColumn(join_column(props)).into(),
        "JoinTable" => join_table(props).into(),
        "OrderBy" => attr::OrderBy {
            value: array(props, "value"),
        }
        .into(),
        "Embedded" => attr::Embedded {
            class: opt_string(props, "class"),
            column_prefix: get(props, "columnPrefix").clone(),
        }
        .into(),
        "InheritanceType" => attr::InheritanceType {
            value: string(props, "value"),
        }
        .into(),
        "DiscriminatorColumn" => attr::DiscriminatorColumn {
            name: opt_string(props, "name"),
            r#type: opt_string(props, "type"),
            length: opt_int(props, "length"),
            column_definition: opt_string(props, "columnDefinition"),
            enum_type: opt_string(props, "enumType"),
            options: array(props, "options"),
        }
        .into(),
        "DiscriminatorMap" => attr::DiscriminatorMap {
            value: array(props, "value"),
        }
        .into(),
        "ChangeTrackingPolicy" => attr::ChangeTrackingPolicy {
            value: string(props, "value"),
        }
        .into(),
        "Cache" => attr::Cache {
            usage: string(props, "usage"),
            region: opt_string(props, "region"),
        }
        .into(),
        "EntityListeners" => attr::EntityListeners {
            value: opt_strings(props, "value").unwrap_or_default(),
        }
        .into(),
        "HasLifecycleCallbacks" => attr::HasLifecycleCallbacks.into(),
        "AssociationOverrides" => attr::AssociationOverrides {
            overrides: objects(props, "overrides")
                .unwrap()
                .iter()
                .map(|o| attr::AssociationOverride {
                    name: string(o, "name"),
                    join_columns: join_columns(o, "joinColumns"),
                    inverse_join_columns: join_columns(o, "inverseJoinColumns"),
                    join_table: match get(o, "joinTable") {
                        Value::Object(object) => Some(join_table(&object.properties())),
                        _ => None,
                    },
                    inversed_by: opt_string(o, "inversedBy"),
                    fetch: opt_string(o, "fetch"),
                })
                .collect(),
        }
        .into(),
        "AttributeOverrides" => attr::AttributeOverrides {
            overrides: objects(props, "overrides")
                .unwrap()
                .iter()
                .map(|o| attr::AttributeOverride {
                    name: string(o, "name"),
                    column: match get(o, "column") {
                        Value::Object(object) => column(&object.properties()),
                        other => panic!("a Column: {other:?}"),
                    },
                })
                .collect(),
        }
        .into(),
        "PrePersist" => attr::PrePersist.into(),
        "PostPersist" => attr::PostPersist.into(),
        "PreUpdate" => attr::PreUpdate.into(),
        "PostUpdate" => attr::PostUpdate.into(),
        "PreRemove" => attr::PreRemove.into(),
        "PostRemove" => attr::PostRemove.into(),
        "PostLoad" => attr::PostLoad.into(),
        "PreFlush" => attr::PreFlush.into(),
        other => panic!("not a mapping attribute: {other}"),
    }
}
