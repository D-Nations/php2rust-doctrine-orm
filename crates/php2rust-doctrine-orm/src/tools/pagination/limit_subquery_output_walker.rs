//! `LimitSubqueryOutputWalker`: wraps the SQL of a query in
//! `SELECT DISTINCT <root ids> FROM (...)` so that the limit and offset
//! apply to the root entities.
//!
//! Upstream: doctrine/orm src/Tools/Pagination/LimitSubqueryOutputWalker.php

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use php_value::{Key, Thrown};

use crate::mapping::ClassMetadata;
use crate::mapping::quote_strategy::platform_is_a;
use crate::query::ast::{
    Node, OrderByClause, PathExpression, SelectExpression, SelectStatement, Statement, Subselect,
};
use crate::query::context::{DqlQuery, Hint, OutputWalkerClass, QueryComponents, QueryContext};
use crate::query::exec::{SingleSelectSqlFinalizer, SqlFinalizer};
use crate::query::sql_walker::{SqlWalkerState, base, dbal_error};
use crate::query::{OutputWalker, Parameter, ParserResult, Query, SqlWalker};

use super::RowNumberOverFunction;
use super::{root_alias, runtime_exception};

/// The query of the walker with the limit and offset reset:
/// `$cloneQuery->setFirstResult(0)->setMaxResults(null)`.
struct WithoutLimits<'q>(&'q dyn DqlQuery);

impl DqlQuery for WithoutLimits<'_> {
    fn get_entity_manager(&self) -> &dyn QueryContext {
        self.0.get_entity_manager()
    }

    fn get_dql(&self) -> String {
        self.0.get_dql()
    }

    fn get_hint(&self, name: &str) -> Hint {
        self.0.get_hint(name)
    }

    fn set_hint(&self, name: &str, value: Hint) {
        self.0.set_hint(name, value);
    }

    fn get_hydration_mode(&self) -> i64 {
        self.0.get_hydration_mode()
    }

    fn get_parameter(&self, key: &str) -> Option<Parameter> {
        self.0.get_parameter(key)
    }

    fn get_first_result(&self) -> i64 {
        0
    }

    fn get_max_results(&self) -> Option<i64> {
        None
    }

    fn has_hint(&self, name: &str) -> bool {
        self.0.has_hint(name)
    }
}

/// `preg_replace('/(?<![a-z0-9_])<table alias>\.<column>(?![a-z0-9_])/i',
/// $fieldAlias, $subject)`, with the alias and the column matched as text.
fn replace_path_expression(subject: &str, table_alias: &str, column: &str, with: &str) -> String {
    let needle = format!("{table_alias}.{column}");
    let is_word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    let bytes = subject.as_bytes();
    let n = needle.len();
    if n == 0 {
        return subject.to_owned();
    }
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if i + n <= bytes.len()
            && bytes[i..i + n].eq_ignore_ascii_case(needle.as_bytes())
            && (i == 0 || !is_word(bytes[i - 1]))
            && bytes.get(i + n).is_none_or(|b| !is_word(*b))
        {
            out.extend_from_slice(with.as_bytes());
            i += n;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Wraps the query in order to select root entity IDs for pagination.
///
/// Given a DQL like `SELECT u FROM User u` it will generate an SQL query like:
/// SELECT DISTINCT <id> FROM (<original SQL>) LIMIT x OFFSET y
///
/// Works with composite keys but cannot deal with queries that have multiple
/// root entities (e.g. `SELECT f, b from Foo, Bar`)
#[derive(Clone)]
pub struct LimitSubqueryOutputWalker<'a> {
    state: SqlWalkerState<'a>,
    first_result: i64,
    max_results: Option<i64>,
    order_by_path_expressions: Vec<PathExpression>,
    /// `$cloneQuery`: the clone of a `Query` with its parameters, hints and
    /// no second-level cache. The walk reads the query it was given
    /// through a view without limit and offset.
    clone_query: Option<Rc<Query>>,
    /// We don't want to add path expressions from sub-selects into the select clause of the containing query.
    /// This state flag simply keeps track on whether we are walking on a subquery or not
    in_sub_select: bool,
}

impl LimitSubqueryOutputWalker<'_> {
    pub const CLASS_NAME: &'static str =
        "Doctrine\\ORM\\Tools\\Pagination\\LimitSubqueryOutputWalker";

    /// `LimitSubqueryOutputWalker::class`.
    pub const CLASS: OutputWalkerClass = OutputWalkerClass {
        name: Self::CLASS_NAME,
        construct,
    };
}

/// Stores various parameters that are otherwise unavailable
/// because Doctrine\ORM\Query\SqlWalker keeps everything private without
/// accessors.
fn construct<'a>(
    query: &'a dyn DqlQuery,
    parser_result: Rc<RefCell<ParserResult>>,
    query_components: QueryComponents,
) -> Box<dyn SqlWalker<'a> + 'a> {
    Box::new(LimitSubqueryOutputWalker::new(
        query,
        parser_result,
        query_components,
    ))
}

impl<'a> LimitSubqueryOutputWalker<'a> {
    /// `new LimitSubqueryOutputWalker($query, $parserResult, $queryComponents)`.
    pub fn new(
        query: &'a dyn DqlQuery,
        parser_result: Rc<RefCell<ParserResult>>,
        query_components: QueryComponents,
    ) -> Self {
        let clone_query = query.as_query().map(|query| {
            let clone_query = query.clone_query();

            clone_query.set_parameters(query.get_parameters());
            clone_query.set_cacheable(false);

            for (name, value) in query.get_hints() {
                clone_query.set_hint(&name, value);
            }

            // Reset limit and offset
            clone_query.set_first_result(0).set_max_results(None);
            Rc::new(clone_query)
        });

        Self {
            state: SqlWalkerState::new(query, parser_result, query_components),
            first_result: query.get_first_result(),
            max_results: query.get_max_results(),
            order_by_path_expressions: Vec::new(),
            clone_query,
            in_sub_select: false,
        }
    }

    /// `getQuery()`: the clone of the query (`None` for a `DqlQuery` that
    /// is not a `Query`).
    pub fn get_clone_query(&self) -> Option<&Query> {
        self.clone_query.as_deref()
    }
}

impl LimitSubqueryOutputWalker<'_> {
    /// Check if the platform supports the ROW_NUMBER window function.
    fn platform_supports_row_number(&self) -> bool {
        let platform = self.get_entity_manager().get_database_platform();
        [
            "Doctrine\\DBAL\\Platforms\\PostgreSQLPlatform",
            "Doctrine\\DBAL\\Platforms\\SQLServerPlatform",
            "Doctrine\\DBAL\\Platforms\\OraclePlatform",
            "Doctrine\\DBAL\\Platforms\\DB2Platform",
        ]
        .iter()
        .any(|class| platform_is_a(platform, class))
    }

    /// Rebuilds a select statement's order by clause for use in a
    /// ROW_NUMBER() OVER() expression.
    fn rebuild_order_by_for_row_number(ast: &mut SelectStatement) {
        let Some(mut order_by_clause) = ast.order_by_clause.take() else {
            return;
        };
        // Get any aliases that are available for select expressions.
        let mut select_alias_to_expression_map: HashMap<String, Node> = HashMap::new();
        for select_expression in &ast.select_clause.select_expressions {
            if let Some(alias) = &select_expression.field_identification_variable {
                select_alias_to_expression_map
                    .insert(alias.clone(), select_expression.expression.clone());
            }
        }

        // Rebuild string orderby expressions to use the select expression they're referencing
        for order_by_item in &mut order_by_clause.order_by_items {
            if let Some(expression) = order_by_item
                .expression
                .as_str()
                .and_then(|alias| select_alias_to_expression_map.get(alias))
            {
                order_by_item.expression = expression.clone();
            }
        }

        let mut func = RowNumberOverFunction::new("dctrn_rownum");
        func.order_by_clause = Some(order_by_clause);
        ast.select_clause
            .select_expressions
            .push(Rc::new(SelectExpression {
                expression: Node::Function(Rc::new(func)),
                field_identification_variable: Some("dctrn_rownum".to_owned()),
                hidden_alias_result_variable: true,
            }));

        // No need for an order by clause, we'll order by rownum in the outer query.
        // (`take()` above.)
    }

    /// The SQL `getFinalizer()` gives the `SingleSelectSqlFinalizer`.
    fn finalizer_sql(&mut self, ast: &SelectStatement) -> Result<String, Thrown> {
        // The walker changes the AST it walks.
        let mut ast = ast.clone();
        if self.platform_supports_row_number() {
            self.create_sql_with_row_number(&mut ast)
        } else {
            self.create_sql_without_row_number(&mut ast, true)
        }
    }

    /// Walks down a SelectStatement AST node, wrapping it in a SELECT DISTINCT.
    /// This method is for use with platforms which support ROW_NUMBER.
    pub fn walk_select_statement_with_row_number(
        &mut self,
        ast: &mut SelectStatement,
    ) -> Result<String, Thrown> {
        // Apply the limit and offset.
        let sql = self.create_sql_with_row_number(ast)?;
        self.get_entity_manager()
            .get_database_platform()
            .modify_limit_query(&sql, self.max_results, self.first_result)
            .map_err(|e| dbal_error(e.into()))
    }

    fn create_sql_with_row_number(&mut self, ast: &mut SelectStatement) -> Result<String, Thrown> {
        let mut has_order_by = false;
        let outer_order_by = " ORDER BY dctrn_minrownum ASC";
        let mut order_group_by = String::new();
        if ast.order_by_clause.is_some() {
            has_order_by = true;
            Self::rebuild_order_by_for_row_number(ast);
        }

        let inner_sql = self.get_inner_sql(ast)?;

        let mut sql_identifier: Vec<(Key, String)> = self
            .get_sql_identifier(ast)?
            .into_iter()
            .map(|(property, alias)| (Key::from(property.as_str()), alias))
            .collect();

        if has_order_by {
            let aliases: Vec<&str> = sql_identifier.iter().map(|(_, a)| a.as_str()).collect();
            order_group_by = format!(" GROUP BY {}", aliases.join(", "));
            // `$sqlIdentifier[] = ...`: the next integer key.
            sql_identifier.push((
                Key::Int(0),
                format!(
                    "MIN({}) AS dctrn_minrownum",
                    self.walk_result_variable("dctrn_rownum")?
                ),
            ));
        }

        // Build the counter query
        let aliases: Vec<&str> = sql_identifier.iter().map(|(_, a)| a.as_str()).collect();
        let mut sql = format!(
            "SELECT DISTINCT {} FROM ({inner_sql}) dctrn_result",
            aliases.join(", ")
        );

        if has_order_by {
            sql.push_str(&order_group_by);
            sql.push_str(outer_order_by);
        }

        // Add the columns to the ResultSetMapping. It's not really nice but
        // it works. Preferably I'd clear the RSM or simply create a new one
        // but that is not possible from inside the output walker, so we dirty
        // up the one we have.
        self.add_scalar_results(sql_identifier);

        Ok(sql)
    }

    fn add_scalar_results(&self, sql_identifier: Vec<(Key, String)>) {
        let parser_result = self.get_parser_result();
        let mut parser_result = parser_result.borrow_mut();
        let rsm = parser_result.get_result_set_mapping_mut();
        for (property, alias) in sql_identifier {
            rsm.add_scalar_result(&alias, property, "string");
        }
    }

    /// Walks down a SelectStatement AST node, wrapping it in a SELECT DISTINCT.
    /// This method is for platforms which DO NOT support ROW_NUMBER.
    pub fn walk_select_statement_without_row_number(
        &mut self,
        ast: &mut SelectStatement,
        add_missing_items_from_order_by_to_select: bool,
    ) -> Result<String, Thrown> {
        // Apply the limit and offset.
        let sql =
            self.create_sql_without_row_number(ast, add_missing_items_from_order_by_to_select)?;
        self.get_entity_manager()
            .get_database_platform()
            .modify_limit_query(&sql, self.max_results, self.first_result)
            .map_err(|e| dbal_error(e.into()))
    }

    fn create_sql_without_row_number(
        &mut self,
        ast: &mut SelectStatement,
        add_missing_items_from_order_by_to_select: bool,
    ) -> Result<String, Thrown> {
        // We don't want to call this recursively!
        if ast.order_by_clause.is_some() && add_missing_items_from_order_by_to_select {
            // In the case of ordering a query by columns from joined tables, we
            // must add those columns to the select clause of the query BEFORE
            // the SQL is generated.
            self.add_missing_items_from_order_by_to_select(ast)?;
        }

        // Remove order by clause from the inner query
        // It will be re-appended in the outer select generated by this method
        let order_by_clause = ast.order_by_clause.take();

        let inner_sql = self.get_inner_sql(ast)?;

        let sql_identifier = self.get_sql_identifier(ast)?;
        let aliases: Vec<String> = sql_identifier.iter().map(|(_, a)| a.clone()).collect();

        // Build the counter query
        let sql = format!(
            "SELECT DISTINCT {} FROM ({inner_sql}) dctrn_result",
            aliases.join(", ")
        );

        // https://github.com/doctrine/orm/issues/2630
        let sql =
            self.preserve_sql_ordering(&aliases, &inner_sql, sql, order_by_clause.as_ref())?;

        // Add the columns to the ResultSetMapping. It's not really nice but
        // it works. Preferably I'd clear the RSM or simply create a new one
        // but that is not possible from inside the output walker, so we dirty
        // up the one we have.
        self.add_scalar_results(
            sql_identifier
                .into_iter()
                .map(|(property, alias)| (Key::from(property.as_str()), alias))
                .collect(),
        );

        // Restore orderByClause
        ast.order_by_clause = order_by_clause;

        Ok(sql)
    }

    /// Finds all PathExpressions in an AST's OrderByClause, and ensures that
    /// the referenced fields are present in the SelectClause of the passed AST.
    fn add_missing_items_from_order_by_to_select(
        &mut self,
        ast: &mut SelectStatement,
    ) -> Result<(), Thrown> {
        self.order_by_path_expressions.clear();

        // We need to do this in another walker because otherwise we'll end up
        // polluting the state of this one.
        let mut walker = self.clone();

        // This will populate $orderByPathExpressions via
        // LimitSubqueryOutputWalker::walkPathExpression, which will be called
        // as the select statement is walked. We'll end up with an array of all
        // path expressions referenced in the query.
        walker.walk_select_statement_without_row_number(ast, false)?;
        let order_by_path_expressions = walker.order_by_path_expressions;

        // Get a map of referenced identifiers to field names.
        let mut selects: Vec<(String, Vec<String>)> = Vec::new();
        for path_expression in order_by_path_expressions {
            let id_var = path_expression.identification_variable;
            let field = path_expression.field.unwrap_or_default();
            match selects.iter_mut().find(|(v, _)| *v == id_var) {
                Some((_, fields)) => {
                    if !fields.contains(&field) {
                        fields.push(field);
                    }
                }
                None => selects.push((id_var, vec![field])),
            }
        }

        // Loop the select clause of the AST and exclude items from $select
        // that are already being selected in the query.
        for select_expression in &ast.select_clause.select_expressions {
            let Some(id_var) = select_expression.expression.as_str() else {
                continue;
            };

            match &select_expression.field_identification_variable {
                // No need to add this select, as we're already fetching the whole object.
                None => selects.retain(|(v, _)| v != id_var),
                Some(field) => {
                    if let Some((_, fields)) = selects.iter_mut().find(|(v, _)| v == id_var) {
                        fields.retain(|f| f != field);
                    }
                }
            }
        }

        // Add select items which were not excluded to the AST's select clause.
        for (id_var, _) in selects {
            ast.select_clause
                .select_expressions
                .push(Rc::new(SelectExpression {
                    expression: Node::Str(id_var),
                    field_identification_variable: None,
                    hidden_alias_result_variable: true,
                }));
        }

        Ok(())
    }

    /// Generates new SQL for statements with an order by clause
    fn preserve_sql_ordering(
        &mut self,
        sql_identifier: &[String],
        inner_sql: &str,
        sql: String,
        order_by_clause: Option<&OrderByClause>,
    ) -> Result<String, Thrown> {
        // If the sql statement has an order by clause, we need to wrap it in a new select distinct statement
        let Some(order_by_clause) = order_by_clause else {
            return Ok(sql);
        };

        // now only select distinct identifier
        Ok(format!(
            "SELECT DISTINCT {} FROM ({}) dctrn_result",
            sql_identifier.join(", "),
            self.recreate_inner_sql(order_by_clause, sql_identifier, inner_sql)?
        ))
    }

    /// Generates a new SQL statement for the inner query to keep the correct sorting
    fn recreate_inner_sql(
        &mut self,
        order_by_clause: &OrderByClause,
        identifiers: &[String],
        inner_sql: &str,
    ) -> Result<String, Thrown> {
        let replacements = self.generate_sql_alias_replacements()?;
        let mut identifiers = identifiers.to_vec();
        let mut order_by_items = Vec::new();

        for order_by_item in &order_by_clause.order_by_items {
            // Walk order by item to get string representation of it and
            // replace path expressions in the order by clause with their column alias
            let mut order_by_item_string = self.walk_order_by_item(order_by_item)?;
            for (table_alias, column_name, field_alias) in &replacements {
                order_by_item_string = replace_path_expression(
                    &order_by_item_string,
                    table_alias,
                    column_name,
                    field_alias,
                );
            }

            // `substr($s, 0, strrpos($s, ' '))`: empty without a space.
            let identifier = order_by_item_string
                .rfind(' ')
                .map_or("", |end| &order_by_item_string[..end])
                .to_owned();

            order_by_items.push(order_by_item_string);

            if !identifiers.contains(&identifier) {
                identifiers.push(identifier);
            }
        }

        Ok(format!(
            "SELECT DISTINCT {} FROM ({inner_sql}) dctrn_result_inner ORDER BY {}",
            identifiers.join(", "),
            order_by_items.join(", ")
        ))
    }

    /// The search patterns and their replacements, as
    /// `(SQL table alias, column name, field alias)`.
    fn generate_sql_alias_replacements(&mut self) -> Result<Vec<(String, String, String)>, Thrown> {
        let (dql_aliases, field_mappings, column_owner_map) = {
            let parser_result = self.get_parser_result();
            let parser_result = parser_result.borrow();
            let rsm = parser_result.get_result_set_mapping();
            (
                rsm.alias_map.keys().map(str::to_owned).collect::<Vec<_>>(),
                rsm.field_mappings
                    .iter()
                    .map(|(alias, field)| (alias.to_owned(), field.clone()))
                    .collect::<Vec<_>>(),
                rsm.column_owner_map.clone(),
            )
        };

        let mut alias_map: HashMap<String, String> = HashMap::new();
        let mut metadata_list: HashMap<String, Arc<ClassMetadata>> = HashMap::new();
        let mut out = Vec::new();

        // Generate DQL alias -> SQL table alias mapping
        for dql_alias in dql_aliases {
            let class = self.get_metadata_for_dql_alias(&dql_alias)?;
            let table_name = class.read().get_table_name()?;
            let sql_table_alias = self.get_sql_table_alias(&table_name, &dql_alias);
            alias_map.insert(dql_alias.clone(), sql_table_alias);
            metadata_list.insert(dql_alias, class);
        }

        let em = self.get_entity_manager();
        let quote_strategy = em.get_quote_strategy();

        // Generate search patterns for each field's path expression in the order by clause
        for (field_alias, field_name) in field_mappings {
            let dql_alias_for_field_alias = column_owner_map
                .get(&field_alias)
                .cloned()
                .unwrap_or_default();
            let Some(class) = metadata_list.get(&dql_alias_for_field_alias).cloned() else {
                return Err(Thrown::new(
                    "Error",
                    "Attempt to read property \"fieldMappings\" on null",
                    0,
                ));
            };
            let info = class.read();

            // If the field is from a joined child table, we won't be ordering on it.
            let Some(field_mapping) = info.field_mappings.get(&field_name) else {
                continue;
            };

            // Get the proper column name as will appear in the select list
            let column_name =
                quote_strategy.get_column_name(&field_name, &info, em.get_database_platform());

            // Get the SQL table alias for the entity and field
            let mut sql_table_alias_for_field_alias = alias_map
                .get(&dql_alias_for_field_alias)
                .cloned()
                .unwrap_or_default();

            if let Some(declared) = &field_mapping.declared
                && *declared != info.name
            {
                // Field was declared in a parent class, so we need to get the proper SQL table alias
                // for the joined parent table.
                let other_class_metadata = em.get_class_metadata(declared)?;
                let other = other_class_metadata.read();

                if !other.is_mapped_superclass {
                    let table_name = other.get_table_name()?;
                    drop(other);
                    sql_table_alias_for_field_alias =
                        self.get_sql_table_alias(&table_name, &dql_alias_for_field_alias);
                }
            }

            // Compose search and replace patterns
            out.push((sql_table_alias_for_field_alias, column_name, field_alias));
        }

        Ok(out)
    }

    /// getter for $orderByPathExpressions
    pub fn get_order_by_path_expressions(&self) -> &[PathExpression] {
        &self.order_by_path_expressions
    }

    fn get_inner_sql(&mut self, ast: &SelectStatement) -> Result<String, Thrown> {
        // Set every select expression as visible(hidden = false) to
        // make $AST have scalar mappings properly - this is relevant for referencing selected
        // fields from outside the subquery, for example in the ORDER BY segment
        let mut visible = ast.clone();
        for expr in &mut visible.select_clause.select_expressions {
            if expr.hidden_alias_result_variable {
                *expr = Rc::new(SelectExpression {
                    hidden_alias_result_variable: false,
                    ..(**expr).clone()
                });
            }
        }

        // `parent::walkSelectStatement($AST)` on the query whose limit and
        // offset were reset.
        let sql = self.create_sql_for_finalizer(&visible)?;
        SingleSelectSqlFinalizer::new(sql).finalize_sql(&WithoutLimits(self.get_query()))
    }

    fn get_sql_identifier(
        &mut self,
        ast: &SelectStatement,
    ) -> Result<Vec<(String, String)>, Thrown> {
        // Find out the SQL alias of the identifier column of the root entity.
        // It may be possible to make this work with multiple root entities but that
        // would probably require issuing multiple queries or doing a UNION SELECT.
        // So for now, it's not supported.

        // Get the root entity and alias from the AST fromClause.
        let from = &ast.from_clause.identification_variable_declarations;
        if from.len() != 1 {
            return Err(runtime_exception(
                "Cannot count query which selects two FROM components, cannot make distinction",
            ));
        }

        let root_alias = root_alias(from)?;
        let root_class = self.get_metadata_for_dql_alias(&root_alias)?;
        let root_class = root_class.read();
        let root_identifier = &root_class.identifier;

        // For every identifier, find out the SQL alias by combing through the ResultSetMapping
        let parser_result = self.get_parser_result();
        let parser_result = parser_result.borrow();
        let rsm = parser_result.get_result_set_mapping();
        let mut sql_identifier: Vec<(String, String)> = Vec::new();
        let mut set = |property: &str, alias: &str| match sql_identifier
            .iter_mut()
            .find(|(p, _)| p == property)
        {
            Some((_, known)) => alias.clone_into(known),
            None => sql_identifier.push((property.to_owned(), alias.to_owned())),
        };
        for property in root_identifier {
            if root_class.field_mappings.contains_key(property) {
                for (alias, _) in rsm.field_mappings.iter().filter(|(_, f)| *f == property) {
                    if rsm.column_owner_map.get(alias) == Some(&root_alias) {
                        set(property, alias);
                    }
                }
            }

            if let Some(association) = root_class.association_mappings.get(property) {
                debug_assert!(association.is_to_one_owning_side());
                let join_column = association
                    .join_columns
                    .first()
                    .map_or_default(|join_column| join_column.name.clone());
                for (alias, _) in rsm.meta_mappings.iter().filter(|(_, c)| **c == join_column) {
                    if rsm.column_owner_map.get(alias) == Some(&root_alias) {
                        set(property, alias);
                    }
                }
            }
        }

        if sql_identifier.is_empty() {
            return Err(runtime_exception(
                "Paginators do not support Queries which only yield ScalarResults.",
            ));
        }

        if root_identifier.len() != sql_identifier.len() {
            let missing: Vec<&str> = root_identifier
                .iter()
                .filter(|property| !sql_identifier.iter().any(|(p, _)| p == *property))
                .map(String::as_str)
                .collect();
            return Err(runtime_exception(format!(
                "Not all identifier properties can be found in the ResultSetMapping: {}",
                missing.join(", ")
            )));
        }

        Ok(sql_identifier)
    }
}

impl<'a> SqlWalker<'a> for LimitSubqueryOutputWalker<'a> {
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
        LimitSubqueryOutputWalker::CLASS_NAME
    }

    fn as_output_walker(&mut self) -> Option<&mut dyn OutputWalker<'a>> {
        Some(self)
    }

    fn walk_select_statement(
        &mut self,
        select_statement: &SelectStatement,
    ) -> Result<String, Thrown> {
        let sql_finalizer = SingleSelectSqlFinalizer::new(self.finalizer_sql(select_statement)?);

        let query = WithoutLimits(self.get_query());

        sql_finalizer.finalize_sql(&query)
    }

    fn walk_path_expression(&mut self, path_expr: &PathExpression) -> Result<String, Thrown> {
        // `in_array($pathExpr, ..., true)` compares objects by identity;
        // the walker sees copies, and a repeated path changes nothing.
        if !self.in_sub_select && !self.platform_supports_row_number() {
            self.order_by_path_expressions.push(path_expr.clone());
        }

        base::walk_path_expression(self.as_dyn(), path_expr)
    }

    fn walk_subselect(&mut self, subselect: &Subselect) -> Result<String, Thrown> {
        self.in_sub_select = true;

        let sql = base::walk_subselect(self.as_dyn(), subselect)?;

        self.in_sub_select = false;

        Ok(sql)
    }
}

impl<'a> OutputWalker<'a> for LimitSubqueryOutputWalker<'a> {
    fn get_finalizer(&mut self, ast: &Statement) -> Result<Box<dyn SqlFinalizer>, Thrown> {
        let Statement::Select(ast) = ast else {
            return Err(Thrown::new(
                "LogicException",
                format!(
                    "{} is to be used on SelectStatements only",
                    LimitSubqueryOutputWalker::CLASS_NAME
                ),
                0,
            ));
        };

        Ok(Box::new(SingleSelectSqlFinalizer::new(
            self.finalizer_sql(ast)?,
        )))
    }
}
