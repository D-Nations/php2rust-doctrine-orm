//! What the DQL parser and the SQL walkers need from the `Query` and its
//! `EntityManager`: PHP passes the `Query` object, here the parser takes a
//! [`DqlQuery`] and reaches the entity manager through
//! [`DqlQuery::get_entity_manager`], a [`QueryContext`]. The query
//! components (`$queryComponents`, an array of arrays) are
//! [`QueryComponent`]s, the query hints [`Hint`]s.
//!
//! Upstream: none

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use php_value::{Thrown, Value};
use php2rust_doctrine_dbal::LockMode;
use php2rust_doctrine_dbal::platforms::AbstractPlatform;

use crate::mapping::ordered_map::OrderedMap;
use crate::mapping::{AssociationMapping, ClassMetadata, ClassMetadataInfo, QuoteStrategy};
use crate::query::Parameter;
use crate::query::ast::{FunctionNode, Node};
use crate::query::parser_result::ParserResult;
use crate::query::sql_walker::SqlWalker;
use crate::query::token_type::TokenType;
use crate::query::tree_walker::TreeWalker;

/// A DQL token (`Doctrine\Common\Lexer\Token<TokenType, string>`).
pub type DqlToken = php2rust_doctrine_lexer::Token<TokenType, php_value::PhpString>;

/// `Query::HINT_REFRESH`.
pub const HINT_REFRESH: &str = "doctrine.refresh";
/// `Query::HINT_FORCE_PARTIAL_LOAD`.
pub const HINT_FORCE_PARTIAL_LOAD: &str = "doctrine.forcePartialLoad";
/// `Query::HINT_INCLUDE_META_COLUMNS`.
pub const HINT_INCLUDE_META_COLUMNS: &str = "doctrine.includeMetaColumns";
/// `Query::HINT_CUSTOM_TREE_WALKERS`.
pub const HINT_CUSTOM_TREE_WALKERS: &str = "doctrine.customTreeWalkers";
/// `Query::HINT_CUSTOM_OUTPUT_WALKER`.
pub const HINT_CUSTOM_OUTPUT_WALKER: &str = "doctrine.customOutputWalker";
/// `Query::HINT_INTERNAL_ITERATION`.
pub const HINT_INTERNAL_ITERATION: &str = "doctrine.internal.iteration";
/// `Query::HINT_LOCK_MODE`.
pub const HINT_LOCK_MODE: &str = "doctrine.lockMode";
/// `SqlWalker::HINT_DISTINCT`.
pub const HINT_DISTINCT: &str = "doctrine.distinct";
/// `SqlWalker::HINT_PARTIAL`.
pub const HINT_PARTIAL: &str = "doctrine.partial";
/// `AbstractQuery::HYDRATE_OBJECT`.
pub const HYDRATE_OBJECT: i64 = 1;
/// `AbstractQuery::HYDRATE_ARRAY`.
pub const HYDRATE_ARRAY: i64 = 2;
/// `AbstractQuery::HYDRATE_SCALAR`.
pub const HYDRATE_SCALAR: i64 = 3;
/// `AbstractQuery::HYDRATE_SINGLE_SCALAR`.
pub const HYDRATE_SINGLE_SCALAR: i64 = 4;
/// `AbstractQuery::HYDRATE_SIMPLEOBJECT`.
pub const HYDRATE_SIMPLEOBJECT: i64 = 5;
/// `AbstractQuery::HYDRATE_SCALAR_COLUMN`.
pub const HYDRATE_SCALAR_COLUMN: i64 = 6;
/// `Query::HINT_CACHE_ENABLED`.
pub const HINT_CACHE_ENABLED: &str = "doctrine.cache.enabled";
/// `Query::HINT_CACHE_EVICT`.
pub const HINT_CACHE_EVICT: &str = "doctrine.cache.evict";
/// `Query::HINT_REFRESH_ENTITY`.
pub const HINT_REFRESH_ENTITY: &str = "doctrine.refresh.entity";
/// `Query::HINT_READ_ONLY`.
pub const HINT_READ_ONLY: &str = "doctrine.readOnly";
/// `LimitSubqueryWalker::FORCE_DBAL_TYPE_CONVERSION`.
pub const FORCE_DBAL_TYPE_CONVERSION: &str = "doctrine_paginator.scalar_result.force_dbal_type_conversion";

/// The query components: `$queryComponents`, keyed by DQL alias.
pub type QueryComponents = OrderedMap<QueryComponent>;

/// A query component: an entry of `$queryComponents`, the array PHP builds
/// for an identification variable (`metadata`, `parent`, `relation`,
/// `map`, `nestingLevel`, `token`) or a result variable
/// (`resultVariable`, `nestingLevel`, `token`).
#[derive(Debug, Clone, Default)]
pub struct QueryComponent {
    /// `'metadata'`.
    pub metadata: Option<Arc<ClassMetadata>>,
    /// `'parent'`: the parent DQL alias of a join.
    pub parent: Option<String>,
    /// `'relation'`: the association of a join.
    pub relation: Option<AssociationMapping>,
    /// `'map'`: the INDEX BY field.
    pub map: Option<String>,
    /// `'resultVariable'`: the selected expression.
    pub result_variable: Option<Node>,
    /// `'resultvariable'` (lower case, as `SimpleSelectExpression()` sets
    /// it): `isset($qComp['resultVariable'])` does not see it.
    pub lowercase_result_variable: Option<Node>,
    /// `'nestingLevel'`.
    pub nesting_level: i64,
    /// `'token'`.
    pub token: Option<DqlToken>,
    /// Whether the array has all of the keys `metadata`, `parent`,
    /// `relation`, `map`, `nestingLevel` and `token` (an identification
    /// variable's component), which `setQueryComponent()` requires.
    pub has_entity_keys: bool,
}

impl QueryComponent {
    /// The component of an identification variable.
    pub fn entity(
        metadata: Arc<ClassMetadata>,
        parent: Option<String>,
        relation: Option<AssociationMapping>,
        nesting_level: i64,
        token: Option<DqlToken>,
    ) -> Self {
        Self {
            metadata: Some(metadata),
            parent,
            relation,
            map: None,
            result_variable: None,
            lowercase_result_variable: None,
            nesting_level,
            token,
            has_entity_keys: true,
        }
    }

    /// The component of a result variable.
    pub fn result(result_variable: Node, nesting_level: i64, token: Option<DqlToken>) -> Self {
        Self {
            result_variable: Some(result_variable),
            nesting_level,
            token,
            ..Self::default()
        }
    }
}

/// `isset($queryComponents[$alias]['metadata'])`.
pub(crate) fn has_metadata(components: &QueryComponents, alias: &str) -> bool {
    components.get(alias).is_some_and(|c| c.metadata.is_some())
}

/// `isset($queryComponents[$alias]['resultVariable'])`.
pub(crate) fn has_result_variable(components: &QueryComponents, alias: &str) -> bool {
    components
        .get(alias)
        .is_some_and(|c| c.result_variable.is_some())
}

/// `$queryComponents[$alias]['metadata'] ?? throw new LogicException(...)`.
pub(crate) fn metadata_for_dql_alias(
    components: &QueryComponents,
    dql_alias: &str,
) -> Result<Arc<ClassMetadata>, Thrown> {
    components
        .get(dql_alias)
        .and_then(|c| c.metadata.clone())
        .ok_or_else(|| {
            crate::query::query_exception::logic(format!("No metadata for DQL alias: {dql_alias}"))
        })
}

/// A function node class (`class-string<FunctionNode>` or a callable
/// `fn(string $name): FunctionNode` of the configuration): called with
/// the function's name.
pub type FunctionFactory = Arc<dyn Fn(&str) -> Box<dyn FunctionNode> + Send + Sync>;

/// `SQLFilter::addFilterConstraint()`: the filters of the entity manager.
pub trait SqlFilterConstraint {
    /// Gets the SQL query part to add to a query.
    fn add_filter_constraint(
        &self,
        target_entity: &ClassMetadataInfo,
        target_table_alias: &str,
    ) -> Result<String, Thrown>;
}

/// The part of `EntityManagerInterface` (with its `Connection`,
/// `Configuration` and metadata factory) that the parser and the walkers
/// use.
pub trait QueryContext {
    /// `getClassMetadata($className)`.
    fn get_class_metadata(&self, class_name: &str) -> Result<Arc<ClassMetadata>, Thrown>;

    /// `getMetadataFactory()->getOwningSide($assoc)`.
    fn get_owning_side(&self, assoc: &AssociationMapping) -> Result<AssociationMapping, Thrown>;

    /// `getConnection()->getDatabasePlatform()`.
    fn get_database_platform(&self) -> &dyn AbstractPlatform;

    /// `getConnection()->quote($value)`.
    fn quote(&self, value: &str) -> Result<String, Thrown>;

    /// `getConfiguration()->getQuoteStrategy()`.
    fn get_quote_strategy(&self) -> Arc<dyn QuoteStrategy>;

    /// `getConfiguration()->getCustomStringFunction($name)` (the name is
    /// case-insensitive).
    fn get_custom_string_function(&self, name: &str) -> Option<FunctionFactory> {
        let _ = name;
        None
    }

    /// `getConfiguration()->getCustomNumericFunction($name)`.
    fn get_custom_numeric_function(&self, name: &str) -> Option<FunctionFactory> {
        let _ = name;
        None
    }

    /// `getConfiguration()->getCustomDatetimeFunction($name)`.
    fn get_custom_datetime_function(&self, name: &str) -> Option<FunctionFactory> {
        let _ = name;
        None
    }

    /// `hasFilters()`.
    fn has_filters(&self) -> bool {
        false
    }

    /// `getFilters()->getEnabledFilters()`.
    fn get_enabled_filters(&self) -> Vec<Arc<dyn SqlFilterConstraint>> {
        Vec::new()
    }
}

/// `new $walkerClass($query, $parserResult, $queryComponents)` for a tree
/// walker class.
pub type TreeWalkerConstructor = for<'a> fn(
    &'a dyn DqlQuery,
    Rc<RefCell<ParserResult>>,
    QueryComponents,
) -> Box<dyn TreeWalker<'a> + 'a>;

/// `new $outputWalkerClass($query, $parserResult, $queryComponents)` for
/// an output walker class.
pub type OutputWalkerConstructor = for<'a> fn(
    &'a dyn DqlQuery,
    Rc<RefCell<ParserResult>>,
    QueryComponents,
) -> Box<dyn SqlWalker<'a> + 'a>;

/// A tree walker class: `class-string<TreeWalker>`, constructed with the
/// query, the parser result and the query components.
#[derive(Clone, Copy)]
pub struct TreeWalkerClass {
    /// The class name.
    pub name: &'static str,
    /// `new $walkerClass($query, $parserResult, $queryComponents)`.
    pub construct: TreeWalkerConstructor,
}

impl std::fmt::Debug for TreeWalkerClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name)
    }
}

/// An output walker class: `class-string<SqlWalker>`.
#[derive(Clone, Copy)]
pub struct OutputWalkerClass {
    /// The class name.
    pub name: &'static str,
    /// `new $outputWalkerClass($query, $parserResult, $queryComponents)`.
    pub construct: OutputWalkerConstructor,
}

impl std::fmt::Debug for OutputWalkerClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name)
    }
}

/// A query hint value.
#[derive(Debug, Clone)]
pub enum Hint {
    /// A PHP value (`false` for a hint that is not set).
    Value(Value),
    /// A `LockMode` (`Query::HINT_LOCK_MODE`).
    LockMode(LockMode),
    /// The tree walker classes (`Query::HINT_CUSTOM_TREE_WALKERS`).
    TreeWalkers(Vec<TreeWalkerClass>),
    /// The output walker class (`Query::HINT_CUSTOM_OUTPUT_WALKER`).
    OutputWalker(OutputWalkerClass),
}

impl Hint {
    /// `(bool) $hint`.
    pub fn is_truthy(&self) -> bool {
        match self {
            Self::Value(value) => value.is_truthy(),
            Self::LockMode(_) | Self::OutputWalker(_) => true,
            Self::TreeWalkers(walkers) => !walkers.is_empty(),
        }
    }

    /// `$hint === true`.
    pub fn is_true(&self) -> bool {
        matches!(self, Self::Value(Value::Bool(true)))
    }

    /// `$hint !== false`.
    pub fn is_set(&self) -> bool {
        !matches!(self, Self::Value(Value::Bool(false)))
    }
}

/// The part of `Doctrine\ORM\Query` that the parser and the walkers use.
pub trait DqlQuery {
    /// `getEntityManager()`.
    fn get_entity_manager(&self) -> &dyn QueryContext;

    /// `getDQL()` (`null` as `""`).
    fn get_dql(&self) -> String;

    /// `getHint($name)`: `false` when the hint is not set.
    fn get_hint(&self, name: &str) -> Hint;

    /// `setHint($name, $value)` (the walkers set hints while they walk).
    fn set_hint(&self, name: &str, value: Hint);

    /// `getHydrationMode()`.
    fn get_hydration_mode(&self) -> i64 {
        HYDRATE_OBJECT
    }

    /// `getParameter($key)`.
    fn get_parameter(&self, key: &str) -> Option<Parameter> {
        let _ = key;
        None
    }

    /// `getFirstResult()`.
    fn get_first_result(&self) -> i64 {
        0
    }

    /// `getMaxResults()`.
    fn get_max_results(&self) -> Option<i64> {
        None
    }
}

/// `$query->getHint(Query::HINT_LOCK_MODE) ?: LockMode::NONE`: `None` for a
/// hint that is not a `LockMode`.
pub(crate) fn lock_mode_hint(query: &dyn DqlQuery) -> Option<LockMode> {
    match query.get_hint(HINT_LOCK_MODE) {
        Hint::LockMode(mode) => Some(mode),
        hint if !hint.is_truthy() => Some(LockMode::None),
        _ => None,
    }
}

/// The function classes `new $className($name)` can construct: the
/// `Configuration` names a custom function by its class, and Rust has no
/// classes to instantiate by name, so each class registers its
/// constructor here.
fn function_classes()
-> &'static std::sync::RwLock<std::collections::HashMap<String, FunctionFactory>> {
    static CLASSES: std::sync::OnceLock<
        std::sync::RwLock<std::collections::HashMap<String, FunctionFactory>>,
    > = std::sync::OnceLock::new();
    CLASSES.get_or_init(Default::default)
}

/// Registers the constructor of a custom DQL function class, for the
/// class names an `EntityManager`'s `Configuration` holds.
pub fn register_function_class(class_name: &str, factory: FunctionFactory) {
    let mut classes = match function_classes().write() {
        Ok(classes) => classes,
        Err(poisoned) => poisoned.into_inner(),
    };
    classes.insert(class_name.trim_start_matches('\\').to_owned(), factory);
}

/// The constructor of a registered function class.
fn function_class(class_name: Option<String>) -> Option<FunctionFactory> {
    let class_name = class_name?;
    let classes = match function_classes().read() {
        Ok(classes) => classes,
        Err(poisoned) => poisoned.into_inner(),
    };
    classes.get(class_name.trim_start_matches('\\')).cloned()
}

/// The entity manager as the parser and the walkers see it.
impl QueryContext for crate::entity_manager::EntityManager {
    fn get_class_metadata(&self, class_name: &str) -> Result<Arc<ClassMetadata>, Thrown> {
        crate::entity_manager::EntityManager::get_class_metadata(self, class_name)
    }

    fn get_owning_side(&self, assoc: &AssociationMapping) -> Result<AssociationMapping, Thrown> {
        self.get_metadata_factory().get_owning_side(assoc)
    }

    fn get_database_platform(&self) -> &dyn AbstractPlatform {
        self.get_platform()
    }

    fn quote(&self, value: &str) -> Result<String, Thrown> {
        // The driver's quoting, once the connection is open.
        php2rust_doctrine_dbal::ConnectionStandIn::quote(&**self.get_connection(), value)
            .map_err(crate::query::sql_walker::dbal_error)
    }

    fn get_quote_strategy(&self) -> Arc<dyn QuoteStrategy> {
        self.get_configuration().get_quote_strategy()
    }

    fn get_custom_string_function(&self, name: &str) -> Option<FunctionFactory> {
        function_class(self.get_configuration().get_custom_string_function(name))
    }

    fn get_custom_numeric_function(&self, name: &str) -> Option<FunctionFactory> {
        function_class(self.get_configuration().get_custom_numeric_function(name))
    }

    fn get_custom_datetime_function(&self, name: &str) -> Option<FunctionFactory> {
        function_class(self.get_configuration().get_custom_datetime_function(name))
    }
}
