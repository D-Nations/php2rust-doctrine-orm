//! The mapping attributes (`Doctrine\ORM\Mapping\*` classes that
//! implement `MappingAttribute`) and the registry that stands in for PHP's
//! attribute syntax.
//!
//! PHP reads `#[ORM\Column(type: 'integer')]` from the source; here a
//! class's attributes are declared once with [`register_attributes`],
//! next to its `php_reflection::register_class` declaration:
//!
//! ```
//! use php2rust_doctrine_orm::mapping::attributes::{
//!     self, ClassAttributes, Column, Entity, Id, MappingAttribute,
//! };
//!
//! attributes::register_attributes(
//!     ClassAttributes::new("App\\Entity\\User")
//!         .class(Entity::default())
//!         .property("id", [Id::default().into(), Column {
//!             r#type: Some("integer".into()),
//!             ..Column::default()
//!         }.into()]),
//! );
//! ```
//!
//! Each attribute is a struct with the public properties of its PHP class
//! and that class's constructor defaults as `Default`, so that a derive
//! macro reading `#[orm(...)]` on a Rust struct can emit the same
//! registration as struct literals. Constructor side effects (the `Table`
//! deprecations) run when the reader instantiates the attribute, as PHP's
//! `ReflectionAttribute::newInstance()` does.
//!
//! Upstream: doctrine/orm src/Mapping/MappingAttribute.php src/Mapping/Entity.php src/Mapping/MappedSuperclass.php src/Mapping/Embeddable.php src/Mapping/Table.php src/Mapping/Index.php src/Mapping/UniqueConstraint.php src/Mapping/Column.php src/Mapping/Id.php src/Mapping/GeneratedValue.php src/Mapping/SequenceGenerator.php src/Mapping/CustomIdGenerator.php src/Mapping/Version.php src/Mapping/OneToOne.php src/Mapping/ManyToOne.php src/Mapping/OneToMany.php src/Mapping/ManyToMany.php src/Mapping/JoinColumnProperties.php src/Mapping/JoinColumn.php src/Mapping/InverseJoinColumn.php src/Mapping/JoinTable.php src/Mapping/OrderBy.php src/Mapping/Embedded.php src/Mapping/InheritanceType.php src/Mapping/DiscriminatorColumn.php src/Mapping/DiscriminatorMap.php src/Mapping/ChangeTrackingPolicy.php src/Mapping/Cache.php src/Mapping/EntityListeners.php src/Mapping/HasLifecycleCallbacks.php src/Mapping/AssociationOverride.php src/Mapping/AssociationOverrides.php src/Mapping/AttributeOverride.php src/Mapping/AttributeOverrides.php src/Mapping/PrePersist.php src/Mapping/PostPersist.php src/Mapping/PreUpdate.php src/Mapping/PostUpdate.php src/Mapping/PreRemove.php src/Mapping/PostRemove.php src/Mapping/PostLoad.php src/Mapping/PreFlush.php

use std::collections::HashMap;
use std::sync::{Arc, PoisonError, RwLock};

use php_value::{PhpArray, Value};

use crate::deprecations;

/// `Doctrine\ORM\Mapping\MappingAttribute`.
pub const MAPPING_ATTRIBUTE: &str = "Doctrine\\ORM\\Mapping\\MappingAttribute";

/// The deprecation of `Table`'s `$indexes` and `$uniqueConstraints`.
pub const TABLE_INDEXES_DEPRECATION: &str = "https://github.com/doctrine/orm/pull/11357";

/// An attribute class: its name, whether it is repeatable
/// (`Attribute::IS_REPEATABLE`) and how to find it in a
/// [`MappingAttribute`].
pub trait AttributeClass: Sized {
    const CLASS: &'static str;
    const REPEATABLE: bool = false;

    fn from_attribute(attribute: &MappingAttribute) -> Option<&Self>;
}

macro_rules! attributes {
    ($( $(#[$doc:meta])* $variant:ident => $class:literal $(, repeatable $rep:literal)?; )*) => {
        /// An instance of a mapping attribute.
        #[derive(Debug, Clone, PartialEq)]
        pub enum MappingAttribute {
            $( $(#[$doc])* $variant($variant), )*
            /// An attribute class of the application that implements
            /// `MappingAttribute`.
            Custom(CustomAttribute),
        }

        impl MappingAttribute {
            /// The attribute's class name.
            pub fn class_name(&self) -> &str {
                match self {
                    $( Self::$variant(_) => $class, )*
                    Self::Custom(custom) => &custom.class,
                }
            }

            /// Whether the attribute class is repeatable.
            pub fn is_repeatable(&self) -> bool {
                match self {
                    $( Self::$variant(_) => <$variant as AttributeClass>::REPEATABLE, )*
                    Self::Custom(custom) => custom.repeatable,
                }
            }
        }

        /// Whether `class` is a repeatable mapping attribute class; `None`
        /// when it is not a mapping attribute.
        pub fn is_repeatable_class(class: &str) -> Option<bool> {
            $(
                if class.eq_ignore_ascii_case($class) {
                    return Some(<$variant as AttributeClass>::REPEATABLE);
                }
            )*
            None
        }

        $(
            impl AttributeClass for $variant {
                const CLASS: &'static str = $class;
                $( const REPEATABLE: bool = $rep; )?

                fn from_attribute(attribute: &MappingAttribute) -> Option<&Self> {
                    match attribute {
                        MappingAttribute::$variant(value) => Some(value),
                        _ => None,
                    }
                }
            }

            impl From<$variant> for MappingAttribute {
                fn from(value: $variant) -> Self {
                    Self::$variant(value)
                }
            }
        )*
    };
}

attributes! {
    Entity => "Doctrine\\ORM\\Mapping\\Entity";
    MappedSuperclass => "Doctrine\\ORM\\Mapping\\MappedSuperclass";
    Embeddable => "Doctrine\\ORM\\Mapping\\Embeddable";
    Table => "Doctrine\\ORM\\Mapping\\Table";
    Index => "Doctrine\\ORM\\Mapping\\Index", repeatable true;
    UniqueConstraint => "Doctrine\\ORM\\Mapping\\UniqueConstraint", repeatable true;
    Column => "Doctrine\\ORM\\Mapping\\Column";
    Id => "Doctrine\\ORM\\Mapping\\Id";
    GeneratedValue => "Doctrine\\ORM\\Mapping\\GeneratedValue";
    SequenceGenerator => "Doctrine\\ORM\\Mapping\\SequenceGenerator";
    CustomIdGenerator => "Doctrine\\ORM\\Mapping\\CustomIdGenerator";
    Version => "Doctrine\\ORM\\Mapping\\Version";
    OneToOne => "Doctrine\\ORM\\Mapping\\OneToOne";
    ManyToOne => "Doctrine\\ORM\\Mapping\\ManyToOne";
    OneToMany => "Doctrine\\ORM\\Mapping\\OneToMany";
    ManyToMany => "Doctrine\\ORM\\Mapping\\ManyToMany";
    JoinColumn => "Doctrine\\ORM\\Mapping\\JoinColumn", repeatable true;
    InverseJoinColumn => "Doctrine\\ORM\\Mapping\\InverseJoinColumn", repeatable true;
    JoinTable => "Doctrine\\ORM\\Mapping\\JoinTable";
    OrderBy => "Doctrine\\ORM\\Mapping\\OrderBy";
    Embedded => "Doctrine\\ORM\\Mapping\\Embedded";
    InheritanceType => "Doctrine\\ORM\\Mapping\\InheritanceType";
    DiscriminatorColumn => "Doctrine\\ORM\\Mapping\\DiscriminatorColumn";
    DiscriminatorMap => "Doctrine\\ORM\\Mapping\\DiscriminatorMap";
    ChangeTrackingPolicy => "Doctrine\\ORM\\Mapping\\ChangeTrackingPolicy";
    Cache => "Doctrine\\ORM\\Mapping\\Cache";
    EntityListeners => "Doctrine\\ORM\\Mapping\\EntityListeners";
    HasLifecycleCallbacks => "Doctrine\\ORM\\Mapping\\HasLifecycleCallbacks";
    AssociationOverrides => "Doctrine\\ORM\\Mapping\\AssociationOverrides";
    AttributeOverrides => "Doctrine\\ORM\\Mapping\\AttributeOverrides";
    PrePersist => "Doctrine\\ORM\\Mapping\\PrePersist";
    PostPersist => "Doctrine\\ORM\\Mapping\\PostPersist";
    PreUpdate => "Doctrine\\ORM\\Mapping\\PreUpdate";
    PostUpdate => "Doctrine\\ORM\\Mapping\\PostUpdate";
    PreRemove => "Doctrine\\ORM\\Mapping\\PreRemove";
    PostRemove => "Doctrine\\ORM\\Mapping\\PostRemove";
    PostLoad => "Doctrine\\ORM\\Mapping\\PostLoad";
    PreFlush => "Doctrine\\ORM\\Mapping\\PreFlush";
}

/// An application's attribute class that implements `MappingAttribute`:
/// the driver reads none of it, but it is in the reader's result.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CustomAttribute {
    pub class: String,
    /// `Attribute::IS_REPEATABLE`.
    pub repeatable: bool,
    /// The instance's public properties.
    pub properties: PhpArray,
}

impl From<CustomAttribute> for MappingAttribute {
    fn from(value: CustomAttribute) -> Self {
        Self::Custom(value)
    }
}

/// `#[Entity]`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Entity {
    pub repository_class: Option<String>,
    pub read_only: bool,
}

/// `#[MappedSuperclass]`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MappedSuperclass {
    pub repository_class: Option<String>,
}

/// `#[Embeddable]`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Embeddable;

/// `#[Table]`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Table {
    pub name: Option<String>,
    pub schema: Option<String>,
    /// Deprecated, without effect: the `Index` attribute replaces it.
    pub indexes: Option<PhpArray>,
    /// Deprecated, without effect: the `UniqueConstraint` attribute
    /// replaces it.
    pub unique_constraints: Option<PhpArray>,
    pub options: PhpArray,
}

/// `#[Index]`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Index {
    pub name: Option<String>,
    pub columns: Option<Vec<String>>,
    pub fields: Option<Vec<String>>,
    pub flags: Option<Vec<String>>,
    pub options: Option<PhpArray>,
}

/// `#[UniqueConstraint]`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UniqueConstraint {
    pub name: Option<String>,
    pub columns: Option<Vec<String>>,
    pub fields: Option<Vec<String>>,
    pub options: Option<PhpArray>,
}

/// `#[Column]`.
#[derive(Debug, Clone, PartialEq)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "the attribute's constructor parameters"
)]
pub struct Column {
    pub name: Option<String>,
    pub r#type: Option<String>,
    pub length: Option<i64>,
    pub precision: Option<i64>,
    pub scale: Option<i64>,
    pub unique: bool,
    pub nullable: bool,
    pub insertable: bool,
    pub updatable: bool,
    pub enum_type: Option<String>,
    pub options: PhpArray,
    pub column_definition: Option<String>,
    /// `'NEVER'`, `'INSERT'` or `'ALWAYS'`.
    pub generated: Option<String>,
    pub index: bool,
}

impl Default for Column {
    fn default() -> Self {
        Self {
            name: None,
            r#type: None,
            length: None,
            precision: None,
            scale: None,
            unique: false,
            nullable: false,
            insertable: true,
            updatable: true,
            enum_type: None,
            options: PhpArray::new(),
            column_definition: None,
            generated: None,
            index: false,
        }
    }
}

/// `#[Id]`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Id {
    pub position: i64,
}

/// `#[GeneratedValue]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedValue {
    /// `'AUTO'`, `'SEQUENCE'`, `'IDENTITY'`, `'NONE'` or `'CUSTOM'`.
    pub strategy: String,
}

impl Default for GeneratedValue {
    fn default() -> Self {
        Self {
            strategy: "AUTO".to_owned(),
        }
    }
}

/// `#[SequenceGenerator]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceGenerator {
    pub sequence_name: Option<String>,
    pub allocation_size: i64,
    pub initial_value: i64,
}

impl Default for SequenceGenerator {
    fn default() -> Self {
        Self {
            sequence_name: None,
            allocation_size: 1,
            initial_value: 1,
        }
    }
}

/// `#[CustomIdGenerator]`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CustomIdGenerator {
    pub class: Option<String>,
}

/// `#[Version]`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Version;

fn lazy() -> String {
    "LAZY".to_owned()
}

/// `#[OneToOne]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OneToOne {
    pub target_entity: Option<String>,
    pub mapped_by: Option<String>,
    pub inversed_by: Option<String>,
    pub cascade: Option<Vec<String>>,
    pub fetch: String,
    pub orphan_removal: bool,
}

impl Default for OneToOne {
    fn default() -> Self {
        Self {
            target_entity: None,
            mapped_by: None,
            inversed_by: None,
            cascade: None,
            fetch: lazy(),
            orphan_removal: false,
        }
    }
}

/// `#[ManyToOne]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManyToOne {
    pub target_entity: Option<String>,
    pub cascade: Option<Vec<String>>,
    pub fetch: String,
    pub inversed_by: Option<String>,
}

impl Default for ManyToOne {
    fn default() -> Self {
        Self {
            target_entity: None,
            cascade: None,
            fetch: lazy(),
            inversed_by: None,
        }
    }
}

/// `#[OneToMany]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OneToMany {
    pub target_entity: Option<String>,
    pub mapped_by: Option<String>,
    pub cascade: Option<Vec<String>>,
    pub fetch: String,
    pub orphan_removal: bool,
    pub index_by: Option<String>,
}

impl Default for OneToMany {
    fn default() -> Self {
        Self {
            target_entity: None,
            mapped_by: None,
            cascade: None,
            fetch: lazy(),
            orphan_removal: false,
            index_by: None,
        }
    }
}

/// `#[ManyToMany]` (`$targetEntity` is required).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManyToMany {
    pub target_entity: String,
    pub mapped_by: Option<String>,
    pub inversed_by: Option<String>,
    pub cascade: Option<Vec<String>>,
    pub fetch: String,
    pub orphan_removal: bool,
    pub index_by: Option<String>,
}

impl ManyToMany {
    pub fn new(target_entity: impl Into<String>) -> Self {
        Self {
            target_entity: target_entity.into(),
            mapped_by: None,
            inversed_by: None,
            cascade: None,
            fetch: lazy(),
            orphan_removal: false,
            index_by: None,
        }
    }
}

/// The properties of `JoinColumnProperties`, which `JoinColumn` and
/// `InverseJoinColumn` use.
#[derive(Debug, Clone, PartialEq)]
pub struct JoinColumnProperties {
    pub name: Option<String>,
    pub referenced_column_name: Option<String>,
    pub deferrable: bool,
    pub unique: bool,
    pub nullable: Option<bool>,
    /// `mixed`.
    pub on_delete: Value,
    pub column_definition: Option<String>,
    pub field_name: Option<String>,
    pub foreign_key_name: Option<String>,
    pub options: PhpArray,
}

impl Default for JoinColumnProperties {
    fn default() -> Self {
        Self {
            name: None,
            referenced_column_name: None,
            deferrable: false,
            unique: false,
            nullable: None,
            on_delete: Value::Null,
            column_definition: None,
            field_name: None,
            foreign_key_name: None,
            options: PhpArray::new(),
        }
    }
}

impl JoinColumnProperties {
    /// `name:` and `referencedColumnName:`, the common case.
    pub fn named(name: impl Into<String>, referenced_column_name: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
            referenced_column_name: Some(referenced_column_name.into()),
            ..Self::default()
        }
    }
}

/// `#[JoinColumn]`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct JoinColumn(pub JoinColumnProperties);

/// `#[InverseJoinColumn]`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InverseJoinColumn(pub JoinColumnProperties);

impl std::ops::Deref for JoinColumn {
    type Target = JoinColumnProperties;

    fn deref(&self) -> &JoinColumnProperties {
        &self.0
    }
}

impl std::ops::Deref for InverseJoinColumn {
    type Target = JoinColumnProperties;

    fn deref(&self) -> &JoinColumnProperties {
        &self.0
    }
}

/// `#[JoinTable]`; a single `JoinColumn` argument is the list of it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct JoinTable {
    pub name: Option<String>,
    pub schema: Option<String>,
    pub join_columns: Vec<JoinColumn>,
    pub inverse_join_columns: Vec<JoinColumn>,
    pub foreign_key_name: Option<String>,
    pub inverse_foreign_key_name: Option<String>,
    pub options: PhpArray,
}

/// `#[OrderBy]`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OrderBy {
    pub value: PhpArray,
}

/// `#[Embedded]`.
#[derive(Debug, Clone, PartialEq)]
pub struct Embedded {
    pub class: Option<String>,
    /// `string|bool|null`.
    pub column_prefix: Value,
}

impl Default for Embedded {
    fn default() -> Self {
        Self {
            class: None,
            column_prefix: Value::Null,
        }
    }
}

/// `#[InheritanceType]`: `'NONE'`, `'JOINED'` or `'SINGLE_TABLE'`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InheritanceType {
    pub value: String,
}

/// `#[DiscriminatorColumn]`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DiscriminatorColumn {
    pub name: Option<String>,
    pub r#type: Option<String>,
    pub length: Option<i64>,
    pub column_definition: Option<String>,
    pub enum_type: Option<String>,
    pub options: PhpArray,
}

/// `#[DiscriminatorMap]`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DiscriminatorMap {
    pub value: PhpArray,
}

/// `#[ChangeTrackingPolicy]`: `'DEFERRED_IMPLICIT'` or
/// `'DEFERRED_EXPLICIT'`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeTrackingPolicy {
    pub value: String,
}

/// `#[Cache]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cache {
    /// `'READ_ONLY'`, `'READ_WRITE'` or `'NONSTRICT_READ_WRITE'`.
    pub usage: String,
    pub region: Option<String>,
}

impl Default for Cache {
    fn default() -> Self {
        Self {
            usage: "READ_ONLY".to_owned(),
            region: None,
        }
    }
}

/// `#[EntityListeners]`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EntityListeners {
    pub value: Vec<String>,
}

/// `#[HasLifecycleCallbacks]`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HasLifecycleCallbacks;

/// `AssociationOverride` (an argument of `#[AssociationOverrides]`); a
/// single `JoinColumn` argument is the list of it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AssociationOverride {
    pub name: String,
    pub join_columns: Option<Vec<JoinColumn>>,
    pub inverse_join_columns: Option<Vec<JoinColumn>>,
    pub join_table: Option<JoinTable>,
    pub inversed_by: Option<String>,
    pub fetch: Option<String>,
}

/// `#[AssociationOverrides]`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AssociationOverrides {
    pub overrides: Vec<AssociationOverride>,
}

/// `AttributeOverride` (an argument of `#[AttributeOverrides]`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AttributeOverride {
    pub name: String,
    pub column: Column,
}

/// `#[AttributeOverrides]`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AttributeOverrides {
    pub overrides: Vec<AttributeOverride>,
}

macro_rules! callbacks {
    ($($name:ident),*) => {
        $(
            #[doc = concat!("`#[", stringify!($name), "]`.")]
            #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
            pub struct $name;
        )*
    };
}

callbacks!(
    PrePersist,
    PostPersist,
    PreUpdate,
    PostUpdate,
    PreRemove,
    PostRemove,
    PostLoad,
    PreFlush
);

/// The attributes of one class, its properties and its methods, in
/// declaration order.
#[derive(Debug, Clone, Default)]
pub struct ClassAttributes {
    name: String,
    class: Vec<MappingAttribute>,
    properties: Vec<(String, Vec<MappingAttribute>)>,
    methods: Vec<(String, Vec<MappingAttribute>)>,
}

impl ClassAttributes {
    pub fn new(class: impl Into<String>) -> Self {
        Self {
            name: class.into(),
            ..Self::default()
        }
    }

    /// Adds a class attribute.
    #[must_use]
    pub fn class(mut self, attribute: impl Into<MappingAttribute>) -> Self {
        self.class.push(attribute.into());
        self
    }

    /// Adds attributes to a property.
    #[must_use]
    pub fn property(
        mut self,
        name: &str,
        attributes: impl IntoIterator<Item = MappingAttribute>,
    ) -> Self {
        let attributes = attributes.into_iter();
        match self.properties.iter_mut().find(|(known, _)| known == name) {
            Some((_, known)) => known.extend(attributes),
            None => self
                .properties
                .push((name.to_owned(), attributes.collect())),
        }
        self
    }

    /// Adds attributes to a method.
    #[must_use]
    pub fn method(
        mut self,
        name: &str,
        attributes: impl IntoIterator<Item = MappingAttribute>,
    ) -> Self {
        let attributes = attributes.into_iter();
        match self
            .methods
            .iter_mut()
            .find(|(known, _)| known.eq_ignore_ascii_case(name))
        {
            Some((_, known)) => known.extend(attributes),
            None => self.methods.push((name.to_owned(), attributes.collect())),
        }
        self
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

type Registry = HashMap<String, Arc<ClassAttributes>>;

fn registry() -> &'static RwLock<Registry> {
    static REGISTRY: std::sync::OnceLock<RwLock<Registry>> = std::sync::OnceLock::new();
    REGISTRY.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Declares a class's attributes, replacing any earlier declaration.
pub fn register_attributes(attributes: ClassAttributes) {
    registry()
        .write()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(attributes.name.to_ascii_lowercase(), Arc::new(attributes));
}

fn attributes_of(class: &str) -> Option<Arc<ClassAttributes>> {
    registry()
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .get(&class.to_ascii_lowercase())
        .cloned()
}

/// `ReflectionClass::getAttributes()` of a mapping class.
pub fn class_attributes(class: &str) -> Vec<MappingAttribute> {
    attributes_of(class).map_or_else(Vec::new, |a| a.class.clone())
}

/// `ReflectionProperty::getAttributes()`, for the class that declares
/// the property.
pub fn property_attributes(class: &str, property: &str) -> Vec<MappingAttribute> {
    attributes_of(class)
        .and_then(|a| {
            a.properties
                .iter()
                .find(|(name, _)| name == property)
                .map(|(_, attributes)| attributes.clone())
        })
        .unwrap_or_default()
}

/// `ReflectionMethod::getAttributes()`, for the class that declares the
/// method.
pub fn method_attributes(class: &str, method: &str) -> Vec<MappingAttribute> {
    attributes_of(class)
        .and_then(|a| {
            a.methods
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case(method))
                .map(|(_, attributes)| attributes.clone())
        })
        .unwrap_or_default()
}

/// `ReflectionAttribute::newInstance()` (or `new` of the attribute
/// class): runs the constructor's side effects.
pub fn new_instance(attribute: &MappingAttribute) -> MappingAttribute {
    if let MappingAttribute::Table(table) = attribute {
        if table.indexes.is_some() {
            deprecations::trigger(TABLE_INDEXES_DEPRECATION, || {
                format!(
                    "Providing the property $indexes on {} does not have any effect and will be removed in Doctrine ORM 4.0. Please use the {} attribute instead.",
                    Table::CLASS,
                    Index::CLASS
                )
            });
        }
        if table.unique_constraints.is_some() {
            deprecations::trigger(TABLE_INDEXES_DEPRECATION, || {
                format!(
                    "Providing the property $uniqueConstraints on {} does not have any effect and will be removed in Doctrine ORM 4.0. Please use the {} attribute instead.",
                    Table::CLASS,
                    UniqueConstraint::CLASS
                )
            });
        }
    }
    attribute.clone()
}
