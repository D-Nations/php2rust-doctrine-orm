//! The identifier generators `ClassMetadata::$idGenerator` holds. The ORM's
//! `Doctrine\ORM\Id` classes generate values through the entity manager,
//! which this port does not have yet: here they are the generator's class
//! and its settings, as `ClassMetadataFactory` configures them.
//!
//! `AssignedGenerator` generates through the unit of work;
//! `SequenceGenerator` allocates its values from the connection. Its
//! `Serializable` methods are not ported.
//!
//! Upstream: doctrine/orm src/Id/AssignedGenerator.php src/Id/SequenceGenerator.php

/// A registered custom generator's `generateId()` implementation.
pub type CustomIdGenerator = fn(
    &crate::entity_manager::EntityManager,
    Option<&php_value::Object>,
) -> Result<php_value::Value, php_value::Thrown>;

fn custom_generators()
-> &'static std::sync::RwLock<std::collections::HashMap<String, CustomIdGenerator>> {
    static GENERATORS: std::sync::OnceLock<
        std::sync::RwLock<std::collections::HashMap<String, CustomIdGenerator>>,
    > = std::sync::OnceLock::new();
    GENERATORS.get_or_init(Default::default)
}

/// Register a custom pre-insert generator by its PHP class name.
pub fn register_custom_generator(class_name: &str, generator: CustomIdGenerator) {
    custom_generators()
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .insert(class_name.trim_start_matches('\\').to_owned(), generator);
}

pub(crate) fn custom_generator(class_name: &str) -> Option<CustomIdGenerator> {
    custom_generators()
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get(class_name.trim_start_matches('\\'))
        .copied()
}

/// `Doctrine\ORM\Id\AbstractIdGenerator` and its subclasses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdGenerator {
    /// `Doctrine\ORM\Id\AssignedGenerator`.
    Assigned,
    /// `Doctrine\ORM\Id\IdentityGenerator`.
    Identity,
    /// `Doctrine\ORM\Id\BigIntegerIdentityGenerator`.
    BigIntegerIdentity,
    /// `Doctrine\ORM\Id\SequenceGenerator`.
    Sequence {
        sequence_name: String,
        allocation_size: i64,
    },
    /// A custom generator class (`#[CustomIdGenerator]`).
    Custom { class: String },
}

impl IdGenerator {
    /// The generator's class name.
    pub fn class_name(&self) -> &str {
        match self {
            Self::Assigned => "Doctrine\\ORM\\Id\\AssignedGenerator",
            Self::Identity => "Doctrine\\ORM\\Id\\IdentityGenerator",
            Self::BigIntegerIdentity => "Doctrine\\ORM\\Id\\BigIntegerIdentityGenerator",
            Self::Sequence { .. } => "Doctrine\\ORM\\Id\\SequenceGenerator",
            Self::Custom { class } => class,
        }
    }

    /// `isPostInsertGenerator()`.
    pub fn is_post_insert_generator(&self) -> bool {
        matches!(self, Self::Identity | Self::BigIntegerIdentity)
    }
}

/// Special generator for application-assigned identifiers (doesn't
/// really generate anything).
#[derive(Debug, Clone, Copy, Default)]
pub struct AssignedGenerator;

impl AssignedGenerator {
    pub fn new() -> Self {
        Self
    }

    /// Returns the identifier assigned to the given entity.
    pub fn generate_id(
        &self,
        em: &crate::entity_manager::EntityManager,
        entity: &php_value::Object,
    ) -> Result<php_value::PhpArray, php_value::Thrown> {
        em.get_unit_of_work().generate_assigned_id(em, entity)
    }
}

/// Represents an ID generator that uses a database sequence.
#[derive(Debug)]
pub struct SequenceGenerator {
    sequence_name: String,
    allocation_size: i64,
    /// `$nextValue` and `$maxValue`.
    values: std::sync::Mutex<(i64, Option<i64>)>,
}

impl SequenceGenerator {
    /// Initializes a new sequence generator.
    pub fn new(sequence_name: impl Into<String>, allocation_size: i64) -> Self {
        Self {
            sequence_name: sequence_name.into(),
            allocation_size,
            values: std::sync::Mutex::new((0, None)),
        }
    }

    fn values(&self) -> std::sync::MutexGuard<'_, (i64, Option<i64>)> {
        self.values
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// `generateId($em, $entity)`.
    pub async fn generate_id(
        &self,
        em: &crate::entity_manager::EntityManager,
        _entity: Option<&php_value::Object>,
    ) -> Result<i64, php_value::Thrown> {
        let (next_value, max_value) = *self.values();
        if max_value.is_none_or(|max_value| next_value == max_value) {
            // Allocate new values
            let connection = em.get_connection();
            let sql = connection
                .get_database_platform()
                .await
                .map_err(|e| crate::exception::from_dbal(&e))?
                .get_sequence_next_val_sql(&self.sequence_name)
                .map_err(|e| crate::exception::from_dbal(&e))?;

            let fetched = connection
                .fetch_one(
                    &sql,
                    &php_value::PhpArray::new(),
                    &php2rust_doctrine_dbal::ParameterTypes::new(),
                )
                .await
                .map_err(|e| crate::exception::from_dbal(&e))?
                .unwrap_or(php_value::Value::Bool(false));
            let next_value = php_value::ops::to_int(&fetched);
            *self.values() = (next_value, Some(next_value + self.allocation_size));
        }

        let mut values = self.values();
        let id = values.0;
        values.0 += 1;
        Ok(id)
    }

    /// Gets the maximum value of the currently allocated bag of values.
    pub fn get_current_max_value(&self) -> Option<i64> {
        self.values().1
    }

    /// Gets the next value that will be returned by `generate()`.
    pub fn get_next_value(&self) -> i64 {
        self.values().0
    }
}
