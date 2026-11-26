//! `WhereInWalker`: adds `WHERE root.id IN (:dpid)` (or `IS NULL` when the
//! paginator has no identifiers) to a query.
//!
//! Upstream: doctrine/orm src/Tools/Pagination/WhereInWalker.php

use std::cell::RefCell;
use std::rc::Rc;

use php_value::Thrown;

use crate::query::ast::{
    ArithmeticExpression, ConditionalExpression, ConditionalPrimary, ConditionalTerm,
    DeleteStatement, InListExpression, InputParameter, Node, NullComparisonExpression,
    PathExpression, SelectStatement, SimpleArithmeticExpression, TYPE_SINGLE_VALUED_ASSOCIATION,
    TYPE_STATE_FIELD, UpdateStatement, WhereClause,
};
use crate::query::context::{DqlQuery, QueryComponents, TreeWalkerClass};
use crate::query::{ParserResult, TreeWalker, TreeWalkerAdapter};

use super::{root_alias, runtime_exception};

/// Appends a condition equivalent to "WHERE IN (:dpid_1, :dpid_2, ...)" to the whereClause of the AST.
///
/// The parameter namespace (dpid) is defined by
/// the PAGINATOR_ID_ALIAS
///
/// The HINT_PAGINATOR_HAS_IDS query hint indicates whether there are
/// any ids in the parameter at all.
pub struct WhereInWalker<'a> {
    adapter: TreeWalkerAdapter<'a>,
}

impl WhereInWalker<'_> {
    /// ID Count hint name.
    pub const HINT_PAGINATOR_HAS_IDS: &'static str = "doctrine.paginator_has_ids";

    /// Primary key alias for query.
    pub const PAGINATOR_ID_ALIAS: &'static str = "dpid";

    /// `WhereInWalker::class`.
    pub const CLASS: TreeWalkerClass = TreeWalkerClass {
        name: "Doctrine\\ORM\\Tools\\Pagination\\WhereInWalker",
        construct,
    };
}

fn construct<'a>(
    query: &'a dyn DqlQuery,
    parser_result: Rc<RefCell<ParserResult>>,
    query_components: QueryComponents,
) -> Box<dyn TreeWalker<'a> + 'a> {
    Box::new(WhereInWalker {
        adapter: TreeWalkerAdapter::new(query, parser_result, query_components),
    })
}

impl<'a> TreeWalker<'a> for WhereInWalker<'a> {
    fn get_query_components(&self) -> &QueryComponents {
        self.adapter.get_query_components()
    }

    fn walk_select_statement(
        &mut self,
        select_statement: &mut SelectStatement,
    ) -> Result<(), Thrown> {
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
        let path_expression = Node::path(path_expression);

        let has_ids = self
            .adapter
            .get_query()
            .get_hint(WhereInWalker::HINT_PAGINATOR_HAS_IDS)
            .is_truthy();

        let expression = if has_ids {
            let arithmetic_expression = ArithmeticExpression {
                simple_arithmetic_expression: Some(Node::SimpleArithmeticExpression(Box::new(
                    SimpleArithmeticExpression {
                        arithmetic_terms: vec![path_expression],
                    },
                ))),
                subselect: None,
            };
            Node::InListExpression(Box::new(InListExpression {
                expression: arithmetic_expression,
                literals: vec![Node::InputParameter(Box::new(InputParameter::new(
                    &format!(":{}", WhereInWalker::PAGINATOR_ID_ALIAS),
                )?))],
                not: false,
            }))
        } else {
            Node::NullComparisonExpression(Box::new(NullComparisonExpression {
                expression: path_expression,
                not: false,
            }))
        };

        let conditional_primary = Node::ConditionalPrimary(Box::new(ConditionalPrimary {
            simple_conditional_expression: Some(expression),
            conditional_expression: None,
        }));

        if let Some(where_clause) = select_statement.where_clause.as_mut() {
            let conditional_expression =
                std::mem::replace(&mut where_clause.conditional_expression, Node::str(""));
            where_clause.conditional_expression = match conditional_expression {
                Node::ConditionalTerm(mut term) => {
                    term.conditional_factors.push(conditional_primary);
                    Node::ConditionalTerm(term)
                }
                primary @ Node::ConditionalPrimary(_) => {
                    Node::ConditionalExpression(Box::new(ConditionalExpression {
                        conditional_terms: vec![Node::ConditionalTerm(Box::new(ConditionalTerm {
                            conditional_factors: vec![primary, conditional_primary],
                        }))],
                    }))
                }
                other => {
                    let tmp_primary = Node::ConditionalPrimary(Box::new(ConditionalPrimary {
                        simple_conditional_expression: None,
                        conditional_expression: Some(other),
                    }));
                    Node::ConditionalTerm(Box::new(ConditionalTerm {
                        conditional_factors: vec![tmp_primary, conditional_primary],
                    }))
                }
            };
        } else {
            select_statement.where_clause = Some(WhereClause {
                conditional_expression: Node::ConditionalExpression(Box::new(
                    ConditionalExpression {
                        conditional_terms: vec![Node::ConditionalTerm(Box::new(ConditionalTerm {
                            conditional_factors: vec![conditional_primary],
                        }))],
                    },
                )),
            });
        }

        Ok(())
    }

    fn walk_update_statement(&mut self, _: &mut UpdateStatement) -> Result<(), Thrown> {
        Ok(())
    }

    fn walk_delete_statement(&mut self, _: &mut DeleteStatement) -> Result<(), Thrown> {
        Ok(())
    }
}
