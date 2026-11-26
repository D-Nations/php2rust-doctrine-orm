//! An `EntityRepository` serves as a repository for entities with generic
//! as well as business specific methods for retrieving entities.
//!
//! This class is designed for inheritance and users can subclass this
//! class to write their own repositories with business-specific methods
//! to locate entities; here a subclass's methods are not ported, and the
//! repository only reports its class name.
//!
//! PHP's `__call()` magic finders are [`EntityRepository::call`].
//!
//! Upstream: doctrine/orm src/EntityRepository.php

use std::fmt;
use std::sync::{Arc, Weak};

use php_value::{Object, PhpArray, Thrown, Value};
use php2rust_doctrine_collections::Criteria;
use php2rust_doctrine_inflector::InflectorFactory;

use crate::entity_manager::EntityManager;
use crate::exception;
use crate::lazy_criteria_collection::LazyCriteriaCollection;
use crate::mapping::ClassMetadata;
use crate::unit_of_work::Hints;

/// `Doctrine\ORM\EntityRepository`.
pub struct EntityRepository {
    entity_name: String,
    em: Weak<EntityManager>,
    class: Arc<ClassMetadata>,
    repository_class: String,
}

impl fmt::Debug for EntityRepository {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EntityRepository")
            .field("entity_name", &self.entity_name)
            .field("repository_class", &self.repository_class)
            .finish_non_exhaustive()
    }
}

impl EntityRepository {
    /// Initializes a new `EntityRepository`.
    pub fn new(em: &Arc<EntityManager>, class: Arc<ClassMetadata>) -> Self {
        Self::with_class(
            em,
            class,
            crate::configuration::ENTITY_REPOSITORY_CLASS.to_owned(),
        )
    }

    /// A repository of the given repository class.
    pub fn with_class(
        em: &Arc<EntityManager>,
        class: Arc<ClassMetadata>,
        repository_class: String,
    ) -> Self {
        let entity_name = class.read().name.clone();
        Self {
            entity_name,
            em: Arc::downgrade(em),
            class,
            repository_class,
        }
    }

    /// The class of the repository (`get_class($repository)`).
    pub fn get_repository_class(&self) -> &str {
        &self.repository_class
    }

    /// Whether its entity manager is still alive.
    pub(crate) fn is_bound(&self) -> bool {
        self.em.strong_count() > 0
    }

    fn em(&self) -> Result<Arc<EntityManager>, Thrown> {
        self.em
            .upgrade()
            .ok_or_else(|| Thrown::new("Error", "The entity manager is gone", 0))
    }

    /// Finds an entity by its primary key / identifier.
    pub async fn find(&self, id: Value) -> Result<Option<Object>, Thrown> {
        self.em()?.find(&self.entity_name, id).await
    }

    /// `find($id, $lockMode, $lockVersion)`.
    pub async fn find_with_lock(
        &self,
        id: Value,
        lock_mode: php2rust_doctrine_dbal::LockMode,
        lock_version: Option<Value>,
    ) -> Result<Option<Object>, Thrown> {
        self.em()?
            .find_with_lock(&self.entity_name, id, lock_mode, lock_version)
            .await
    }

    /// Finds all entities in the repository.
    pub async fn find_all(&self) -> Result<Vec<Object>, Thrown> {
        self.find_by(&PhpArray::new(), None, None, None).await
    }

    /// Finds entities by a set of criteria.
    pub async fn find_by(
        &self,
        criteria: &PhpArray,
        order_by: Option<&PhpArray>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<Object>, Thrown> {
        let persister = self
            .em()?
            .get_unit_of_work()
            .get_entity_persister(&self.entity_name)?;
        persister.load_all(criteria, order_by, limit, offset).await
    }

    /// Finds a single entity by a set of criteria.
    pub async fn find_one_by(
        &self,
        criteria: &PhpArray,
        order_by: Option<&PhpArray>,
    ) -> Result<Option<Object>, Thrown> {
        let persister = self
            .em()?
            .get_unit_of_work()
            .get_entity_persister(&self.entity_name)?;
        persister
            .load(criteria, None, None, Hints::default(), Some(1), order_by)
            .await
    }

    /// Counts entities by a set of criteria.
    pub async fn count(&self, criteria: &PhpArray) -> Result<i64, Thrown> {
        self.em()?
            .get_unit_of_work()
            .get_entity_persister(&self.entity_name)?
            .count(criteria)
            .await
    }

    /// `__call()`: adds support for magic method calls (`findByX`,
    /// `findOneByX`, `countByX`). `findBy` gives a list, `findOneBy` the
    /// entity or `null`, `countBy` an int.
    pub async fn call(&self, method: &str, arguments: Vec<Value>) -> Result<Value, Thrown> {
        if let Some(by) = method.strip_prefix("findBy") {
            return self.resolve_magic_call("findBy", by, arguments).await;
        }
        if let Some(by) = method.strip_prefix("findOneBy") {
            return self.resolve_magic_call("findOneBy", by, arguments).await;
        }
        if let Some(by) = method.strip_prefix("countBy") {
            return self.resolve_magic_call("count", by, arguments).await;
        }
        Err(Thrown::new(
            "BadMethodCallException",
            format!(
                "Undefined method \"{method}\". The method name must start with either findBy, findOneBy or countBy!"
            ),
            0,
        ))
    }

    fn get_entity_name(&self) -> &str {
        &self.entity_name
    }

    pub fn get_class_name(&self) -> &str {
        self.get_entity_name()
    }

    pub fn get_entity_manager(&self) -> Result<Arc<EntityManager>, Thrown> {
        self.em()
    }

    pub fn get_class_metadata(&self) -> &Arc<ClassMetadata> {
        &self.class
    }

    /// Resolves a magic method call to the proper existent method at
    /// `EntityRepository`.
    async fn resolve_magic_call(
        &self,
        method: &str,
        by: &str,
        arguments: Vec<Value>,
    ) -> Result<Value, Thrown> {
        let mut arguments = arguments.into_iter();
        let Some(first) = arguments.next() else {
            return Err(exception::on_missing_parameter(&format!("{method}{by}")));
        };

        let inflector = InflectorFactory::create()
            .and_then(|factory| factory.build())
            .map_err(|e| Thrown::new("Error", e.to_string(), 0))?;
        let classified = inflector
            .classify(by.as_bytes())
            .to_string_lossy()
            .into_owned();
        let field_name = lcfirst(&classified);

        let known = {
            let info = self.class.read();
            info.has_field(&field_name) || info.has_association(&field_name)
        };
        if !known {
            return Err(exception::because_field_not_found_in(
                &self.entity_name,
                &field_name,
                &format!("{method}{by}"),
            ));
        }

        let mut criteria = PhpArray::new();
        criteria.insert(field_name.as_str(), first);
        let rest: Vec<Value> = arguments.collect();
        let order_by = match rest.first() {
            Some(Value::Array(order_by)) => Some(order_by.clone()),
            _ => None,
        };
        let int_at = |i: usize| match rest.get(i) {
            Some(Value::Int(n)) => Some(*n),
            _ => None,
        };
        match method {
            "findBy" => {
                let found = self
                    .find_by(&criteria, order_by.as_ref(), int_at(1), int_at(2))
                    .await?;
                Ok(Value::Array(PhpArray::list(
                    found.into_iter().map(Value::Object),
                )))
            }
            "findOneBy" => Ok(self
                .find_one_by(&criteria, order_by.as_ref())
                .await?
                .map_or(Value::Null, Value::Object)),
            _ => Ok(Value::Int(self.count(&criteria).await?)),
        }
    }
}

fn lcfirst(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_ascii_lowercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

/// The query builder and `matching()` of the repository.
impl EntityRepository {
    /// Creates a new result set mapping builder for this entity.
    ///
    /// The column naming strategy is "INCREMENT".
    pub fn create_result_set_mapping_builder(
        &self,
        alias: &str,
    ) -> Result<crate::query::ResultSetMappingBuilder, Thrown> {
        let mut rsm = crate::query::ResultSetMappingBuilder::new(
            &self.em()?,
            Some(crate::query::ResultSetMappingBuilder::COLUMN_RENAMING_INCREMENT),
        );
        rsm.add_root_entity_from_class_metadata(
            &self.entity_name,
            alias,
            &crate::mapping::ordered_map::OrderedMap::new(),
            None,
        )?;
        Ok(rsm)
    }

    /// Select all elements from a selectable that match the expression and
    /// return a new collection containing these elements.
    ///
    /// The collection keeps a copy of the criteria (PHP keeps the object,
    /// so a later change to it reaches the collection).
    pub fn matching(&self, criteria: &Criteria) -> Result<Arc<LazyCriteriaCollection>, Thrown> {
        let persister = self
            .em()?
            .get_unit_of_work()
            .get_entity_persister(&self.entity_name)?;
        Ok(LazyCriteriaCollection::new(persister, criteria.clone()))
    }

    /// Creates a new `QueryBuilder` instance that is prepopulated for this entity name.
    pub fn create_query_builder(
        &self,
        alias: &str,
        index_by: Option<&str>,
    ) -> Result<crate::query_builder::QueryBuilder, Thrown> {
        let mut query_builder = self.em()?.create_query_builder();
        query_builder
            .select(vec![Value::from(alias)])?
            .from(&self.entity_name, alias, index_by)?;
        Ok(query_builder)
    }
}
