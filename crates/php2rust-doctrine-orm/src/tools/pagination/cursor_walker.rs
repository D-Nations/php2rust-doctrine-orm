//! `CursorWalker`: the tree walker of cursor-based pagination, with the
//! `OrderDirection` and `CursorOrderByItem` it uses.
//!
//! The ORDER BY items reach the paginator as `CursorOrderByItem` objects
//! in the `HINT_CURSOR_ORDER_BY_ITEMS` hint; an item keeps the alias and
//! field of a path expression (the AST node is not `Send`) or the string
//! of another expression. The cursor parameters are bound with the
//! query's `setParameter()` when the walker walks a `Query`.
//!
//! Upstream: doctrine/orm src/Tools/Pagination/CursorWalker.php src/Tools/Pagination/OrderDirection.php src/Tools/Pagination/CursorOrderByItem.php

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use php_value::{Key, PhpArray, PhpObject, Thrown, Value};

use crate::mapping::ClassMetadata;
use crate::query::ast::{
    ComparisonExpression, ConditionalExpression, ConditionalPrimary, ConditionalTerm,
    DeleteStatement, InputParameter, Node, OrderByClause, OrderByItem, SelectStatement,
    UpdateStatement, WhereClause,
};
use crate::query::context::{DqlQuery, Hint, QueryComponents, TreeWalkerClass};
use crate::query::{ParserResult, TreeWalker, TreeWalkerAdapter};

/// `OrderDirection` (`@internal`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderDirection {
    Ascending,
    Descending,
}

impl OrderDirection {
    /// The case's value.
    pub fn value(self) -> &'static str {
        match self {
            Self::Ascending => "ASC",
            Self::Descending => "DESC",
        }
    }

    pub fn from_order_by_item(item: &OrderByItem, reverse: bool) -> Self {
        let direction = if item.is_asc() {
            Self::Ascending
        } else {
            Self::Descending
        };

        if reverse {
            direction.reversed()
        } else {
            direction
        }
    }

    pub fn operator(self) -> &'static str {
        match self {
            Self::Ascending => ">",
            Self::Descending => "<",
        }
    }

    #[must_use]
    pub fn reversed(self) -> Self {
        match self {
            Self::Ascending => Self::Descending,
            Self::Descending => Self::Ascending,
        }
    }
}

/// The `expression` of a `CursorOrderByItem`: `PathExpression|string`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CursorOrderByExpression {
    /// A path expression's identification variable and field.
    PathExpression {
        identification_variable: String,
        field: Option<String>,
    },
    /// Another expression, as a string.
    String(String),
}

/// `CursorOrderByItem` (`@internal`).
#[derive(Debug, Clone)]
pub struct CursorOrderByItem {
    pub expression: CursorOrderByExpression,
    pub direction: OrderDirection,
    pub param_key: String,
    pub metadata: Option<Arc<ClassMetadata>>,
}

impl CursorOrderByItem {
    pub const CLASS: &'static str = "Doctrine\\ORM\\Tools\\Pagination\\CursorOrderByItem";
}

impl PhpObject for CursorOrderByItem {
    fn class(&self) -> &str {
        Self::CLASS
    }

    fn properties(&self) -> PhpArray {
        let mut properties = PhpArray::new();
        properties.insert("direction", Value::from(self.direction.value()));
        properties.insert("paramKey", Value::from(self.param_key.as_str()));
        properties
    }
}

/// `$query->getHint(CursorWalker::HINT_CURSOR_ORDER_BY_ITEMS) ?: []`.
pub(crate) fn order_by_items_hint(hint: &Hint) -> Vec<CursorOrderByItem> {
    let Hint::Value(Value::Array(items)) = hint else {
        return Vec::new();
    };
    items
        .values()
        .filter_map(|item| {
            item.as_object()
                .and_then(|object| object.downcast_ref::<CursorOrderByItem>())
                .cloned()
        })
        .collect()
}

/// TreeWalker for cursor-based pagination.
///
/// Responsibilities:
/// - Extract ORDER BY columns from AST
/// - Inject WHERE conditions for cursor navigation
/// - Reverse ORDER BY direction for previous page navigation
pub struct CursorWalker<'a> {
    adapter: TreeWalkerAdapter<'a>,
}

impl CursorWalker<'_> {
    pub const HINT_CURSOR_PARAMETERS: &'static str = "doctrine.cursor.parameters";
    pub const HINT_CURSOR_REVERSE: &'static str = "doctrine.cursor.reverse";
    pub const HINT_CURSOR_ORDER_BY_ITEMS: &'static str = "doctrine.cursor.order_by_items";
    pub const HINT_QUERY_PRODUCES_DUPLICATES: &'static str =
        "doctrine.cursor.query_produces_duplicates";

    /// Prefix of the parameters bound for the cursor predicate. Names are
    /// namespaced so that they cannot collide with the parameters of the
    /// paginated query, whose values would then be silently overwritten.
    const CURSOR_PARAMETER_PREFIX: &'static str = "dctrn_cursor_";

    /// `CursorWalker::class`.
    pub const CLASS: TreeWalkerClass = TreeWalkerClass {
        name: "Doctrine\\ORM\\Tools\\Pagination\\CursorWalker",
        construct,
    };
}

fn construct<'a>(
    query: &'a dyn DqlQuery,
    parser_result: Rc<RefCell<ParserResult>>,
    query_components: QueryComponents,
) -> Box<dyn TreeWalker<'a> + 'a> {
    Box::new(CursorWalker {
        adapter: TreeWalkerAdapter::new(query, parser_result, query_components),
    })
}

fn primary(simple: Option<Node>, conditional: Option<Node>) -> Node {
    Node::ConditionalPrimary(Box::new(ConditionalPrimary {
        simple_conditional_expression: simple,
        conditional_expression: conditional,
    }))
}

fn term(factors: Vec<Node>) -> Node {
    Node::ConditionalTerm(Box::new(ConditionalTerm {
        conditional_factors: factors,
    }))
}

fn expression(terms: Vec<Node>) -> Node {
    Node::ConditionalExpression(Box::new(ConditionalExpression {
        conditional_terms: terms,
    }))
}

impl CursorWalker<'_> {
    /// Recursively builds a cursor condition:
    /// (col1 > :val1) OR (col1 = :val1 AND col2 > :val2) OR ...
    fn build_cursor_condition(
        &self,
        order_by_items: &[(Node, CursorOrderByItem)],
        cursor_parameters: &Value,
        index: usize,
    ) -> Result<Option<Node>, Thrown> {
        let Some((expression_node, order_by_item)) = order_by_items.get(index) else {
            return Ok(None);
        };
        let direction = order_by_item.direction;
        let param_key = &order_by_item.param_key;

        let operator = direction.operator();

        let param_name = format!("{}{index}", CursorWalker::CURSOR_PARAMETER_PREFIX);
        // `$cursorParameters[$paramKey] ?? null`.
        let param_value = match cursor_parameters {
            Value::Array(parameters) => parameters
                .get(&Key::from(param_key.as_str()))
                .cloned()
                .unwrap_or(Value::Null),
            _ => Value::Null,
        };

        // Security note: $paramKey is derived from the DQL ORDER BY AST, not from the
        // cursor payload. A tampered cursor can only influence the *values* used as pivot
        // points, not the columns being filtered. All values are bound via setParameter(),
        // so SQL injection is not possible. The worst a user can do is navigate to an
        // arbitrary position in the result set, while remaining bound by the original
        // query's WHERE constraints.
        if let Some(query) = self.adapter.get_query().as_query() {
            query.set_parameter(param_name.as_str(), param_value, None);
        }

        let comparison = |operator: &str| -> Result<Node, Thrown> {
            Ok(Node::ComparisonExpression(Box::new(ComparisonExpression {
                left_expression: expression_node.clone(),
                operator: operator.to_owned(),
                right_expression: Node::InputParameter(Box::new(InputParameter::new(&format!(
                    ":{param_name}"
                ))?)),
            })))
        };

        let comparison_primary = primary(Some(comparison(operator)?), None);

        if index + 1 == order_by_items.len() {
            return Ok(Some(expression(vec![term(vec![comparison_primary])])));
        }

        let next_condition =
            self.build_cursor_condition(order_by_items, cursor_parameters, index + 1)?;

        let equality_primary = primary(Some(comparison("=")?), None);

        let next_primary = primary(None, next_condition);

        let and_primary = primary(
            None,
            Some(expression(vec![term(vec![equality_primary, next_primary])])),
        );

        Ok(Some(expression(vec![
            term(vec![comparison_primary]),
            term(vec![and_primary]),
        ])))
    }

    /// Returns the parameter key for the given expression.
    fn get_parameter_key(expression: &Node) -> Result<CursorOrderByExpression, Thrown> {
        match expression {
            Node::PathExpression(path) => {
                let path = path.borrow();
                Ok(CursorOrderByExpression::PathExpression {
                    identification_variable: path.identification_variable.clone(),
                    field: path.field.clone(),
                })
            }
            Node::Str(s) => Ok(CursorOrderByExpression::String(s.clone())),
            // `(string) $expression` on an AST node.
            other => Err(Thrown::new(
                "Error",
                format!(
                    "Object of class {} could not be converted to string",
                    other.debug_type()
                ),
                0,
            )),
        }
    }
}

impl<'a> TreeWalker<'a> for CursorWalker<'a> {
    fn get_query_components(&self) -> &QueryComponents {
        self.adapter.get_query_components()
    }

    fn walk_select_statement(
        &mut self,
        select_statement: &mut SelectStatement,
    ) -> Result<(), Thrown> {
        let query = self.adapter.get_query();
        let should_reverse = query.get_hint(CursorWalker::HINT_CURSOR_REVERSE).is_true();
        let cursor_parameters = query.get_hint(CursorWalker::HINT_CURSOR_PARAMETERS);

        let Some(order_by_clause) = &select_statement.order_by_clause else {
            return Err(Thrown::new(
                "LogicException",
                "No ORDER BY clause found. Cursor pagination requires a deterministic sort order.",
                0,
            ));
        };

        let mut order_by_items = Vec::new();
        let mut new_order_by_items = Vec::new();

        for order_by_item in &order_by_clause.order_by_items {
            let direction = OrderDirection::from_order_by_item(order_by_item, should_reverse);

            let expression = Self::get_parameter_key(&order_by_item.expression)?;
            let (param_key, metadata) = match &expression {
                CursorOrderByExpression::PathExpression {
                    identification_variable,
                    field,
                } => (
                    format!(
                        "{identification_variable}.{}",
                        field.as_deref().unwrap_or_default()
                    ),
                    Some(
                        self.adapter
                            .get_metadata_for_dql_alias(identification_variable)?,
                    ),
                ),
                CursorOrderByExpression::String(s) => (s.clone(), None),
            };

            order_by_items.push((
                order_by_item.expression.clone(),
                CursorOrderByItem {
                    expression,
                    direction,
                    param_key,
                    metadata,
                },
            ));

            new_order_by_items.push(OrderByItem {
                expression: order_by_item.expression.clone(),
                type_: direction.value().to_owned(),
            });
        }

        select_statement.order_by_clause = Some(OrderByClause {
            order_by_items: new_order_by_items,
            include_collection_order_by_items: true,
        });

        query.set_hint(
            CursorWalker::HINT_CURSOR_ORDER_BY_ITEMS,
            Hint::Value(Value::Array(PhpArray::list(
                order_by_items
                    .iter()
                    .map(|(_, item)| Value::Object(Arc::new(item.clone()))),
            ))),
        );

        // empty($cursorParameters)
        let cursor_parameters = match cursor_parameters {
            Hint::Value(value) if value.is_truthy() => value,
            Hint::Value(_) => return Ok(()),
            // A walker or lock mode hint: not an array of parameters.
            _ => Value::Null,
        };

        let Some(condition) =
            self.build_cursor_condition(&order_by_items, &cursor_parameters, 0)?
        else {
            return Ok(());
        };

        let conditional_primary = primary(None, Some(condition));

        if let Some(where_clause) = select_statement.where_clause.as_mut() {
            let conditional_expression =
                std::mem::replace(&mut where_clause.conditional_expression, Node::str(""));
            where_clause.conditional_expression = match conditional_expression {
                Node::ConditionalTerm(mut existing) => {
                    existing.conditional_factors.push(conditional_primary);
                    Node::ConditionalTerm(existing)
                }
                existing @ Node::ConditionalPrimary(_) => {
                    expression(vec![term(vec![existing, conditional_primary])])
                }
                other => {
                    let existing_primary = primary(None, Some(other));
                    term(vec![existing_primary, conditional_primary])
                }
            };
        } else {
            select_statement.where_clause = Some(WhereClause {
                conditional_expression: expression(vec![term(vec![conditional_primary])]),
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
