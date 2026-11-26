//! Port of `CustomTreeWalkersTest`: tree walkers that change the AST and
//! an output walker that sets an invalid query component, on the test
//! entity manager (`tests/common/dql.rs`).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Query/CustomTreeWalkersTest.php

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

#[path = "common/custom_tree_walker_join.rs"]
mod custom_tree_walker_join;
#[path = "common/dql.rs"]
mod dql;
#[path = "common/models.rs"]
mod models;

use std::cell::RefCell;
use std::rc::Rc;

use php_value::Thrown;
use php2rust_doctrine_orm::query::ast::{
    ComparisonExpression, ConditionalExpression, ConditionalFactor, ConditionalPrimary,
    ConditionalTerm, DeleteStatement, Node, PathExpression, SelectStatement, TYPE_STATE_FIELD,
    UpdateStatement, WhereClause,
};
use php2rust_doctrine_orm::query::context::{
    DqlQuery, HINT_CUSTOM_TREE_WALKERS, Hint, OutputWalkerClass, QueryComponent, QueryComponents,
    TreeWalkerClass,
};
use php2rust_doctrine_orm::query::sql_walker::{SqlWalkerState, base};
use php2rust_doctrine_orm::query::{
    OutputWalker, ParserResult, SqlWalker, TreeWalker, TreeWalkerAdapter,
};
use php2rust_doctrine_persistence::php_reflection::thrown_is_a;

use custom_tree_walker_join::CustomTreeWalkerJoin;
use dql::{TestEntityManager, output_walker, thrown};

const CMS_USER: &str = r"Doctrine\Tests\Models\CMS\CmsUser";
const QUERY_EXCEPTION: &str = r"Doctrine\ORM\Query\QueryException";

fn generate_sql(
    dql_to_be_tested: &str,
    tree_walkers: Vec<TreeWalkerClass>,
    output_walker_class: Option<OutputWalkerClass>,
) -> Result<String, Thrown> {
    let entity_manager = TestEntityManager::new();
    let query = entity_manager.create_query(dql_to_be_tested);
    query.hint(HINT_CUSTOM_TREE_WALKERS, Hint::TreeWalkers(tree_walkers));
    if let Some(class) = output_walker_class {
        let (name, hint) = output_walker(class);
        query.hint(name, hint);
    }
    query.get_single_sql()
}

fn assert_sql_generation(
    dql_to_be_tested: &str,
    sql_to_be_confirmed: &str,
    tree_walkers: Vec<TreeWalkerClass>,
) {
    assert_eq!(
        sql_to_be_confirmed,
        generate_sql(dql_to_be_tested, tree_walkers, None).unwrap()
    );
}

#[test]
fn testSupportsQueriesWithoutWhere() {
    assert_sql_generation(
        r"select u from Doctrine\Tests\Models\CMS\CmsUser u",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_ WHERE c0_.id = 1",
        vec![CustomTreeWalker::CLASS],
    );
}

#[test]
fn testSupportsQueriesWithMultipleConditionalExpressions() {
    assert_sql_generation(
        r"select u from Doctrine\Tests\Models\CMS\CmsUser u where u.name = :name or u.name = :otherName",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_ WHERE (c0_.name = ? OR c0_.name = ?) AND c0_.id = 1",
        vec![CustomTreeWalker::CLASS],
    );
}

#[test]
fn testSupportsQueriesWithSimpleConditionalExpression() {
    assert_sql_generation(
        r"select u from Doctrine\Tests\Models\CMS\CmsUser u where u.name = :name",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c0_.email_id AS email_id_4 FROM cms_users c0_ WHERE c0_.name = ? AND c0_.id = 1",
        vec![CustomTreeWalker::CLASS],
    );
}

#[test]
fn testSetUnknownQueryComponentThrowsException() {
    let error = generate_sql(
        r"select u from Doctrine\Tests\Models\CMS\CmsUser u",
        vec![],
        Some(AddUnknownQueryComponentWalker::CLASS),
    )
    .unwrap_err();
    assert!(thrown_is_a(&error, QUERY_EXCEPTION), "{:?}", thrown(&error));
    assert_eq!(
        "Invalid query component given for DQL alias 'x', requires 'metadata', 'parent', 'relation', 'map', 'nestingLevel' and 'token' keys.",
        thrown(&error).1
    );
}

#[test]
fn testSupportsSeveralHintsQueries() {
    assert_sql_generation(
        r"select u from Doctrine\Tests\Models\CMS\CmsUser u",
        "SELECT c0_.id AS id_0, c0_.status AS status_1, c0_.username AS username_2, c0_.name AS name_3, c1_.id AS id_4, c1_.country AS country_5, c1_.zip AS zip_6, c1_.city AS city_7, c0_.email_id AS email_id_8, c1_.user_id AS user_id_9 FROM cms_users c0_ LEFT JOIN cms_addresses c1_ ON c0_.id = c1_.user_id WHERE c0_.id = 1",
        vec![CustomTreeWalkerJoin::CLASS, CustomTreeWalker::CLASS],
    );
}

/// `AddUnknownQueryComponentWalker`: an `SqlOutputWalker` whose
/// `createSqlForFinalizer()` sets the query component `[]`.
struct AddUnknownQueryComponentWalker<'a> {
    state: SqlWalkerState<'a>,
}

impl AddUnknownQueryComponentWalker<'_> {
    const CLASS: OutputWalkerClass = OutputWalkerClass {
        name: "Doctrine\\Tests\\ORM\\Query\\AddUnknownQueryComponentWalker",
        construct: construct_add_unknown,
    };
}

fn construct_add_unknown<'a>(
    query: &'a dyn DqlQuery,
    parser_result: Rc<RefCell<ParserResult>>,
    query_components: QueryComponents,
) -> Box<dyn SqlWalker<'a> + 'a> {
    Box::new(AddUnknownQueryComponentWalker {
        state: SqlWalkerState::new(query, parser_result, query_components),
    })
}

impl<'a> SqlWalker<'a> for AddUnknownQueryComponentWalker<'a> {
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
        Self::CLASS.name
    }

    fn as_output_walker(&mut self) -> Option<&mut dyn OutputWalker<'a>> {
        Some(self)
    }

    fn create_sql_for_finalizer(
        &mut self,
        select_statement: &SelectStatement,
    ) -> Result<String, Thrown> {
        self.set_query_component("x", QueryComponent::default())?;

        base::create_sql_for_finalizer(self, select_statement)
    }
}

impl<'a> OutputWalker<'a> for AddUnknownQueryComponentWalker<'a> {}

/// `CustomTreeWalker`: adds `<alias>.id = 1` to the WHERE clause for each
/// `CmsUser`.
struct CustomTreeWalker<'a> {
    adapter: TreeWalkerAdapter<'a>,
}

impl CustomTreeWalker<'_> {
    const CLASS: TreeWalkerClass = TreeWalkerClass {
        name: "Doctrine\\Tests\\ORM\\Query\\CustomTreeWalker",
        construct: construct_custom,
    };
}

fn construct_custom<'a>(
    query: &'a dyn DqlQuery,
    parser_result: Rc<RefCell<ParserResult>>,
    query_components: QueryComponents,
) -> Box<dyn TreeWalker<'a> + 'a> {
    Box::new(CustomTreeWalker {
        adapter: TreeWalkerAdapter::new(query, parser_result, query_components),
    })
}

impl<'a> TreeWalker<'a> for CustomTreeWalker<'a> {
    fn get_query_components(&self) -> &QueryComponents {
        self.adapter.get_query_components()
    }

    fn walk_select_statement(
        &mut self,
        select_statement: &mut SelectStatement,
    ) -> Result<(), Thrown> {
        // Get the DQL aliases of all the classes we want to modify
        let mut dql_aliases = Vec::new();

        for (dql_alias, comp) in self.get_query_components().iter() {
            // Hard-coded check just for demonstration: We want to modify the query if
            // it involves the CmsUser class.
            if comp
                .metadata
                .as_ref()
                .is_some_and(|m| m.read().name == CMS_USER)
            {
                dql_aliases.push(dql_alias.to_owned());
            }
        }

        // Create our conditions for all involved classes
        let mut factors = Vec::new();
        for alias in &dql_aliases {
            let mut path_expr = PathExpression::new(TYPE_STATE_FIELD, alias, Some("id"));
            path_expr.type_ = Some(TYPE_STATE_FIELD);
            let comparison_expr = ComparisonExpression {
                left_expression: Node::path(path_expr),
                operator: "=".to_owned(),
                right_expression: Node::str("1"),
            };

            let cond_primary = ConditionalPrimary {
                simple_conditional_expression: Some(Node::ComparisonExpression(Box::new(
                    comparison_expr,
                ))),
                conditional_expression: None,
            };

            let factor = ConditionalFactor {
                conditional_primary: cond_primary,
                not: false,
            };
            factors.push(Node::ConditionalFactor(Box::new(factor)));
        }

        if let Some(where_clause) = &mut select_statement.where_clause {
            // There is already a WHERE clause, so append the conditions

            // Since Phase 1 AST optimizations were included, we need to re-add the ConditionalExpression
            if !matches!(
                where_clause.conditional_expression,
                Node::ConditionalExpression(_)
            ) {
                let cond_expr =
                    std::mem::replace(&mut where_clause.conditional_expression, Node::str(""));
                where_clause.conditional_expression =
                    Node::ConditionalExpression(Box::new(ConditionalExpression {
                        conditional_terms: vec![cond_expr],
                    }));
            }
            let Node::ConditionalExpression(conditional_expression) =
                &mut where_clause.conditional_expression
            else {
                unreachable!("the WHERE clause holds a ConditionalExpression");
            };

            let existing_terms = &mut conditional_expression.conditional_terms;

            if existing_terms.len() > 1 {
                // More than one term, so we need to wrap all these terms in a single root term
                // i.e: "WHERE u.name = :foo or u.other = :bar" => "WHERE (u.name = :foo or u.other = :bar) AND <our condition>"

                let primary = ConditionalPrimary {
                    simple_conditional_expression: None,
                    conditional_expression: Some(Node::ConditionalExpression(Box::new(
                        ConditionalExpression {
                            conditional_terms: std::mem::take(existing_terms),
                        },
                    ))),
                };
                let existing_factor = Node::ConditionalFactor(Box::new(ConditionalFactor {
                    conditional_primary: primary,
                    not: false,
                }));
                let mut term_factors = vec![existing_factor];
                term_factors.extend(factors);
                let term = ConditionalTerm {
                    conditional_factors: term_factors,
                };

                *existing_terms = vec![Node::ConditionalTerm(Box::new(term))];
            } else {
                // Just one term so we can simply append our factors to that term
                let mut single_term = existing_terms.remove(0);

                // Since Phase 1 AST optimizations were included, we need to re-add the ConditionalExpression
                if !matches!(single_term, Node::ConditionalTerm(_)) {
                    single_term = Node::ConditionalTerm(Box::new(ConditionalTerm {
                        conditional_factors: vec![single_term],
                    }));
                }
                if let Node::ConditionalTerm(term) = &mut single_term {
                    term.conditional_factors.extend(factors);
                }
                *existing_terms = vec![single_term];
            }
        } else {
            // Create a new WHERE clause with our factors
            let term = ConditionalTerm {
                conditional_factors: factors,
            };
            let cond_expr = ConditionalExpression {
                conditional_terms: vec![Node::ConditionalTerm(Box::new(term))],
            };
            select_statement.where_clause = Some(WhereClause {
                conditional_expression: Node::ConditionalExpression(Box::new(cond_expr)),
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
