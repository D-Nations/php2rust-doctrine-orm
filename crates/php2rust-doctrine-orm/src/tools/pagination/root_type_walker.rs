//! `RootTypeWalker`: an output walker whose "SQL" is the DBAL type name of
//! the root entity's identifier, which the paginator uses to convert the
//! identifiers it found.
//!
//! Upstream: doctrine/orm src/Tools/Pagination/RootTypeWalker.php

use std::cell::RefCell;
use std::rc::Rc;

use php_value::Thrown;

use crate::query::ast::{SelectStatement, Statement};
use crate::query::context::{DqlQuery, OutputWalkerClass, QueryComponents};
use crate::query::exec::{
    ExecutorKind, PreparedExecutorFinalizer, SqlExecutor, SqlFinalizer, SqlStatements,
};
use crate::query::sql_walker::SqlWalkerState;
use crate::query::utility::get_type_of_field;
use crate::query::{OutputWalker, ParserResult, SqlWalker};

use super::{root_alias, runtime_exception};

/// Infers the DBAL type of the #Id (identifier) column of the given query's root entity, and
/// returns it in place of a real SQL statement.
///
/// Obtaining this type is a necessary intermediate step for \Doctrine\ORM\Tools\Pagination\Paginator.
/// We can best do this from a tree walker because it gives us access to the AST.
///
/// Returning the type instead of a "real" SQL statement is a slight hack. However, it has the
/// benefit that the DQL -> root entity id type resolution can be cached in the query cache.
pub struct RootTypeWalker<'a> {
    state: SqlWalkerState<'a>,
}

impl RootTypeWalker<'_> {
    pub const CLASS_NAME: &'static str = "Doctrine\\ORM\\Tools\\Pagination\\RootTypeWalker";

    /// `RootTypeWalker::class`.
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
    Box::new(RootTypeWalker {
        state: SqlWalkerState::new(query, parser_result, query_components),
    })
}

impl<'a> SqlWalker<'a> for RootTypeWalker<'a> {
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
        RootTypeWalker::CLASS_NAME
    }

    fn as_output_walker(&mut self) -> Option<&mut dyn OutputWalker<'a>> {
        Some(self)
    }

    fn walk_select_statement(
        &mut self,
        select_statement: &SelectStatement,
    ) -> Result<String, Thrown> {
        // Get the root entity and alias from the AST fromClause
        let from = &select_statement
            .from_clause
            .identification_variable_declarations;

        if from.len() > 1 {
            return Err(runtime_exception(
                "Can only process queries that select only one FROM component",
            ));
        }

        let root_alias = root_alias(from)?;
        let root_class = self.get_metadata_for_dql_alias(&root_alias)?;
        let identifier_field_name = root_class.read().get_single_identifier_field_name()?;

        get_type_of_field(
            &identifier_field_name,
            &root_class,
            self.get_query().get_entity_manager(),
        )?
        .into_iter()
        .next()
        .ok_or_else(|| {
            Thrown::new(
                "TypeError",
                "Doctrine\\ORM\\Tools\\Pagination\\RootTypeWalker::walkSelectStatement(): Return value must be of type string, null returned",
                0,
            )
        })
    }
}

impl<'a> OutputWalker<'a> for RootTypeWalker<'a> {
    fn get_finalizer(&mut self, ast: &Statement) -> Result<Box<dyn SqlFinalizer>, Thrown> {
        let Statement::Select(ast) = ast else {
            return Err(runtime_exception(format!(
                "{} is to be used on SelectStatements only",
                RootTypeWalker::CLASS_NAME
            )));
        };

        Ok(Box::new(PreparedExecutorFinalizer::new(SqlExecutor::new(
            ExecutorKind::FinalizedSelect,
            SqlStatements::One(self.walk_select_statement(ast)?),
        ))))
    }
}
