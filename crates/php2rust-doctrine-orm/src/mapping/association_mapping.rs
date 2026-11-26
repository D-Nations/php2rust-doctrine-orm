//! The association mappings. PHP's class hierarchy (`AssociationMapping`,
//! `OwningSideMapping`, `InverseSideMapping`, the to-one and to-many
//! sides and the six final classes) is one struct whose
//! [`AssociationClass`] says which of the classes it is, and so which
//! properties it has.
//!
//! Upstream: doctrine/orm src/Mapping/AssociationMapping.php src/Mapping/OwningSideMapping.php src/Mapping/InverseSideMapping.php src/Mapping/ToOneAssociationMapping.php src/Mapping/ToManyAssociationMapping.php src/Mapping/ToManyAssociationMappingImplementation.php src/Mapping/ToOneOwningSideMapping.php src/Mapping/ToOneInverseSideMapping.php src/Mapping/ToManyOwningSideMapping.php src/Mapping/ToManyInverseSideMapping.php src/Mapping/OneToOneAssociationMapping.php src/Mapping/OneToOneOwningSideMapping.php src/Mapping/OneToOneInverseSideMapping.php src/Mapping/ManyToOneAssociationMapping.php src/Mapping/OneToManyAssociationMapping.php src/Mapping/ManyToManyAssociationMapping.php src/Mapping/ManyToManyOwningSideMapping.php src/Mapping/ManyToManyInverseSideMapping.php

use php_value::{Key, PhpArray, Thrown, Value};

use crate::deprecations;
use crate::mapping::join_column_mapping::{JoinColumnMapping, JoinTableMapping};
use crate::mapping::mapping_exception;
use crate::mapping::naming_strategy::NamingStrategy;
use crate::mapping::property_value::{
    self as pv, Sleep, from_opt_array, from_opt_bool, from_opt_int, from_opt_string,
};

/// `ClassMetadata::ONE_TO_ONE`.
pub const ONE_TO_ONE: i64 = 1;
/// `ClassMetadata::MANY_TO_ONE`.
pub const MANY_TO_ONE: i64 = 2;
/// `ClassMetadata::ONE_TO_MANY`.
pub const ONE_TO_MANY: i64 = 4;
/// `ClassMetadata::MANY_TO_MANY`.
pub const MANY_TO_MANY: i64 = 8;

/// The deprecation of `nullable` on identifier and many-to-many join
/// columns.
pub const NULLABLE_JOIN_COLUMN_DEPRECATION: &str = "https://github.com/doctrine/orm/pull/12126";

/// One of the six final association mapping classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssociationKind {
    OneToOneOwningSide,
    OneToOneInverseSide,
    ManyToOne,
    OneToMany,
    ManyToManyOwningSide,
    ManyToManyInverseSide,
}

impl AssociationKind {
    pub fn class_name(self) -> &'static str {
        match self {
            Self::OneToOneOwningSide => "Doctrine\\ORM\\Mapping\\OneToOneOwningSideMapping",
            Self::OneToOneInverseSide => "Doctrine\\ORM\\Mapping\\OneToOneInverseSideMapping",
            Self::ManyToOne => "Doctrine\\ORM\\Mapping\\ManyToOneAssociationMapping",
            Self::OneToMany => "Doctrine\\ORM\\Mapping\\OneToManyAssociationMapping",
            Self::ManyToManyOwningSide => "Doctrine\\ORM\\Mapping\\ManyToManyOwningSideMapping",
            Self::ManyToManyInverseSide => "Doctrine\\ORM\\Mapping\\ManyToManyInverseSideMapping",
        }
    }

    pub const ALL: [Self; 6] = [
        Self::OneToOneOwningSide,
        Self::OneToOneInverseSide,
        Self::ManyToOne,
        Self::OneToMany,
        Self::ManyToManyOwningSide,
        Self::ManyToManyInverseSide,
    ];
}

/// The abstract class a user-defined association mapping class extends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AbstractBase {
    /// `AssociationMapping`.
    Association,
    /// `OwningSideMapping`.
    OwningSide,
    /// `InverseSideMapping`.
    InverseSide,
}

/// The class of an association mapping.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AssociationClass {
    Final(AssociationKind),
    /// A class of the application that extends one of the abstract
    /// classes, optionally using `ToManyAssociationMappingImplementation`
    /// (and implementing `ToManyAssociationMapping`).
    Custom {
        name: String,
        base: AbstractBase,
        to_many: bool,
    },
}

impl AssociationClass {
    pub fn name(&self) -> &str {
        match self {
            Self::Final(kind) => kind.class_name(),
            Self::Custom { name, .. } => name,
        }
    }

    fn kind(&self) -> Option<AssociationKind> {
        match self {
            Self::Final(kind) => Some(*kind),
            Self::Custom { .. } => None,
        }
    }

    /// `instanceof OwningSideMapping`.
    fn is_owning(&self) -> bool {
        match self {
            Self::Final(kind) => matches!(
                kind,
                AssociationKind::OneToOneOwningSide
                    | AssociationKind::ManyToOne
                    | AssociationKind::ManyToManyOwningSide
            ),
            Self::Custom { base, .. } => *base == AbstractBase::OwningSide,
        }
    }

    /// `instanceof InverseSideMapping`.
    fn is_inverse(&self) -> bool {
        match self {
            Self::Final(kind) => matches!(
                kind,
                AssociationKind::OneToOneInverseSide
                    | AssociationKind::OneToMany
                    | AssociationKind::ManyToManyInverseSide
            ),
            Self::Custom { base, .. } => *base == AbstractBase::InverseSide,
        }
    }

    /// Uses `ToManyAssociationMappingImplementation`.
    fn has_to_many_properties(&self) -> bool {
        match self {
            Self::Final(kind) => matches!(
                kind,
                AssociationKind::OneToMany
                    | AssociationKind::ManyToManyOwningSide
                    | AssociationKind::ManyToManyInverseSide
            ),
            Self::Custom { to_many, .. } => *to_many,
        }
    }

    /// `instanceof ToManyAssociationMapping` (`ToManyOwningSideMapping`
    /// does not implement it, but both of its final classes do).
    fn is_to_many(&self) -> bool {
        self.has_to_many_properties()
    }

    /// `instanceof ToOneOwningSideMapping`.
    fn is_to_one_owning(&self) -> bool {
        matches!(
            self.kind(),
            Some(AssociationKind::OneToOneOwningSide | AssociationKind::ManyToOne)
        )
    }

    /// `instanceof ToOneAssociationMapping`.
    fn is_to_one(&self) -> bool {
        matches!(
            self.kind(),
            Some(
                AssociationKind::OneToOneOwningSide
                    | AssociationKind::OneToOneInverseSide
                    | AssociationKind::ManyToOne
            )
        )
    }

    fn is_many_to_many_owning(&self) -> bool {
        self.kind() == Some(AssociationKind::ManyToManyOwningSide)
    }

    /// The property names, in PHP's declaration order (parent class
    /// first).
    pub fn properties(&self) -> Vec<&'static str> {
        let mut names = vec![
            "cascade",
            "fetch",
            "inherited",
            "declared",
            "cache",
            "id",
            "isOnDeleteCascade",
            "originalClass",
            "originalField",
            "orphanRemoval",
            "unique",
            "fieldName",
            "sourceEntity",
            "targetEntity",
        ];
        if self.is_owning() {
            names.push("inversedBy");
        }
        if self.is_inverse() {
            names.push("mappedBy");
        }
        if self.is_to_one_owning() {
            names.extend([
                "sourceToTargetKeyColumns",
                "targetToSourceKeyColumns",
                "joinColumns",
                "joinColumnFieldNames",
            ]);
        }
        if self.has_to_many_properties() {
            names.extend(["indexBy", "orderBy"]);
        }
        if self.is_many_to_many_owning() {
            names.extend([
                "joinTable",
                "joinTableColumns",
                "relationToSourceKeyColumns",
                "relationToTargetKeyColumns",
            ]);
        }
        names
    }
}

/// An association mapping (`Doctrine\ORM\Mapping\AssociationMapping` and
/// its subclasses). The properties a class does not have stay at their
/// defaults.
#[derive(Debug, Clone, PartialEq)]
pub struct AssociationMapping {
    pub class: AssociationClass,
    /// The names of persistence operations to cascade on the association.
    pub cascade: Vec<String>,
    /// The fetching strategy to use for the association, usually
    /// defaults to `FETCH_LAZY`.
    pub fetch: Option<i64>,
    /// The class this association is inherited from.
    pub inherited: Option<String>,
    /// The class that declares this association.
    pub declared: Option<String>,
    pub cache: Option<PhpArray>,
    pub id: Option<bool>,
    pub is_on_delete_cascade: Option<bool>,
    pub original_class: Option<String>,
    pub original_field: Option<String>,
    pub orphan_removal: bool,
    pub unique: Option<bool>,
    /// The name of the field in the entity the association is mapped to.
    pub field_name: String,
    /// The class name of the source entity. In the case of to-many
    /// associations initially present in mapped superclasses, the
    /// nearest entity subclasses will be considered the respective
    /// source entities.
    pub source_entity: String,
    /// The class name of the target entity.
    pub target_entity: String,
    /// Owning side: the field on the inverse side.
    pub inversed_by: Option<String>,
    /// Inverse side: the field on the owning side (an uninitialized
    /// typed property until set).
    pub mapped_by: Option<String>,
    /// To-many: the field to index the collection by.
    pub index_by: Option<String>,
    /// To-many: the order of the collection.
    pub order_by: PhpArray,
    /// To-one owning side.
    pub source_to_target_key_columns: PhpArray,
    pub target_to_source_key_columns: PhpArray,
    pub join_columns: Vec<JoinColumnMapping>,
    pub join_column_field_names: PhpArray,
    /// Many-to-many owning side: the join table (an uninitialized typed
    /// property until set).
    pub join_table: Option<JoinTableMapping>,
    pub join_table_columns: Vec<String>,
    pub relation_to_source_key_columns: PhpArray,
    pub relation_to_target_key_columns: PhpArray,
}

fn out_of_range(message: String) -> Thrown {
    Thrown::new("OutOfRangeException", message, 0)
}

fn unknown_property(offset: &str, class: &str) -> Thrown {
    out_of_range(format!("Unknown property \"{offset}\" on class {class}"))
}

/// `trim($name, BACKTICK)`: strips the backticks.
fn trim_backticks(name: &str) -> String {
    name.trim_matches('`').to_owned()
}

/// `array_flip()` of a string-to-string map.
fn array_flip(array: &PhpArray) -> PhpArray {
    let mut flipped = PhpArray::new();
    for (key, value) in array.iter() {
        if let Some(value_key) = Key::from_value(value) {
            flipped.insert(value_key, key.to_value());
        }
    }
    flipped
}

impl AssociationMapping {
    /// `new static($fieldName, $sourceEntity, $targetEntity)`.
    pub fn new(
        class: AssociationClass,
        field_name: impl Into<String>,
        source_entity: impl Into<String>,
        target_entity: impl Into<String>,
    ) -> Self {
        Self {
            class,
            cascade: Vec::new(),
            fetch: None,
            inherited: None,
            declared: None,
            cache: None,
            id: None,
            is_on_delete_cascade: None,
            original_class: None,
            original_field: None,
            orphan_removal: false,
            unique: None,
            field_name: field_name.into(),
            source_entity: source_entity.into(),
            target_entity: target_entity.into(),
            inversed_by: None,
            mapped_by: None,
            index_by: None,
            order_by: PhpArray::new(),
            source_to_target_key_columns: PhpArray::new(),
            target_to_source_key_columns: PhpArray::new(),
            join_columns: Vec::new(),
            join_column_field_names: PhpArray::new(),
            join_table: None,
            join_table_columns: Vec::new(),
            relation_to_source_key_columns: PhpArray::new(),
            relation_to_target_key_columns: PhpArray::new(),
        }
    }

    /// `static::fromMappingArray()`.
    pub fn from_mapping_array(
        class: AssociationClass,
        mapping_array: &PhpArray,
    ) -> Result<Self, Thrown> {
        let mut mapping_array = mapping_array.clone();
        let join_columns = if class.is_to_one_owning() {
            mapping_array.remove(&Key::from("joinColumns"))
        } else {
            None
        };
        let mut mapping = Self::base_from_mapping_array(class, mapping_array)?;
        if mapping.class.is_to_one_owning() {
            if let Some(Value::Array(columns)) = join_columns {
                for column in columns.values() {
                    mapping
                        .join_columns
                        .push(JoinColumnMapping::from_value(column)?);
                }
            }
            if mapping.orphan_removal {
                if !mapping.is_cascade_remove() {
                    mapping.cascade.push("remove".to_owned());
                }
                mapping.unique = None;
            }
        }
        if mapping.class.kind() == Some(AssociationKind::OneToMany)
            && mapping.orphan_removal
            && !mapping.is_cascade_remove()
        {
            mapping.cascade.push("remove".to_owned());
        }
        Ok(mapping)
    }

    /// `AssociationMapping::fromMappingArray()`.
    fn base_from_mapping_array(
        class: AssociationClass,
        mut mapping_array: PhpArray,
    ) -> Result<Self, Thrown> {
        mapping_array.remove(&Key::from("isOwningSide"));
        mapping_array.remove(&Key::from("type"));
        let name = class.name().to_owned();
        let mut mapping = Self::new(
            class,
            pv::string(
                pv::required(&mapping_array, "fieldName"),
                &name,
                "fieldName",
            )?,
            pv::string(
                pv::required(&mapping_array, "sourceEntity"),
                &name,
                "sourceEntity",
            )?,
            pv::string(
                pv::required(&mapping_array, "targetEntity"),
                &name,
                "targetEntity",
            )?,
        );
        for key in ["fieldName", "sourceEntity", "targetEntity"] {
            mapping_array.remove(&Key::from(key));
        }
        for (key, value) in mapping_array.iter() {
            let key = pv::key_name(key);
            if key == "joinTable" {
                if matches!(value, Value::Null) || matches!(value, Value::Array(a) if a.is_empty())
                {
                    continue;
                }
                if !mapping.class.is_many_to_many_owning() {
                    return Err(Thrown::new(
                        mapping_exception::CLASS,
                        format!(
                            "Mapping error on field '{}' in {} : 'joinTable' can only be set on many-to-many owning side.",
                            mapping.field_name, mapping.source_entity
                        ),
                        0,
                    ));
                }
                mapping.join_table = Some(JoinTableMapping::from_value(value)?);
                continue;
            }
            if !mapping.set_property(&key, value.clone())? {
                return Err(out_of_range(format!(
                    "Unknown property {key} on class {name}"
                )));
            }
        }
        Ok(mapping)
    }

    /// `ToOneOwningSideMapping::fromMappingArrayAndName()`. `table_is_set`
    /// is whether the class metadata's table is not empty.
    pub fn to_one_owning_side_from_mapping_array_and_name(
        kind: AssociationKind,
        mapping_array: &PhpArray,
        naming_strategy: &dyn NamingStrategy,
        name: &str,
        table_is_set: bool,
        is_inheritance_type_single_table: bool,
    ) -> Result<Self, Thrown> {
        let mut mapping_array = mapping_array.clone();
        let field_name = pv::get(&mapping_array, "fieldName")
            .and_then(Value::to_php_string)
            .map(|s| pv::text(&s))
            .unwrap_or_default();
        if let Some(Value::Array(join_columns)) = mapping_array.get_mut(&Key::from("joinColumns")) {
            let keys: Vec<Key> = join_columns.keys().cloned().collect();
            for index in keys {
                if let Some(Value::Array(join_column)) = join_columns.get_mut(&index) {
                    if pv::empty(pv::get(join_column, "name")) {
                        join_column.insert(
                            "name",
                            Value::from(naming_strategy.join_column_name(&field_name, name)),
                        );
                    }
                    if pv::empty(pv::get(join_column, "referencedColumnName")) {
                        join_column.insert(
                            "referencedColumnName",
                            Value::from(naming_strategy.reference_column_name()),
                        );
                    }
                }
            }
        }
        let mut mapping = Self::from_mapping_array(AssociationClass::Final(kind), &mapping_array)?;
        if mapping.join_columns.is_empty() {
            // Apply default join column
            mapping.join_columns = vec![JoinColumnMapping::new(
                naming_strategy.join_column_name(&mapping.field_name, name),
                naming_strategy.reference_column_name(),
            )];
        }
        let mut unique_constraint_columns = Vec::new();
        let count = mapping.join_columns.len();
        let is_one_to_one = mapping.is_one_to_one();
        let id = mapping.id;
        for join_column in &mut mapping.join_columns {
            if id == Some(true) {
                if join_column.nullable.is_some() {
                    let (source, field) =
                        (mapping.source_entity.clone(), mapping.field_name.clone());
                    deprecations::trigger(NULLABLE_JOIN_COLUMN_DEPRECATION, || {
                        format!(
                            "Specifying the \"nullable\" attribute for join columns in to-one associations (here, {source}::${field}) that are part of the identifier is a no-op.\nThe ORM will always set it to false.\nDoing so is deprecated and will be an error in 4.0."
                        )
                    });
                }
                join_column.nullable = Some(false);
            } else if join_column.nullable.is_none() {
                join_column.nullable = Some(true);
            }
            if is_one_to_one && !is_inheritance_type_single_table {
                if count == 1 {
                    if id != Some(true) {
                        join_column.unique = Some(true);
                    }
                } else {
                    unique_constraint_columns.push(join_column.name.clone());
                }
            }
            if join_column.referenced_column_name.is_empty()
                || join_column.referenced_column_name == "0"
            {
                join_column.referenced_column_name = naming_strategy.reference_column_name();
            }
            if join_column.name.starts_with('`') {
                join_column.name = trim_backticks(&join_column.name);
                join_column.quoted = Some(true);
            }
            if join_column.referenced_column_name.starts_with('`') {
                join_column.referenced_column_name =
                    trim_backticks(&join_column.referenced_column_name);
                join_column.quoted = Some(true);
            }
            mapping.source_to_target_key_columns.insert(
                join_column.name.as_str(),
                Value::from(join_column.referenced_column_name.as_str()),
            );
            mapping.join_column_field_names.insert(
                join_column.name.as_str(),
                Value::from(
                    join_column
                        .field_name
                        .as_deref()
                        .unwrap_or(&join_column.name),
                ),
            );
        }
        if !unique_constraint_columns.is_empty() && !table_is_set {
            return Err(Thrown::new(
                "RuntimeException",
                "ClassMetadata::setTable() has to be called before defining a one to one relationship.",
                0,
            ));
        }
        mapping.target_to_source_key_columns = array_flip(&mapping.source_to_target_key_columns);
        Ok(mapping)
    }

    /// `ToOneInverseSideMapping::fromMappingArrayAndName()`.
    pub fn to_one_inverse_side_from_mapping_array_and_name(
        mapping_array: &PhpArray,
        name: &str,
    ) -> Result<Self, Thrown> {
        let mut mapping = Self::from_mapping_array(
            AssociationClass::Final(AssociationKind::OneToOneInverseSide),
            mapping_array,
        )?;
        if mapping.id == Some(true) {
            return Err(mapping_exception::illegal_inverse_identifier_association(
                name,
                &mapping.field_name,
            ));
        }
        if mapping.orphan_removal {
            if !mapping.is_cascade_remove() {
                mapping.cascade.push("remove".to_owned());
            }
            mapping.unique = None;
        }
        Ok(mapping)
    }

    /// `OneToManyAssociationMapping::fromMappingArrayAndName()`.
    pub fn one_to_many_from_mapping_array_and_name(
        mapping_array: &PhpArray,
        name: &str,
    ) -> Result<Self, Thrown> {
        let mapping = Self::from_mapping_array(
            AssociationClass::Final(AssociationKind::OneToMany),
            mapping_array,
        )?;
        // OneToMany-side MUST be inverse (must have mappedBy)
        if mapping.mapped_by.is_none() {
            return Err(mapping_exception::one_to_many_requires_mapped_by(
                name,
                &mapping.field_name,
            ));
        }
        Ok(mapping)
    }

    /// `ManyToManyOwningSideMapping::fromMappingArrayAndNamingStrategy()`.
    #[allow(clippy::too_many_lines, reason = "one PHP method")]
    pub fn many_to_many_owning_side_from_mapping_array_and_naming_strategy(
        mapping_array: &PhpArray,
        naming_strategy: &dyn NamingStrategy,
    ) -> Result<Self, Thrown> {
        let mut mapping_array = mapping_array.clone();
        let string_of = |array: &PhpArray, key: &str| {
            pv::get(array, key)
                .and_then(Value::to_php_string)
                .map(|s| pv::text(&s))
                .unwrap_or_default()
        };
        let source_entity = string_of(&mapping_array, "sourceEntity");
        let target_entity = string_of(&mapping_array, "targetEntity");
        let field_name = string_of(&mapping_array, "fieldName");
        if let Some(Value::Array(join_table)) = mapping_array.get_mut(&Key::from("joinTable")) {
            for (key, entity) in [
                ("joinColumns", &source_entity),
                ("inverseJoinColumns", &target_entity),
            ] {
                if let Some(Value::Array(columns)) = join_table.get_mut(&Key::from(key)) {
                    let keys: Vec<Key> = columns.keys().cloned().collect();
                    for index in keys {
                        if let Some(Value::Array(join_column)) = columns.get_mut(&index) {
                            if pv::empty(pv::get(join_column, "referencedColumnName")) {
                                join_column.insert(
                                    "referencedColumnName",
                                    Value::from(naming_strategy.reference_column_name()),
                                );
                            }
                            if pv::empty(pv::get(join_column, "name")) {
                                let referenced = string_of(join_column, "referencedColumnName");
                                join_column.insert(
                                    "name",
                                    Value::from(
                                        naming_strategy
                                            .join_key_column_name(entity, Some(&referenced)),
                                    ),
                                );
                            }
                        }
                    }
                }
            }
        }
        // owning side MUST have a join table
        let has_name = matches!(
            pv::get(&mapping_array, "joinTable"),
            Some(Value::Array(join_table)) if pv::isset(join_table, "name")
        );
        if !has_name {
            let name = naming_strategy.join_table_name(&source_entity, &target_entity, &field_name);
            if let Some(Value::Array(join_table)) = mapping_array.get_mut(&Key::from("joinTable")) {
                join_table.insert("name", Value::from(name));
            } else {
                let mut join_table = PhpArray::new();
                join_table.insert("name", Value::from(name));
                mapping_array.insert("joinTable", Value::Array(join_table));
            }
        }
        let mut mapping = Self::from_mapping_array(
            AssociationClass::Final(AssociationKind::ManyToManyOwningSide),
            &mapping_array,
        )?;
        let mut join_table = mapping
            .join_table
            .take()
            .unwrap_or_else(|| JoinTableMapping::new(String::new()));
        let self_referencing_entity_without_join_columns = mapping.source_entity
            == mapping.target_entity
            && join_table.join_columns.is_empty()
            && join_table.inverse_join_columns.is_empty();
        if join_table.join_columns.is_empty() {
            let mut column = JoinColumnMapping::new(
                naming_strategy.join_key_column_name(
                    &mapping.source_entity,
                    self_referencing_entity_without_join_columns.then_some("source"),
                ),
                naming_strategy.reference_column_name(),
            );
            column.on_delete = Some("CASCADE".to_owned());
            join_table.join_columns = vec![column];
        }
        if join_table.inverse_join_columns.is_empty() {
            let mut column = JoinColumnMapping::new(
                naming_strategy.join_key_column_name(
                    &mapping.target_entity,
                    self_referencing_entity_without_join_columns.then_some("target"),
                ),
                naming_strategy.reference_column_name(),
            );
            column.on_delete = Some("CASCADE".to_owned());
            join_table.inverse_join_columns = vec![column];
        }
        mapping.join_table_columns = Vec::new();
        for (inverse, columns) in [
            (false, &mut join_table.join_columns),
            (true, &mut join_table.inverse_join_columns),
        ] {
            for join_column in columns.iter_mut() {
                if join_column.nullable.is_some() {
                    let entity = if inverse {
                        mapping.target_entity.clone()
                    } else {
                        mapping.source_entity.clone()
                    };
                    let field = mapping.field_name.clone();
                    deprecations::trigger(NULLABLE_JOIN_COLUMN_DEPRECATION, || {
                        format!(
                            "Specifying the \"nullable\" attribute for join columns in many-to-many associations (here, {entity}::${field}) is a no-op.\nThe ORM will always set it to false.\nDoing so is deprecated and will be an error in 4.0."
                        )
                    });
                }
                join_column.nullable = Some(false);
                if join_column.referenced_column_name.is_empty()
                    || join_column.referenced_column_name == "0"
                {
                    join_column.referenced_column_name = naming_strategy.reference_column_name();
                }
                if join_column.name.starts_with('`') {
                    join_column.name = trim_backticks(&join_column.name);
                    join_column.quoted = Some(true);
                }
                if join_column.referenced_column_name.starts_with('`') {
                    join_column.referenced_column_name =
                        trim_backticks(&join_column.referenced_column_name);
                    join_column.quoted = Some(true);
                }
                if join_column
                    .on_delete
                    .as_deref()
                    .is_some_and(|on_delete| on_delete.eq_ignore_ascii_case("cascade"))
                {
                    mapping.is_on_delete_cascade = Some(true);
                }
                let target = if inverse {
                    &mut mapping.relation_to_target_key_columns
                } else {
                    &mut mapping.relation_to_source_key_columns
                };
                target.insert(
                    join_column.name.as_str(),
                    Value::from(join_column.referenced_column_name.as_str()),
                );
                mapping.join_table_columns.push(join_column.name.clone());
            }
        }
        mapping.join_table = Some(join_table);
        Ok(mapping)
    }

    pub fn class_name(&self) -> &str {
        self.class.name()
    }

    /// `isOwningSide()`.
    pub fn is_owning_side(&self) -> bool {
        self.class.is_owning()
    }

    /// `isToOne()`.
    pub fn is_to_one(&self) -> bool {
        self.class.is_to_one()
    }

    /// `isToMany()`.
    pub fn is_to_many(&self) -> bool {
        self.class.is_to_many()
    }

    /// `isOneToOneOwningSide()`.
    pub fn is_one_to_one_owning_side(&self) -> bool {
        self.is_one_to_one() && self.is_owning_side()
    }

    /// `isToOneOwningSide()`.
    pub fn is_to_one_owning_side(&self) -> bool {
        self.is_to_one() && self.is_owning_side()
    }

    /// `isManyToManyOwningSide()`.
    pub fn is_many_to_many_owning_side(&self) -> bool {
        self.class.is_many_to_many_owning()
    }

    /// `isOneToOne()`.
    pub fn is_one_to_one(&self) -> bool {
        matches!(
            self.class.kind(),
            Some(AssociationKind::OneToOneOwningSide | AssociationKind::OneToOneInverseSide)
        )
    }

    /// `isOneToMany()`.
    pub fn is_one_to_many(&self) -> bool {
        self.class.kind() == Some(AssociationKind::OneToMany)
    }

    /// `isManyToOne()`.
    pub fn is_many_to_one(&self) -> bool {
        self.class.kind() == Some(AssociationKind::ManyToOne)
    }

    /// `isManyToMany()`.
    pub fn is_many_to_many(&self) -> bool {
        matches!(
            self.class.kind(),
            Some(AssociationKind::ManyToManyOwningSide | AssociationKind::ManyToManyInverseSide)
        )
    }

    /// `isOrdered()`.
    pub fn is_ordered(&self) -> bool {
        self.is_to_many() && !self.order_by.is_empty()
    }

    /// `isIndexed()`.
    pub fn is_indexed(&self) -> bool {
        self.class.has_to_many_properties() && self.index_by.is_some()
    }

    /// `indexBy()`: a `LogicException` when the mapping is not indexed.
    pub fn index_by(&self) -> Result<&str, Thrown> {
        match &self.index_by {
            Some(index_by) if self.class.has_to_many_properties() => Ok(index_by),
            _ => {
                let class = self.class.name();
                Err(Thrown::new(
                    "LogicException",
                    format!(
                        "This mapping is not indexed. Use {class}::isIndexed() to check that before calling {class}::indexBy."
                    ),
                    0,
                ))
            }
        }
    }

    /// `orderBy()`.
    pub fn order_by(&self) -> &PhpArray {
        &self.order_by
    }

    /// `InverseSideMapping::backRefFieldName()`.
    pub fn back_ref_field_name(&self) -> Option<&str> {
        self.mapped_by.as_deref()
    }

    /// `type()`: `ClassMetadata::ONE_TO_ONE` and the others.
    pub fn r#type(&self) -> Result<i64, Thrown> {
        match self.class.kind() {
            Some(AssociationKind::OneToOneOwningSide | AssociationKind::OneToOneInverseSide) => {
                Ok(ONE_TO_ONE)
            }
            Some(AssociationKind::OneToMany) => Ok(ONE_TO_MANY),
            Some(AssociationKind::ManyToOne) => Ok(MANY_TO_ONE),
            Some(
                AssociationKind::ManyToManyOwningSide | AssociationKind::ManyToManyInverseSide,
            ) => Ok(MANY_TO_MANY),
            None => Err(Thrown::new(
                "Exception",
                format!("Cannot determine type for {}", self.class.name()),
                0,
            )),
        }
    }

    /// `isCascadeRemove()`.
    pub fn is_cascade_remove(&self) -> bool {
        self.cascade.iter().any(|c| c == "remove")
    }

    /// `isCascadePersist()`.
    pub fn is_cascade_persist(&self) -> bool {
        self.cascade.iter().any(|c| c == "persist")
    }

    /// `isCascadeRefresh()`.
    pub fn is_cascade_refresh(&self) -> bool {
        self.cascade.iter().any(|c| c == "refresh")
    }

    /// `isCascadeDetach()`.
    pub fn is_cascade_detach(&self) -> bool {
        self.cascade.iter().any(|c| c == "detach")
    }

    /// `property_exists($this, $name)`.
    pub fn property_exists(&self, name: &str) -> bool {
        self.class.properties().contains(&name)
    }

    /// The property's value; `None` when the class has no such property
    /// or it is an uninitialized typed property.
    pub fn property(&self, name: &str) -> Option<Value> {
        if !self.property_exists(name) {
            return None;
        }
        Some(match name {
            "cascade" => pv::from_strings(&self.cascade),
            "fetch" => from_opt_int(self.fetch),
            "inherited" => from_opt_string(self.inherited.as_ref()),
            "declared" => from_opt_string(self.declared.as_ref()),
            "cache" => from_opt_array(self.cache.as_ref()),
            "id" => from_opt_bool(self.id),
            "isOnDeleteCascade" => from_opt_bool(self.is_on_delete_cascade),
            "originalClass" => from_opt_string(self.original_class.as_ref()),
            "originalField" => from_opt_string(self.original_field.as_ref()),
            "orphanRemoval" => Value::Bool(self.orphan_removal),
            "unique" => from_opt_bool(self.unique),
            "fieldName" => Value::from(self.field_name.as_str()),
            "sourceEntity" => Value::from(self.source_entity.as_str()),
            "targetEntity" => Value::from(self.target_entity.as_str()),
            "inversedBy" => from_opt_string(self.inversed_by.as_ref()),
            "mappedBy" => Value::from(self.mapped_by.as_deref()?),
            "indexBy" => from_opt_string(self.index_by.as_ref()),
            "orderBy" => Value::Array(self.order_by.clone()),
            "sourceToTargetKeyColumns" => Value::Array(self.source_to_target_key_columns.clone()),
            "targetToSourceKeyColumns" => Value::Array(self.target_to_source_key_columns.clone()),
            "joinColumns" => Value::Array(PhpArray::list(
                self.join_columns.iter().map(JoinColumnMapping::to_object),
            )),
            "joinColumnFieldNames" => Value::Array(self.join_column_field_names.clone()),
            "joinTable" => self.join_table.as_ref()?.to_object(),
            "joinTableColumns" => pv::from_strings(&self.join_table_columns),
            "relationToSourceKeyColumns" => {
                Value::Array(self.relation_to_source_key_columns.clone())
            }
            "relationToTargetKeyColumns" => {
                Value::Array(self.relation_to_target_key_columns.clone())
            }
            _ => return None,
        })
    }

    /// Assigns a property as PHP's typed properties take it; `Ok(false)`
    /// when the class has no such property.
    #[allow(
        clippy::needless_pass_by_value,
        reason = "PHP's assignment takes the value"
    )]
    pub fn set_property(&mut self, name: &str, value: Value) -> Result<bool, Thrown> {
        if !self.property_exists(name) {
            return Ok(false);
        }
        let class = self.class.name().to_owned();
        let c = class.as_str();
        match name {
            "cascade" => self.cascade = pv::strings(&value, c, name)?,
            "fetch" => self.fetch = pv::opt_int(&value, c, name)?,
            "inherited" => self.inherited = pv::opt_string(&value, c, name)?,
            "declared" => self.declared = pv::opt_string(&value, c, name)?,
            "cache" => self.cache = pv::opt_array(&value, c, name)?,
            "id" => self.id = pv::opt_bool(&value, c, name)?,
            "isOnDeleteCascade" => self.is_on_delete_cascade = pv::opt_bool(&value, c, name)?,
            "originalClass" => self.original_class = pv::opt_string(&value, c, name)?,
            "originalField" => self.original_field = pv::opt_string(&value, c, name)?,
            "orphanRemoval" => self.orphan_removal = pv::bool(&value, c, name)?,
            "unique" => self.unique = pv::opt_bool(&value, c, name)?,
            "fieldName" => self.field_name = pv::string(&value, c, name)?,
            "sourceEntity" => self.source_entity = pv::string(&value, c, name)?,
            "targetEntity" => self.target_entity = pv::string(&value, c, name)?,
            "inversedBy" => self.inversed_by = pv::opt_string(&value, c, name)?,
            "mappedBy" => self.mapped_by = Some(pv::string(&value, c, name)?),
            "indexBy" => self.index_by = pv::opt_string(&value, c, name)?,
            "orderBy" => self.order_by = pv::array(&value, c, name)?,
            "sourceToTargetKeyColumns" => {
                self.source_to_target_key_columns = pv::array(&value, c, name)?;
            }
            "targetToSourceKeyColumns" => {
                self.target_to_source_key_columns = pv::array(&value, c, name)?;
            }
            "joinColumns" => {
                self.join_columns = pv::array(&value, c, name)?
                    .values()
                    .map(JoinColumnMapping::from_value)
                    .collect::<Result<_, _>>()?;
            }
            "joinColumnFieldNames" => self.join_column_field_names = pv::array(&value, c, name)?,
            "joinTable" => self.join_table = Some(JoinTableMapping::from_value(&value)?),
            "joinTableColumns" => self.join_table_columns = pv::strings(&value, c, name)?,
            "relationToSourceKeyColumns" => {
                self.relation_to_source_key_columns = pv::array(&value, c, name)?;
            }
            "relationToTargetKeyColumns" => {
                self.relation_to_target_key_columns = pv::array(&value, c, name)?;
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// `offsetExists()`.
    pub fn offset_exists(&self, offset: &str) -> bool {
        self.property(offset)
            .is_some_and(|value| !matches!(value, Value::Null))
            || ["isOwningSide", "type"].contains(&offset)
    }

    /// `offsetGet()`.
    pub fn offset_get(&self, offset: &str) -> Result<Value, Thrown> {
        Ok(match offset {
            "isOwningSide" => Value::Bool(self.is_owning_side()),
            "type" => Value::Int(self.r#type()?),
            "isCascadeRemove" => Value::Bool(self.is_cascade_remove()),
            "isCascadePersist" => Value::Bool(self.is_cascade_persist()),
            "isCascadeRefresh" => Value::Bool(self.is_cascade_refresh()),
            "isCascadeDetach" => Value::Bool(self.is_cascade_detach()),
            _ => {
                if !self.property_exists(offset) {
                    return Err(unknown_property(offset, self.class.name()));
                }
                match self.property(offset) {
                    Some(value) => value,
                    None => {
                        return Err(Thrown::new(
                            "Error",
                            format!(
                                "Typed property {}::${offset} must not be accessed before initialization",
                                self.class.name()
                            ),
                            0,
                        ));
                    }
                }
            }
        })
    }

    /// `offsetSet()`: `joinTable` is given as a mapping array, the join
    /// columns of a to-one owning side as a list of them.
    pub fn offset_set(&mut self, offset: &str, value: Value) -> Result<(), Thrown> {
        if !self.property_exists(offset) {
            return Err(unknown_property(offset, self.class.name()));
        }
        self.set_property(offset, value).map(|_| ())
    }

    /// `offsetUnset()`: the property becomes null.
    pub fn offset_unset(&mut self, offset: &str) -> Result<(), Thrown> {
        if !self.property_exists(offset) {
            return Err(unknown_property(offset, self.class.name()));
        }
        self.set_property(offset, Value::Null).map(|_| ())
    }

    /// `toArray()`: the initialized properties, `isOwningSide` and
    /// `type`; join columns and the join table as arrays.
    pub fn to_array(&self) -> Result<PhpArray, Thrown> {
        let mut array = PhpArray::new();
        for name in self.class.properties() {
            let value = match name {
                "joinColumns" => {
                    Value::Array(PhpArray::list(self.join_columns.iter().map(|column| {
                        let mut column = column.to_array();
                        if self.id == Some(true) {
                            column.remove(&Key::from("nullable"));
                        }
                        Value::Array(column)
                    })))
                }
                "joinTable" => match &self.join_table {
                    Some(join_table) => Value::Array(join_table.to_array()),
                    None => continue,
                },
                _ => match self.property(name) {
                    Some(value) => value,
                    None => continue,
                },
            };
            array.insert(name, value);
        }
        array.insert("isOwningSide", Value::Bool(self.is_owning_side()));
        array.insert("type", Value::Int(self.r#type()?));
        Ok(array)
    }

    /// `__sleep()`.
    pub fn sleep_names(&self) -> Vec<&'static str> {
        let mut serialized = vec!["fieldName", "sourceEntity", "targetEntity"];
        if !self.cascade.is_empty() {
            serialized.push("cascade");
        }
        for name in [
            "fetch",
            "inherited",
            "declared",
            "cache",
            "originalClass",
            "originalField",
        ] {
            if !matches!(self.property(name), Some(Value::Null) | None) {
                serialized.push(name);
            }
        }
        for (name, set) in [
            ("id", self.id == Some(true)),
            ("orphanRemoval", self.orphan_removal),
            ("isOnDeleteCascade", self.is_on_delete_cascade == Some(true)),
            ("unique", self.unique == Some(true)),
        ] {
            if set {
                serialized.push(name);
            }
        }
        if self.class.is_owning() && self.inversed_by.is_some() {
            serialized.push("inversedBy");
        }
        if self.class.is_inverse() {
            serialized.push("mappedBy");
        }
        if self.class.has_to_many_properties() {
            if self.index_by.is_some() {
                serialized.push("indexBy");
            }
            if !self.order_by.is_empty() {
                serialized.push("orderBy");
            }
        }
        if self.class.is_to_one_owning() {
            serialized.extend([
                "joinColumns",
                "joinColumnFieldNames",
                "sourceToTargetKeyColumns",
                "targetToSourceKeyColumns",
            ]);
        }
        if self.class.is_many_to_many_owning() {
            serialized.extend([
                "joinTable",
                "joinTableColumns",
                "relationToSourceKeyColumns",
                "relationToTargetKeyColumns",
            ]);
        }
        serialized
    }

    /// `unserialize()` of a serialized association mapping of one of
    /// the final classes or of `custom`.
    pub fn unserialize(data: &[u8], custom: Option<&AssociationClass>) -> Option<Self> {
        let mut names: Vec<&str> = AssociationKind::ALL
            .iter()
            .map(|k| k.class_name())
            .collect();
        if let Some(custom) = custom {
            names.push(custom.name());
        }
        names.extend([JoinColumnMapping::CLASS, JoinTableMapping::CLASS]);
        let (class, properties) = pv::unserialize_object(data, &names)?;
        Self::from_properties(&class, &properties, custom)
    }

    /// The object of class `class` (a final class, or `custom`) from its
    /// serialized properties.
    pub fn from_properties(
        class: &str,
        properties: &PhpArray,
        custom: Option<&AssociationClass>,
    ) -> Option<Self> {
        let class = AssociationKind::ALL
            .iter()
            .find(|kind| kind.class_name().eq_ignore_ascii_case(class))
            .map(|kind| AssociationClass::Final(*kind))
            .or_else(|| {
                custom
                    .filter(|c| c.name().eq_ignore_ascii_case(class))
                    .cloned()
            })?;
        let field = |key: &str| {
            pv::get(properties, key)
                .and_then(Value::to_php_string)
                .map(|s| pv::text(&s))
        };
        let mut mapping = Self::new(
            class,
            field("fieldName")?,
            field("sourceEntity")?,
            field("targetEntity")?,
        );
        for (key, value) in properties.iter() {
            mapping
                .set_property(&pv::key_name(key), value.clone())
                .ok()?;
        }
        Some(mapping)
    }
}

impl Sleep for AssociationMapping {
    fn class_name(&self) -> String {
        self.class.name().to_owned()
    }

    fn sleep(&self) -> PhpArray {
        let mut array = PhpArray::new();
        for name in self.sleep_names() {
            if let Some(value) = self.property(name) {
                array.insert(name, value);
            }
        }
        array
    }
}
