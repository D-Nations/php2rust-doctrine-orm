//! `CountOutputWalker`: wraps the SQL of a query in
//! `SELECT COUNT(*) ... FROM (SELECT DISTINCT <root ids> FROM (...))`.
//!
//! Upstream: doctrine/orm src/Tools/Pagination/CountOutputWalker.php

use std::cell::RefCell;
use std::rc::Rc;

use php_value::Thrown;

use crate::mapping::quote_strategy::platform_is_a;
use crate::query::ast::SelectStatement;
use crate::query::context::{DqlQuery, OutputWalkerClass, QueryComponents};
use crate::query::sql_walker::{SqlWalkerState, base};
use crate::query::{OutputWalker, ParserResult, SqlWalker};

use super::{root_alias, runtime_exception};

/// Wraps the query in order to accurately count the root objects.
///
/// Given a DQL like `SELECT u FROM User u` it will generate an SQL query like:
/// SELECT COUNT(*) (SELECT DISTINCT <id> FROM (<original SQL>))
///
/// Works with composite keys but cannot deal with queries that have multiple
/// root entities (e.g. `SELECT f, b from Foo, Bar`)
///
/// Note that the ORDER BY clause is not removed. Many SQL implementations (e.g. MySQL)
/// are able to cache subqueries. By keeping the ORDER BY clause of the original query
/// in the subquery, we allow the database to cache the subquery and use it again
/// for other queries that are equal except for their ORDER BY clauses.
pub struct CountOutputWalker<'a> {
    state: SqlWalkerState<'a>,
}

impl CountOutputWalker<'_> {
    pub const CLASS_NAME: &'static str = "Doctrine\\ORM\\Tools\\Pagination\\CountOutputWalker";

    /// `CountOutputWalker::class`.
    pub const CLASS: OutputWalkerClass = OutputWalkerClass {
        name: Self::CLASS_NAME,
        construct,
    };
}

fn construct<'a>(
    query: &'a dyn DqlQuery,
    parser_result: Rc<RefCell<ParserResult>>,
    query_components: QueryComponents,
) -> Box<dyn SqlWalker<'a> + 'a> {
    Box::new(CountOutputWalker {
        state: SqlWalkerState::new(query, parser_result, query_components),
    })
}

impl<'a> SqlWalker<'a> for CountOutputWalker<'a> {
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
        CountOutputWalker::CLASS_NAME
    }

    fn as_output_walker(&mut self) -> Option<&mut dyn OutputWalker<'a>> {
        Some(self)
    }

    fn create_sql_for_finalizer(
        &mut self,
        select_statement: &SelectStatement,
    ) -> Result<String, Thrown> {
        let platform = self.get_entity_manager().get_database_platform();
        let sql = if platform_is_a(platform, "Doctrine\\DBAL\\Platforms\\SQLServerPlatform") {
            let mut select_statement = select_statement.clone();
            select_statement.order_by_clause = None;
            base::create_sql_for_finalizer(self.as_dyn(), &select_statement)?
        } else {
            base::create_sql_for_finalizer(self.as_dyn(), select_statement)?
        };

        if select_statement.group_by_clause.is_some() {
            return Ok(format!(
                "SELECT COUNT(*) AS dctrn_count FROM ({sql}) dctrn_table"
            ));
        }

        // Find out the SQL alias of the identifier column of the root entity
        // It may be possible to make this work with multiple root entities but that
        // would probably require issuing multiple queries or doing a UNION SELECT
        // so for now, It's not supported.

        // Get the root entity and alias from the AST fromClause
        let from = &select_statement
            .from_clause
            .identification_variable_declarations;
        if from.len() > 1 {
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

        // Build the counter query
        let aliases: Vec<&str> = sql_identifier.iter().map(|(_, a)| a.as_str()).collect();
        Ok(format!(
            "SELECT COUNT(*) AS dctrn_count FROM (SELECT DISTINCT {} FROM ({sql}) dctrn_result) dctrn_table",
            aliases.join(", ")
        ))
    }
}

impl<'a> OutputWalker<'a> for CountOutputWalker<'a> {}
