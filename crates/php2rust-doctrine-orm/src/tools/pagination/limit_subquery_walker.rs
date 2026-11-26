//! `LimitSubqueryWalker`: selects the distinct identifiers of the root
//! entity (and the ORDER BY items), for the paginator's limit subquery.
//!
//! Upstream: doctrine/orm src/Tools/Pagination/LimitSubqueryWalker.php

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use php_value::{PhpArray, PhpObject, Thrown, Value};
use php2rust_doctrine_dbal::types::{self, Type};

use crate::mapping::AssociationMapping;
use crate::query::ast::functions::IdentityFunction;
use crate::query::ast::{
    DeleteStatement, Node, PathExpression, SelectExpression, SelectStatement,
    TYPE_SINGLE_VALUED_ASSOCIATION, TYPE_STATE_FIELD, UpdateStatement,
};
use crate::query::context::{DqlQuery, Hint, QueryComponents, TreeWalkerClass};
use crate::query::sql_walker::dbal_error;
use crate::query::{ParserResult, TreeWalker, TreeWalkerAdapter};

use super::{HINT_ENABLE_DISTINCT, root_alias, runtime_exception};

/// Replaces the selectClause of the AST with a SELECT DISTINCT root.id equivalent.
pub struct LimitSubqueryWalker<'a> {
    adapter: TreeWalkerAdapter<'a>,
    /// Counter for generating unique order column aliases.
    alias_counter: i64,
}

/// A DBAL `Type` object as a query hint value
/// (`LimitSubqueryWalker::IDENTIFIER_TYPE`).
#[derive(Debug, Clone)]
pub struct TypeObject(pub Arc<dyn Type>);

impl PhpObject for TypeObject {
    fn class(&self) -> &str {
        self.0.class()
    }

    fn properties(&self) -> PhpArray {
        PhpArray::new()
    }
}

impl LimitSubqueryWalker<'_> {
    pub const IDENTIFIER_TYPE: &'static str = "doctrine_paginator.id.type";

    pub const FORCE_DBAL_TYPE_CONVERSION: &'static str =
        "doctrine_paginator.scalar_result.force_dbal_type_conversion";

    /// `LimitSubqueryWalker::class`.
    pub const CLASS: TreeWalkerClass = TreeWalkerClass {
        name: "Doctrine\\ORM\\Tools\\Pagination\\LimitSubqueryWalker",
        construct,
    };
}

fn construct<'a>(
    query: &'a dyn DqlQuery,
    parser_result: Rc<RefCell<ParserResult>>,
    query_components: QueryComponents,
) -> Box<dyn TreeWalker<'a> + 'a> {
    Box::new(LimitSubqueryWalker {
        adapter: TreeWalkerAdapter::new(query, parser_result, query_components),
        alias_counter: 0,
    })
}

impl LimitSubqueryWalker<'_> {
    /// Prevent LimitSubqueryWalker from being used with queries that include
    /// a limit, a fetched to-many join, and an order by condition that
    /// references a column from the fetch joined table.
    fn validate(&self, ast: &SelectStatement) -> Result<(), Thrown> {
        let query_components = self.adapter.get_query_components();
        let query = self.adapter.get_query();
        let from = &ast.from_clause.identification_variable_declarations;
        let has_joins = from.first().is_some_and(|root| !root.joins.is_empty());

        if query.get_max_results().is_some()
            && let Some(order_by_clause) = &ast.order_by_clause
            && has_joins
        {
            // Check each orderby item.
            // TODO: check complex orderby items too...
            for order_by_item in &order_by_clause.order_by_items {
                let Node::PathExpression(expression) = &order_by_item.expression else {
                    continue;
                };
                let identification_variable = expression.borrow().identification_variable.clone();
                if let Some(query_component) = query_components.get(&identification_variable)
                    && query_component.parent.is_some()
                    && query_component
                        .relation
                        .as_ref()
                        .is_some_and(AssociationMapping::is_to_many)
                {
                    return Err(runtime_exception(
                        "Cannot select distinct identifiers from query with LIMIT and ORDER BY on a column from a fetch joined to-many association. Use output walkers.",
                    ));
                }
            }
        }

        Ok(())
    }

    fn create_select_expression_item(path_expression: &PathExpression) -> Node {
        if path_expression.type_ == Some(TYPE_SINGLE_VALUED_ASSOCIATION) {
            let mut identity = IdentityFunction::new("identity");
            identity.path_expression = Some(Rc::new(RefCell::new(path_expression.clone())));

            return Node::Function(Rc::new(identity));
        }

        Node::path(path_expression.clone())
    }
}

fn select_expression(
    expression: Node,
    field_identification_variable: &str,
) -> Rc<SelectExpression> {
    Rc::new(SelectExpression {
        expression,
        field_identification_variable: Some(field_identification_variable.to_owned()),
        hidden_alias_result_variable: false,
    })
}

impl<'a> TreeWalker<'a> for LimitSubqueryWalker<'a> {
    fn get_query_components(&self) -> &QueryComponents {
        self.adapter.get_query_components()
    }

    fn walk_select_statement(
        &mut self,
        select_statement: &mut SelectStatement,
    ) -> Result<(), Thrown> {
        // Get the root entity and alias from the AST fromClause
        let root_alias = root_alias(
            &select_statement
                .from_clause
                .identification_variable_declarations,
        )?;
        let root_class = self.adapter.get_metadata_for_dql_alias(&root_alias)?;

        self.validate(select_statement)?;
        let identifier = root_class.read().get_single_identifier_field_name()?;

        if root_class
            .read()
            .association_mappings
            .contains_key(&identifier)
        {
            return Err(runtime_exception(
                "Paginating an entity with foreign key as identifier only works when using the Output Walkers. Pass $useOutputWalkers = true to the paginator constructor.",
            ));
        }

        let query = self.adapter.get_query();
        let type_name = root_class
            .read()
            .field_mappings
            .get(&identifier)
            .map(|mapping| mapping.r#type.clone())
            .ok_or_else(|| Thrown::new("Error", "Attempt to read property \"type\" on null", 0))?;
        let type_ = types::get_type(&type_name).map_err(|e| dbal_error(e.into()))?;
        query.set_hint(
            LimitSubqueryWalker::IDENTIFIER_TYPE,
            Hint::Value(Value::Object(Arc::new(TypeObject(type_)))),
        );

        query.set_hint(
            LimitSubqueryWalker::FORCE_DBAL_TYPE_CONVERSION,
            Hint::Value(Value::Bool(true)),
        );

        let mut path_expression = PathExpression::new(
            TYPE_STATE_FIELD | TYPE_SINGLE_VALUED_ASSOCIATION,
            &root_alias,
            Some(&identifier),
        );

        path_expression.type_ = Some(TYPE_STATE_FIELD);

        select_statement.select_clause.select_expressions =
            vec![select_expression(Node::path(path_expression), "_dctrn_id")];
        // `($query->getHints()[HINT_ENABLE_DISTINCT] ?? true) === true`.
        select_statement.select_clause.is_distinct =
            !query.has_hint(HINT_ENABLE_DISTINCT) || query.get_hint(HINT_ENABLE_DISTINCT).is_true();

        let Some(order_by_clause) = &select_statement.order_by_clause else {
            return Ok(());
        };

        let query_components = self.adapter.get_query_components();
        let mut added = Vec::new();
        for item in &order_by_clause.order_by_items {
            if let Node::PathExpression(expression) = &item.expression {
                let alias = format!("_dctrn_ord{}", self.alias_counter);
                self.alias_counter += 1;
                added.push(select_expression(
                    Self::create_select_expression_item(&expression.borrow()),
                    &alias,
                ));

                continue;
            }

            if let Node::Str(expression) = &item.expression
                && let Some(q_comp) = query_components.get(expression)
                && let Some(result_variable) = &q_comp.result_variable
            {
                added.push(select_expression(result_variable.clone(), expression));
            }
        }
        select_statement
            .select_clause
            .select_expressions
            .extend(added);

        Ok(())
    }

    fn walk_update_statement(&mut self, _: &mut UpdateStatement) -> Result<(), Thrown> {
        Ok(())
    }

    fn walk_delete_statement(&mut self, _: &mut DeleteStatement) -> Result<(), Thrown> {
        Ok(())
    }
}
