//! The builder API for mapping metadata: `ClassMetadataBuilder` and the
//! field, embedded, association and entity listener builders.
//!
//! The builders return the same `ClassMetadataBuilder` they were created
//! from (`&ClassMetadataBuilder`), so a chain of calls stays fluent; the
//! nested builders take and give themselves by value.
//!
//! Upstream: doctrine/orm src/Mapping/Builder/ClassMetadataBuilder.php src/Mapping/Builder/FieldBuilder.php src/Mapping/Builder/EmbeddedBuilder.php src/Mapping/Builder/AssociationBuilder.php src/Mapping/Builder/OneToManyAssociationBuilder.php src/Mapping/Builder/ManyToManyAssociationBuilder.php src/Mapping/Builder/EntityListenerBuilder.php

use std::sync::Arc;

use php_value::{Key, PhpArray, Thrown, Value};

use crate::mapping::association_mapping::{MANY_TO_MANY, MANY_TO_ONE, ONE_TO_MANY, ONE_TO_ONE};
use crate::mapping::class_metadata::{self as cm, ClassMetadata, ClassMetadataInfo};
use crate::mapping::driver::attribute_driver;

fn list<S: AsRef<str>>(values: &[S]) -> Value {
    let mut array = PhpArray::new();
    for value in values {
        let _ = array.push(Value::from(value.as_ref()));
    }
    Value::Array(array)
}

fn opt_str(value: Option<&str>) -> Value {
    value.map_or(Value::Null, Value::from)
}

fn rows(rows: &[PhpArray]) -> Value {
    let mut array = PhpArray::new();
    for row in rows {
        let _ = array.push(Value::Array(row.clone()));
    }
    Value::Array(array)
}

/// `Doctrine\ORM\Mapping\Builder\ClassMetadataBuilder`.
#[derive(Debug)]
pub struct ClassMetadataBuilder {
    cm: Arc<ClassMetadata>,
}

impl ClassMetadataBuilder {
    pub fn new(cm: Arc<ClassMetadata>) -> Self {
        Self { cm }
    }

    /// `getClassMetadata()`.
    pub fn get_class_metadata(&self) -> &Arc<ClassMetadata> {
        &self.cm
    }

    fn with<T>(&self, f: impl FnOnce(&mut ClassMetadataInfo) -> T) -> T {
        f(&mut self.cm.write())
    }

    /// Marks the class as mapped superclass.
    pub fn set_mapped_super_class(&self) -> &Self {
        self.with(|cm| {
            cm.is_mapped_superclass = true;
            cm.is_embedded_class = false;
        });
        self
    }

    /// Marks the class as embeddable.
    pub fn set_embeddable(&self) -> &Self {
        self.with(|cm| {
            cm.is_embedded_class = true;
            cm.is_mapped_superclass = false;
        });
        self
    }

    /// Adds and embedded class.
    pub fn add_embedded(
        &self,
        field_name: &str,
        class: &str,
        column_prefix: Option<ColumnPrefixArgument<'_>>,
    ) -> Result<&Self, Thrown> {
        let mut mapping = PhpArray::new();
        mapping.insert("fieldName", Value::from(field_name));
        mapping.insert("class", Value::from(class));
        mapping.insert(
            "columnPrefix",
            column_prefix.map_or(Value::Null, ColumnPrefixArgument::value),
        );
        self.with(|cm| cm.map_embedded(mapping))?;
        Ok(self)
    }

    /// Sets custom Repository class name.
    pub fn set_custom_repository_class(&self, repository_class_name: &str) -> &Self {
        self.with(|cm| cm.set_custom_repository_class(Some(repository_class_name)));
        self
    }

    /// Marks class read only.
    pub fn set_read_only(&self) -> &Self {
        self.with(ClassMetadataInfo::mark_read_only);
        self
    }

    /// Sets the table name.
    pub fn set_table(&self, name: &str) -> &Self {
        let mut table = PhpArray::new();
        table.insert("name", Value::from(name));
        self.with(|cm| cm.set_primary_table(&table));
        self
    }

    fn add_table_index(&self, index_type: &str, columns: &[&str], name: &str) {
        self.with(|cm| {
            let key = Key::from(index_type);
            if !matches!(cm.table.get(&key), Some(Value::Array(_))) {
                cm.table.insert(key.clone(), Value::Array(PhpArray::new()));
            }
            if let Some(Value::Array(indexes)) = cm.table.get_mut(&key) {
                let mut index = PhpArray::new();
                index.insert("columns", list(columns));
                indexes.insert(name, Value::Array(index));
            }
        });
    }

    /// Adds Index.
    pub fn add_index(&self, columns: &[&str], name: &str) -> &Self {
        self.add_table_index("indexes", columns, name);
        self
    }

    /// Adds Unique Constraint.
    pub fn add_unique_constraint(&self, columns: &[&str], name: &str) -> &Self {
        self.add_table_index("uniqueConstraints", columns, name);
        self
    }

    /// Sets class as root of a joined table inheritance hierarchy.
    pub fn set_joined_table_inheritance(&self) -> Result<&Self, Thrown> {
        self.with(|cm| cm.set_inheritance_type(cm::INHERITANCE_TYPE_JOINED))?;
        Ok(self)
    }

    /// Sets class as root of a single table inheritance hierarchy.
    pub fn set_single_table_inheritance(&self) -> Result<&Self, Thrown> {
        self.with(|cm| cm.set_inheritance_type(cm::INHERITANCE_TYPE_SINGLE_TABLE))?;
        Ok(self)
    }

    /// Sets the discriminator column details (PHP's defaults: `'string'`,
    /// `255`, `null`, `null`, `[]`).
    pub fn set_discriminator_column(
        &self,
        name: &str,
        r#type: &str,
        length: i64,
        column_definition: Option<&str>,
        enum_type: Option<&str>,
        options: PhpArray,
    ) -> Result<&Self, Thrown> {
        let mut column = PhpArray::new();
        column.insert("name", Value::from(name));
        column.insert("type", Value::from(r#type));
        column.insert("length", Value::Int(length));
        column.insert("columnDefinition", opt_str(column_definition));
        column.insert("enumType", opt_str(enum_type));
        column.insert("options", Value::Array(options));
        self.with(|cm| cm.set_discriminator_column(Some(column)))?;
        Ok(self)
    }

    /// Adds a subclass to this inheritance hierarchy.
    pub fn add_discriminator_map_class(&self, name: &str, class: &str) -> Result<&Self, Thrown> {
        self.with(|cm| cm.add_discriminator_map_class(&Key::from(name), class))?;
        Ok(self)
    }

    /// Sets deferred explicit change tracking policy.
    pub fn set_change_tracking_policy_deferred_explicit(&self) -> &Self {
        self.with(|cm| cm.set_change_tracking_policy(cm::CHANGETRACKING_DEFERRED_EXPLICIT));
        self
    }

    /// Adds lifecycle event.
    pub fn add_lifecycle_event(&self, method_name: &str, event: &str) -> Result<&Self, Thrown> {
        self.with(|cm| cm.add_lifecycle_callback(method_name, event))?;
        Ok(self)
    }

    /// Adds Field.
    pub fn add_field(
        &self,
        name: &str,
        r#type: &str,
        mut mapping: PhpArray,
    ) -> Result<&Self, Thrown> {
        mapping.insert("fieldName", Value::from(name));
        mapping.insert("type", Value::from(r#type));
        self.with(|cm| cm.map_field(mapping))?;
        Ok(self)
    }

    /// Creates a field builder.
    pub fn create_field(&self, name: &str, r#type: &str) -> FieldBuilder<'_> {
        let mut mapping = PhpArray::new();
        mapping.insert("fieldName", Value::from(name));
        mapping.insert("type", Value::from(r#type));
        FieldBuilder::new(self, mapping)
    }

    /// Creates an embedded builder.
    pub fn create_embedded(&self, field_name: &str, class: &str) -> EmbeddedBuilder<'_> {
        let mut mapping = PhpArray::new();
        mapping.insert("fieldName", Value::from(field_name));
        mapping.insert("class", Value::from(class));
        mapping.insert("columnPrefix", Value::Null);
        EmbeddedBuilder::new(self, mapping)
    }

    fn association(name: &str, target_entity: &str) -> PhpArray {
        let mut mapping = PhpArray::new();
        mapping.insert("fieldName", Value::from(name));
        mapping.insert("targetEntity", Value::from(target_entity));
        mapping
    }

    /// Adds a simple many to one association, optionally with the
    /// inversed by field.
    pub fn add_many_to_one(
        &self,
        name: &str,
        target_entity: &str,
        inversed_by: Option<&str>,
    ) -> Result<&Self, Thrown> {
        let mut builder = self.create_many_to_one(name, target_entity);
        if let Some(inversed_by) = inversed_by {
            builder = builder.inversed_by(inversed_by);
        }
        builder.build()
    }

    /// Creates a `ManyToOne` Association Builder.
    pub fn create_many_to_one(&self, name: &str, target_entity: &str) -> AssociationBuilder<'_> {
        AssociationBuilder::new(self, Self::association(name, target_entity), MANY_TO_ONE)
    }

    /// Creates a `OneToOne` Association Builder.
    pub fn create_one_to_one(&self, name: &str, target_entity: &str) -> AssociationBuilder<'_> {
        AssociationBuilder::new(self, Self::association(name, target_entity), ONE_TO_ONE)
    }

    /// Adds simple inverse one-to-one association.
    pub fn add_inverse_one_to_one(
        &self,
        name: &str,
        target_entity: &str,
        mapped_by: &str,
    ) -> Result<&Self, Thrown> {
        self.create_one_to_one(name, target_entity)
            .mapped_by(mapped_by)
            .build()
    }

    /// Adds simple owning one-to-one association.
    pub fn add_owning_one_to_one(
        &self,
        name: &str,
        target_entity: &str,
        inversed_by: Option<&str>,
    ) -> Result<&Self, Thrown> {
        let mut builder = self.create_one_to_one(name, target_entity);
        if let Some(inversed_by) = inversed_by {
            builder = builder.inversed_by(inversed_by);
        }
        builder.build()
    }

    /// Creates a `ManyToMany` Association Builder.
    pub fn create_many_to_many(
        &self,
        name: &str,
        target_entity: &str,
    ) -> ManyToManyAssociationBuilder<'_> {
        ManyToManyAssociationBuilder::new(self, Self::association(name, target_entity))
    }

    /// Adds a simple owning many to many association.
    pub fn add_owning_many_to_many(
        &self,
        name: &str,
        target_entity: &str,
        inversed_by: Option<&str>,
    ) -> Result<&Self, Thrown> {
        let mut builder = self.create_many_to_many(name, target_entity);
        if let Some(inversed_by) = inversed_by {
            builder = builder.inversed_by(inversed_by);
        }
        builder.build()
    }

    /// Adds a simple inverse many to many association.
    pub fn add_inverse_many_to_many(
        &self,
        name: &str,
        target_entity: &str,
        mapped_by: &str,
    ) -> Result<&Self, Thrown> {
        self.create_many_to_many(name, target_entity)
            .mapped_by(mapped_by)
            .build()
    }

    /// Creates a one to many association builder.
    pub fn create_one_to_many(
        &self,
        name: &str,
        target_entity: &str,
    ) -> OneToManyAssociationBuilder<'_> {
        OneToManyAssociationBuilder::new(self, Self::association(name, target_entity))
    }

    /// Adds simple `OneToMany` association.
    pub fn add_one_to_many(
        &self,
        name: &str,
        target_entity: &str,
        mapped_by: &str,
    ) -> Result<&Self, Thrown> {
        self.create_one_to_many(name, target_entity)
            .mapped_by(mapped_by)
            .build()
    }
}

/// `addEmbedded()`'s `string|false|null $columnPrefix`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnPrefixArgument<'a> {
    Prefix(&'a str),
    /// `false`: no prefix.
    False,
}

impl ColumnPrefixArgument<'_> {
    fn value(self) -> Value {
        match self {
            Self::Prefix(prefix) => Value::from(prefix),
            Self::False => Value::Bool(false),
        }
    }
}

/// `Doctrine\ORM\Mapping\Builder\FieldBuilder`.
#[derive(Debug)]
pub struct FieldBuilder<'a> {
    builder: &'a ClassMetadataBuilder,
    mapping: PhpArray,
    version: bool,
    generated_value: Option<String>,
    sequence_def: Option<PhpArray>,
    custom_id_generator: Option<String>,
}

impl<'a> FieldBuilder<'a> {
    pub fn new(builder: &'a ClassMetadataBuilder, mapping: PhpArray) -> Self {
        Self {
            builder,
            mapping,
            version: false,
            generated_value: None,
            sequence_def: None,
            custom_id_generator: None,
        }
    }

    fn set(mut self, key: &str, value: Value) -> Self {
        self.mapping.insert(key, value);
        self
    }

    /// Sets length.
    #[must_use]
    pub fn length(self, length: i64) -> Self {
        self.set("length", Value::Int(length))
    }

    /// Sets nullable (PHP's default: `true`).
    #[must_use]
    pub fn nullable(self, flag: bool) -> Self {
        self.set("nullable", Value::Bool(flag))
    }

    /// Sets Unique (PHP's default: `true`).
    #[must_use]
    pub fn unique(self, flag: bool) -> Self {
        self.set("unique", Value::Bool(flag))
    }

    /// Sets index (PHP's default: `true`).
    #[must_use]
    pub fn index(self, flag: bool) -> Self {
        self.set("index", Value::Bool(flag))
    }

    /// Sets column name.
    #[must_use]
    pub fn column_name(self, name: &str) -> Self {
        self.set("columnName", Value::from(name))
    }

    /// Sets Precision.
    #[must_use]
    pub fn precision(self, p: i64) -> Self {
        self.set("precision", Value::Int(p))
    }

    /// Sets insertable (PHP's default: `true`).
    #[must_use]
    pub fn insertable(self, flag: bool) -> Self {
        if flag {
            self
        } else {
            self.set("notInsertable", Value::Bool(true))
        }
    }

    /// Sets updatable (PHP's default: `true`).
    #[must_use]
    pub fn updatable(self, flag: bool) -> Self {
        if flag {
            self
        } else {
            self.set("notUpdatable", Value::Bool(true))
        }
    }

    /// Sets scale.
    #[must_use]
    pub fn scale(self, s: i64) -> Self {
        self.set("scale", Value::Int(s))
    }

    /// Sets field as primary key.
    #[must_use]
    pub fn make_primary_key(self) -> Self {
        self.set("id", Value::Bool(true))
    }

    /// Sets an option.
    #[must_use]
    pub fn option(mut self, name: &str, value: Value) -> Self {
        let key = Key::from("options");
        if !matches!(self.mapping.get(&key), Some(Value::Array(_))) {
            self.mapping
                .insert(key.clone(), Value::Array(PhpArray::new()));
        }
        if let Some(Value::Array(options)) = self.mapping.get_mut(&key) {
            options.insert(name, value);
        }
        self
    }

    /// Sets the id generator strategy (PHP's default: `'AUTO'`).
    #[must_use]
    pub fn generated_value(mut self, strategy: &str) -> Self {
        self.generated_value = Some(strategy.to_owned());
        self
    }

    /// Sets field versioned.
    #[must_use]
    pub fn is_version_field(mut self) -> Self {
        self.version = true;
        self
    }

    /// Sets Sequence Generator (PHP's defaults: `1`, `1`).
    #[must_use]
    pub fn set_sequence_generator(
        mut self,
        sequence_name: &str,
        allocation_size: i64,
        initial_value: i64,
    ) -> Self {
        let mut definition = PhpArray::new();
        definition.insert("sequenceName", Value::from(sequence_name));
        definition.insert("allocationSize", Value::Int(allocation_size));
        definition.insert("initialValue", Value::Int(initial_value));
        self.sequence_def = Some(definition);
        self
    }

    /// Sets column definition.
    #[must_use]
    pub fn column_definition(self, def: &str) -> Self {
        self.set("columnDefinition", Value::from(def))
    }

    /// Set the FQCN of the custom ID generator.
    /// This class must extend `Doctrine\ORM\Id\AbstractIdGenerator`.
    #[must_use]
    pub fn set_custom_id_generator(mut self, custom_id_generator: &str) -> Self {
        self.custom_id_generator = Some(custom_id_generator.to_owned());
        self
    }

    /// Finalizes this field and attach it to the `ClassMetadata`.
    pub fn build(mut self) -> Result<&'a ClassMetadataBuilder, Thrown> {
        let mut cm = self.builder.get_class_metadata().write();

        if let Some(generated_value) = self.generated_value.filter(|value| !value.is_empty()) {
            let name = format!("GENERATOR_TYPE_{generated_value}");
            let generator_type = cm::constant(&name).ok_or_else(|| {
                Thrown::new(
                    "Error",
                    format!("Undefined constant {}::{name}", cm::CLASS),
                    0,
                )
            })?;
            cm.set_id_generator_type(generator_type);
        }

        if self.version {
            cm.set_version_mapping(&mut self.mapping)?;
        }

        cm.map_field(self.mapping)?;
        if let Some(sequence_def) = self.sequence_def.filter(|def| !def.is_empty()) {
            cm.set_sequence_generator_definition(sequence_def)?;
        }

        if let Some(custom_id_generator) =
            self.custom_id_generator.filter(|class| !class.is_empty())
        {
            let mut definition = PhpArray::new();
            definition.insert("class", Value::from(custom_id_generator));
            cm.set_custom_generator_definition(definition);
        }

        Ok(self.builder)
    }
}

/// `Doctrine\ORM\Mapping\Builder\EmbeddedBuilder`.
#[derive(Debug)]
pub struct EmbeddedBuilder<'a> {
    builder: &'a ClassMetadataBuilder,
    mapping: PhpArray,
}

impl<'a> EmbeddedBuilder<'a> {
    pub fn new(builder: &'a ClassMetadataBuilder, mapping: PhpArray) -> Self {
        Self { builder, mapping }
    }

    /// Sets the column prefix for all of the embedded columns.
    #[must_use]
    pub fn set_column_prefix(mut self, column_prefix: &str) -> Self {
        self.mapping
            .insert("columnPrefix", Value::from(column_prefix));
        self
    }

    /// Finalizes this embeddable and attach it to the `ClassMetadata`.
    pub fn build(self) -> Result<&'a ClassMetadataBuilder, Thrown> {
        self.builder
            .get_class_metadata()
            .write()
            .map_embedded(self.mapping)?;

        Ok(self.builder)
    }
}

/// `Doctrine\ORM\Mapping\Builder\AssociationBuilder`.
#[derive(Debug)]
pub struct AssociationBuilder<'a> {
    builder: &'a ClassMetadataBuilder,
    mapping: PhpArray,
    join_columns: Vec<PhpArray>,
    r#type: i64,
}

impl<'a> AssociationBuilder<'a> {
    pub fn new(builder: &'a ClassMetadataBuilder, mapping: PhpArray, r#type: i64) -> Self {
        Self {
            builder,
            mapping,
            join_columns: Vec::new(),
            r#type,
        }
    }

    fn set(mut self, key: &str, value: Value) -> Self {
        self.mapping.insert(key, value);
        self
    }

    fn cascade(mut self, operation: &str) -> Self {
        let key = Key::from("cascade");
        if !matches!(self.mapping.get(&key), Some(Value::Array(_))) {
            self.mapping
                .insert(key.clone(), Value::Array(PhpArray::new()));
        }
        if let Some(Value::Array(cascade)) = self.mapping.get_mut(&key) {
            let _ = cascade.push(Value::from(operation));
        }
        self
    }

    fn is_id(&self) -> bool {
        self.mapping
            .get(&Key::from("id"))
            .is_some_and(Value::is_truthy)
    }

    #[must_use]
    pub fn mapped_by(self, field_name: &str) -> Self {
        self.set("mappedBy", Value::from(field_name))
    }

    #[must_use]
    pub fn inversed_by(self, field_name: &str) -> Self {
        self.set("inversedBy", Value::from(field_name))
    }

    #[must_use]
    pub fn cascade_all(self) -> Self {
        self.set("cascade", list(&["ALL"]))
    }

    #[must_use]
    pub fn cascade_persist(self) -> Self {
        self.cascade("persist")
    }

    #[must_use]
    pub fn cascade_remove(self) -> Self {
        self.cascade("remove")
    }

    #[must_use]
    pub fn cascade_detach(self) -> Self {
        self.cascade("detach")
    }

    #[must_use]
    pub fn cascade_refresh(self) -> Self {
        self.cascade("refresh")
    }

    #[must_use]
    pub fn fetch_extra_lazy(self) -> Self {
        self.set("fetch", Value::Int(cm::FETCH_EXTRA_LAZY))
    }

    #[must_use]
    pub fn fetch_eager(self) -> Self {
        self.set("fetch", Value::Int(cm::FETCH_EAGER))
    }

    #[must_use]
    pub fn fetch_lazy(self) -> Self {
        self.set("fetch", Value::Int(cm::FETCH_LAZY))
    }

    /// Add Join Columns (PHP's defaults: `true`, `false`, `null`,
    /// `null`).
    #[must_use]
    pub fn add_join_column(
        mut self,
        column_name: &str,
        referenced_column_name: &str,
        nullable: bool,
        unique: bool,
        on_delete: Option<&str>,
        column_def: Option<&str>,
    ) -> Self {
        let nullable = if self.is_id() {
            Value::Null
        } else {
            Value::Bool(nullable)
        };

        let mut join_column = PhpArray::new();
        join_column.insert("name", Value::from(column_name));
        join_column.insert("referencedColumnName", Value::from(referenced_column_name));
        join_column.insert("nullable", nullable);
        join_column.insert("unique", Value::Bool(unique));
        join_column.insert("onDelete", opt_str(on_delete));
        join_column.insert("columnDefinition", opt_str(column_def));
        self.join_columns.push(join_column);

        self
    }

    /// Sets field as primary key.
    #[must_use]
    pub fn make_primary_key(mut self) -> Self {
        self.mapping.insert("id", Value::Bool(true));
        for join_column in &mut self.join_columns {
            join_column.insert("nullable", Value::Null);
        }

        self
    }

    /// Removes orphan entities when detached from their parent.
    #[must_use]
    pub fn orphan_removal(self) -> Self {
        self.set("orphanRemoval", Value::Bool(true))
    }

    fn mapping_with_join_columns(&self) -> PhpArray {
        let mut mapping = self.mapping.clone();
        if !self.join_columns.is_empty() {
            mapping.insert("joinColumns", rows(&self.join_columns));
        }
        mapping
    }

    pub fn build(self) -> Result<&'a ClassMetadataBuilder, Thrown> {
        let mapping = self.mapping_with_join_columns();

        let mut cm = self.builder.get_class_metadata().write();
        if self.r#type == MANY_TO_ONE {
            cm.map_many_to_one(mapping)?;
        } else if self.r#type == ONE_TO_ONE {
            cm.map_one_to_one(mapping)?;
        } else {
            return Err(Thrown::new(
                "InvalidArgumentException",
                "Type should be a ToOne Association here",
                0,
            ));
        }

        Ok(self.builder)
    }
}

/// The `AssociationBuilder` methods a subclass inherits.
macro_rules! inherited_association_methods {
    ($($method:ident),*) => {
        $(
            #[must_use]
            pub fn $method(mut self) -> Self {
                self.inner = self.inner.$method();
                self
            }
        )*
    };
}

/// `Doctrine\ORM\Mapping\Builder\OneToManyAssociationBuilder`.
#[derive(Debug)]
pub struct OneToManyAssociationBuilder<'a> {
    inner: AssociationBuilder<'a>,
}

impl<'a> OneToManyAssociationBuilder<'a> {
    pub fn new(builder: &'a ClassMetadataBuilder, mapping: PhpArray) -> Self {
        Self::with_type(builder, mapping, ONE_TO_MANY)
    }

    fn with_type(builder: &'a ClassMetadataBuilder, mapping: PhpArray, r#type: i64) -> Self {
        Self {
            inner: AssociationBuilder::new(builder, mapping, r#type),
        }
    }

    inherited_association_methods!(
        cascade_all,
        cascade_persist,
        cascade_remove,
        cascade_detach,
        cascade_refresh,
        fetch_extra_lazy,
        fetch_eager,
        fetch_lazy,
        make_primary_key,
        orphan_removal
    );

    #[must_use]
    pub fn mapped_by(mut self, field_name: &str) -> Self {
        self.inner = self.inner.mapped_by(field_name);
        self
    }

    #[must_use]
    pub fn inversed_by(mut self, field_name: &str) -> Self {
        self.inner = self.inner.inversed_by(field_name);
        self
    }

    /// `addJoinColumn()`, as `AssociationBuilder` has it.
    #[must_use]
    pub fn add_join_column(
        mut self,
        column_name: &str,
        referenced_column_name: &str,
        nullable: bool,
        unique: bool,
        on_delete: Option<&str>,
        column_def: Option<&str>,
    ) -> Self {
        self.inner = self.inner.add_join_column(
            column_name,
            referenced_column_name,
            nullable,
            unique,
            on_delete,
            column_def,
        );
        self
    }

    /// `setOrderBy()`: field name => `SortDirection` case (or, deprecated,
    /// `'ASC'`/`'DESC'`).
    #[must_use]
    pub fn set_order_by(mut self, field_names: PhpArray) -> Self {
        self.inner = self.inner.set("orderBy", Value::Array(field_names));
        self
    }

    #[must_use]
    pub fn set_index_by(mut self, field_name: &str) -> Self {
        self.inner = self.inner.set("indexBy", Value::from(field_name));
        self
    }

    pub fn build(self) -> Result<&'a ClassMetadataBuilder, Thrown> {
        let mapping = self.inner.mapping_with_join_columns();

        self.inner
            .builder
            .get_class_metadata()
            .write()
            .map_one_to_many(mapping)?;

        Ok(self.inner.builder)
    }
}

/// `Doctrine\ORM\Mapping\Builder\ManyToManyAssociationBuilder`.
#[derive(Debug)]
pub struct ManyToManyAssociationBuilder<'a> {
    inner: OneToManyAssociationBuilder<'a>,
    join_table_name: Option<String>,
    inverse_join_columns: Vec<PhpArray>,
}

/// A join column of the join table: `ManyToManyAssociationBuilder`
/// leaves `nullable` out.
fn join_table_column(
    column_name: &str,
    referenced_column_name: &str,
    unique: bool,
    on_delete: Option<&str>,
    column_def: Option<&str>,
) -> PhpArray {
    let mut join_column = PhpArray::new();
    join_column.insert("name", Value::from(column_name));
    join_column.insert("referencedColumnName", Value::from(referenced_column_name));
    join_column.insert("unique", Value::Bool(unique));
    join_column.insert("onDelete", opt_str(on_delete));
    join_column.insert("columnDefinition", opt_str(column_def));
    join_column
}

impl<'a> ManyToManyAssociationBuilder<'a> {
    pub fn new(builder: &'a ClassMetadataBuilder, mapping: PhpArray) -> Self {
        Self {
            inner: OneToManyAssociationBuilder::with_type(builder, mapping, MANY_TO_MANY),
            join_table_name: None,
            inverse_join_columns: Vec::new(),
        }
    }

    inherited_association_methods!(
        cascade_all,
        cascade_persist,
        cascade_remove,
        cascade_detach,
        cascade_refresh,
        fetch_extra_lazy,
        fetch_eager,
        fetch_lazy,
        make_primary_key,
        orphan_removal
    );

    #[must_use]
    pub fn mapped_by(mut self, field_name: &str) -> Self {
        self.inner = self.inner.mapped_by(field_name);
        self
    }

    #[must_use]
    pub fn inversed_by(mut self, field_name: &str) -> Self {
        self.inner = self.inner.inversed_by(field_name);
        self
    }

    #[must_use]
    pub fn set_order_by(mut self, field_names: PhpArray) -> Self {
        self.inner = self.inner.set_order_by(field_names);
        self
    }

    #[must_use]
    pub fn set_index_by(mut self, field_name: &str) -> Self {
        self.inner = self.inner.set_index_by(field_name);
        self
    }

    #[must_use]
    pub fn set_join_table(mut self, name: &str) -> Self {
        self.join_table_name = Some(name.to_owned());
        self
    }

    /// Adds Join Columns (PHP's defaults: `true`, `false`, `null`,
    /// `null`); `$nullable` is not used.
    #[must_use]
    pub fn add_join_column(
        mut self,
        column_name: &str,
        referenced_column_name: &str,
        _nullable: bool,
        unique: bool,
        on_delete: Option<&str>,
        column_def: Option<&str>,
    ) -> Self {
        self.inner.inner.join_columns.push(join_table_column(
            column_name,
            referenced_column_name,
            unique,
            on_delete,
            column_def,
        ));
        self
    }

    /// Adds Inverse Join Columns (PHP's defaults: `true`, `false`,
    /// `null`, `null`); `$nullable` is not used.
    #[must_use]
    pub fn add_inverse_join_column(
        mut self,
        column_name: &str,
        referenced_column_name: &str,
        _nullable: bool,
        unique: bool,
        on_delete: Option<&str>,
        column_def: Option<&str>,
    ) -> Self {
        self.inverse_join_columns.push(join_table_column(
            column_name,
            referenced_column_name,
            unique,
            on_delete,
            column_def,
        ));
        self
    }

    pub fn build(self) -> Result<&'a ClassMetadataBuilder, Thrown> {
        let association = self.inner.inner;
        let mut mapping = association.mapping.clone();
        let mut join_table = PhpArray::new();
        if !association.join_columns.is_empty() {
            join_table.insert("joinColumns", rows(&association.join_columns));
        }

        if !self.inverse_join_columns.is_empty() {
            join_table.insert("inverseJoinColumns", rows(&self.inverse_join_columns));
        }

        if let Some(join_table_name) = self.join_table_name.filter(|name| !name.is_empty()) {
            join_table.insert("name", Value::from(join_table_name));
        }
        mapping.insert("joinTable", Value::Array(join_table));

        association
            .builder
            .get_class_metadata()
            .write()
            .map_many_to_many(mapping)?;

        Ok(association.builder)
    }
}

/// `Doctrine\ORM\Mapping\Builder\EntityListenerBuilder`.
#[derive(Debug, Clone, Copy)]
pub struct EntityListenerBuilder;

impl EntityListenerBuilder {
    /// Lookup the entity class to find methods that match to event
    /// lifecycle names.
    pub fn bind_entity_listener(
        metadata: &mut ClassMetadataInfo,
        class_name: &str,
    ) -> Result<(), Thrown> {
        attribute_driver::bind_entity_listener(metadata, class_name)
    }
}
