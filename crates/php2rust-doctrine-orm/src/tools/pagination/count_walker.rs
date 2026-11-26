//! `CountWalker`: replaces the selected expressions of a query with
//! `count()` of the root entity's identifier (or `count(*)`).
//!
//! Upstream: doctrine/orm src/Tools/Pagination/CountWalker.php

use std::cell::RefCell;
use std::rc::Rc;

use php_value::Thrown;

use crate::query::ast::{
    AggregateExpression, DeleteStatement, Node, PathExpression, SelectExpression, SelectStatement,
    TYPE_SINGLE_VALUED_ASSOCIATION, TYPE_STATE_FIELD, UpdateStatement,
};
use crate::query::context::{DqlQuery, QueryComponents, TreeWalkerClass};
use crate::query::{ParserResult, TreeWalker, TreeWalkerAdapter};

use super::{root_alias, runtime_exception};

/// Replaces the selectClause of the AST with a COUNT statement.
pub struct CountWalker<'a> {
    adapter: TreeWalkerAdapter<'a>,
}

impl CountWalker<'_> {
    /// Distinct mode hint name.
    pub const HINT_DISTINCT: &'static str = "doctrine_paginator.distinct";

    /// `CountWalker::class`.
    pub const CLASS: TreeWalkerClass = TreeWalkerClass {
        name: "Doctrine\\ORM\\Tools\\Pagination\\CountWalker",
        construct,
    };
}

fn construct<'a>(
    query: &'a dyn DqlQuery,
    parser_result: Rc<RefCell<ParserResult>>,
    query_components: QueryComponents,
) -> Box<dyn TreeWalker<'a> + 'a> {
    Box::new(CountWalker {
        adapter: TreeWalkerAdapter::new(query, parser_result, query_components),
    })
}

impl<'a> TreeWalker<'a> for CountWalker<'a> {
    fn get_query_components(&self) -> &QueryComponents {
        self.adapter.get_query_components()
    }

    fn walk_select_statement(
        &mut self,
        select_statement: &mut SelectStatement,
    ) -> Result<(), Thrown> {
        if select_statement.having_clause.is_some() {
            return Err(runtime_exception(
                "Cannot count query that uses a HAVING clause. Use the output walkers for pagination",
            ));
        }

        // Get the root entity and alias from the AST fromClause
        let from = &select_statement
            .from_clause
            .identification_variable_declarations;

        if from.len() > 1 {
            return Err(runtime_exception(
                "Cannot count query which selects two FROM components, cannot make distinction",
            ));
        }

        let distinct = self
            .adapter
            .get_query()
            .get_hint(CountWalker::HINT_DISTINCT)
            .is_truthy();

        let mut count_path_expression_or_literal = Node::str("*");

        if distinct {
            let root_alias = root_alias(from)?;
            let root_class = self.adapter.get_metadata_for_dql_alias(&root_alias)?;
            let identifier_field_name = root_class.read().get_single_identifier_field_name()?;

            let mut path_type = TYPE_STATE_FIELD;
            if root_class
                .read()
                .association_mappings
                .contains_key(&identifier_field_name)
            {
                path_type = TYPE_SINGLE_VALUED_ASSOCIATION;
            }

            let mut path_expression = PathExpression::new(
                TYPE_STATE_FIELD | TYPE_SINGLE_VALUED_ASSOCIATION,
                &root_alias,
                Some(&identifier_field_name),
            );
            path_expression.type_ = Some(path_type);
            count_path_expression_or_literal = Node::path(path_expression);
        }

        select_statement.select_clause.select_expressions = vec![Rc::new(SelectExpression {
            expression: Node::AggregateExpression(Box::new(AggregateExpression {
                function_name: "count".to_owned(),
                path_expression: count_path_expression_or_literal,
                is_distinct: distinct,
            })),
            field_identification_variable: None,
            hidden_alias_result_variable: false,
        })];

        // ORDER BY is not needed, only increases query execution through unnecessary sorting.
        select_statement.order_by_clause = None;

        Ok(())
    }

    fn walk_update_statement(&mut self, _: &mut UpdateStatement) -> Result<(), Thrown> {
        Ok(())
    }

    fn walk_delete_statement(&mut self, _: &mut DeleteStatement) -> Result<(), Thrown> {
        Ok(())
    }
}
