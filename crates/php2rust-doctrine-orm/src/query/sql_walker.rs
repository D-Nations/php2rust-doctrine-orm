//! `Doctrine\ORM\Query\SqlWalker`, `OutputWalker` and `SqlOutputWalker`:
//! the walkers that turn a DQL AST into SQL.
//!
//! PHP's walker is a class that custom output walkers extend; here
//! [`SqlWalker`] is a trait whose provided methods are the methods of the
//! class. A provided method calls the function of the same name in
//! [`base`] (the body of the PHP method), which calls back into the trait
//! for every `$this->walk*()` call, so a walker that overrides a method
//! changes what the others generate, as in PHP; an override calls
//! `base::<method>()` for `parent::<method>()`. The private state of the
//! PHP class is a [`SqlWalkerState`] the walker holds.
//!
//! Upstream: doctrine/orm src/Query/SqlWalker.php src/Query/OutputWalker.php src/Query/SqlOutputWalker.php

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::rc::Rc;
use std::sync::Arc;

use php_value::numeric::is_numeric;
use php_value::object::EnumCase;
use php_value::{Key, PhpArray, PhpObject, Thrown, Value};
use php2rust_doctrine_dbal::{Exception, LockMode, WrapperParameterType};

use crate::deprecations;
use crate::mapping::class_metadata::FETCH_EAGER;
use crate::mapping::ordered_map::OrderedMap;
use crate::mapping::{AssociationMapping, ClassMetadata};
use crate::query::ast::functions::SizeFunction;
use crate::query::ast::{
    self, AggregateExpression, ArithmeticExpression, BetweenExpression, CoalesceExpression,
    CollectionMemberExpression, ComparisonExpression, ConditionalPrimary, DeleteClause,
    DeleteStatement, EmptyCollectionComparisonExpression, EntityAsDtoArgumentExpression,
    ExistsExpression, FromClause, FunctionNode, GeneralCaseExpression, GroupByClause, HavingClause,
    IdentificationVariableDeclaration, InListExpression, InSubselectExpression, IndexBy,
    InputParameter, InstanceOfExpression, Join, JoinAssociationDeclaration, LikeExpression,
    Literal, NewObjectExpression, Node, NullComparisonExpression, NullIfExpression, OrderByClause,
    OrderByItem, ParenthesisExpression, PathExpression, QuantifiedExpression,
    RangeVariableDeclaration, SelectClause, SelectExpression, SelectStatement,
    SimpleCaseExpression, SimpleSelectClause, SimpleSelectExpression, Statement, Subselect,
    SubselectFromClause, UpdateClause, UpdateItem, UpdateStatement, WhereClause,
};
use crate::query::context::{
    DqlQuery, HINT_DISTINCT, HINT_FORCE_PARTIAL_LOAD, HINT_INCLUDE_META_COLUMNS,
    HINT_INTERNAL_ITERATION, HINT_PARTIAL, HYDRATE_OBJECT, Hint, OutputWalkerClass, QueryComponent,
    QueryComponents, QueryContext, lock_mode_hint, metadata_for_dql_alias,
};
use crate::query::exec::{
    PreparedExecutorFinalizer, SingleSelectSqlFinalizer, SqlExecutor, SqlFinalizer,
};
use crate::query::parser_result::ParserResult;
use crate::query::query_exception;
use crate::query::result_set_mapping::ResultSetMapping;
use crate::query::sort_direction::SortDirection;
use crate::query::utility;

/// `Doctrine\ORM\Query\SqlWalker`.
pub const SQL_WALKER_CLASS: &str = "Doctrine\\ORM\\Query\\SqlWalker";
/// `Doctrine\ORM\Query\SqlOutputWalker`.
pub const SQL_OUTPUT_WALKER_CLASS: &str = "Doctrine\\ORM\\Query\\SqlOutputWalker";
/// `Doctrine\ORM\Query\OutputWalker`.
pub const OUTPUT_WALKER_INTERFACE: &str = "Doctrine\\ORM\\Query\\OutputWalker";

/// A DBAL exception as a `Thrown` of the same class and message.
#[allow(
    clippy::needless_pass_by_value,
    reason = "used as a `map_err` function"
)]
pub(crate) fn dbal_error(error: Exception) -> Thrown {
    Thrown::new(error.class(), error.message().clone(), 0)
}

/// `(bool) $string`.
fn truthy(value: &str) -> bool {
    !(value.is_empty() || value == "0")
}

/// `(string) $value`.
fn text(value: &Value) -> String {
    value
        .to_php_string()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// The `TypeError` PHP throws for an argument of the wrong type.
fn type_error(method: &str, argument: &str, expected: &str, given: &str) -> Thrown {
    Thrown::new(
        "TypeError",
        format!(
            "{SQL_WALKER_CLASS}::{method}(): Argument #1 (${argument}) must be of type {expected}, {given} given"
        ),
        0,
    )
}

fn node_type_error(method: &str, argument: &str, expected: &str, given: &Node) -> Thrown {
    type_error(method, argument, expected, &given.debug_type())
}

/// `Error: Call to undefined method`.
fn undefined_method(class: &str, method: &str) -> Thrown {
    Thrown::new(
        "Error",
        format!("Call to undefined method {class}::{method}()"),
        0,
    )
}

/// `Error: Attempt to read property "..." on null`.
fn property_on_null(property: &str) -> Thrown {
    Thrown::new(
        "Error",
        format!("Attempt to read property \"{property}\" on null"),
        0,
    )
}

/// `Error: Call to a member function ...() on null`.
fn method_on_null(method: &str) -> Thrown {
    Thrown::new(
        "Error",
        format!("Call to a member function {method}() on null"),
        0,
    )
}

/// An entry of `$scalarResultAliasMap`: a column alias, or the column
/// aliases of an entity selected with a result alias.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResultAlias {
    One(String),
    Many(Vec<String>),
}

/// An entry of `$selectedClasses`.
#[derive(Debug, Clone)]
pub struct SelectedClass {
    pub class: Arc<ClassMetadata>,
    pub dql_alias: String,
    pub result_alias: Option<String>,
    pub partial: bool,
}

/// The private state of PHP's `SqlWalker`.
pub struct SqlWalkerState<'a> {
    query: &'a dyn DqlQuery,
    parser_result: Rc<RefCell<ParserResult>>,
    query_components: QueryComponents,
    em: &'a dyn QueryContext,
    alias_counter: i64,
    table_alias_counter: i64,
    scalar_result_counter: i64,
    sql_param_index: i64,
    new_object_counter: i64,
    /// Map from result variable names to their SQL column alias names.
    table_alias_map: HashMap<String, String>,
    scalar_result_alias_map: HashMap<Key, ResultAlias>,
    /// Map from Table-Alias + Column-Name to OrderBy-Direction.
    ordered_columns_map: HashMap<String, String>,
    /// Map from DQL-Alias + Field-Name to SQL Column Alias.
    scalar_fields: HashMap<(String, String), String>,
    /// A list of classes that appear in non-scalar `SelectExpressions`.
    selected_classes: OrderedMap<SelectedClass>,
    /// The DQL alias of the root class of the currently traversed query.
    root_aliases: Vec<String>,
    /// Flag that indicates whether to generate SQL table aliases in the
    /// SQL. These should only be generated for SELECT queries, not for
    /// UPDATE/DELETE.
    use_sql_table_aliases: bool,
}

impl<'a> SqlWalkerState<'a> {
    pub fn new(
        query: &'a dyn DqlQuery,
        parser_result: Rc<RefCell<ParserResult>>,
        query_components: QueryComponents,
    ) -> Self {
        Self {
            query,
            parser_result,
            query_components,
            em: query.get_entity_manager(),
            alias_counter: 0,
            table_alias_counter: 0,
            scalar_result_counter: 1,
            sql_param_index: 0,
            new_object_counter: 0,
            table_alias_map: HashMap::new(),
            scalar_result_alias_map: HashMap::new(),
            ordered_columns_map: HashMap::new(),
            scalar_fields: HashMap::new(),
            selected_classes: OrderedMap::new(),
            root_aliases: Vec::new(),
            use_sql_table_aliases: true,
        }
    }

    /// `$this->parserResult`.
    pub fn parser_result(&self) -> &Rc<RefCell<ParserResult>> {
        &self.parser_result
    }

    /// `$this->selectedClasses`.
    pub fn selected_classes(&self) -> &OrderedMap<SelectedClass> {
        &self.selected_classes
    }
}

fn rsm<R>(w: &dyn SqlWalker<'_>, f: impl FnOnce(&mut ResultSetMapping) -> R) -> R {
    let parser_result = w.walker_state().parser_result.clone();
    let mut parser_result = parser_result.borrow_mut();
    f(parser_result.get_result_set_mapping_mut())
}

/// `$node->dispatch($walker)`: the double dispatch of an AST node.
pub fn dispatch(w: &mut dyn SqlWalker<'_>, node: &Node) -> Result<String, Thrown> {
    match node {
        Node::Str(_) => Err(Thrown::new(
            "Error",
            "Call to a member function dispatch() on string",
            0,
        )),
        Node::AggregateExpression(n) => w.walk_aggregate_expression(n),
        Node::ArithmeticExpression(n) => w.walk_arithmetic_expression(n),
        Node::ArithmeticFactor(_) => w.walk_arithmetic_factor(node),
        Node::ArithmeticTerm(_) => w.walk_arithmetic_term(node),
        Node::BetweenExpression(n) => w.walk_between_expression(n),
        Node::CoalesceExpression(n) => w.walk_coalesce_expression(n),
        Node::CollectionMemberExpression(n) => w.walk_collection_member_expression(n),
        Node::ComparisonExpression(n) => w.walk_comparison_expression(n),
        Node::ConditionalExpression(_) => w.walk_conditional_expression(node),
        Node::ConditionalFactor(_) => w.walk_conditional_factor(node),
        Node::ConditionalPrimary(n) => w.walk_conditional_primary(n),
        Node::ConditionalTerm(_) => w.walk_conditional_term(node),
        Node::DeleteClause(n) => w.walk_delete_clause(n),
        Node::DeleteStatement(n) => w.walk_delete_statement(n),
        Node::EmptyCollectionComparisonExpression(n) => {
            w.walk_empty_collection_comparison_expression(n)
        }
        Node::EntityAsDtoArgumentExpression(n) => w.walk_entity_as_dto_argument_expression(n),
        Node::ExistsExpression(n) => w.walk_exists_expression(n),
        Node::FromClause(n) => w.walk_from_clause(n),
        Node::Function(f) => w.walk_function(f.as_ref()),
        Node::GeneralCaseExpression(n) => w.walk_general_case_expression(n),
        Node::GroupByClause(n) => w.walk_group_by_clause(n),
        Node::HavingClause(n) => w.walk_having_clause(n),
        Node::IdentificationVariableDeclaration(n) => w.walk_identification_variable_declaration(n),
        Node::InListExpression(n) => w.walk_in_list_expression(n),
        Node::InSubselectExpression(n) => w.walk_in_subselect_expression(n),
        Node::IndexBy(n) => {
            w.walk_index_by(n)?;
            Ok(String::new())
        }
        Node::InputParameter(n) => w.walk_input_parameter(n),
        Node::InstanceOfExpression(n) => w.walk_instance_of_expression(n),
        Node::Join(n) => w.walk_join(n),
        Node::JoinAssociationDeclaration(n) => {
            w.walk_join_association_declaration(n, ast::JOIN_TYPE_INNER, None)
        }
        Node::JoinClassPathExpression(_) => {
            Err(undefined_method(w.class_name(), "walkJoinPathExpression"))
        }
        Node::JoinVariableDeclaration(_) => Err(undefined_method(
            w.class_name(),
            "walkJoinVariableDeclaration",
        )),
        Node::LikeExpression(n) => w.walk_like_expression(n),
        Node::Literal(n) => w.walk_literal(n),
        Node::NewObjectExpression(n) => {
            let expression = n.borrow().clone();
            w.walk_new_object(&expression, None)
        }
        Node::NullComparisonExpression(n) => w.walk_null_comparison_expression(n),
        Node::NullIfExpression(n) => w.walk_null_if_expression(n),
        Node::OrderByClause(n) => w.walk_order_by_clause(n),
        Node::OrderByItem(n) => w.walk_order_by_item(n),
        Node::ParenthesisExpression(n) => w.walk_parenthesis_expression(n),
        Node::PartialObjectExpression(_) | Node::JoinAssociationPathExpression(_) => {
            Err(query_exception::no_dispatch_for_node(&node.debug_type()))
        }
        Node::PathExpression(p) => {
            let path = p.borrow().clone();
            w.walk_path_expression(&path)
        }
        Node::QuantifiedExpression(n) => w.walk_quantified_expression(n),
        Node::RangeVariableDeclaration(n) => w.walk_range_variable_declaration(n),
        Node::SelectClause(n) => w.walk_select_clause(n),
        Node::SelectExpression(n) => w.walk_select_expression(n),
        Node::SelectStatement(n) => w.walk_select_statement(n),
        Node::SimpleArithmeticExpression(_) => w.walk_simple_arithmetic_expression(node),
        Node::SimpleCaseExpression(n) => w.walk_simple_case_expression(n),
        Node::SimpleSelectClause(n) => w.walk_simple_select_clause(n),
        Node::SimpleSelectExpression(n) => w.walk_simple_select_expression(n),
        Node::SimpleWhenClause(_) | Node::WhenClause(_) => {
            Err(undefined_method(w.class_name(), "walkWhenClauseExpression"))
        }
        Node::Subselect(n) => w.walk_subselect(n),
        Node::SubselectFromClause(n) => w.walk_subselect_from_clause(n),
        Node::UpdateClause(n) => w.walk_update_clause(n),
        Node::UpdateItem(n) => w.walk_update_item(n),
        Node::UpdateStatement(n) => w.walk_update_statement(n),
        Node::WhereClause(n) => w.walk_where_clause(Some(n)),
    }
}

/// `Doctrine\ORM\Query\OutputWalker`: an output walker that provides an
/// SQL finalizer.
pub trait OutputWalker<'a>: SqlWalker<'a> {
    fn get_finalizer(&mut self, ast: &Statement) -> Result<Box<dyn SqlFinalizer>, Thrown> {
        base::get_finalizer(self.as_dyn(), ast)
    }
}

/// The methods of `Doctrine\ORM\Query\SqlWalker`: the `SqlWalker` is a
/// `TreeWalker` that walks over a DQL AST and constructs the corresponding
/// SQL.
pub trait SqlWalker<'a> {
    /// The walker's state.
    fn walker_state(&self) -> &SqlWalkerState<'a>;

    /// The walker's state, to change it.
    fn walker_state_mut(&mut self) -> &mut SqlWalkerState<'a>;

    /// The walker as a trait object.
    fn as_dyn(&mut self) -> &mut dyn SqlWalker<'a>;

    /// The walker's class name.
    fn class_name(&self) -> &'static str {
        SQL_WALKER_CLASS
    }

    /// `$walker instanceof OutputWalker`.
    fn as_output_walker(&mut self) -> Option<&mut dyn OutputWalker<'a>> {
        None
    }

    /// Gets the Query instance used by the walker.
    fn get_query(&self) -> &'a dyn DqlQuery {
        self.walker_state().query
    }

    /// Gets the `EntityManager` used by the walker.
    fn get_entity_manager(&self) -> &'a dyn QueryContext {
        self.walker_state().em
    }

    /// Gets the parser result of the query (`$this->parserResult`).
    fn get_parser_result(&self) -> Rc<RefCell<ParserResult>> {
        self.walker_state().parser_result.clone()
    }

    /// Gets the information about a single query component.
    fn get_query_component<'s>(&'s self, dql_alias: &str) -> Option<&'s QueryComponent>
    where
        'a: 's,
    {
        self.walker_state().query_components.get(dql_alias)
    }

    fn get_metadata_for_dql_alias(&self, dql_alias: &str) -> Result<Arc<ClassMetadata>, Thrown> {
        metadata_for_dql_alias(&self.walker_state().query_components, dql_alias)
    }

    /// Returns internal queryComponents array.
    fn get_query_components<'s>(&'s self) -> &'s QueryComponents
    where
        'a: 's,
    {
        &self.walker_state().query_components
    }

    /// Sets or overrides a query component for a given dql alias.
    fn set_query_component(
        &mut self,
        dql_alias: &str,
        query_component: QueryComponent,
    ) -> Result<(), Thrown> {
        if !query_component.has_entity_keys {
            return Err(query_exception::invalid_query_component(dql_alias));
        }
        self.walker_state_mut()
            .query_components
            .insert(dql_alias.to_owned(), query_component);
        Ok(())
    }

    /// Gets an executor that can be used to execute the result of this
    /// walker (deprecated: output walkers should implement
    /// `OutputWalker`).
    fn get_executor(&mut self, statement: &Statement) -> Result<SqlExecutor, Thrown> {
        base::get_executor(self.as_dyn(), statement)
    }

    fn create_update_statement_executor(
        &mut self,
        ast: &UpdateStatement,
    ) -> Result<SqlExecutor, Thrown> {
        base::create_update_statement_executor(self.as_dyn(), ast)
    }

    fn create_delete_statement_executor(
        &mut self,
        ast: &DeleteStatement,
    ) -> Result<SqlExecutor, Thrown> {
        base::create_delete_statement_executor(self.as_dyn(), ast)
    }

    /// Generates a unique, short SQL table alias.
    fn get_sql_table_alias(&mut self, table_name: &str, dql_alias: &str) -> String {
        base::get_sql_table_alias(self.walker_state_mut(), table_name, dql_alias)
    }

    /// Forces the `SqlWalker` to use a specific alias for a table name,
    /// rather than generating an alias on its own.
    fn set_sql_table_alias(&mut self, table_name: &str, alias: &str, dql_alias: &str) -> String {
        let table_name = if truthy(dql_alias) {
            format!("{table_name}@[{dql_alias}]")
        } else {
            table_name.to_owned()
        };
        self.walker_state_mut()
            .table_alias_map
            .insert(table_name, alias.to_owned());
        alias.to_owned()
    }

    /// Gets an SQL column alias for a column name.
    fn get_sql_column_alias(&mut self, column_name: &str) -> String {
        let state = self.walker_state_mut();
        let counter = state.alias_counter;
        state.alias_counter += 1;
        let em = state.em;
        em.get_quote_strategy().get_column_alias(
            column_name,
            counter,
            em.get_database_platform(),
            None,
        )
    }

    fn walk_select_statement(
        &mut self,
        select_statement: &SelectStatement,
    ) -> Result<String, Thrown> {
        base::walk_select_statement(self.as_dyn(), select_statement)
    }

    fn create_sql_for_finalizer(
        &mut self,
        select_statement: &SelectStatement,
    ) -> Result<String, Thrown> {
        base::create_sql_for_finalizer(self.as_dyn(), select_statement)
    }

    fn walk_update_statement(
        &mut self,
        update_statement: &UpdateStatement,
    ) -> Result<String, Thrown> {
        base::walk_update_statement(self.as_dyn(), update_statement)
    }

    fn walk_delete_statement(
        &mut self,
        delete_statement: &DeleteStatement,
    ) -> Result<String, Thrown> {
        base::walk_delete_statement(self.as_dyn(), delete_statement)
    }

    /// Walks down an `IdentificationVariable` AST node, thereby generating
    /// the appropriate SQL.
    fn walk_entity_identification_variable(
        &mut self,
        ident_variable: &str,
    ) -> Result<String, Thrown> {
        base::walk_entity_identification_variable(self.as_dyn(), ident_variable)
    }

    fn walk_entity_as_dto_argument_expression(
        &mut self,
        expr: &EntityAsDtoArgumentExpression,
    ) -> Result<String, Thrown> {
        base::walk_entity_as_dto_argument_expression(self.as_dyn(), expr)
    }

    /// Walks down an `IdentificationVariable` (no AST node associated),
    /// thereby generating the SQL.
    fn walk_identification_variable(
        &mut self,
        identification_variable: &str,
        field_name: Option<&str>,
    ) -> Result<String, Thrown> {
        base::walk_identification_variable(self.as_dyn(), identification_variable, field_name)
    }

    fn walk_path_expression(&mut self, path_expr: &PathExpression) -> Result<String, Thrown> {
        base::walk_path_expression(self.as_dyn(), path_expr)
    }

    fn walk_select_clause(&mut self, select_clause: &SelectClause) -> Result<String, Thrown> {
        base::walk_select_clause(self.as_dyn(), select_clause)
    }

    fn walk_from_clause(&mut self, from_clause: &FromClause) -> Result<String, Thrown> {
        base::walk_from_clause(self.as_dyn(), from_clause)
    }

    fn walk_identification_variable_declaration(
        &mut self,
        identification_variable_decl: &IdentificationVariableDeclaration,
    ) -> Result<String, Thrown> {
        base::walk_identification_variable_declaration(self.as_dyn(), identification_variable_decl)
    }

    /// Walks down a `IndexBy` AST node.
    fn walk_index_by(&mut self, index_by: &IndexBy) -> Result<(), Thrown> {
        base::walk_index_by(self.as_dyn(), index_by)
    }

    fn walk_range_variable_declaration(
        &mut self,
        range_variable_declaration: &RangeVariableDeclaration,
    ) -> Result<String, Thrown> {
        base::generate_range_variable_declaration_sql(
            self.as_dyn(),
            range_variable_declaration,
            false,
        )
    }

    /// Walks down a `JoinAssociationDeclaration` AST node, thereby
    /// generating the appropriate SQL.
    fn walk_join_association_declaration(
        &mut self,
        join_association_declaration: &JoinAssociationDeclaration,
        join_type: i64,
        cond_expr: Option<&Node>,
    ) -> Result<String, Thrown> {
        base::walk_join_association_declaration(
            self.as_dyn(),
            join_association_declaration,
            join_type,
            cond_expr,
        )
    }

    fn walk_function(&mut self, function: &dyn FunctionNode) -> Result<String, Thrown> {
        function.get_sql(self.as_dyn())
    }

    fn walk_order_by_clause(&mut self, order_by_clause: &OrderByClause) -> Result<String, Thrown> {
        base::walk_order_by_clause(self.as_dyn(), order_by_clause)
    }

    fn walk_order_by_item(&mut self, order_by_item: &OrderByItem) -> Result<String, Thrown> {
        base::walk_order_by_item(self.as_dyn(), order_by_item)
    }

    fn walk_having_clause(&mut self, having_clause: &HavingClause) -> Result<String, Thrown> {
        let sql = self.walk_conditional_expression(&having_clause.conditional_expression)?;
        Ok(format!(" HAVING {sql}"))
    }

    fn walk_join(&mut self, join: &Join) -> Result<String, Thrown> {
        base::walk_join(self.as_dyn(), join)
    }

    fn walk_coalesce_expression(
        &mut self,
        coalesce_expression: &CoalesceExpression,
    ) -> Result<String, Thrown> {
        base::walk_coalesce_expression(self.as_dyn(), coalesce_expression)
    }

    fn walk_null_if_expression(
        &mut self,
        null_if_expression: &NullIfExpression,
    ) -> Result<String, Thrown> {
        base::walk_null_if_expression(self.as_dyn(), null_if_expression)
    }

    fn walk_general_case_expression(
        &mut self,
        general_case_expression: &GeneralCaseExpression,
    ) -> Result<String, Thrown> {
        base::walk_general_case_expression(self.as_dyn(), general_case_expression)
    }

    fn walk_simple_case_expression(
        &mut self,
        simple_case_expression: &SimpleCaseExpression,
    ) -> Result<String, Thrown> {
        base::walk_simple_case_expression(self.as_dyn(), simple_case_expression)
    }

    fn walk_select_expression(
        &mut self,
        select_expression: &SelectExpression,
    ) -> Result<String, Thrown> {
        base::walk_select_expression(self.as_dyn(), select_expression)
    }

    /// Walks down an `ObjectExpression` (the selection of an entity):
    /// returns the SQL of its columns.
    fn walk_object_expression(
        &mut self,
        dql_alias: &str,
        partial_field_set: &[String],
        result_alias: Option<&str>,
    ) -> Result<Vec<String>, Thrown> {
        base::walk_object_expression(self.as_dyn(), dql_alias, partial_field_set, result_alias)
    }

    fn walk_quantified_expression(
        &mut self,
        q_expr: &QuantifiedExpression,
    ) -> Result<String, Thrown> {
        let sql = self.walk_subselect(&q_expr.subselect)?;
        Ok(format!(" {}({sql})", q_expr.type_.to_ascii_uppercase()))
    }

    fn walk_subselect(&mut self, subselect: &Subselect) -> Result<String, Thrown> {
        base::walk_subselect(self.as_dyn(), subselect)
    }

    fn walk_subselect_from_clause(
        &mut self,
        subselect_from_clause: &SubselectFromClause,
    ) -> Result<String, Thrown> {
        let mut sql_parts = Vec::new();
        for decl in &subselect_from_clause.identification_variable_declarations {
            sql_parts.push(self.walk_identification_variable_declaration(decl)?);
        }
        Ok(format!(" FROM {}", sql_parts.join(", ")))
    }

    fn walk_simple_select_clause(
        &mut self,
        simple_select_clause: &SimpleSelectClause,
    ) -> Result<String, Thrown> {
        let expression =
            self.walk_simple_select_expression(&simple_select_clause.simple_select_expression)?;
        Ok(format!(
            "SELECT{}{expression}",
            if simple_select_clause.is_distinct {
                " DISTINCT"
            } else {
                ""
            }
        ))
    }

    fn walk_parenthesis_expression(
        &mut self,
        parenthesis_expression: &ParenthesisExpression,
    ) -> Result<String, Thrown> {
        let sql = dispatch(self.as_dyn(), &parenthesis_expression.expression)?;
        Ok(format!("({sql})"))
    }

    fn walk_new_object(
        &mut self,
        new_object_expression: &NewObjectExpression,
        new_object_result_alias: Option<&str>,
    ) -> Result<String, Thrown> {
        base::walk_new_object(
            self.as_dyn(),
            new_object_expression,
            new_object_result_alias,
        )
    }

    fn walk_simple_select_expression(
        &mut self,
        simple_select_expression: &SimpleSelectExpression,
    ) -> Result<String, Thrown> {
        base::walk_simple_select_expression(self.as_dyn(), simple_select_expression)
    }

    fn walk_aggregate_expression(
        &mut self,
        agg_expression: &AggregateExpression,
    ) -> Result<String, Thrown> {
        let sql = self.walk_simple_arithmetic_expression(&agg_expression.path_expression)?;
        Ok(format!(
            "{}({}{sql})",
            agg_expression.function_name,
            if agg_expression.is_distinct {
                "DISTINCT "
            } else {
                ""
            }
        ))
    }

    fn walk_group_by_clause(&mut self, group_by_clause: &GroupByClause) -> Result<String, Thrown> {
        let mut sql_parts = Vec::new();
        for item in &group_by_clause.group_by_items {
            sql_parts.push(self.walk_group_by_item(item)?);
        }
        Ok(format!(" GROUP BY {}", sql_parts.join(", ")))
    }

    /// Walks down a `GroupByItem` (a `PathExpression` or a string) AST node.
    fn walk_group_by_item(&mut self, group_by_item: &Node) -> Result<String, Thrown> {
        base::walk_group_by_item(self.as_dyn(), group_by_item)
    }

    fn walk_delete_clause(&mut self, delete_clause: &DeleteClause) -> Result<String, Thrown> {
        base::walk_delete_clause(self.as_dyn(), delete_clause)
    }

    fn walk_update_clause(&mut self, update_clause: &UpdateClause) -> Result<String, Thrown> {
        base::walk_update_clause(self.as_dyn(), update_clause)
    }

    fn walk_update_item(&mut self, update_item: &UpdateItem) -> Result<String, Thrown> {
        base::walk_update_item(self.as_dyn(), update_item)
    }

    /// Walks down a `WhereClause` AST node, thereby generating the
    /// appropriate SQL. `WhereClause` or not, the appropriate discriminator
    /// sql is added.
    fn walk_where_clause(&mut self, where_clause: Option<&WhereClause>) -> Result<String, Thrown> {
        base::walk_where_clause(self.as_dyn(), where_clause)
    }

    /// Walk down a `ConditionalExpression` (or a `Phase2OptimizableConditional`)
    /// AST node.
    fn walk_conditional_expression(&mut self, cond_expr: &Node) -> Result<String, Thrown> {
        match cond_expr {
            Node::ConditionalExpression(expr) => {
                let mut terms = Vec::new();
                for term in &expr.conditional_terms {
                    terms.push(self.walk_conditional_term(term)?);
                }
                Ok(terms.join(" OR "))
            }
            other => self.walk_conditional_term(other),
        }
    }

    /// Walks down a `ConditionalTerm` (or a `ConditionalFactor` or
    /// `ConditionalPrimary`) AST node.
    fn walk_conditional_term(&mut self, cond_term: &Node) -> Result<String, Thrown> {
        match cond_term {
            Node::ConditionalTerm(term) => {
                let mut factors = Vec::new();
                for factor in &term.conditional_factors {
                    factors.push(self.walk_conditional_factor(factor)?);
                }
                Ok(factors.join(" AND "))
            }
            Node::ConditionalFactor(_) | Node::ConditionalPrimary(_) => {
                self.walk_conditional_factor(cond_term)
            }
            other => Err(node_type_error(
                "walkConditionalTerm",
                "condTerm",
                "Doctrine\\ORM\\Query\\AST\\ConditionalTerm|Doctrine\\ORM\\Query\\AST\\ConditionalPrimary|Doctrine\\ORM\\Query\\AST\\ConditionalFactor",
                other,
            )),
        }
    }

    /// Walks down a `ConditionalFactor` (or a `ConditionalPrimary`) AST node.
    fn walk_conditional_factor(&mut self, factor: &Node) -> Result<String, Thrown> {
        match factor {
            Node::ConditionalFactor(factor) => {
                let sql = self.walk_conditional_primary(&factor.conditional_primary)?;
                Ok(format!("{}{sql}", if factor.not { "NOT " } else { "" }))
            }
            Node::ConditionalPrimary(primary) => self.walk_conditional_primary(primary),
            other => Err(node_type_error(
                "walkConditionalFactor",
                "factor",
                "Doctrine\\ORM\\Query\\AST\\ConditionalFactor|Doctrine\\ORM\\Query\\AST\\ConditionalPrimary",
                other,
            )),
        }
    }

    fn walk_conditional_primary(&mut self, primary: &ConditionalPrimary) -> Result<String, Thrown> {
        if let Some(simple) = &primary.simple_conditional_expression {
            return dispatch(self.as_dyn(), simple);
        }
        if let Some(cond_expr) = &primary.conditional_expression {
            let sql = self.walk_conditional_expression(cond_expr)?;
            return Ok(format!("({sql})"));
        }
        Err(query_exception::logic(
            "Unexpected state of ConditionalPrimary node.",
        ))
    }

    fn walk_exists_expression(&mut self, exists_expr: &ExistsExpression) -> Result<String, Thrown> {
        let sql = self.walk_subselect(&exists_expr.subselect)?;
        Ok(format!(
            "{}EXISTS ({sql})",
            if exists_expr.not { "NOT " } else { "" }
        ))
    }

    fn walk_collection_member_expression(
        &mut self,
        coll_member_expr: &CollectionMemberExpression,
    ) -> Result<String, Thrown> {
        base::walk_collection_member_expression(self.as_dyn(), coll_member_expr)
    }

    fn walk_empty_collection_comparison_expression(
        &mut self,
        empty_coll_comp_expr: &EmptyCollectionComparisonExpression,
    ) -> Result<String, Thrown> {
        let mut size_func = SizeFunction::new("size");
        size_func.collection_path_expression = Some(empty_coll_comp_expr.expression.clone());
        let sql = size_func.get_sql(self.as_dyn())?;
        Ok(format!(
            "{sql}{}",
            if empty_coll_comp_expr.not {
                " > 0"
            } else {
                " = 0"
            }
        ))
    }

    fn walk_null_comparison_expression(
        &mut self,
        null_comp_expr: &NullComparisonExpression,
    ) -> Result<String, Thrown> {
        let comparison = format!(" IS{} NULL", if null_comp_expr.not { " NOT" } else { "" });
        let expression = &null_comp_expr.expression;
        if let Node::Str(name) = expression
            && crate::query::context::has_result_variable(self.get_query_components(), name)
        {
            return Ok(format!("{}{comparison}", self.walk_result_variable(name)?));
        }
        if let Node::InputParameter(parameter) = expression {
            return Ok(format!(
                "{}{comparison}",
                self.walk_input_parameter(parameter)?
            ));
        }
        Ok(format!(
            "{}{comparison}",
            dispatch(self.as_dyn(), expression)?
        ))
    }

    fn walk_in_list_expression(&mut self, in_expr: &InListExpression) -> Result<String, Thrown> {
        let expression = self.walk_arithmetic_expression(&in_expr.expression)?;
        let mut literals = Vec::new();
        for literal in &in_expr.literals {
            literals.push(self.walk_in_parameter(literal)?);
        }
        Ok(format!(
            "{expression}{} IN ({})",
            if in_expr.not { " NOT" } else { "" },
            literals.join(", ")
        ))
    }

    fn walk_in_subselect_expression(
        &mut self,
        in_expr: &InSubselectExpression,
    ) -> Result<String, Thrown> {
        let expression = self.walk_arithmetic_expression(&in_expr.expression)?;
        let subselect = self.walk_subselect(&in_expr.subselect)?;
        Ok(format!(
            "{expression}{} IN ({subselect})",
            if in_expr.not { " NOT" } else { "" }
        ))
    }

    fn walk_instance_of_expression(
        &mut self,
        instance_of_expr: &InstanceOfExpression,
    ) -> Result<String, Thrown> {
        base::walk_instance_of_expression(self.as_dyn(), instance_of_expr)
    }

    /// Walks down an `InParameter` (an `InputParameter` or an
    /// `ArithmeticExpression`).
    fn walk_in_parameter(&mut self, in_param: &Node) -> Result<String, Thrown> {
        match in_param {
            Node::InputParameter(parameter) => self.walk_input_parameter(parameter),
            Node::ArithmeticExpression(expr) => self.walk_arithmetic_expression(expr),
            other => Err(node_type_error(
                "walkArithmeticExpression",
                "arithmeticExpr",
                "Doctrine\\ORM\\Query\\AST\\ArithmeticExpression",
                other,
            )),
        }
    }

    fn walk_literal(&mut self, literal: &Literal) -> Result<String, Thrown> {
        base::walk_literal(self.as_dyn(), literal)
    }

    fn walk_between_expression(
        &mut self,
        between_expr: &BetweenExpression,
    ) -> Result<String, Thrown> {
        let mut sql = self.walk_arithmetic_expression(&between_expr.expression)?;
        if between_expr.not {
            sql.push_str(" NOT");
        }
        let left = self.walk_arithmetic_expression(&between_expr.left_between_expression)?;
        let right = self.walk_arithmetic_expression(&between_expr.right_between_expression)?;
        let _ = write!(sql, " BETWEEN {left} AND {right}");
        Ok(sql)
    }

    fn walk_like_expression(&mut self, like_expr: &LikeExpression) -> Result<String, Thrown> {
        base::walk_like_expression(self.as_dyn(), like_expr)
    }

    fn walk_state_field_path_expression(
        &mut self,
        state_field_path_expression: &PathExpression,
    ) -> Result<String, Thrown> {
        self.walk_path_expression(state_field_path_expression)
    }

    fn walk_comparison_expression(
        &mut self,
        comp_expr: &ComparisonExpression,
    ) -> Result<String, Thrown> {
        let left = base::comparison_operand(self.as_dyn(), &comp_expr.left_expression)?;
        let right = base::comparison_operand(self.as_dyn(), &comp_expr.right_expression)?;
        Ok(format!("{left} {} {right}", comp_expr.operator))
    }

    fn walk_input_parameter(&mut self, input_param: &InputParameter) -> Result<String, Thrown> {
        base::walk_input_parameter(self.as_dyn(), input_param)
    }

    fn walk_arithmetic_expression(
        &mut self,
        arithmetic_expr: &ArithmeticExpression,
    ) -> Result<String, Thrown> {
        if arithmetic_expr.is_simple_arithmetic_expression()
            && let Some(simple) = &arithmetic_expr.simple_arithmetic_expression
        {
            return self.walk_simple_arithmetic_expression(simple);
        }
        let Some(subselect) = &arithmetic_expr.subselect else {
            return Err(type_error(
                "walkSubselect",
                "subselect",
                "Doctrine\\ORM\\Query\\AST\\Subselect",
                "null",
            ));
        };
        Ok(format!("({})", self.walk_subselect(subselect)?))
    }

    fn walk_simple_arithmetic_expression(
        &mut self,
        simple_arithmetic_expr: &Node,
    ) -> Result<String, Thrown> {
        let Node::SimpleArithmeticExpression(expr) = simple_arithmetic_expr else {
            return self.walk_arithmetic_term(simple_arithmetic_expr);
        };
        let mut terms = Vec::new();
        for term in &expr.arithmetic_terms {
            terms.push(self.walk_arithmetic_term(term)?);
        }
        Ok(terms.join(" "))
    }

    fn walk_arithmetic_term(&mut self, term: &Node) -> Result<String, Thrown> {
        match term {
            Node::Str(term) => base::result_variable_or_string(self.as_dyn(), term),
            Node::ArithmeticTerm(term) => {
                let mut factors = Vec::new();
                for factor in &term.arithmetic_factors {
                    factors.push(self.walk_arithmetic_factor(factor)?);
                }
                Ok(factors.join(" "))
            }
            other => self.walk_arithmetic_factor(other),
        }
    }

    fn walk_arithmetic_factor(&mut self, factor: &Node) -> Result<String, Thrown> {
        match factor {
            Node::Str(factor) => base::result_variable_or_string(self.as_dyn(), factor),
            Node::ArithmeticFactor(factor) => {
                let sign = if factor.is_negative_signed() {
                    "-"
                } else if factor.is_positive_signed() {
                    "+"
                } else {
                    ""
                };
                Ok(format!(
                    "{sign}{}",
                    self.walk_arithmetic_primary(&factor.arithmetic_primary)?
                ))
            }
            other => self.walk_arithmetic_primary(other),
        }
    }

    /// Walks down an `ArithmeticPrimary` that represents an AST node,
    /// thereby generating the appropriate SQL.
    fn walk_arithmetic_primary(&mut self, primary: &Node) -> Result<String, Thrown> {
        match primary {
            Node::SimpleArithmeticExpression(_) => Ok(format!(
                "({})",
                self.walk_simple_arithmetic_expression(primary)?
            )),
            Node::Str(identification_variable) => {
                self.walk_entity_identification_variable(identification_variable)
            }
            other => dispatch(self.as_dyn(), other),
        }
    }

    /// Walks down a `StringPrimary` that represents an AST node, thereby
    /// generating the appropriate SQL.
    fn walk_string_primary(&mut self, string_primary: &Node) -> Result<String, Thrown> {
        match string_primary {
            Node::Str(value) => self.get_entity_manager().quote(value),
            other => dispatch(self.as_dyn(), other),
        }
    }

    /// Walks down a `ResultVariable` that represents an AST node, thereby
    /// generating the appropriate SQL.
    fn walk_result_variable(&mut self, result_variable: &str) -> Result<String, Thrown> {
        match self
            .walker_state()
            .scalar_result_alias_map
            .get(&Key::from(result_variable))
        {
            None => Err(Thrown::new(
                "InvalidArgumentException",
                format!("Unknown result variable: {result_variable}"),
                0,
            )),
            Some(ResultAlias::One(alias)) => Ok(alias.clone()),
            Some(ResultAlias::Many(aliases)) => Ok(aliases.join(", ")),
        }
    }
}

/// The bodies of the methods of `SqlWalker` (`parent::<method>()` for a
/// walker that overrides one).
pub mod base {
    use std::fmt::Write as _;

    use super::{
        Arc, AssociationMapping, ClassMetadata, CoalesceExpression, CollectionMemberExpression,
        DeleteClause, DeleteStatement, EntityAsDtoArgumentExpression, EnumCase, FETCH_EAGER,
        FromClause, GeneralCaseExpression, HINT_DISTINCT, HINT_FORCE_PARTIAL_LOAD,
        HINT_INCLUDE_META_COLUMNS, HINT_INTERNAL_ITERATION, HINT_PARTIAL, HYDRATE_OBJECT, Hint,
        IdentificationVariableDeclaration, IndexBy, InputParameter, InstanceOfExpression, Join,
        JoinAssociationDeclaration, Key, LikeExpression, Literal, LockMode, NewObjectExpression,
        Node, NullIfExpression, OUTPUT_WALKER_INTERFACE, OrderByClause, OrderByItem,
        PathExpression, PhpArray, PhpObject, PreparedExecutorFinalizer, QueryContext,
        RangeVariableDeclaration, ResultAlias, SQL_WALKER_CLASS, SelectClause, SelectExpression,
        SelectStatement, SelectedClass, SimpleCaseExpression, SimpleSelectExpression,
        SingleSelectSqlFinalizer, SortDirection, SqlExecutor, SqlFinalizer, SqlWalker,
        SqlWalkerState, Statement, Subselect, Thrown, UpdateClause, UpdateItem, UpdateStatement,
        Value, WhereClause, WrapperParameterType, ast, dbal_error, deprecations, dispatch,
        is_numeric, lock_mode_hint, method_on_null, node_type_error, property_on_null,
        query_exception, rsm, text, truthy, type_error, utility,
    };

    /// `getExecutor()`.
    pub fn get_executor(
        w: &mut dyn SqlWalker<'_>,
        statement: &Statement,
    ) -> Result<SqlExecutor, Thrown> {
        deprecations::trigger("https://github.com/doctrine/orm/pull/11188/", || {
            format!(
                "Output walkers should implement {OUTPUT_WALKER_INTERFACE}. That way, the {SQL_WALKER_CLASS}::getExecutor method is no longer needed and will be removed in 4.0"
            )
        });
        match statement {
            Statement::Update(ast) => w.create_update_statement_executor(ast),
            Statement::Delete(ast) => w.create_delete_statement_executor(ast),
            Statement::Select(ast) => SqlExecutor::single_select(ast, w),
        }
    }

    /// `SqlOutputWalker::getFinalizer()`.
    pub fn get_finalizer(
        w: &mut dyn SqlWalker<'_>,
        ast: &Statement,
    ) -> Result<Box<dyn SqlFinalizer>, Thrown> {
        Ok(match ast {
            Statement::Select(ast) => Box::new(SingleSelectSqlFinalizer::new(
                w.create_sql_for_finalizer(ast)?,
            )),
            Statement::Update(ast) => Box::new(PreparedExecutorFinalizer::new(
                w.create_update_statement_executor(ast)?,
            )),
            Statement::Delete(ast) => Box::new(PreparedExecutorFinalizer::new(
                w.create_delete_statement_executor(ast)?,
            )),
        })
    }

    pub fn create_update_statement_executor(
        w: &mut dyn SqlWalker<'_>,
        ast: &UpdateStatement,
    ) -> Result<SqlExecutor, Thrown> {
        let primary_class = w
            .get_entity_manager()
            .get_class_metadata(&ast.update_clause.abstract_schema_name)?;
        let joined = primary_class.read().is_inheritance_type_joined();
        if joined {
            SqlExecutor::multi_table_update(ast, w)
        } else {
            SqlExecutor::single_table_update(ast, w)
        }
    }

    pub fn create_delete_statement_executor(
        w: &mut dyn SqlWalker<'_>,
        ast: &DeleteStatement,
    ) -> Result<SqlExecutor, Thrown> {
        let primary_class = w
            .get_entity_manager()
            .get_class_metadata(&ast.delete_clause.abstract_schema_name)?;
        let joined = primary_class.read().is_inheritance_type_joined();
        if joined {
            SqlExecutor::multi_table_delete(ast, w)
        } else {
            SqlExecutor::single_table_delete(ast, w)
        }
    }

    /// `getSQLTableAlias()`.
    pub fn get_sql_table_alias(
        state: &mut SqlWalkerState<'_>,
        table_name: &str,
        dql_alias: &str,
    ) -> String {
        let table_name = if truthy(dql_alias) {
            format!("{table_name}@[{dql_alias}]")
        } else {
            table_name.to_owned()
        };
        if let Some(alias) = state.table_alias_map.get(&table_name) {
            return alias.clone();
        }
        let first = table_name.as_bytes().first().copied();
        let prefix = match first {
            Some(c) if c.is_ascii_alphabetic() => (c.to_ascii_lowercase() as char).to_string(),
            _ => "t".to_owned(),
        };
        let alias = format!("{prefix}{}_", state.table_alias_counter);
        state.table_alias_counter += 1;
        state.table_alias_map.insert(table_name, alias.clone());
        alias
    }

    fn table_name(class: &Arc<ClassMetadata>) -> Result<String, Thrown> {
        class.read().get_table_name()
    }

    fn has_relation(w: &dyn SqlWalker<'_>, dql_alias: &str) -> bool {
        w.get_query_components()
            .get(dql_alias)
            .is_some_and(|c| c.relation.is_some())
    }

    /// Generates the class table inheritance joins of a class.
    pub fn generate_class_table_inheritance_joins(
        w: &mut dyn SqlWalker<'_>,
        class: &Arc<ClassMetadata>,
        dql_alias: &str,
    ) -> Result<String, Thrown> {
        let em = w.get_entity_manager();
        let platform = em.get_database_platform();
        let quote_strategy = em.get_quote_strategy();
        let mut sql = String::new();

        let base_table_alias = w.get_sql_table_alias(&table_name(class)?, dql_alias);
        let (parent_classes, sub_classes) = {
            let info = class.read();
            (info.parent_classes.clone(), info.sub_classes.clone())
        };

        // INNER JOIN parent class tables
        for parent_class_name in &parent_classes {
            let parent_class = em.get_class_metadata(parent_class_name)?;
            let table_alias = w.get_sql_table_alias(&table_name(&parent_class)?, dql_alias);

            // If this is a joined association we must use left joins to preserve the correct result.
            sql.push_str(if has_relation(w, dql_alias) {
                " LEFT "
            } else {
                " INNER "
            });
            let _ = write!(
                sql,
                "JOIN {} {table_alias} ON ",
                quote_strategy.get_table_name(&parent_class.read(), platform)
            );

            let mut sql_parts = Vec::new();
            for column_name in quote_strategy.get_identifier_column_names(&class.read(), platform) {
                sql_parts.push(format!(
                    "{base_table_alias}.{column_name} = {table_alias}.{column_name}"
                ));
            }

            // Add filters on the root class
            sql_parts.push(generate_filter_condition_sql(
                w,
                &parent_class,
                &table_alias,
            )?);

            let sql_parts: Vec<String> = sql_parts.into_iter().filter(|p| truthy(p)).collect();
            sql.push_str(&sql_parts.join(" AND "));
        }

        // Ignore subclassing inclusion if partial objects is disallowed
        if w.get_query().get_hint(HINT_FORCE_PARTIAL_LOAD).is_truthy() {
            return Ok(sql);
        }

        // LEFT JOIN child class tables
        for sub_class_name in &sub_classes {
            let sub_class = em.get_class_metadata(sub_class_name)?;
            let table_alias = w.get_sql_table_alias(&table_name(&sub_class)?, dql_alias);

            let _ = write!(
                sql,
                " LEFT JOIN {} {table_alias} ON ",
                quote_strategy.get_table_name(&sub_class.read(), platform)
            );

            let mut sql_parts = Vec::new();
            for column_name in
                quote_strategy.get_identifier_column_names(&sub_class.read(), platform)
            {
                sql_parts.push(format!(
                    "{base_table_alias}.{column_name} = {table_alias}.{column_name}"
                ));
            }

            sql.push_str(&sql_parts.join(" AND "));
        }

        Ok(sql)
    }

    /// The ORDER BY items of the ordered collections of the selected
    /// classes.
    pub fn generate_ordered_collection_order_by_items(
        w: &mut dyn SqlWalker<'_>,
    ) -> Result<String, Thrown> {
        let em = w.get_entity_manager();
        let platform = em.get_database_platform();
        let quote_strategy = em.get_quote_strategy();
        let mut ordered_columns = Vec::new();

        let selected: Vec<SelectedClass> = w
            .walker_state()
            .selected_classes
            .values()
            .cloned()
            .collect();
        for selected_class in selected {
            let dql_alias = selected_class.dql_alias;
            let Some(q_comp) = w.get_query_components().get(&dql_alias).cloned() else {
                continue;
            };
            let Some(relation) = q_comp.relation else {
                continue;
            };
            let Some(metadata) = q_comp.metadata else {
                continue;
            };

            for (field_name, orientation) in relation.order_by.iter() {
                let field_name = String::from_utf8_lossy(&field_name.bytes()).into_owned();
                let column_name =
                    quote_strategy.get_column_name(&field_name, &metadata.read(), platform);
                let joined = metadata.read().is_inheritance_type_joined();
                let table_name = if joined {
                    utility::get_owning_table(&metadata, &field_name, em)?
                } else {
                    table_name(&metadata)?
                };

                let ordered_column = format!(
                    "{}.{column_name}",
                    w.get_sql_table_alias(&table_name, &dql_alias)
                );

                // OrderByClause should replace an ordered relation. see - DDC-2475
                if w.walker_state()
                    .ordered_columns_map
                    .contains_key(&ordered_column)
                {
                    continue;
                }

                let orientation = match orientation
                    .as_object()
                    .and_then(|o| o.downcast_ref::<EnumCase>())
                    .filter(|case| case.class() == SortDirection::CLASS)
                {
                    Some(case) if case.name() == SortDirection::Ascending.name() => {
                        "ASC".to_owned()
                    }
                    Some(_) => "DESC".to_owned(),
                    None => text(orientation),
                };
                w.walker_state_mut()
                    .ordered_columns_map
                    .insert(ordered_column.clone(), orientation.clone());
                ordered_columns.push(format!("{ordered_column} {orientation}"));
            }
        }

        Ok(ordered_columns.join(", "))
    }

    /// `$class->discriminatorValue` as an SQL value.
    fn discriminator_sql_value(
        em: &dyn QueryContext,
        discriminator_column_type: &str,
        value: &Value,
    ) -> Result<String, Thrown> {
        match value {
            Value::Int(i) if discriminator_column_type == "integer" => Ok(i.to_string()),
            other => em.quote(&text(other)),
        }
    }

    /// Generates a discriminator column SQL condition for the class with
    /// the given DQL alias.
    pub fn generate_discriminator_column_condition_sql(
        w: &mut dyn SqlWalker<'_>,
        dql_aliases: &[String],
    ) -> Result<String, Thrown> {
        let em = w.get_entity_manager();
        let mut sql_parts = Vec::new();

        for dql_alias in dql_aliases {
            let class = w.get_metadata_for_dql_alias(dql_alias)?;

            if !class.read().is_inheritance_type_single_table() {
                continue;
            }

            let sql_table_alias = if w.walker_state().use_sql_table_aliases {
                format!(
                    "{}.",
                    w.get_sql_table_alias(&table_name(&class)?, dql_alias)
                )
            } else {
                String::new()
            };

            let mut values = Vec::new();
            let (discriminator_value, discriminator_column, sub_classes) = {
                let info = class.read();
                (
                    info.discriminator_value.clone(),
                    info.get_discriminator_column()?.clone(),
                    info.sub_classes.clone(),
                )
            };

            if !matches!(discriminator_value, Value::Null) {
                // discriminators can be 0
                values.push(discriminator_sql_value(
                    em,
                    &discriminator_column.r#type,
                    &discriminator_value,
                )?);
            }

            for subclass_name in &sub_classes {
                let subclass_metadata = em.get_class_metadata(subclass_name)?;
                let (value, column_type) = {
                    let info = subclass_metadata.read();
                    if matches!(info.discriminator_value, Value::Null) {
                        continue;
                    }
                    (
                        info.discriminator_value.clone(),
                        info.get_discriminator_column()?.r#type.clone(),
                    )
                };
                values.push(discriminator_sql_value(em, &column_type, &value)?);
            }

            if values.is_empty() {
                sql_parts.push("1=0".to_owned()); // impossible condition
            } else {
                sql_parts.push(format!(
                    "{sql_table_alias}{} IN ({})",
                    discriminator_column.name,
                    values.join(", ")
                ));
            }
        }

        let sql = sql_parts.join(" AND ");
        Ok(if sql_parts.len() > 1 {
            format!("({sql})")
        } else {
            sql
        })
    }

    /// Generates the filter SQL for a given entity and table alias.
    pub fn generate_filter_condition_sql(
        w: &mut dyn SqlWalker<'_>,
        target_entity: &Arc<ClassMetadata>,
        target_table_alias: &str,
    ) -> Result<String, Thrown> {
        let em = w.get_entity_manager();
        if !em.has_filters() {
            return Ok(String::new());
        }

        let (none, joined, single_table, name, root_entity_name) = {
            let info = target_entity.read();
            (
                info.is_inheritance_type_none(),
                info.is_inheritance_type_joined(),
                info.is_inheritance_type_single_table(),
                info.name.clone(),
                info.root_entity_name.clone(),
            )
        };
        let target_entity = if none {
            target_entity.clone()
        } else if joined {
            // The classes in the inheritance will be added to the query one by one,
            // but only the root node is getting filtered
            if name != root_entity_name {
                return Ok(String::new());
            }
            target_entity.clone()
        } else if single_table {
            // With STI the table will only be queried once, make sure that the filters
            // are added to the root entity
            em.get_class_metadata(&root_entity_name)?
        } else {
            // TODO: Filtering for table per class strategy
            return Ok(String::new());
        };

        let mut filter_clauses = Vec::new();
        for filter in em.get_enabled_filters() {
            let filter_expr =
                filter.add_filter_constraint(&target_entity.read(), target_table_alias)?;
            if !filter_expr.is_empty() {
                filter_clauses.push(format!("({filter_expr})"));
            }
        }

        Ok(filter_clauses.join(" AND "))
    }

    pub fn walk_select_statement(
        w: &mut dyn SqlWalker<'_>,
        select_statement: &SelectStatement,
    ) -> Result<String, Thrown> {
        let sql = w.create_sql_for_finalizer(select_statement)?;
        let finalizer = SingleSelectSqlFinalizer::new(sql);
        finalizer.finalize_sql(w.get_query())
    }

    pub fn create_sql_for_finalizer(
        w: &mut dyn SqlWalker<'_>,
        select_statement: &SelectStatement,
    ) -> Result<String, Thrown> {
        let mut sql = w.walk_select_clause(&select_statement.select_clause)?;
        sql.push_str(&w.walk_from_clause(&select_statement.from_clause)?);
        sql.push_str(&w.walk_where_clause(select_statement.where_clause.as_ref())?);

        if let Some(group_by_clause) = &select_statement.group_by_clause {
            sql.push_str(&w.walk_group_by_clause(group_by_clause)?);
        }

        if let Some(having_clause) = &select_statement.having_clause {
            sql.push_str(&w.walk_having_clause(having_clause)?);
        }

        if let Some(order_by_clause) = &select_statement.order_by_clause {
            sql.push_str(&w.walk_order_by_clause(order_by_clause)?);
        }

        let order_by_sql = generate_ordered_collection_order_by_items(w)?;
        if select_statement.order_by_clause.is_none() && truthy(&order_by_sql) {
            let _ = write!(sql, " ORDER BY {order_by_sql}");
        }

        assert_optimistic_locking_has_all_classes_versioned(w)?;

        Ok(sql)
    }

    fn assert_optimistic_locking_has_all_classes_versioned(
        w: &mut dyn SqlWalker<'_>,
    ) -> Result<(), Thrown> {
        if lock_mode_hint(w.get_query()) == Some(LockMode::Optimistic) {
            for selected_class in w.walker_state().selected_classes.values() {
                if !selected_class.class.read().is_versioned {
                    return Err(query_exception::optimistic_lock_failed());
                }
            }
        }
        Ok(())
    }

    pub fn walk_update_statement(
        w: &mut dyn SqlWalker<'_>,
        update_statement: &UpdateStatement,
    ) -> Result<String, Thrown> {
        w.walker_state_mut().use_sql_table_aliases = false;
        rsm(w, |rsm| rsm.is_select = false);

        let mut sql = w.walk_update_clause(&update_statement.update_clause)?;
        sql.push_str(&w.walk_where_clause(update_statement.where_clause.as_ref())?);
        Ok(sql)
    }

    pub fn walk_delete_statement(
        w: &mut dyn SqlWalker<'_>,
        delete_statement: &DeleteStatement,
    ) -> Result<String, Thrown> {
        w.walker_state_mut().use_sql_table_aliases = false;
        rsm(w, |rsm| rsm.is_select = false);

        let mut sql = w.walk_delete_clause(&delete_statement.delete_clause)?;
        sql.push_str(&w.walk_where_clause(delete_statement.where_clause.as_ref())?);
        Ok(sql)
    }

    pub fn walk_entity_identification_variable(
        w: &mut dyn SqlWalker<'_>,
        ident_variable: &str,
    ) -> Result<String, Thrown> {
        let em = w.get_entity_manager();
        let class = w.get_metadata_for_dql_alias(ident_variable)?;
        let table_alias = w.get_sql_table_alias(&table_name(&class)?, ident_variable);

        let sql_parts: Vec<String> = em
            .get_quote_strategy()
            .get_identifier_column_names(&class.read(), em.get_database_platform())
            .into_iter()
            .map(|column_name| format!("{table_alias}.{column_name}"))
            .collect();

        Ok(sql_parts.join(", "))
    }

    pub fn walk_entity_as_dto_argument_expression(
        w: &mut dyn SqlWalker<'_>,
        expr: &EntityAsDtoArgumentExpression,
    ) -> Result<String, Thrown> {
        let identification_variable = expr
            .identification_variable
            .as_deref()
            .filter(|v| truthy(v));
        Ok(
            w.walk_object_expression(&expr.expression, &[], identification_variable)?
                .join(", "),
        )
    }

    pub fn walk_identification_variable(
        w: &mut dyn SqlWalker<'_>,
        identification_variable: &str,
        field_name: Option<&str>,
    ) -> Result<String, Thrown> {
        let mut class = w.get_metadata_for_dql_alias(identification_variable)?;

        if let Some(field_name) = field_name {
            let inherited = {
                let info = class.read();
                if info.is_inheritance_type_joined() {
                    info.field_mappings
                        .get(field_name)
                        .and_then(|m| m.inherited.clone())
                } else {
                    None
                }
            };
            if let Some(inherited) = inherited {
                class = w.get_entity_manager().get_class_metadata(&inherited)?;
            }
        }

        Ok(w.get_sql_table_alias(&table_name(&class)?, identification_variable))
    }

    /// `$class->associationMappings[$fieldName]`, of the class that
    /// declares it, for a single-valued association path: the column of
    /// its (single) join column.
    fn single_valued_association_column(
        w: &mut dyn SqlWalker<'_>,
        path_expr: &PathExpression,
        class: Arc<ClassMetadata>,
    ) -> Result<(Arc<ClassMetadata>, String), Thrown> {
        let field_name = path_expr.field.as_deref().unwrap_or("");
        let inherited = class
            .read()
            .association_mappings
            .get(field_name)
            .and_then(|a| a.inherited.clone());
        let class = match inherited {
            Some(inherited) => w.get_entity_manager().get_class_metadata(&inherited)?,
            None => class,
        };

        let assoc = class
            .read()
            .association_mappings
            .get(field_name)
            .cloned()
            .ok_or_else(|| method_on_null("isOwningSide"))?;

        if !assoc.is_owning_side() {
            return Err(
                query_exception::association_path_inverse_side_not_supported(
                    &path_expr.identification_variable,
                    path_expr.field.as_deref(),
                ),
            );
        }

        // COMPOSITE KEYS NOT (YET?) SUPPORTED
        if assoc.source_to_target_key_columns.len() > 1 {
            return Err(query_exception::association_path_composite_key_not_supported());
        }

        let column = assoc
            .target_to_source_key_columns
            .values()
            .next()
            .map(text)
            .unwrap_or_default();
        Ok((class, column))
    }

    pub fn walk_path_expression(
        w: &mut dyn SqlWalker<'_>,
        path_expr: &PathExpression,
    ) -> Result<String, Thrown> {
        let em = w.get_entity_manager();
        let mut sql = String::new();
        let field_name = path_expr.field.clone().unwrap_or_default();
        let dql_alias = path_expr.identification_variable.as_str();

        match path_expr.type_ {
            Some(ast::TYPE_STATE_FIELD) => {
                let class = w.get_metadata_for_dql_alias(dql_alias)?;

                if w.walker_state().use_sql_table_aliases {
                    sql.push_str(&w.walk_identification_variable(dql_alias, Some(&field_name))?);
                    sql.push('.');
                }

                sql.push_str(&em.get_quote_strategy().get_column_name(
                    &field_name,
                    &class.read(),
                    em.get_database_platform(),
                ));
            }
            Some(ast::TYPE_SINGLE_VALUED_ASSOCIATION) => {
                // 1- the owning side:
                //    Just use the foreign key, i.e. u.group_id
                let class = w.get_metadata_for_dql_alias(dql_alias)?;
                let (class, column) = single_valued_association_column(w, path_expr, class)?;

                if w.walker_state().use_sql_table_aliases {
                    sql.push_str(&w.get_sql_table_alias(&table_name(&class)?, dql_alias));
                    sql.push('.');
                }

                sql.push_str(&column);
            }
            _ => {
                return Err(query_exception::invalid_path_expression(
                    &path_expr.identification_variable,
                    path_expr.field.as_deref(),
                ));
            }
        }

        Ok(sql)
    }

    #[allow(clippy::too_many_lines, reason = "one PHP method")]
    pub fn walk_select_clause(
        w: &mut dyn SqlWalker<'_>,
        select_clause: &SelectClause,
    ) -> Result<String, Thrown> {
        let em = w.get_entity_manager();
        let platform = em.get_database_platform();
        let quote_strategy = em.get_quote_strategy();
        let query = w.get_query();

        let sql = format!(
            "SELECT {}",
            if select_clause.is_distinct {
                "DISTINCT "
            } else {
                ""
            }
        );
        let mut sql_select_expressions = Vec::new();
        for select_expression in &select_clause.select_expressions {
            let expression = w.walk_select_expression(select_expression)?;
            if truthy(&expression) {
                sql_select_expressions.push(expression);
            }
        }

        if query.get_hint(HINT_INTERNAL_ITERATION).is_true() && select_clause.is_distinct {
            query.set_hint(HINT_DISTINCT, Hint::Value(Value::Bool(true)));
        }

        let add_meta_columns = (!query.get_hint(HINT_FORCE_PARTIAL_LOAD).is_truthy()
            && query.get_hydration_mode() == HYDRATE_OBJECT)
            || query.get_hint(HINT_INCLUDE_META_COLUMNS).is_truthy();

        let selected: Vec<SelectedClass> = w
            .walker_state()
            .selected_classes
            .values()
            .cloned()
            .collect();
        for selected_class in selected {
            let class = selected_class.class;
            let dql_alias = selected_class.dql_alias.as_str();
            let result_alias = selected_class.result_alias.as_deref();
            let is_partial = selected_class.partial;
            let class_name = class.read().name.clone();

            let component = w.get_query_components().get(dql_alias).cloned();
            match component.as_ref().and_then(|c| c.relation.clone()) {
                None => rsm(w, |rsm| {
                    rsm.add_entity_result(&class_name, dql_alias, result_alias);
                    if is_partial {
                        rsm.mark_partial_entity_result(dql_alias);
                    }
                }),
                Some(relation) => {
                    let parent = component
                        .as_ref()
                        .and_then(|c| c.parent.clone())
                        .unwrap_or_default();
                    rsm(w, |rsm| {
                        rsm.add_joined_entity_result(
                            &class_name,
                            dql_alias,
                            &parent,
                            &relation.field_name,
                            is_partial,
                        );
                    });
                }
            }

            let (single_table, joined, root_entity_name, contains_foreign_identifier) = {
                let info = class.read();
                (
                    info.is_inheritance_type_single_table(),
                    info.is_inheritance_type_joined(),
                    info.root_entity_name.clone(),
                    info.contains_foreign_identifier,
                )
            };

            if single_table || joined {
                // Add discriminator columns to SQL
                let root_class = em.get_class_metadata(&root_entity_name)?;
                let tbl_alias = w.get_sql_table_alias(&table_name(&root_class)?, dql_alias);
                let discr_column = root_class.read().get_discriminator_column()?.clone();
                let column_alias = w.get_sql_column_alias(&discr_column.name);

                sql_select_expressions.push(format!(
                    "{tbl_alias}.{} AS {column_alias}",
                    discr_column.name
                ));

                // Add discriminator columns to result mapping
                rsm(w, |rsm| {
                    rsm.set_discriminator_column(dql_alias, &column_alias);
                    rsm.add_meta_result(
                        dql_alias,
                        &column_alias,
                        &discr_column.field_name,
                        false,
                        Some(&discr_column.r#type),
                    );
                    if let Some(enum_type) = discr_column.enum_type.as_deref().filter(|e| truthy(e))
                    {
                        rsm.add_enum_result(&column_alias, enum_type);
                    }
                });
            }

            // Add foreign key columns to SQL, if necessary
            if !add_meta_columns && !contains_foreign_identifier {
                continue;
            }

            // Add foreign key columns of class and also parent classes
            let associations: Vec<AssociationMapping> = class
                .read()
                .association_mappings
                .values()
                .cloned()
                .collect();
            for assoc in associations {
                if !assoc.is_to_one_owning_side() || (!add_meta_columns && assoc.id.is_none()) {
                    continue;
                }

                let target_class = em.get_class_metadata(&assoc.target_entity)?;
                let is_identifier = assoc.id == Some(true);
                let owning_class = match &assoc.inherited {
                    Some(inherited) => em.get_class_metadata(inherited)?,
                    None => class.clone(),
                };
                let sql_table_alias = w.get_sql_table_alias(&table_name(&owning_class)?, dql_alias);

                for join_column in &assoc.join_columns {
                    let column_name = &join_column.name;
                    let column_alias = w.get_sql_column_alias(column_name);
                    let column_type = utility::get_type_of_column(
                        &join_column.referenced_column_name,
                        &target_class,
                        em,
                    )?;

                    let quoted_column_name =
                        quote_strategy.get_join_column_name(join_column, &class.read(), platform);
                    sql_select_expressions.push(format!(
                        "{sql_table_alias}.{quoted_column_name} AS {column_alias}"
                    ));

                    rsm(w, |rsm| {
                        rsm.add_meta_result(
                            dql_alias,
                            &column_alias,
                            column_name,
                            is_identifier,
                            Some(&column_type),
                        );
                    });
                }
            }

            // Add foreign key columns to SQL, if necessary
            if !add_meta_columns {
                continue;
            }

            // Add foreign key columns of subclasses
            let sub_classes = class.read().sub_classes.clone();
            for sub_class_name in &sub_classes {
                let sub_class = em.get_class_metadata(sub_class_name)?;
                let sql_table_alias = w.get_sql_table_alias(&table_name(&sub_class)?, dql_alias);

                let associations: Vec<AssociationMapping> = sub_class
                    .read()
                    .association_mappings
                    .values()
                    .cloned()
                    .collect();
                for assoc in associations {
                    // Skip if association is inherited
                    if assoc.inherited.is_some() {
                        continue;
                    }

                    if assoc.is_to_one_owning_side() {
                        let target_class = em.get_class_metadata(&assoc.target_entity)?;

                        for join_column in &assoc.join_columns {
                            let column_name = &join_column.name;
                            let column_alias = w.get_sql_column_alias(column_name);
                            let column_type = utility::get_type_of_column(
                                &join_column.referenced_column_name,
                                &target_class,
                                em,
                            )?;

                            let quoted_column_name = quote_strategy.get_join_column_name(
                                join_column,
                                &sub_class.read(),
                                platform,
                            );
                            sql_select_expressions.push(format!(
                                "{sql_table_alias}.{quoted_column_name} AS {column_alias}"
                            ));

                            let is_identifier = sub_class.read().is_identifier(column_name);
                            rsm(w, |rsm| {
                                rsm.add_meta_result(
                                    dql_alias,
                                    &column_alias,
                                    column_name,
                                    is_identifier,
                                    Some(&column_type),
                                );
                            });
                        }
                    }
                }
            }
        }

        Ok(format!("{sql}{}", sql_select_expressions.join(", ")))
    }

    pub fn walk_from_clause(
        w: &mut dyn SqlWalker<'_>,
        from_clause: &FromClause,
    ) -> Result<String, Thrown> {
        let mut sql_parts = Vec::new();
        for decl in &from_clause.identification_variable_declarations {
            sql_parts.push(w.walk_identification_variable_declaration(decl)?);
        }
        Ok(format!(" FROM {}", sql_parts.join(", ")))
    }

    pub fn walk_identification_variable_declaration(
        w: &mut dyn SqlWalker<'_>,
        decl: &IdentificationVariableDeclaration,
    ) -> Result<String, Thrown> {
        let Some(range_variable_declaration) = &decl.range_variable_declaration else {
            return Err(type_error(
                "walkRangeVariableDeclaration",
                "rangeVariableDeclaration",
                "Doctrine\\ORM\\Query\\AST\\RangeVariableDeclaration",
                "null",
            ));
        };
        let mut sql = w.walk_range_variable_declaration(range_variable_declaration)?;

        if let Some(index_by) = &decl.index_by {
            w.walk_index_by(index_by)?;
        }

        for join in &decl.joins {
            sql.push_str(&w.walk_join(join)?);
        }

        Ok(sql)
    }

    pub fn walk_index_by(w: &mut dyn SqlWalker<'_>, index_by: &IndexBy) -> Result<(), Thrown> {
        let path_expression = index_by.single_valued_path_expression.borrow().clone();
        let alias = path_expression.identification_variable.clone();

        let field = match path_expression.type_ {
            Some(ast::TYPE_STATE_FIELD) => path_expression.field.clone().unwrap_or_default(),
            Some(ast::TYPE_SINGLE_VALUED_ASSOCIATION) => {
                // Just use the foreign key, i.e. u.group_id
                let class = w.get_metadata_for_dql_alias(&alias)?;
                single_valued_association_column(w, &path_expression, class)?.1
            }
            _ => {
                return Err(query_exception::invalid_path_expression(
                    &path_expression.identification_variable,
                    path_expression.field.as_deref(),
                ));
            }
        };

        if let Some(column) = w
            .walker_state()
            .scalar_fields
            .get(&(alias.clone(), field.clone()))
            .cloned()
        {
            rsm(w, |rsm| {
                rsm.add_index_by_scalar(&column);
            });
            return Ok(());
        }

        rsm(w, |rsm| {
            rsm.add_index_by(&alias, &field);
        });
        Ok(())
    }

    /// `$this->query->getHint(Query::HINT_LOCK_MODE) ?: LockMode::NONE`,
    /// passed where a `LockMode` is required.
    fn lock_mode(w: &dyn SqlWalker<'_>) -> Result<LockMode, Thrown> {
        lock_mode_hint(w.get_query()).ok_or_else(|| {
            Thrown::new(
                "TypeError",
                "Doctrine\\DBAL\\Platforms\\AbstractPlatform::appendLockHint(): Argument #2 ($lockMode) must be of type Doctrine\\DBAL\\LockMode",
                0,
            )
        })
    }

    /// Generate appropriate SQL for `RangeVariableDeclaration` AST node.
    pub fn generate_range_variable_declaration_sql(
        w: &mut dyn SqlWalker<'_>,
        range_variable_declaration: &RangeVariableDeclaration,
        build_nested_joins: bool,
    ) -> Result<String, Thrown> {
        let em = w.get_entity_manager();
        let platform = em.get_database_platform();
        let class = em.get_class_metadata(&range_variable_declaration.abstract_schema_name)?;
        let dql_alias = range_variable_declaration
            .alias_identification_variable
            .as_str();

        if range_variable_declaration.is_root {
            w.walker_state_mut().root_aliases.push(dql_alias.to_owned());
        }

        let from = format!(
            "{} {}",
            em.get_quote_strategy()
                .get_table_name(&class.read(), platform),
            w.get_sql_table_alias(&table_name(&class)?, dql_alias)
        );
        let sql = platform.append_lock_hint(&from, lock_mode(w)?);

        if !class.read().is_inheritance_type_joined() {
            return Ok(sql);
        }

        let class_table_inheritance_joins =
            generate_class_table_inheritance_joins(w, &class, dql_alias)?;

        if !build_nested_joins {
            return Ok(format!("{sql}{class_table_inheritance_joins}"));
        }

        Ok(if class_table_inheritance_joins.is_empty() {
            sql
        } else {
            format!("({sql}{class_table_inheritance_joins})")
        })
    }

    /// `$this->query->getHint('fetchMode')[$sourceEntity][$fieldName] ?? $relation->fetch`.
    fn fetch_mode(
        w: &dyn SqlWalker<'_>,
        assoc: &AssociationMapping,
        relation: &AssociationMapping,
    ) -> Value {
        if let Hint::Value(Value::Array(modes)) = w.get_query().get_hint("fetchMode")
            && let Some(Value::Array(by_field)) =
                modes.get(&Key::from(assoc.source_entity.as_str()))
            && let Some(mode) = by_field.get(&Key::from(assoc.field_name.as_str()))
            && !matches!(mode, Value::Null)
        {
            return mode.clone();
        }
        relation.fetch.map_or(Value::Null, Value::Int)
    }

    #[allow(clippy::too_many_lines, reason = "one PHP method")]
    pub fn walk_join_association_declaration(
        w: &mut dyn SqlWalker<'_>,
        join_association_declaration: &JoinAssociationDeclaration,
        join_type: i64,
        cond_expr: Option<&Node>,
    ) -> Result<String, Thrown> {
        let em = w.get_entity_manager();
        let platform = em.get_database_platform();
        let quote_strategy = em.get_quote_strategy();
        let query = w.get_query();
        let mut sql = String::new();

        let association_path_expression =
            &join_association_declaration.join_association_path_expression;
        let joined_dql_alias = join_association_declaration
            .alias_identification_variable
            .as_str();
        let index_by = &join_association_declaration.index_by;

        let relation = w
            .get_query_components()
            .get(joined_dql_alias)
            .and_then(|c| c.relation.clone())
            .ok_or_else(|| property_on_null("targetEntity"))?;

        let target_class = em.get_class_metadata(&relation.target_entity)?;
        let source_class = em.get_class_metadata(&relation.source_entity)?;
        let target_table_name = quote_strategy.get_table_name(&target_class.read(), platform);

        let target_table_alias =
            w.get_sql_table_alias(&table_name(&target_class)?, joined_dql_alias);
        let source_table_alias = w.get_sql_table_alias(
            &table_name(&source_class)?,
            &association_path_expression.identification_variable,
        );

        // Ensure we got the owning side, since it has all mapping info
        let assoc = em.get_owning_side(&relation)?;

        if query.get_hint(HINT_INTERNAL_ITERATION).is_true()
            && (!query.get_hint(HINT_DISTINCT).is_truthy()
                || w.walker_state()
                    .selected_classes
                    .contains_key(joined_dql_alias))
            && relation.is_to_many()
        {
            return Err(query_exception::iterate_with_fetch_join_not_allowed(&assoc));
        }

        let fetch_mode = fetch_mode(w, &assoc, &relation);

        if fetch_mode == Value::Int(FETCH_EAGER) && cond_expr.is_some() {
            return Err(query_exception::eager_fetch_join_with_not_allowed(
                &assoc.source_entity,
                &assoc.field_name,
            ));
        }

        let target_table_join_table;
        let target_table_join_condition;

        if assoc.is_to_one() {
            let mut conditions = Vec::new();

            for join_column in &assoc.join_columns {
                let quoted_source_column = quote_strategy.get_join_column_name(
                    join_column,
                    &target_class.read(),
                    platform,
                );
                let quoted_target_column = quote_strategy.get_referenced_join_column_name(
                    join_column,
                    &target_class.read(),
                    platform,
                );

                if relation.is_owning_side() {
                    conditions.push(format!(
                        "{source_table_alias}.{quoted_source_column} = {target_table_alias}.{quoted_target_column}"
                    ));
                    continue;
                }

                conditions.push(format!(
                    "{source_table_alias}.{quoted_target_column} = {target_table_alias}.{quoted_source_column}"
                ));
            }

            // Apply remaining inheritance restrictions
            let discr_sql =
                generate_discriminator_column_condition_sql(w, &[joined_dql_alias.to_owned()])?;
            if truthy(&discr_sql) {
                conditions.push(discr_sql);
            }

            // Apply the filters
            let filter_expr = generate_filter_condition_sql(w, &target_class, &target_table_alias)?;
            if truthy(&filter_expr) {
                conditions.push(filter_expr);
            }

            target_table_join_table = format!("{target_table_name} {target_table_alias}");
            target_table_join_condition = conditions.join(" AND ");
        } else if assoc.is_many_to_many() {
            // Join relation table
            let join_table = assoc
                .join_table
                .as_ref()
                .ok_or_else(|| property_on_null("name"))?;
            let join_table_alias = w.get_sql_table_alias(&join_table.name, joined_dql_alias);
            let join_table_name =
                quote_strategy.get_join_table_name(&assoc, &source_class.read(), platform);

            let mut conditions = Vec::new();
            let relation_columns = if relation.is_owning_side() {
                &join_table.join_columns
            } else {
                &join_table.inverse_join_columns
            };

            for join_column in relation_columns {
                let quoted_source_column = quote_strategy.get_join_column_name(
                    join_column,
                    &target_class.read(),
                    platform,
                );
                let quoted_target_column = quote_strategy.get_referenced_join_column_name(
                    join_column,
                    &target_class.read(),
                    platform,
                );

                conditions.push(format!(
                    "{source_table_alias}.{quoted_target_column} = {join_table_alias}.{quoted_source_column}"
                ));
            }

            let _ = write!(
                sql,
                "{join_table_name} {join_table_alias} ON {}",
                conditions.join(" AND ")
            );

            // Join target table
            sql.push_str(
                if join_type == ast::JOIN_TYPE_LEFT || join_type == ast::JOIN_TYPE_LEFTOUTER {
                    " LEFT JOIN "
                } else {
                    " INNER JOIN "
                },
            );

            let mut conditions = Vec::new();
            let relation_columns = if relation.is_owning_side() {
                &join_table.inverse_join_columns
            } else {
                &join_table.join_columns
            };

            for join_column in relation_columns {
                let quoted_source_column = quote_strategy.get_join_column_name(
                    join_column,
                    &target_class.read(),
                    platform,
                );
                let quoted_target_column = quote_strategy.get_referenced_join_column_name(
                    join_column,
                    &target_class.read(),
                    platform,
                );

                conditions.push(format!(
                    "{target_table_alias}.{quoted_target_column} = {join_table_alias}.{quoted_source_column}"
                ));
            }

            // Apply remaining inheritance restrictions
            let discr_sql =
                generate_discriminator_column_condition_sql(w, &[joined_dql_alias.to_owned()])?;
            if truthy(&discr_sql) {
                conditions.push(discr_sql);
            }

            // Apply the filters
            let filter_expr = generate_filter_condition_sql(w, &target_class, &target_table_alias)?;
            if truthy(&filter_expr) {
                conditions.push(filter_expr);
            }

            target_table_join_table = format!("{target_table_name} {target_table_alias}");
            target_table_join_condition = conditions.join(" AND ");
        } else {
            return Err(Thrown::new(
                "BadMethodCallException",
                "Type of association must be one of *_TO_ONE or MANY_TO_MANY",
                0,
            ));
        }

        // Handle WITH clause
        let with_condition = match cond_expr {
            None => String::new(),
            Some(cond_expr) => format!("({})", w.walk_conditional_expression(cond_expr)?),
        };

        if target_class.read().is_inheritance_type_joined() {
            let cti_joins =
                generate_class_table_inheritance_joins(w, &target_class, joined_dql_alias)?;
            // If we have WITH condition, we need to build nested joins for target class table and cti joins
            if truthy(&with_condition) && truthy(&cti_joins) {
                let _ = write!(
                    sql,
                    "({target_table_join_table}{cti_joins}) ON {target_table_join_condition}"
                );
            } else {
                let _ = write!(
                    sql,
                    "{target_table_join_table} ON {target_table_join_condition}{cti_joins}"
                );
            }
        } else {
            let _ = write!(
                sql,
                "{target_table_join_table} ON {target_table_join_condition}"
            );
        }

        if truthy(&with_condition) {
            let _ = write!(sql, " AND {with_condition}");
        }

        // Apply the indexes
        if let Some(index_by) = index_by {
            // For Many-To-One or One-To-One associations this obviously makes no sense, but is ignored silently.
            w.walk_index_by(index_by)?;
        } else if relation.is_indexed() {
            let index_by = relation.index_by()?.to_owned();
            rsm(w, |rsm| {
                rsm.add_index_by(joined_dql_alias, &index_by);
            });
        }

        Ok(sql)
    }

    pub fn walk_order_by_clause(
        w: &mut dyn SqlWalker<'_>,
        order_by_clause: &OrderByClause,
    ) -> Result<String, Thrown> {
        let mut order_by_items = Vec::new();
        for item in &order_by_clause.order_by_items {
            order_by_items.push(w.walk_order_by_item(item)?);
        }

        if order_by_clause.include_collection_order_by_items {
            let collection_order_by_items = generate_ordered_collection_order_by_items(w)?;
            if !collection_order_by_items.is_empty() {
                order_by_items.push(collection_order_by_items);
            }
        }

        Ok(format!(" ORDER BY {}", order_by_items.join(", ")))
    }

    /// `$this->queryComponents[$name]['token']->value`.
    fn component_token_value(w: &dyn SqlWalker<'_>, name: &str) -> Result<String, Thrown> {
        w.get_query_components()
            .get(name)
            .and_then(|c| c.token.as_ref())
            .map(|t| t.value.to_string_lossy().into_owned())
            .ok_or_else(|| property_on_null("value"))
    }

    pub fn walk_order_by_item(
        w: &mut dyn SqlWalker<'_>,
        order_by_item: &OrderByItem,
    ) -> Result<String, Thrown> {
        let type_ = order_by_item.type_.to_ascii_uppercase();
        let expr = &order_by_item.expression;
        let sql = match expr {
            Node::Str(name) => {
                let value = component_token_value(w, name)?;
                w.walk_result_variable(&value)?
            }
            node => dispatch(w, node)?,
        };

        w.walker_state_mut()
            .ordered_columns_map
            .insert(sql.clone(), type_.clone());

        if let Node::Subselect(_) = expr {
            return Ok(format!("({sql}) {type_}"));
        }

        Ok(format!("{sql} {type_}"))
    }

    pub fn walk_join(w: &mut dyn SqlWalker<'_>, join: &Join) -> Result<String, Thrown> {
        let em = w.get_entity_manager();
        let join_type = join.join_type;
        let left = join_type == ast::JOIN_TYPE_LEFT || join_type == ast::JOIN_TYPE_LEFTOUTER;

        let mut sql = if left {
            " LEFT JOIN ".to_owned()
        } else {
            " INNER JOIN ".to_owned()
        };

        match &join.join_association_declaration {
            Some(Node::RangeVariableDeclaration(join_declaration)) => {
                let class = em.get_class_metadata(&join_declaration.abstract_schema_name)?;
                let dql_alias = join_declaration.alias_identification_variable.as_str();
                let table_alias = w.get_sql_table_alias(&table_name(&class)?, dql_alias);
                let mut conditions = Vec::new();

                if let Some(cond) = &join.conditional_expression {
                    conditions.push(format!("({})", w.walk_conditional_expression(cond)?));
                }

                let is_unconditional_join = conditions.is_empty();
                let cond_expr_conjunction = if class.read().is_inheritance_type_joined()
                    && !left
                    && is_unconditional_join
                {
                    " AND "
                } else {
                    " ON "
                };

                sql.push_str(&generate_range_variable_declaration_sql(
                    w,
                    join_declaration,
                    !is_unconditional_join,
                )?);

                // Apply remaining inheritance restrictions
                let discr_sql =
                    generate_discriminator_column_condition_sql(w, &[dql_alias.to_owned()])?;
                if truthy(&discr_sql) {
                    conditions.push(discr_sql);
                }

                // Apply the filters
                let filter_expr = generate_filter_condition_sql(w, &class, &table_alias)?;
                if truthy(&filter_expr) {
                    conditions.push(filter_expr);
                }

                if !conditions.is_empty() {
                    sql.push_str(cond_expr_conjunction);
                    sql.push_str(&conditions.join(" AND "));
                }
            }
            Some(Node::JoinAssociationDeclaration(join_declaration)) => {
                sql.push_str(&w.walk_join_association_declaration(
                    join_declaration,
                    join_type,
                    join.conditional_expression.as_ref(),
                )?);
            }
            _ => {}
        }

        Ok(sql)
    }

    pub fn walk_coalesce_expression(
        w: &mut dyn SqlWalker<'_>,
        coalesce_expression: &CoalesceExpression,
    ) -> Result<String, Thrown> {
        let mut scalar_expressions = Vec::new();
        for scalar_expression in &coalesce_expression.scalar_expressions {
            scalar_expressions.push(w.walk_simple_arithmetic_expression(scalar_expression)?);
        }
        Ok(format!("COALESCE({})", scalar_expressions.join(", ")))
    }

    fn null_if_operand(w: &mut dyn SqlWalker<'_>, expression: &Node) -> Result<String, Thrown> {
        match expression {
            Node::Str(value) => w.get_entity_manager().quote(value),
            other => w.walk_simple_arithmetic_expression(other),
        }
    }

    pub fn walk_null_if_expression(
        w: &mut dyn SqlWalker<'_>,
        null_if_expression: &NullIfExpression,
    ) -> Result<String, Thrown> {
        let first = null_if_operand(w, &null_if_expression.first_expression)?;
        let second = null_if_operand(w, &null_if_expression.second_expression)?;
        Ok(format!("NULLIF({first}, {second})"))
    }

    pub fn walk_general_case_expression(
        w: &mut dyn SqlWalker<'_>,
        general_case_expression: &GeneralCaseExpression,
    ) -> Result<String, Thrown> {
        let mut sql = "CASE".to_owned();
        for when_clause in &general_case_expression.when_clauses {
            let _ = write!(
                sql,
                " WHEN {}",
                w.walk_conditional_expression(&when_clause.case_condition_expression)?
            );
            let _ = write!(
                sql,
                " THEN {}",
                w.walk_simple_arithmetic_expression(&when_clause.then_scalar_expression)?
            );
        }
        let _ = write!(
            sql,
            " ELSE {} END",
            w.walk_simple_arithmetic_expression(&general_case_expression.else_scalar_expression)?
        );
        Ok(sql)
    }

    pub fn walk_simple_case_expression(
        w: &mut dyn SqlWalker<'_>,
        simple_case_expression: &SimpleCaseExpression,
    ) -> Result<String, Thrown> {
        let Some(case_operand) = &simple_case_expression.case_operand else {
            return Err(type_error(
                "walkStateFieldPathExpression",
                "stateFieldPathExpression",
                "Doctrine\\ORM\\Query\\AST\\PathExpression",
                "null",
            ));
        };
        let case_operand = case_operand.borrow().clone();
        let mut sql = format!(
            "CASE {}",
            w.walk_state_field_path_expression(&case_operand)?
        );
        for when_clause in &simple_case_expression.simple_when_clauses {
            let _ = write!(
                sql,
                " WHEN {}",
                w.walk_simple_arithmetic_expression(&when_clause.case_scalar_expression)?
            );
            let _ = write!(
                sql,
                " THEN {}",
                w.walk_simple_arithmetic_expression(&when_clause.then_scalar_expression)?
            );
        }
        let _ = write!(
            sql,
            " ELSE {} END",
            w.walk_simple_arithmetic_expression(&simple_case_expression.else_scalar_expression)?
        );
        Ok(sql)
    }

    /// `$selectExpression->fieldIdentificationVariable ?: $this->scalarResultCounter++`.
    fn result_alias_or_counter(w: &mut dyn SqlWalker<'_>, alias: Option<&str>) -> Key {
        if let Some(alias) = alias.filter(|a| truthy(a)) {
            Key::from(alias)
        } else {
            let state = w.walker_state_mut();
            let counter = state.scalar_result_counter;
            state.scalar_result_counter += 1;
            Key::Int(counter)
        }
    }

    fn convert_to_php_value_sql(
        w: &dyn SqlWalker<'_>,
        type_name: &str,
        column: &str,
    ) -> Result<String, Thrown> {
        let type_ =
            php2rust_doctrine_dbal::types::get_type(type_name).map_err(|e| dbal_error(e.into()))?;
        let platform = w.get_entity_manager().get_database_platform();
        Ok(type_
            .convert_to_php_value_sql(column.as_bytes(), platform)
            .to_string_lossy()
            .into_owned())
    }

    #[allow(clippy::too_many_lines, reason = "one PHP method")]
    pub fn walk_select_expression(
        w: &mut dyn SqlWalker<'_>,
        select_expression: &SelectExpression,
    ) -> Result<String, Thrown> {
        let em = w.get_entity_manager();
        let mut sql = String::new();
        let expr = &select_expression.expression;
        let hidden = select_expression.hidden_alias_result_variable;
        let field_identification_variable =
            select_expression.field_identification_variable.as_deref();

        match expr {
            Node::PathExpression(path) => {
                let path = path.borrow().clone();
                if path.type_ != Some(ast::TYPE_STATE_FIELD) {
                    return Err(query_exception::invalid_path_expression(
                        &path.identification_variable,
                        path.field.as_deref(),
                    ));
                }

                let field_name = path.field.clone().unwrap_or_default();
                let dql_alias = path.identification_variable.as_str();
                let class = w.get_metadata_for_dql_alias(dql_alias)?;

                let result_alias = field_identification_variable
                    .filter(|a| truthy(a))
                    .unwrap_or(&field_name)
                    .to_owned();
                let table_name = if class.read().is_inheritance_type_joined() {
                    utility::get_owning_table(&class, &field_name, em)?
                } else {
                    table_name(&class)?
                };

                let sql_table_alias = w.get_sql_table_alias(&table_name, dql_alias);
                let field_mapping = class
                    .read()
                    .field_mappings
                    .get(&field_name)
                    .cloned()
                    .ok_or_else(|| property_on_null("columnName"))?;
                let column_name = em.get_quote_strategy().get_column_name(
                    &field_name,
                    &class.read(),
                    em.get_database_platform(),
                );
                let column_alias = w.get_sql_column_alias(&field_mapping.column_name);

                let col = format!("{sql_table_alias}.{column_name}");
                let col = convert_to_php_value_sql(w, &field_mapping.r#type, &col)?;

                let _ = write!(sql, "{col} AS {column_alias}");

                w.walker_state_mut().scalar_result_alias_map.insert(
                    Key::from(result_alias.as_str()),
                    ResultAlias::One(column_alias.clone()),
                );

                if !hidden {
                    rsm(w, |rsm| {
                        rsm.add_scalar_result(
                            &column_alias,
                            Key::from(result_alias.as_str()),
                            &field_mapping.r#type,
                        );
                    });
                    w.walker_state_mut().scalar_fields.insert(
                        (dql_alias.to_owned(), field_name.clone()),
                        column_alias.clone(),
                    );

                    if let Some(enum_type) =
                        field_mapping.enum_type.as_deref().filter(|e| truthy(e))
                    {
                        rsm(w, |rsm| {
                            rsm.add_enum_result(&column_alias, enum_type);
                        });
                    }
                }
            }

            Node::AggregateExpression(_)
            | Node::Function(_)
            | Node::SimpleArithmeticExpression(_)
            | Node::ArithmeticTerm(_)
            | Node::ArithmeticFactor(_)
            | Node::ParenthesisExpression(_)
            | Node::Literal(_)
            | Node::NullIfExpression(_)
            | Node::CoalesceExpression(_)
            | Node::GeneralCaseExpression(_)
            | Node::SimpleCaseExpression(_) => {
                let column_alias = w.get_sql_column_alias("sclr");
                let result_alias = result_alias_or_counter(w, field_identification_variable);

                let _ = write!(sql, "{} AS {column_alias}", dispatch(w, expr)?);

                w.walker_state_mut()
                    .scalar_result_alias_map
                    .insert(result_alias.clone(), ResultAlias::One(column_alias.clone()));

                if hidden {
                    return Ok(sql);
                }

                if let Node::Function(function) = expr {
                    if let Some(type_name) = function.return_type_name() {
                        rsm(w, |rsm| {
                            rsm.add_scalar_result(&column_alias, result_alias.clone(), &type_name);
                        });
                        return Ok(sql);
                    }

                    if let Some(type_) = function.return_type() {
                        deprecations::trigger("https://github.com/doctrine/orm/pull/12543", || {
                            "Implementing Doctrine\\ORM\\Query\\AST\\TypedExpression is deprecated, implement Doctrine\\ORM\\Query\\AST\\ExpressionWithReturnType instead.".to_owned()
                        });
                        let type_ = type_?;
                        let type_name = php2rust_doctrine_dbal::types::lookup_name(&type_)
                            .map_err(|e| dbal_error(e.into()))?;
                        rsm(w, |rsm| {
                            rsm.add_scalar_result(&column_alias, result_alias.clone(), &type_name);
                        });
                        return Ok(sql);
                    }
                }

                rsm(w, |rsm| {
                    rsm.add_scalar_result(&column_alias, result_alias.clone(), "string");
                });
            }

            Node::Subselect(subselect) => {
                let column_alias = w.get_sql_column_alias("sclr");
                let result_alias = result_alias_or_counter(w, field_identification_variable);

                let _ = write!(sql, "({}) AS {column_alias}", w.walk_subselect(subselect)?);

                w.walker_state_mut()
                    .scalar_result_alias_map
                    .insert(result_alias.clone(), ResultAlias::One(column_alias.clone()));

                if !hidden {
                    rsm(w, |rsm| {
                        rsm.add_scalar_result(&column_alias, result_alias.clone(), "string");
                    });
                }
            }

            Node::NewObjectExpression(new_object) => {
                let new_object = new_object.borrow().clone();
                sql.push_str(&w.walk_new_object(&new_object, field_identification_variable)?);
            }

            Node::PartialObjectExpression(partial) => {
                w.get_query()
                    .set_hint(HINT_PARTIAL, Hint::Value(Value::Bool(true)));
                let parts = w.walk_object_expression(
                    &partial.identification_variable,
                    &partial.partial_field_set,
                    field_identification_variable.filter(|a| truthy(a)),
                )?;
                sql.push_str(&parts.join(", "));
            }

            Node::Str(dql_alias) => {
                let parts = w.walk_object_expression(
                    dql_alias,
                    &[],
                    field_identification_variable.filter(|a| truthy(a)),
                )?;
                sql.push_str(&parts.join(", "));
            }

            other => {
                return Err(node_type_error(
                    "walkObjectExpression",
                    "dqlAlias",
                    "string",
                    other,
                ));
            }
        }

        Ok(sql)
    }

    /// `$this->scalarResultAliasMap[$resultAlias][] = $columnAlias`.
    fn push_result_alias(
        w: &mut dyn SqlWalker<'_>,
        result_alias: &str,
        column_alias: &str,
    ) -> Result<(), Thrown> {
        let map = &mut w.walker_state_mut().scalar_result_alias_map;
        match map
            .entry(Key::from(result_alias))
            .or_insert_with(|| ResultAlias::Many(Vec::new()))
        {
            ResultAlias::Many(aliases) => {
                aliases.push(column_alias.to_owned());
                Ok(())
            }
            ResultAlias::One(_) => Err(Thrown::new(
                "Error",
                "[] operator not supported for strings",
                0,
            )),
        }
    }

    pub fn walk_object_expression(
        w: &mut dyn SqlWalker<'_>,
        dql_alias: &str,
        partial_field_set: &[String],
        result_alias: Option<&str>,
    ) -> Result<Vec<String>, Thrown> {
        let em = w.get_entity_manager();
        let platform = em.get_database_platform();
        let quote_strategy = em.get_quote_strategy();
        let class = w.get_metadata_for_dql_alias(dql_alias)?;
        let class_name = class.read().name.clone();

        if !w.walker_state().selected_classes.contains_key(dql_alias) {
            w.walker_state_mut().selected_classes.insert(
                dql_alias.to_owned(),
                SelectedClass {
                    class: class.clone(),
                    dql_alias: dql_alias.to_owned(),
                    result_alias: result_alias.map(str::to_owned),
                    partial: !partial_field_set.is_empty(),
                },
            );
        }

        let mut sql_parts = Vec::new();

        // Select all fields from the queried class
        let field_mappings: Vec<_> = class
            .read()
            .field_mappings
            .iter()
            .map(|(name, mapping)| (name.to_owned(), mapping.clone()))
            .collect();
        for (field_name, mapping) in field_mappings {
            if !partial_field_set.is_empty() && !partial_field_set.contains(&field_name) {
                continue;
            }

            let table_name = match &mapping.inherited {
                Some(inherited) => table_name(&em.get_class_metadata(inherited)?)?,
                None => table_name(&class)?,
            };

            let sql_table_alias = w.get_sql_table_alias(&table_name, dql_alias);
            let column_alias = w.get_sql_column_alias(&mapping.column_name);
            let quoted_column_name =
                quote_strategy.get_column_name(&field_name, &class.read(), platform);

            let col = format!("{sql_table_alias}.{quoted_column_name}");
            let col = convert_to_php_value_sql(w, &mapping.r#type, &col)?;

            sql_parts.push(format!("{col} AS {column_alias}"));

            if let Some(result_alias) = result_alias {
                push_result_alias(w, result_alias, &column_alias)?;
            }

            rsm(w, |rsm| {
                rsm.add_field_result(dql_alias, &column_alias, &field_name, Some(&class_name));
                if let Some(enum_type) = mapping.enum_type.as_deref().filter(|e| truthy(e)) {
                    rsm.add_enum_result(&column_alias, enum_type);
                }
            });
        }

        // Add any additional fields of subclasses (excluding inherited fields)
        // 1) on Single Table Inheritance: always, since its marginal overhead
        // 2) on Class Table Inheritance only if partial objects are disallowed,
        //    since it requires outer joining subtables.
        if class.read().is_inheritance_type_single_table()
            || !w.get_query().get_hint(HINT_FORCE_PARTIAL_LOAD).is_truthy()
        {
            let sub_classes = class.read().sub_classes.clone();
            for sub_class_name in &sub_classes {
                let sub_class = em.get_class_metadata(sub_class_name)?;
                let sql_table_alias = w.get_sql_table_alias(&table_name(&sub_class)?, dql_alias);

                let field_mappings: Vec<_> = sub_class
                    .read()
                    .field_mappings
                    .iter()
                    .map(|(name, mapping)| (name.to_owned(), mapping.clone()))
                    .collect();
                for (field_name, mapping) in field_mappings {
                    if mapping.inherited.is_some()
                        || (!partial_field_set.is_empty()
                            && !partial_field_set.contains(&field_name))
                    {
                        continue;
                    }

                    let column_alias = w.get_sql_column_alias(&mapping.column_name);
                    let quoted_column_name =
                        quote_strategy.get_column_name(&field_name, &sub_class.read(), platform);

                    let col = format!("{sql_table_alias}.{quoted_column_name}");
                    let col = convert_to_php_value_sql(w, &mapping.r#type, &col)?;

                    sql_parts.push(format!("{col} AS {column_alias}"));

                    if let Some(result_alias) = result_alias {
                        push_result_alias(w, result_alias, &column_alias)?;
                    }

                    rsm(w, |rsm| {
                        rsm.add_field_result(
                            dql_alias,
                            &column_alias,
                            &field_name,
                            Some(sub_class_name),
                        );
                    });
                }
            }
        }

        Ok(sql_parts)
    }

    pub fn walk_subselect(
        w: &mut dyn SqlWalker<'_>,
        subselect: &Subselect,
    ) -> Result<String, Thrown> {
        let use_aliases_before = w.walker_state().use_sql_table_aliases;
        let root_aliases_before = std::mem::take(&mut w.walker_state_mut().root_aliases);

        // reset the rootAliases for the subselect
        w.walker_state_mut().use_sql_table_aliases = true;

        let mut sql = w.walk_simple_select_clause(&subselect.simple_select_clause)?;
        sql.push_str(&w.walk_subselect_from_clause(&subselect.subselect_from_clause)?);
        sql.push_str(&w.walk_where_clause(subselect.where_clause.as_ref())?);

        if let Some(group_by_clause) = &subselect.group_by_clause {
            sql.push_str(&w.walk_group_by_clause(group_by_clause)?);
        }
        if let Some(having_clause) = &subselect.having_clause {
            sql.push_str(&w.walk_having_clause(having_clause)?);
        }
        if let Some(order_by_clause) = &subselect.order_by_clause {
            sql.push_str(&w.walk_order_by_clause(order_by_clause)?);
        }

        // put the main aliases back
        let state = w.walker_state_mut();
        state.root_aliases = root_aliases_before;
        state.use_sql_table_aliases = use_aliases_before;

        Ok(sql)
    }

    fn arg_entry(entries: &[(&str, Value)]) -> Value {
        let mut array = PhpArray::new();
        for (key, value) in entries {
            array.insert(*key, value.clone());
        }
        Value::Array(array)
    }

    #[allow(clippy::too_many_lines, reason = "one PHP method")]
    pub fn walk_new_object(
        w: &mut dyn SqlWalker<'_>,
        new_object_expression: &NewObjectExpression,
        new_object_result_alias: Option<&str>,
    ) -> Result<String, Thrown> {
        let mut sql_select_expressions = Vec::new();
        let obj_index = if let Some(alias) = new_object_result_alias.filter(|a| truthy(a)) {
            Key::from(alias)
        } else {
            let state = w.walker_state_mut();
            let counter = state.new_object_counter;
            state.new_object_counter += 1;
            Key::Int(counter)
        };

        for (arg_index, e) in &new_object_expression.args {
            let result_alias = {
                let state = w.walker_state_mut();
                let counter = state.scalar_result_counter;
                state.scalar_result_counter += 1;
                counter
            };
            let column_alias = w.get_sql_column_alias("sclr");
            let mut field_type = "string".to_owned();

            match e {
                Node::NewObjectExpression(nested) => {
                    let nested = nested.borrow().clone();
                    sql_select_expressions.push(w.walk_new_object(&nested, Some(&column_alias))?);
                    let entry = arg_entry(&[
                        ("ownerIndex", obj_index.to_value()),
                        ("argIndex", arg_index.to_value()),
                        ("argAlias", Value::from(column_alias.as_str())),
                    ]);
                    rsm(w, |rsm| {
                        rsm.nested_new_object_arguments
                            .insert(column_alias.as_str(), entry);
                    });
                }

                Node::Subselect(_) => {
                    sql_select_expressions.push(format!("({}) AS {column_alias}", dispatch(w, e)?));
                }

                Node::PathExpression(path) => {
                    let path = path.borrow().clone();
                    let dql_alias = path.identification_variable.as_str();
                    let class = w.get_metadata_for_dql_alias(dql_alias)?;
                    let field_name = path.field.clone().unwrap_or_default();
                    let field_mapping = class
                        .read()
                        .field_mappings
                        .get(&field_name)
                        .cloned()
                        .ok_or_else(|| property_on_null("type"))?;
                    field_type.clone_from(&field_mapping.r#type);
                    let col = w.walk_path_expression(&path)?.trim().to_owned();

                    let col = convert_to_php_value_sql(w, &field_type, &col)?;

                    sql_select_expressions.push(format!("{col} AS {column_alias}"));

                    if let Some(enum_type) =
                        field_mapping.enum_type.as_deref().filter(|e| truthy(e))
                    {
                        rsm(w, |rsm| {
                            rsm.add_enum_result(&column_alias, enum_type);
                        });
                    }
                }

                Node::Literal(literal) => {
                    match literal.type_ {
                        ast::LITERAL_BOOLEAN => "boolean".clone_into(&mut field_type),
                        // The value of a numeric literal is the token's
                        // text, which is_float() never accepts.
                        ast::LITERAL_NUMERIC => "integer".clone_into(&mut field_type),
                        _ => {}
                    }

                    sql_select_expressions
                        .push(format!("{} AS {column_alias}", dispatch(w, e)?.trim()));
                }

                Node::EntityAsDtoArgumentExpression(dto) => {
                    let alias = dto
                        .identification_variable
                        .clone()
                        .filter(|v| truthy(v))
                        .unwrap_or_else(|| column_alias.clone());
                    let argument = arg_entry(&[
                        ("ownerIndex", obj_index.to_value()),
                        ("argIndex", arg_index.to_value()),
                        ("argAlias", Value::from(alias.as_str())),
                    ]);
                    let entity = arg_entry(&[
                        ("parent", obj_index.to_value()),
                        ("argIndex", arg_index.to_value()),
                        ("type", Value::from("entity")),
                    ]);
                    rsm(w, |rsm| {
                        rsm.nested_new_object_arguments
                            .insert(column_alias.as_str(), argument);
                        rsm.nested_entities.insert(alias.as_str(), entity);
                    });
                    sql_select_expressions.push(dispatch(w, e)?.trim().to_owned());
                }

                _ => {
                    sql_select_expressions
                        .push(format!("{} AS {column_alias}", dispatch(w, e)?.trim()));
                }
            }

            w.walker_state_mut().scalar_result_alias_map.insert(
                Key::Int(result_alias),
                ResultAlias::One(column_alias.clone()),
            );
            let mapping = arg_entry(&[
                ("objIndex", obj_index.to_value()),
                ("argIndex", arg_index.to_value()),
            ]);
            rsm(w, |rsm| {
                rsm.add_scalar_result(&column_alias, Key::Int(result_alias), &field_type);
                rsm.new_object_mappings
                    .insert(column_alias.as_str(), mapping);
            });
        }

        let class_name = Value::from(new_object_expression.class_name.as_str());
        rsm(w, |rsm| rsm.new_object.insert(obj_index, class_name));

        Ok(sql_select_expressions.join(", "))
    }

    pub fn walk_simple_select_expression(
        w: &mut dyn SqlWalker<'_>,
        simple_select_expression: &SimpleSelectExpression,
    ) -> Result<String, Thrown> {
        let expr = &simple_select_expression.expression;
        let mut sql = " ".to_owned();
        let field_identification_variable = simple_select_expression
            .field_identification_variable
            .as_deref();

        match expr {
            Node::PathExpression(path) => {
                let path = path.borrow().clone();
                sql.push_str(&w.walk_path_expression(&path)?);
            }

            Node::Subselect(subselect) => {
                let alias = result_alias_or_counter(w, field_identification_variable);

                let column_alias = {
                    let state = w.walker_state_mut();
                    let counter = state.alias_counter;
                    state.alias_counter += 1;
                    format!("sclr{counter}")
                };
                w.walker_state_mut()
                    .scalar_result_alias_map
                    .insert(alias, ResultAlias::One(column_alias.clone()));

                let _ = write!(sql, "({}) AS {column_alias}", w.walk_subselect(subselect)?);
            }

            Node::Function(_)
            | Node::SimpleArithmeticExpression(_)
            | Node::ArithmeticTerm(_)
            | Node::ArithmeticFactor(_)
            | Node::Literal(_)
            | Node::NullIfExpression(_)
            | Node::CoalesceExpression(_)
            | Node::GeneralCaseExpression(_)
            | Node::SimpleCaseExpression(_) => {
                let alias = result_alias_or_counter(w, field_identification_variable);

                let column_alias = w.get_sql_column_alias("sclr");
                w.walker_state_mut()
                    .scalar_result_alias_map
                    .insert(alias, ResultAlias::One(column_alias.clone()));

                let _ = write!(sql, "{} AS {column_alias}", dispatch(w, expr)?);
            }

            Node::ParenthesisExpression(expression) => {
                sql.push_str(&w.walk_parenthesis_expression(expression)?);
            }

            // IdentificationVariable
            Node::Str(identification_variable) => {
                sql.push_str(&w.walk_entity_identification_variable(identification_variable)?);
            }

            other => {
                return Err(node_type_error(
                    "walkEntityIdentificationVariable",
                    "identVariable",
                    "string",
                    other,
                ));
            }
        }

        Ok(sql)
    }

    /// `$resultVariable->pathExpression` of a result variable used as a
    /// `GroupByItem`.
    fn group_by_path_expression(result_variable: &Node) -> Option<Node> {
        match result_variable {
            Node::AggregateExpression(aggregate) => Some(aggregate.path_expression.clone()),
            Node::Function(function) => function.path_expression().map(Node::PathExpression),
            _ => None,
        }
    }

    fn walk_path_node(w: &mut dyn SqlWalker<'_>, node: &Node) -> Result<String, Thrown> {
        match node {
            Node::PathExpression(path) => {
                let path = path.borrow().clone();
                w.walk_path_expression(&path)
            }
            other => Err(node_type_error(
                "walkPathExpression",
                "pathExpr",
                "Doctrine\\ORM\\Query\\AST\\PathExpression",
                other,
            )),
        }
    }

    pub fn walk_group_by_item(
        w: &mut dyn SqlWalker<'_>,
        group_by_item: &Node,
    ) -> Result<String, Thrown> {
        // StateFieldPathExpression
        let Node::Str(group_by_item) = group_by_item else {
            return walk_path_node(w, group_by_item);
        };

        // ResultVariable
        if let Some(result_variable) = w
            .get_query_components()
            .get(group_by_item)
            .and_then(|c| c.result_variable.clone())
        {
            if let Node::PathExpression(_) = result_variable {
                return walk_path_node(w, &result_variable);
            }

            if let Some(path_expression) = group_by_path_expression(&result_variable) {
                return walk_path_node(w, &path_expression);
            }

            return w.walk_result_variable(group_by_item);
        }

        // IdentificationVariable
        let class = w.get_metadata_for_dql_alias(group_by_item)?;
        let (field_names, association_mappings) = {
            let info = class.read();
            (
                info.field_names.values().cloned().collect::<Vec<_>>(),
                info.association_mappings
                    .values()
                    .cloned()
                    .collect::<Vec<_>>(),
            )
        };
        let mut sql_parts = Vec::new();

        for field in field_names {
            let mut item = PathExpression::new(ast::TYPE_STATE_FIELD, group_by_item, Some(&field));
            item.type_ = Some(ast::TYPE_STATE_FIELD);

            sql_parts.push(w.walk_path_expression(&item)?);
        }

        for mapping in association_mappings {
            if mapping.is_to_one_owning_side() {
                let mut item = PathExpression::new(
                    ast::TYPE_SINGLE_VALUED_ASSOCIATION,
                    group_by_item,
                    Some(&mapping.field_name),
                );
                item.type_ = Some(ast::TYPE_SINGLE_VALUED_ASSOCIATION);

                sql_parts.push(w.walk_path_expression(&item)?);
            }
        }

        Ok(sql_parts.join(", "))
    }

    pub fn walk_delete_clause(
        w: &mut dyn SqlWalker<'_>,
        delete_clause: &DeleteClause,
    ) -> Result<String, Thrown> {
        let em = w.get_entity_manager();
        let class = em.get_class_metadata(&delete_clause.abstract_schema_name)?;
        let table_name = table_name(&class)?;
        let sql = format!(
            "DELETE FROM {}",
            em.get_quote_strategy()
                .get_table_name(&class.read(), em.get_database_platform())
        );

        w.set_sql_table_alias(
            &table_name,
            &table_name,
            &delete_clause.alias_identification_variable,
        );
        w.walker_state_mut()
            .root_aliases
            .push(delete_clause.alias_identification_variable.clone());

        Ok(sql)
    }

    pub fn walk_update_clause(
        w: &mut dyn SqlWalker<'_>,
        update_clause: &UpdateClause,
    ) -> Result<String, Thrown> {
        let em = w.get_entity_manager();
        let class = em.get_class_metadata(&update_clause.abstract_schema_name)?;
        let table_name = table_name(&class)?;
        let sql = format!(
            "UPDATE {}",
            em.get_quote_strategy()
                .get_table_name(&class.read(), em.get_database_platform())
        );

        w.set_sql_table_alias(
            &table_name,
            &table_name,
            &update_clause.alias_identification_variable,
        );
        w.walker_state_mut()
            .root_aliases
            .push(update_clause.alias_identification_variable.clone());

        let mut items = Vec::new();
        for update_item in &update_clause.update_items {
            items.push(w.walk_update_item(update_item)?);
        }

        Ok(format!("{sql} SET {}", items.join(", ")))
    }

    pub fn walk_update_item(
        w: &mut dyn SqlWalker<'_>,
        update_item: &UpdateItem,
    ) -> Result<String, Thrown> {
        let use_table_aliases_before = w.walker_state().use_sql_table_aliases;
        w.walker_state_mut().use_sql_table_aliases = false;

        let path = update_item.path_expression.borrow().clone();
        let mut sql = format!("{} = ", w.walk_path_expression(&path)?);

        match &update_item.new_value {
            Some(new_value) => sql.push_str(&dispatch(w, new_value)?),
            None => sql.push_str("NULL"),
        }

        w.walker_state_mut().use_sql_table_aliases = use_table_aliases_before;

        Ok(sql)
    }

    pub fn walk_where_clause(
        w: &mut dyn SqlWalker<'_>,
        where_clause: Option<&WhereClause>,
    ) -> Result<String, Thrown> {
        let em = w.get_entity_manager();
        let mut cond_sql = match where_clause {
            Some(where_clause) => {
                w.walk_conditional_expression(&where_clause.conditional_expression)?
            }
            None => String::new(),
        };
        let root_aliases = w.walker_state().root_aliases.clone();
        let discr_sql = generate_discriminator_column_condition_sql(w, &root_aliases)?;

        if em.has_filters() {
            let mut filter_clauses = Vec::new();
            for dql_alias in &root_aliases {
                let class = w.get_metadata_for_dql_alias(dql_alias)?;
                let table_alias = w.get_sql_table_alias(&table_name(&class)?, dql_alias);

                let filter_expr = generate_filter_condition_sql(w, &class, &table_alias)?;
                if truthy(&filter_expr) {
                    filter_clauses.push(filter_expr);
                }
            }

            if !filter_clauses.is_empty() {
                if truthy(&cond_sql) {
                    cond_sql = format!("({cond_sql}) AND ");
                }

                cond_sql.push_str(&filter_clauses.join(" AND "));
            }
        }

        if truthy(&cond_sql) {
            return Ok(if truthy(&discr_sql) {
                format!(" WHERE ({cond_sql}) AND {discr_sql}")
            } else {
                format!(" WHERE {cond_sql}")
            });
        }

        if truthy(&discr_sql) {
            return Ok(format!(" WHERE {discr_sql}"));
        }

        Ok(String::new())
    }

    #[allow(clippy::too_many_lines, reason = "one PHP method")]
    pub fn walk_collection_member_expression(
        w: &mut dyn SqlWalker<'_>,
        coll_member_expr: &CollectionMemberExpression,
    ) -> Result<String, Thrown> {
        let em = w.get_entity_manager();
        let platform = em.get_database_platform();
        let quote_strategy = em.get_quote_strategy();
        let mut sql = if coll_member_expr.not {
            "NOT ".to_owned()
        } else {
            String::new()
        };
        sql.push_str("EXISTS (SELECT 1 FROM ");

        let entity_expr = &coll_member_expr.entity_expression;
        let coll_path_expr = coll_member_expr
            .collection_valued_path_expression
            .borrow()
            .clone();

        let field_name = coll_path_expr.field.clone().unwrap_or_default();
        let dql_alias = coll_path_expr.identification_variable.as_str();

        let class = w.get_metadata_for_dql_alias(dql_alias)?;

        let mut dql_param_key = None;
        let entity_sql = match entity_expr {
            // InputParameter
            Node::InputParameter(parameter) => {
                dql_param_key = Some(parameter.name.clone());
                "?".to_owned()
            }
            // SingleValuedAssociationPathExpression | IdentificationVariable
            Node::PathExpression(path) => {
                let path = path.borrow().clone();
                w.walk_path_expression(&path)?
            }
            _ => {
                return Err(Thrown::new("BadMethodCallException", "Not implemented", 0));
            }
        };

        let assoc = class
            .read()
            .association_mappings
            .get(&field_name)
            .cloned()
            .ok_or_else(|| method_on_null("isOneToMany"))?;

        let add_parameter_mapping = |w: &mut dyn SqlWalker<'_>| {
            if let Some(key) = &dql_param_key {
                let state = w.walker_state_mut();
                let index = state.sql_param_index;
                state.sql_param_index += 1;
                state
                    .parser_result
                    .borrow_mut()
                    .add_parameter_mapping(Key::from(key.as_str()), index);
            }
        };

        if assoc.is_one_to_many() {
            let target_class = em.get_class_metadata(&assoc.target_entity)?;
            let target_table_alias = w.get_sql_table_alias(&table_name(&target_class)?, "");
            let source_table_alias = w.get_sql_table_alias(&table_name(&class)?, dql_alias);

            let _ = write!(
                sql,
                "{} {target_table_alias} WHERE ",
                quote_strategy.get_table_name(&target_class.read(), platform)
            );

            let mapped_by = assoc.mapped_by.clone().unwrap_or_default();
            let owning_assoc = target_class
                .read()
                .association_mappings
                .get(&mapped_by)
                .cloned()
                .ok_or_else(|| property_on_null("targetToSourceKeyColumns"))?;

            let mut sql_parts = Vec::new();

            for (target_column, source_column) in owning_assoc.target_to_source_key_columns.iter() {
                let target_column = String::from_utf8_lossy(&target_column.bytes()).into_owned();
                let field = class
                    .read()
                    .field_names
                    .get(&target_column)
                    .cloned()
                    .unwrap_or_default();
                let target_column = quote_strategy.get_column_name(&field, &class.read(), platform);

                sql_parts.push(format!(
                    "{source_table_alias}.{target_column} = {target_table_alias}.{}",
                    text(source_column)
                ));
            }

            for target_column_name in
                quote_strategy.get_identifier_column_names(&target_class.read(), platform)
            {
                add_parameter_mapping(w);

                sql_parts.push(format!(
                    "{target_table_alias}.{target_column_name} = {entity_sql}"
                ));
            }

            sql.push_str(&sql_parts.join(" AND "));
        } else {
            // many-to-many
            let target_class = em.get_class_metadata(&assoc.target_entity)?;

            let owning_assoc = em.get_owning_side(&assoc)?;
            let join_table = owning_assoc
                .join_table
                .clone()
                .ok_or_else(|| property_on_null("name"))?;

            // SQL table aliases
            let join_table_alias = w.get_sql_table_alias(&join_table.name, "");
            let source_table_alias = w.get_sql_table_alias(&table_name(&class)?, dql_alias);

            let _ = write!(
                sql,
                "{} {join_table_alias} WHERE ",
                quote_strategy.get_join_table_name(&owning_assoc, &target_class.read(), platform)
            );

            let join_columns = if assoc.is_owning_side() {
                &join_table.join_columns
            } else {
                &join_table.inverse_join_columns
            };
            let mut sql_parts = Vec::new();

            for join_column in join_columns {
                let field = class
                    .read()
                    .field_names
                    .get(&join_column.referenced_column_name)
                    .cloned()
                    .unwrap_or_default();
                let target_column = quote_strategy.get_column_name(&field, &class.read(), platform);

                sql_parts.push(format!(
                    "{join_table_alias}.{} = {source_table_alias}.{target_column}",
                    join_column.name
                ));
            }

            let join_columns = if assoc.is_owning_side() {
                &join_table.inverse_join_columns
            } else {
                &join_table.join_columns
            };

            for join_column in join_columns {
                add_parameter_mapping(w);

                sql_parts.push(format!(
                    "{join_table_alias}.{} IN ({entity_sql})",
                    join_column.name
                ));
            }

            sql.push_str(&sql_parts.join(" AND "));
        }

        Ok(format!("{sql})"))
    }

    pub fn walk_instance_of_expression(
        w: &mut dyn SqlWalker<'_>,
        instance_of_expr: &InstanceOfExpression,
    ) -> Result<String, Thrown> {
        let mut sql = String::new();

        let dql_alias = instance_of_expr.identification_variable.as_str();
        let class = w.get_metadata_for_dql_alias(dql_alias)?;
        let mut discr_class = class.clone();

        let (has_discriminator_column, root_entity_name) = {
            let info = class.read();
            (
                info.discriminator_column.is_some(),
                info.root_entity_name.clone(),
            )
        };
        if has_discriminator_column {
            discr_class = w
                .get_entity_manager()
                .get_class_metadata(&root_entity_name)?;
        }

        if w.walker_state().use_sql_table_aliases {
            sql.push_str(&w.get_sql_table_alias(&table_name(&discr_class)?, dql_alias));
            sql.push('.');
        }

        let column_name = class.read().get_discriminator_column()?.name.clone();
        sql.push_str(&column_name);
        sql.push_str(if instance_of_expr.not {
            " NOT IN "
        } else {
            " IN "
        });
        sql.push_str(&get_child_discriminators_from_class_metadata(
            w,
            &discr_class,
            instance_of_expr,
        )?);

        Ok(sql)
    }

    pub fn walk_literal(w: &mut dyn SqlWalker<'_>, literal: &Literal) -> Result<String, Thrown> {
        let em = w.get_entity_manager();
        let value = literal.value.clone().unwrap_or_default();
        match literal.type_ {
            ast::LITERAL_STRING => em.quote(&value),
            ast::LITERAL_BOOLEAN => Ok(text(
                &em.get_database_platform()
                    .convert_booleans(&Value::Bool(value.to_lowercase() == "true"))
                    .map_err(dbal_error)?,
            )),
            ast::LITERAL_NUMERIC => Ok(value),
            ast::LITERAL_NULL => Ok("NULL".to_owned()),
            _ => Err(query_exception::invalid_literal(&value)),
        }
    }

    pub fn walk_like_expression(
        w: &mut dyn SqlWalker<'_>,
        like_expr: &LikeExpression,
    ) -> Result<String, Thrown> {
        let string_expr = &like_expr.string_expression;
        let left_expr = match string_expr {
            Node::Str(name) => {
                if !crate::query::context::has_result_variable(w.get_query_components(), name) {
                    return Err(query_exception::logic(format!(
                        "No result variable found for string expression \"{name}\"."
                    )));
                }
                w.walk_result_variable(name)?
            }
            other => dispatch(w, other)?,
        };

        let mut sql = format!(
            "{left_expr}{} LIKE ",
            if like_expr.not { " NOT" } else { "" }
        );

        match &like_expr.string_pattern {
            Node::InputParameter(parameter) => sql.push_str(&w.walk_input_parameter(parameter)?),
            Node::Function(function) => sql.push_str(&w.walk_function(function.as_ref())?),
            Node::PathExpression(path) => {
                let path = path.borrow().clone();
                sql.push_str(&w.walk_path_expression(&path)?);
            }
            Node::Literal(literal) => sql.push_str(&w.walk_literal(literal)?),
            other => {
                return Err(node_type_error(
                    "walkLiteral",
                    "literal",
                    "Doctrine\\ORM\\Query\\AST\\Literal",
                    other,
                ));
            }
        }

        if let Some(escape_char) = &like_expr.escape_char {
            let _ = write!(sql, " ESCAPE {}", w.walk_literal(escape_char)?);
        }

        Ok(sql)
    }

    /// An operand of a `ComparisonExpression`.
    pub fn comparison_operand(w: &mut dyn SqlWalker<'_>, expr: &Node) -> Result<String, Thrown> {
        match expr {
            Node::Str(value) if is_numeric(value.as_bytes()) => Ok(value.clone()),
            Node::Str(value) => w.get_entity_manager().quote(value),
            other => dispatch(w, other),
        }
    }

    pub fn walk_input_parameter(
        w: &mut dyn SqlWalker<'_>,
        input_param: &InputParameter,
    ) -> Result<String, Thrown> {
        {
            let state = w.walker_state_mut();
            let index = state.sql_param_index;
            state.sql_param_index += 1;
            state
                .parser_result
                .borrow_mut()
                .add_parameter_mapping(Key::from(input_param.name.as_str()), index);
        }

        if let Some(parameter) = w.get_query().get_parameter(&input_param.name)
            && let WrapperParameterType::Name(type_name) = parameter.get_type()
        {
            let type_name = type_name.to_string_lossy().into_owned();
            if php2rust_doctrine_dbal::types::has_type(&type_name) {
                let type_ = php2rust_doctrine_dbal::types::get_type(&type_name)
                    .map_err(|e| dbal_error(e.into()))?;
                return Ok(type_
                    .convert_to_database_value_sql(
                        b"?",
                        w.get_entity_manager().get_database_platform(),
                    )
                    .to_string_lossy()
                    .into_owned());
            }
        }

        Ok("?".to_owned())
    }

    /// A string `ArithmeticTerm` or `ArithmeticFactor`: the result variable of
    /// the query component of that name, or the string itself.
    pub fn result_variable_or_string(
        w: &mut dyn SqlWalker<'_>,
        term: &str,
    ) -> Result<String, Thrown> {
        if w.get_query_components().contains_key(term) {
            let value = component_token_value(w, term)?;
            return w.walk_result_variable(&value);
        }
        Ok(term.to_owned())
    }

    fn get_child_discriminators_from_class_metadata(
        w: &mut dyn SqlWalker<'_>,
        root_class: &Arc<ClassMetadata>,
        instance_of_expr: &InstanceOfExpression,
    ) -> Result<String, Thrown> {
        let em = w.get_entity_manager();
        let mut sql_parameter_list = Vec::new();
        let mut discriminators = PhpArray::new();
        let root_name = root_class.read().name.clone();

        for parameter in &instance_of_expr.value {
            if let Node::InputParameter(input) = parameter {
                let name = Value::from(input.name.as_str());
                rsm(w, |rsm| {
                    rsm.discriminator_parameters
                        .insert(input.name.as_str(), name);
                });
                sql_parameter_list.push(w.walk_in_parameter(parameter)?);
                continue;
            }

            let class_name = parameter.as_str().unwrap_or("");
            let metadata = em.get_class_metadata(class_name)?;
            let metadata_name = metadata.read().name.clone();

            if metadata_name != root_name
                && !php2rust_doctrine_persistence::php_reflection::is_a(&metadata_name, &root_name)
            {
                return Err(query_exception::instance_of_unrelated_class(
                    class_name, &root_name,
                ));
            }

            let resolved = utility::resolve_discriminators_for_class(&metadata, em)?;
            for (key, value) in resolved.iter() {
                if discriminators.get(key).is_none() {
                    discriminators.insert(key.clone(), value.clone());
                }
            }
        }

        let column_type = root_class.read().get_discriminator_column()?.r#type.clone();
        for discriminator_value in discriminators.keys() {
            match discriminator_value {
                Key::Int(i) if column_type == "integer" => sql_parameter_list.push(i.to_string()),
                key => {
                    let value = String::from_utf8_lossy(&key.bytes()).into_owned();
                    sql_parameter_list.push(em.quote(&value)?);
                }
            }
        }

        Ok(format!("({})", sql_parameter_list.join(", ")))
    }
}

/// `new SqlWalker($query, $parserResult, $queryComponents)`: the walker
/// without the `OutputWalker` interface.
pub struct DefaultSqlWalker<'a> {
    state: SqlWalkerState<'a>,
}

impl<'a> DefaultSqlWalker<'a> {
    pub const CLASS: OutputWalkerClass = OutputWalkerClass {
        name: SQL_WALKER_CLASS,
        construct: construct_default,
    };

    pub fn new(
        query: &'a dyn DqlQuery,
        parser_result: Rc<RefCell<ParserResult>>,
        query_components: QueryComponents,
    ) -> Self {
        Self {
            state: SqlWalkerState::new(query, parser_result, query_components),
        }
    }
}

fn construct_default<'a>(
    query: &'a dyn DqlQuery,
    parser_result: Rc<RefCell<ParserResult>>,
    query_components: QueryComponents,
) -> Box<dyn SqlWalker<'a> + 'a> {
    Box::new(DefaultSqlWalker::new(
        query,
        parser_result,
        query_components,
    ))
}

impl<'a> SqlWalker<'a> for DefaultSqlWalker<'a> {
    fn walker_state(&self) -> &SqlWalkerState<'a> {
        &self.state
    }

    fn walker_state_mut(&mut self) -> &mut SqlWalkerState<'a> {
        &mut self.state
    }

    fn as_dyn(&mut self) -> &mut dyn SqlWalker<'a> {
        self
    }
}

/// `Doctrine\ORM\Query\SqlOutputWalker`: the default output walker.
pub struct SqlOutputWalker<'a> {
    state: SqlWalkerState<'a>,
}

impl<'a> SqlOutputWalker<'a> {
    pub const CLASS: OutputWalkerClass = OutputWalkerClass {
        name: SQL_OUTPUT_WALKER_CLASS,
        construct: construct_output,
    };

    pub fn new(
        query: &'a dyn DqlQuery,
        parser_result: Rc<RefCell<ParserResult>>,
        query_components: QueryComponents,
    ) -> Self {
        Self {
            state: SqlWalkerState::new(query, parser_result, query_components),
        }
    }
}

fn construct_output<'a>(
    query: &'a dyn DqlQuery,
    parser_result: Rc<RefCell<ParserResult>>,
    query_components: QueryComponents,
) -> Box<dyn SqlWalker<'a> + 'a> {
    Box::new(SqlOutputWalker::new(query, parser_result, query_components))
}

impl<'a> SqlWalker<'a> for SqlOutputWalker<'a> {
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
        SQL_OUTPUT_WALKER_CLASS
    }

    fn as_output_walker(&mut self) -> Option<&mut dyn OutputWalker<'a>> {
        Some(self)
    }
}

impl<'a> OutputWalker<'a> for SqlOutputWalker<'a> {}

/// `clone $walker`: PHP copies the walker's arrays and shares its objects
/// (the parser result and its result set mapping).
impl Clone for SqlWalkerState<'_> {
    fn clone(&self) -> Self {
        Self {
            query: self.query,
            parser_result: Rc::clone(&self.parser_result),
            query_components: self.query_components.clone(),
            em: self.em,
            alias_counter: self.alias_counter,
            table_alias_counter: self.table_alias_counter,
            scalar_result_counter: self.scalar_result_counter,
            sql_param_index: self.sql_param_index,
            new_object_counter: self.new_object_counter,
            table_alias_map: self.table_alias_map.clone(),
            scalar_result_alias_map: self.scalar_result_alias_map.clone(),
            ordered_columns_map: self.ordered_columns_map.clone(),
            scalar_fields: self.scalar_fields.clone(),
            selected_classes: self.selected_classes.clone(),
            root_aliases: self.root_aliases.clone(),
            use_sql_table_aliases: self.use_sql_table_aliases,
        }
    }
}
