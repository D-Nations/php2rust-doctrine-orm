//! The test entity manager of the DQL tests (`OrmTestCase::getTestEntityManager()`):
//! a `QueryContext` over a `ClassMetadataFactory` with the attribute
//! driver, on the `SQLite` platform, whose connection quotes a string as
//! `'%s'` (the connection stub of `createConnectionStub()`), and a
//! `DqlQuery` standing in for `Doctrine\ORM\Query`.
//!
//! Upstream: doctrine/orm tests/Tests/OrmTestCase.php tests/Tests/Mocks/EntityManagerMock.php tests/Tests/Mocks/NullSqlWalker.php

#![allow(
    dead_code,
    clippy::unwrap_used,
    reason = "test helpers; each test uses a part of them"
)]

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use php_value::{Thrown, Value};
use php2rust_doctrine_dbal::WrapperParameterType;
use php2rust_doctrine_dbal::platforms::{AbstractPlatform, SQLitePlatform};
use php2rust_doctrine_orm::mapping::driver::AttributeDriver;
use php2rust_doctrine_orm::mapping::{
    AssociationMapping, ClassMetadata, ClassMetadataFactory, DefaultQuoteStrategy,
    FactoryConfiguration, QuoteStrategy,
};
use php2rust_doctrine_orm::query::ast::{
    DeleteStatement, SelectStatement, Statement, UpdateStatement,
};
use php2rust_doctrine_orm::query::context::{
    DqlQuery, FunctionFactory, HINT_CUSTOM_OUTPUT_WALKER, HYDRATE_OBJECT, Hint, OutputWalkerClass,
    QueryComponents, QueryContext, SqlFilterConstraint,
};
use php2rust_doctrine_orm::query::exec::{
    ExecutorKind, PreparedExecutorFinalizer, SqlExecutor, SqlFinalizer, SqlStatements,
};
use php2rust_doctrine_orm::query::sql_walker::{OutputWalker, SqlWalkerState};
use php2rust_doctrine_orm::query::{Parameter, Parser, ParserResult, SqlWalker};
use php2rust_doctrine_persistence::mapping::AbstractClassMetadataFactory;

use crate::models;

/// `EntityManagerMock`, as far as the parser and the walkers use it.
pub struct TestEntityManager {
    factory: ClassMetadataFactory,
    platform: Box<dyn AbstractPlatform>,
    quote_strategy: DefaultQuoteStrategy,
    pub custom_string_functions: RefCell<HashMap<String, FunctionFactory>>,
    pub custom_numeric_functions: RefCell<HashMap<String, FunctionFactory>>,
    pub custom_datetime_functions: RefCell<HashMap<String, FunctionFactory>>,
    pub filters: RefCell<Vec<Arc<dyn SqlFilterConstraint>>>,
}

impl TestEntityManager {
    /// `getTestEntityManager()`.
    pub fn new() -> Self {
        Self::with_platform(Box::new(SQLitePlatform::new()))
    }

    /// `createTestEntityManagerWithPlatform($platform)`.
    pub fn with_platform(platform: Box<dyn AbstractPlatform>) -> Self {
        models::register_query_models();
        let factory = ClassMetadataFactory::new();
        let driver = Arc::new(AttributeDriver::new(Vec::<String>::new(), true).unwrap());
        factory.set_entity_manager(FactoryConfiguration::new(driver));
        Self {
            factory,
            platform,
            quote_strategy: DefaultQuoteStrategy::new(),
            custom_string_functions: RefCell::new(HashMap::new()),
            custom_numeric_functions: RefCell::new(HashMap::new()),
            custom_datetime_functions: RefCell::new(HashMap::new()),
            filters: RefCell::new(Vec::new()),
        }
    }

    /// `createQuery($dql)`.
    pub fn create_query(&self, dql: &str) -> TestQuery<'_> {
        TestQuery::new(self, dql)
    }

    /// `getConfiguration()->addCustomStringFunction($name, $class)`.
    pub fn add_custom_string_function(&self, name: &str, factory: FunctionFactory) {
        self.custom_string_functions
            .borrow_mut()
            .insert(name.to_lowercase(), factory);
    }

    /// `getConfiguration()->addCustomNumericFunction($name, $class)`.
    pub fn add_custom_numeric_function(&self, name: &str, factory: FunctionFactory) {
        self.custom_numeric_functions
            .borrow_mut()
            .insert(name.to_lowercase(), factory);
    }

    /// `getConfiguration()->addCustomDatetimeFunction($name, $class)`.
    pub fn add_custom_datetime_function(&self, name: &str, factory: FunctionFactory) {
        self.custom_datetime_functions
            .borrow_mut()
            .insert(name.to_lowercase(), factory);
    }
}

impl QueryContext for TestEntityManager {
    fn get_class_metadata(&self, class_name: &str) -> Result<Arc<ClassMetadata>, Thrown> {
        self.factory.get_metadata_for(class_name)
    }

    fn get_owning_side(&self, assoc: &AssociationMapping) -> Result<AssociationMapping, Thrown> {
        self.factory.get_owning_side(assoc)
    }

    fn get_database_platform(&self) -> &dyn AbstractPlatform {
        self.platform.as_ref()
    }

    fn quote(&self, value: &str) -> Result<String, Thrown> {
        Ok(format!("'{value}'"))
    }

    fn get_quote_strategy(&self) -> &dyn QuoteStrategy {
        &self.quote_strategy
    }

    fn get_custom_string_function(&self, name: &str) -> Option<FunctionFactory> {
        self.custom_string_functions
            .borrow()
            .get(&name.to_lowercase())
            .cloned()
    }

    fn get_custom_numeric_function(&self, name: &str) -> Option<FunctionFactory> {
        self.custom_numeric_functions
            .borrow()
            .get(&name.to_lowercase())
            .cloned()
    }

    fn get_custom_datetime_function(&self, name: &str) -> Option<FunctionFactory> {
        self.custom_datetime_functions
            .borrow()
            .get(&name.to_lowercase())
            .cloned()
    }

    fn has_filters(&self) -> bool {
        !self.filters.borrow().is_empty()
    }

    fn get_enabled_filters(&self) -> Vec<Arc<dyn SqlFilterConstraint>> {
        self.filters.borrow().clone()
    }
}

/// `Doctrine\ORM\Query`, as far as the parser and the walkers use it.
pub struct TestQuery<'em> {
    em: &'em TestEntityManager,
    dql: RefCell<String>,
    hints: RefCell<HashMap<String, Hint>>,
    parameters: RefCell<Vec<Parameter>>,
    hydration_mode: Cell<i64>,
    first_result: Cell<i64>,
    max_results: Cell<Option<i64>>,
}

impl<'em> TestQuery<'em> {
    pub fn new(em: &'em TestEntityManager, dql: &str) -> Self {
        Self {
            em,
            dql: RefCell::new(dql.to_owned()),
            hints: RefCell::new(HashMap::new()),
            parameters: RefCell::new(Vec::new()),
            hydration_mode: Cell::new(HYDRATE_OBJECT),
            first_result: Cell::new(0),
            max_results: Cell::new(None),
        }
    }

    pub fn set_dql(&self, dql: &str) {
        dql.clone_into(&mut self.dql.borrow_mut());
    }

    pub fn set_hydration_mode(&self, mode: i64) -> &Self {
        self.hydration_mode.set(mode);
        self
    }

    pub fn set_first_result(&self, first_result: i64) -> &Self {
        self.first_result.set(first_result);
        self
    }

    pub fn set_max_results(&self, max_results: Option<i64>) -> &Self {
        self.max_results.set(max_results);
        self
    }

    /// `setParameter($key, $value, $type)`.
    pub fn set_parameter(
        &self,
        key: &str,
        value: impl Into<Value>,
        type_: Option<WrapperParameterType>,
    ) -> &Self {
        let parameter = Parameter::new(key.to_owned(), value, type_);
        let mut parameters = self.parameters.borrow_mut();
        parameters.retain(|p| p.get_name() != parameter.get_name());
        parameters.push(parameter);
        self
    }

    /// `setHint($name, $value)`.
    pub fn hint(&self, name: &str, value: Hint) -> &Self {
        self.set_hint(name, value);
        self
    }

    /// `$this->parse()`: a new parser's result.
    pub fn parse(&self) -> Result<Rc<RefCell<ParserResult>>, Thrown> {
        Parser::new(self)?.parse()
    }

    /// `getSQL()`.
    pub fn get_sql(&self) -> Result<SqlStatements, Thrown> {
        let parser_result = self.parse()?;
        let executor = parser_result.borrow().prepare_sql_executor(self)?;
        Ok(executor.get_sql_statements().clone())
    }

    /// `getSQL()` of a query with one statement.
    pub fn get_single_sql(&self) -> Result<String, Thrown> {
        match self.get_sql()? {
            SqlStatements::One(sql) => Ok(sql),
            SqlStatements::Many(sql) => Ok(sql.join("; ")),
        }
    }
}

impl DqlQuery for TestQuery<'_> {
    fn get_entity_manager(&self) -> &dyn QueryContext {
        self.em
    }

    fn get_dql(&self) -> String {
        self.dql.borrow().clone()
    }

    fn get_hint(&self, name: &str) -> Hint {
        self.hints
            .borrow()
            .get(name)
            .cloned()
            .unwrap_or(Hint::Value(Value::Bool(false)))
    }

    fn set_hint(&self, name: &str, value: Hint) {
        self.hints.borrow_mut().insert(name.to_owned(), value);
    }

    fn get_hydration_mode(&self) -> i64 {
        self.hydration_mode.get()
    }

    fn get_parameter(&self, key: &str) -> Option<Parameter> {
        let key = Parameter::normalize_name(key.to_owned());
        self.parameters
            .borrow()
            .iter()
            .find(|p| p.get_name() == key)
            .cloned()
    }

    fn get_first_result(&self) -> i64 {
        self.first_result.get()
    }

    fn get_max_results(&self) -> Option<i64> {
        self.max_results.get()
    }
}

/// `Doctrine\Tests\Mocks\NullSqlWalker`: an output walker that generates
/// no SQL.
pub struct NullSqlWalker<'a> {
    state: SqlWalkerState<'a>,
}

impl NullSqlWalker<'_> {
    pub const CLASS: OutputWalkerClass = OutputWalkerClass {
        name: "Doctrine\\Tests\\Mocks\\NullSqlWalker",
        construct: construct_null_walker,
    };
}

fn construct_null_walker<'a>(
    query: &'a dyn DqlQuery,
    parser_result: Rc<RefCell<ParserResult>>,
    query_components: QueryComponents,
) -> Box<dyn SqlWalker<'a> + 'a> {
    Box::new(NullSqlWalker {
        state: SqlWalkerState::new(query, parser_result, query_components),
    })
}

impl<'a> SqlWalker<'a> for NullSqlWalker<'a> {
    fn walker_state(&self) -> &SqlWalkerState<'a> {
        &self.state
    }

    fn walker_state_mut(&mut self) -> &mut SqlWalkerState<'a> {
        &mut self.state
    }

    fn as_dyn(&mut self) -> &mut dyn SqlWalker<'a> {
        self
    }

    fn class_name(&self) -> &'static str {
        "Doctrine\\Tests\\Mocks\\NullSqlWalker"
    }

    fn as_output_walker(&mut self) -> Option<&mut dyn OutputWalker<'a>> {
        Some(self)
    }

    fn walk_select_statement(&mut self, _: &SelectStatement) -> Result<String, Thrown> {
        Ok(String::new())
    }

    fn walk_update_statement(&mut self, _: &UpdateStatement) -> Result<String, Thrown> {
        Ok(String::new())
    }

    fn walk_delete_statement(&mut self, _: &DeleteStatement) -> Result<String, Thrown> {
        Ok(String::new())
    }
}

impl<'a> OutputWalker<'a> for NullSqlWalker<'a> {
    fn get_finalizer(&mut self, _: &Statement) -> Result<Box<dyn SqlFinalizer>, Thrown> {
        Ok(Box::new(PreparedExecutorFinalizer::new(SqlExecutor::new(
            ExecutorKind::FinalizedSelect,
            SqlStatements::Many(Vec::new()),
        ))))
    }
}

/// The custom output walker hint.
pub fn output_walker(class: OutputWalkerClass) -> (&'static str, Hint) {
    (HINT_CUSTOM_OUTPUT_WALKER, Hint::OutputWalker(class))
}

/// The class name of a thrown exception and its message.
pub fn thrown(error: &Thrown) -> (String, String) {
    (
        error.class.clone(),
        String::from_utf8_lossy(error.message.as_bytes()).into_owned(),
    )
}

/// `Doctrine\Tests\DbalTypes\NegativeToPositiveType`.
#[derive(Debug)]
pub struct NegativeToPositiveType;

impl php2rust_doctrine_dbal::types::Type for NegativeToPositiveType {
    fn class(&self) -> &'static str {
        "Doctrine\\Tests\\DbalTypes\\NegativeToPositiveType"
    }

    fn get_sql_declaration(
        &self,
        column: &php_value::PhpArray,
        platform: &dyn AbstractPlatform,
    ) -> Result<php_value::PhpString, php2rust_doctrine_dbal::Exception> {
        platform.get_integer_type_declaration_sql(column)
    }

    fn convert_to_database_value_sql(
        &self,
        sql_expr: &[u8],
        _platform: &dyn AbstractPlatform,
    ) -> php_value::PhpString {
        let mut sql = b"ABS(".to_vec();
        sql.extend_from_slice(sql_expr);
        sql.push(b')');
        php_value::PhpString::from(sql)
    }

    fn convert_to_php_value_sql(
        &self,
        sql_expr: &[u8],
        _platform: &dyn AbstractPlatform,
    ) -> php_value::PhpString {
        let mut sql = b"-(".to_vec();
        sql.extend_from_slice(sql_expr);
        sql.push(b')');
        php_value::PhpString::from(sql)
    }
}

/// `DBALType::addType('negative_to_positive', NegativeToPositiveType::class)`
/// (or `overrideType()` once it is there).
pub fn register_negative_to_positive_type() {
    use php2rust_doctrine_dbal::types::{TypeArgument, add_type, has_type, override_type};
    let type_ = || TypeArgument::Instance(Arc::new(NegativeToPositiveType));
    if has_type("negative_to_positive") {
        override_type("negative_to_positive", type_()).unwrap();
    } else {
        // Another test thread may have added it meanwhile.
        if add_type("negative_to_positive", type_()).is_err() {
            override_type("negative_to_positive", type_()).unwrap();
        }
    }
}

/// `expectDeprecationWithIdentifier($link)`: the deprecation must have
/// been triggered when the guard is dropped.
pub struct ExpectDeprecation {
    link: String,
}

impl ExpectDeprecation {
    pub fn new(link: &str) -> Self {
        php2rust_doctrine_orm::deprecations::enable_tracking_deprecations();
        Self {
            link: link.to_owned(),
        }
    }
}

impl Drop for ExpectDeprecation {
    fn drop(&mut self) {
        let count = php2rust_doctrine_orm::deprecations::count(&self.link);
        php2rust_doctrine_orm::deprecations::disable();
        if !std::thread::panicking() {
            assert!(count > 0, "the deprecation {} was not triggered", self.link);
        }
    }
}
