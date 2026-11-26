//! `Doctrine\ORM\Query\AST`: the nodes of a parsed DQL query.
//!
//! PHP's nodes are objects of the abstract class `Node`; a property typed
//! `Node|string` or `mixed` is a [`Node`] here ([`Node::Str`] for a
//! string), a property of a single class is that struct. Nodes are owned
//! values, except where PHP relies on sharing one object: a
//! `PathExpression` and a `NewObjectExpression` (the parser completes them
//! after parsing) are `Rc<RefCell<_>>`, a `SelectExpression` is an `Rc`
//! (the parser finds it again by identity) and a function node is an
//! `Rc<dyn FunctionNode>`. `Node::dispatch()` is
//! [`crate::query::sql_walker::dispatch`]; `Node::__toString()`/`dump()`
//! are not ported.
//!
//! Upstream: doctrine/orm src/Query/AST/Node.php src/Query/AST/AggregateExpression.php src/Query/AST/ArithmeticExpression.php src/Query/AST/ArithmeticFactor.php src/Query/AST/ArithmeticTerm.php src/Query/AST/BetweenExpression.php src/Query/AST/CoalesceExpression.php src/Query/AST/CollectionMemberExpression.php src/Query/AST/ComparisonExpression.php src/Query/AST/ConditionalExpression.php src/Query/AST/ConditionalFactor.php src/Query/AST/ConditionalPrimary.php src/Query/AST/ConditionalTerm.php src/Query/AST/DeleteClause.php src/Query/AST/DeleteStatement.php src/Query/AST/EmptyCollectionComparisonExpression.php src/Query/AST/EntityAsDtoArgumentExpression.php src/Query/AST/ExistsExpression.php src/Query/AST/ExpressionWithReturnType.php src/Query/AST/FromClause.php src/Query/AST/GeneralCaseExpression.php src/Query/AST/GroupByClause.php src/Query/AST/HavingClause.php src/Query/AST/IdentificationVariableDeclaration.php src/Query/AST/InListExpression.php src/Query/AST/InSubselectExpression.php src/Query/AST/IndexBy.php src/Query/AST/InputParameter.php src/Query/AST/InstanceOfExpression.php src/Query/AST/Join.php src/Query/AST/JoinAssociationDeclaration.php src/Query/AST/JoinAssociationPathExpression.php src/Query/AST/JoinClassPathExpression.php src/Query/AST/JoinVariableDeclaration.php src/Query/AST/LikeExpression.php src/Query/AST/Literal.php src/Query/AST/NewObjectExpression.php src/Query/AST/NullComparisonExpression.php src/Query/AST/NullIfExpression.php src/Query/AST/OrderByClause.php src/Query/AST/OrderByItem.php src/Query/AST/ParenthesisExpression.php src/Query/AST/PartialObjectExpression.php src/Query/AST/PathExpression.php src/Query/AST/Phase2OptimizableConditional.php src/Query/AST/QuantifiedExpression.php src/Query/AST/RangeVariableDeclaration.php src/Query/AST/SelectClause.php src/Query/AST/SelectExpression.php src/Query/AST/SelectStatement.php src/Query/AST/SimpleArithmeticExpression.php src/Query/AST/SimpleCaseExpression.php src/Query/AST/SimpleSelectClause.php src/Query/AST/SimpleSelectExpression.php src/Query/AST/SimpleWhenClause.php src/Query/AST/Subselect.php src/Query/AST/SubselectFromClause.php src/Query/AST/SubselectIdentificationVariableDeclaration.php src/Query/AST/TypedExpression.php src/Query/AST/UpdateClause.php src/Query/AST/UpdateItem.php src/Query/AST/UpdateStatement.php src/Query/AST/WhenClause.php src/Query/AST/WhereClause.php

pub mod functions;

use std::cell::RefCell;
use std::rc::Rc;

use php_value::numeric::is_numeric;
use php_value::{Key, Thrown};

use super::query_exception;
pub use functions::FunctionNode;

/// A `PathExpression` the parser and the walkers share.
pub type SharedPathExpression = Rc<RefCell<PathExpression>>;

/// A `NewObjectExpression` the parser completes after parsing.
pub type SharedNewObjectExpression = Rc<RefCell<NewObjectExpression>>;

/// A node, or a string where PHP's property is `Node|string` or `mixed`.
#[derive(Debug, Clone)]
pub enum Node {
    /// A string: an identification variable, a result variable or an
    /// arithmetic operator.
    Str(String),
    AggregateExpression(Box<AggregateExpression>),
    ArithmeticExpression(Box<ArithmeticExpression>),
    ArithmeticFactor(Box<ArithmeticFactor>),
    ArithmeticTerm(Box<ArithmeticTerm>),
    BetweenExpression(Box<BetweenExpression>),
    CoalesceExpression(Box<CoalesceExpression>),
    CollectionMemberExpression(Box<CollectionMemberExpression>),
    ComparisonExpression(Box<ComparisonExpression>),
    ConditionalExpression(Box<ConditionalExpression>),
    ConditionalFactor(Box<ConditionalFactor>),
    ConditionalPrimary(Box<ConditionalPrimary>),
    ConditionalTerm(Box<ConditionalTerm>),
    DeleteClause(Box<DeleteClause>),
    DeleteStatement(Box<DeleteStatement>),
    EmptyCollectionComparisonExpression(Box<EmptyCollectionComparisonExpression>),
    EntityAsDtoArgumentExpression(Box<EntityAsDtoArgumentExpression>),
    ExistsExpression(Box<ExistsExpression>),
    FromClause(Box<FromClause>),
    Function(Rc<dyn FunctionNode>),
    GeneralCaseExpression(Box<GeneralCaseExpression>),
    GroupByClause(Box<GroupByClause>),
    HavingClause(Box<HavingClause>),
    IdentificationVariableDeclaration(Box<IdentificationVariableDeclaration>),
    InListExpression(Box<InListExpression>),
    InSubselectExpression(Box<InSubselectExpression>),
    IndexBy(Box<IndexBy>),
    InputParameter(Box<InputParameter>),
    InstanceOfExpression(Box<InstanceOfExpression>),
    Join(Box<Join>),
    JoinAssociationDeclaration(Box<JoinAssociationDeclaration>),
    JoinAssociationPathExpression(Box<JoinAssociationPathExpression>),
    JoinClassPathExpression(Box<JoinClassPathExpression>),
    JoinVariableDeclaration(Box<JoinVariableDeclaration>),
    LikeExpression(Box<LikeExpression>),
    Literal(Box<Literal>),
    NewObjectExpression(SharedNewObjectExpression),
    NullComparisonExpression(Box<NullComparisonExpression>),
    NullIfExpression(Box<NullIfExpression>),
    OrderByClause(Box<OrderByClause>),
    OrderByItem(Box<OrderByItem>),
    ParenthesisExpression(Box<ParenthesisExpression>),
    PartialObjectExpression(Box<PartialObjectExpression>),
    PathExpression(SharedPathExpression),
    QuantifiedExpression(Box<QuantifiedExpression>),
    RangeVariableDeclaration(Box<RangeVariableDeclaration>),
    SelectClause(Box<SelectClause>),
    SelectExpression(Rc<SelectExpression>),
    SelectStatement(Box<SelectStatement>),
    SimpleArithmeticExpression(Box<SimpleArithmeticExpression>),
    SimpleCaseExpression(Box<SimpleCaseExpression>),
    SimpleSelectClause(Box<SimpleSelectClause>),
    SimpleSelectExpression(Box<SimpleSelectExpression>),
    SimpleWhenClause(Box<SimpleWhenClause>),
    Subselect(Box<Subselect>),
    SubselectFromClause(Box<SubselectFromClause>),
    UpdateClause(Box<UpdateClause>),
    UpdateItem(Box<UpdateItem>),
    UpdateStatement(Box<UpdateStatement>),
    WhenClause(Box<WhenClause>),
    WhereClause(Box<WhereClause>),
}

impl Node {
    /// A string node.
    pub fn str(value: impl Into<String>) -> Self {
        Self::Str(value.into())
    }

    /// A path expression node.
    pub fn path(path: PathExpression) -> Self {
        Self::PathExpression(Rc::new(RefCell::new(path)))
    }

    /// The string, for [`Node::Str`] (`is_string()`).
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Str(s) => Some(s),
            _ => None,
        }
    }

    /// The path expression, for [`Node::PathExpression`].
    pub fn as_path(&self) -> Option<&SharedPathExpression> {
        match self {
            Self::PathExpression(p) => Some(p),
            _ => None,
        }
    }

    /// `(bool) $value`: a node object is truthy, a string unless it is
    /// `""` or `"0"`.
    pub fn is_truthy(&self) -> bool {
        match self {
            Self::Str(s) => !(s.is_empty() || s == "0"),
            _ => true,
        }
    }

    /// `get_debug_type($node)`.
    pub fn debug_type(&self) -> String {
        let short = match self {
            Self::Str(_) => return "string".to_owned(),
            Self::Function(f) => return f.class().to_owned(),
            Self::AggregateExpression(_) => "AggregateExpression",
            Self::ArithmeticExpression(_) => "ArithmeticExpression",
            Self::ArithmeticFactor(_) => "ArithmeticFactor",
            Self::ArithmeticTerm(_) => "ArithmeticTerm",
            Self::BetweenExpression(_) => "BetweenExpression",
            Self::CoalesceExpression(_) => "CoalesceExpression",
            Self::CollectionMemberExpression(_) => "CollectionMemberExpression",
            Self::ComparisonExpression(_) => "ComparisonExpression",
            Self::ConditionalExpression(_) => "ConditionalExpression",
            Self::ConditionalFactor(_) => "ConditionalFactor",
            Self::ConditionalPrimary(_) => "ConditionalPrimary",
            Self::ConditionalTerm(_) => "ConditionalTerm",
            Self::DeleteClause(_) => "DeleteClause",
            Self::DeleteStatement(_) => "DeleteStatement",
            Self::EmptyCollectionComparisonExpression(_) => "EmptyCollectionComparisonExpression",
            Self::EntityAsDtoArgumentExpression(_) => "EntityAsDtoArgumentExpression",
            Self::ExistsExpression(_) => "ExistsExpression",
            Self::FromClause(_) => "FromClause",
            Self::GeneralCaseExpression(_) => "GeneralCaseExpression",
            Self::GroupByClause(_) => "GroupByClause",
            Self::HavingClause(_) => "HavingClause",
            Self::IdentificationVariableDeclaration(_) => "IdentificationVariableDeclaration",
            Self::InListExpression(_) => "InListExpression",
            Self::InSubselectExpression(_) => "InSubselectExpression",
            Self::IndexBy(_) => "IndexBy",
            Self::InputParameter(_) => "InputParameter",
            Self::InstanceOfExpression(_) => "InstanceOfExpression",
            Self::Join(_) => "Join",
            Self::JoinAssociationDeclaration(_) => "JoinAssociationDeclaration",
            Self::JoinAssociationPathExpression(_) => "JoinAssociationPathExpression",
            Self::JoinClassPathExpression(_) => "JoinClassPathExpression",
            Self::JoinVariableDeclaration(_) => "JoinVariableDeclaration",
            Self::LikeExpression(_) => "LikeExpression",
            Self::Literal(_) => "Literal",
            Self::NewObjectExpression(_) => "NewObjectExpression",
            Self::NullComparisonExpression(_) => "NullComparisonExpression",
            Self::NullIfExpression(_) => "NullIfExpression",
            Self::OrderByClause(_) => "OrderByClause",
            Self::OrderByItem(_) => "OrderByItem",
            Self::ParenthesisExpression(_) => "ParenthesisExpression",
            Self::PartialObjectExpression(_) => "PartialObjectExpression",
            Self::PathExpression(_) => "PathExpression",
            Self::QuantifiedExpression(_) => "QuantifiedExpression",
            Self::RangeVariableDeclaration(_) => "RangeVariableDeclaration",
            Self::SelectClause(_) => "SelectClause",
            Self::SelectExpression(_) => "SelectExpression",
            Self::SelectStatement(_) => "SelectStatement",
            Self::SimpleArithmeticExpression(_) => "SimpleArithmeticExpression",
            Self::SimpleCaseExpression(_) => "SimpleCaseExpression",
            Self::SimpleSelectClause(_) => "SimpleSelectClause",
            Self::SimpleSelectExpression(_) => "SimpleSelectExpression",
            Self::SimpleWhenClause(_) => "SimpleWhenClause",
            Self::Subselect(_) => "Subselect",
            Self::SubselectFromClause(_) => "SubselectFromClause",
            Self::UpdateClause(_) => "UpdateClause",
            Self::UpdateItem(_) => "UpdateItem",
            Self::UpdateStatement(_) => "UpdateStatement",
            Self::WhenClause(_) => "WhenClause",
            Self::WhereClause(_) => "WhereClause",
        };
        format!("Doctrine\\ORM\\Query\\AST\\{short}")
    }
}

/// A statement: what `Parser::getAST()` returns.
#[derive(Debug, Clone)]
pub enum Statement {
    Select(SelectStatement),
    Update(UpdateStatement),
    Delete(DeleteStatement),
}

#[derive(Debug, Clone)]
pub struct AggregateExpression {
    pub function_name: String,
    pub path_expression: Node,
    pub is_distinct: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ArithmeticExpression {
    pub simple_arithmetic_expression: Option<Node>,
    pub subselect: Option<Subselect>,
}

impl ArithmeticExpression {
    pub fn is_simple_arithmetic_expression(&self) -> bool {
        self.simple_arithmetic_expression
            .as_ref()
            .is_some_and(Node::is_truthy)
    }

    pub fn is_subselect(&self) -> bool {
        self.subselect.is_some()
    }
}

#[derive(Debug, Clone)]
pub struct ArithmeticFactor {
    pub arithmetic_primary: Node,
    pub sign: Option<bool>,
}

impl ArithmeticFactor {
    pub fn is_positive_signed(&self) -> bool {
        self.sign == Some(true)
    }

    pub fn is_negative_signed(&self) -> bool {
        self.sign == Some(false)
    }
}

#[derive(Debug, Clone)]
pub struct ArithmeticTerm {
    pub arithmetic_factors: Vec<Node>,
}

#[derive(Debug, Clone)]
pub struct BetweenExpression {
    pub expression: ArithmeticExpression,
    pub left_between_expression: ArithmeticExpression,
    pub right_between_expression: ArithmeticExpression,
    pub not: bool,
}

#[derive(Debug, Clone)]
pub struct CoalesceExpression {
    pub scalar_expressions: Vec<Node>,
}

#[derive(Debug, Clone)]
pub struct CollectionMemberExpression {
    pub entity_expression: Node,
    pub collection_valued_path_expression: SharedPathExpression,
    pub not: bool,
}

#[derive(Debug, Clone)]
pub struct ComparisonExpression {
    pub left_expression: Node,
    pub operator: String,
    pub right_expression: Node,
}

#[derive(Debug, Clone)]
pub struct ConditionalExpression {
    pub conditional_terms: Vec<Node>,
}

/// `Phase2OptimizableConditional`.
#[derive(Debug, Clone)]
pub struct ConditionalFactor {
    pub conditional_primary: ConditionalPrimary,
    pub not: bool,
}

/// `Phase2OptimizableConditional`.
#[derive(Debug, Clone, Default)]
pub struct ConditionalPrimary {
    pub simple_conditional_expression: Option<Node>,
    pub conditional_expression: Option<Node>,
}

impl ConditionalPrimary {
    pub fn is_simple_conditional_expression(&self) -> bool {
        self.simple_conditional_expression.is_some()
    }

    pub fn is_conditional_expression(&self) -> bool {
        self.conditional_expression.is_some()
    }
}

/// `Phase2OptimizableConditional`.
#[derive(Debug, Clone)]
pub struct ConditionalTerm {
    pub conditional_factors: Vec<Node>,
}

#[derive(Debug, Clone)]
pub struct DeleteClause {
    pub abstract_schema_name: String,
    pub alias_identification_variable: String,
}

#[derive(Debug, Clone)]
pub struct DeleteStatement {
    pub delete_clause: DeleteClause,
    pub where_clause: Option<WhereClause>,
}

#[derive(Debug, Clone)]
pub struct EmptyCollectionComparisonExpression {
    pub expression: SharedPathExpression,
    pub not: bool,
}

#[derive(Debug, Clone)]
pub struct EntityAsDtoArgumentExpression {
    /// `mixed $expression`: the identification variable.
    pub expression: String,
    pub identification_variable: Option<String>,
    pub alias_variable: Option<String>,
}

impl EntityAsDtoArgumentExpression {
    pub fn new(
        expression: String,
        identification_variable: Option<String>,
        alias_variable: Option<String>,
    ) -> Self {
        // if (! $aliasVariable) $this->aliasVariable = $expression;
        let alias_variable = match alias_variable {
            Some(alias) if !(alias.is_empty() || alias == "0") => Some(alias),
            _ => Some(expression.clone()),
        };
        Self {
            expression,
            identification_variable,
            alias_variable,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExistsExpression {
    pub subselect: Subselect,
    pub not: bool,
}

#[derive(Debug, Clone)]
pub struct FromClause {
    pub identification_variable_declarations: Vec<IdentificationVariableDeclaration>,
}

#[derive(Debug, Clone)]
pub struct GeneralCaseExpression {
    pub when_clauses: Vec<WhenClause>,
    pub else_scalar_expression: Node,
}

#[derive(Debug, Clone)]
pub struct GroupByClause {
    /// `PathExpression|string` items.
    pub group_by_items: Vec<Node>,
}

#[derive(Debug, Clone)]
pub struct HavingClause {
    pub conditional_expression: Node,
}

#[derive(Debug, Clone, Default)]
pub struct IdentificationVariableDeclaration {
    pub range_variable_declaration: Option<RangeVariableDeclaration>,
    pub index_by: Option<IndexBy>,
    pub joins: Vec<Join>,
}

#[derive(Debug, Clone)]
pub struct InListExpression {
    pub expression: ArithmeticExpression,
    pub literals: Vec<Node>,
    pub not: bool,
}

#[derive(Debug, Clone)]
pub struct InSubselectExpression {
    pub expression: ArithmeticExpression,
    pub subselect: Subselect,
    pub not: bool,
}

#[derive(Debug, Clone)]
pub struct IndexBy {
    pub single_valued_path_expression: SharedPathExpression,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputParameter {
    pub is_named: bool,
    pub name: String,
}

impl InputParameter {
    /// `new InputParameter($value)`: `QueryException::invalidParameterFormat()`
    /// for a value of one character.
    pub fn new(value: &str) -> Result<Self, Thrown> {
        if value.len() == 1 {
            return Err(query_exception::invalid_parameter_format(value));
        }
        let param = value.get(1..).unwrap_or("").to_owned();
        Ok(Self {
            is_named: !is_numeric(param.as_bytes()),
            name: param,
        })
    }
}

#[derive(Debug, Clone)]
pub struct InstanceOfExpression {
    pub identification_variable: String,
    /// `InputParameter|string` values.
    pub value: Vec<Node>,
    pub not: bool,
}

/// `Join::JOIN_TYPE_LEFT`.
pub const JOIN_TYPE_LEFT: i64 = 1;
/// `Join::JOIN_TYPE_LEFTOUTER`.
pub const JOIN_TYPE_LEFTOUTER: i64 = 2;
/// `Join::JOIN_TYPE_INNER`.
pub const JOIN_TYPE_INNER: i64 = 3;

#[derive(Debug, Clone)]
pub struct Join {
    pub join_type: i64,
    /// A `RangeVariableDeclaration` or a `JoinAssociationDeclaration`.
    pub join_association_declaration: Option<Node>,
    pub conditional_expression: Option<Node>,
}

impl Join {
    pub fn new(join_type: i64, join_association_declaration: Option<Node>) -> Self {
        Self {
            join_type,
            join_association_declaration,
            conditional_expression: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct JoinAssociationDeclaration {
    pub join_association_path_expression: JoinAssociationPathExpression,
    pub alias_identification_variable: String,
    pub index_by: Option<IndexBy>,
}

#[derive(Debug, Clone)]
pub struct JoinAssociationPathExpression {
    pub identification_variable: String,
    pub association_field: String,
}

#[derive(Debug, Clone)]
pub struct JoinClassPathExpression {
    pub abstract_schema_name: String,
    pub alias_identification_variable: String,
}

#[derive(Debug, Clone)]
pub struct JoinVariableDeclaration {
    pub join: Join,
    pub index_by: Option<IndexBy>,
}

#[derive(Debug, Clone)]
pub struct LikeExpression {
    pub string_expression: Node,
    /// `InputParameter|FunctionNode|PathExpression|Literal`.
    pub string_pattern: Node,
    pub escape_char: Option<Literal>,
    pub not: bool,
}

/// `Literal::STRING`.
pub const LITERAL_STRING: i64 = 1;
/// `Literal::BOOLEAN`.
pub const LITERAL_BOOLEAN: i64 = 2;
/// `Literal::NUMERIC`.
pub const LITERAL_NUMERIC: i64 = 3;
/// `Literal::NULL`.
pub const LITERAL_NULL: i64 = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Literal {
    pub type_: i64,
    /// The token's text (`null` for `NULL`).
    pub value: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewObjectExpression {
    pub class_name: String,
    /// The arguments, keyed by position or (named arguments) by name.
    pub args: Vec<(Key, Node)>,
}

#[derive(Debug, Clone)]
pub struct NullComparisonExpression {
    pub expression: Node,
    pub not: bool,
}

#[derive(Debug, Clone)]
pub struct NullIfExpression {
    pub first_expression: Node,
    pub second_expression: Node,
}

#[derive(Debug, Clone)]
pub struct OrderByClause {
    pub order_by_items: Vec<OrderByItem>,
    pub include_collection_order_by_items: bool,
}

#[derive(Debug, Clone)]
pub struct OrderByItem {
    pub expression: Node,
    pub type_: String,
}

impl OrderByItem {
    pub fn is_asc(&self) -> bool {
        self.type_.eq_ignore_ascii_case("ASC")
    }

    pub fn is_desc(&self) -> bool {
        self.type_.eq_ignore_ascii_case("DESC")
    }
}

#[derive(Debug, Clone)]
pub struct ParenthesisExpression {
    pub expression: Node,
}

#[derive(Debug, Clone)]
pub struct PartialObjectExpression {
    pub identification_variable: String,
    pub partial_field_set: Vec<String>,
}

/// `PathExpression::TYPE_COLLECTION_VALUED_ASSOCIATION`.
pub const TYPE_COLLECTION_VALUED_ASSOCIATION: i64 = 2;
/// `PathExpression::TYPE_SINGLE_VALUED_ASSOCIATION`.
pub const TYPE_SINGLE_VALUED_ASSOCIATION: i64 = 4;
/// `PathExpression::TYPE_STATE_FIELD`.
pub const TYPE_STATE_FIELD: i64 = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathExpression {
    pub type_: Option<i64>,
    pub expected_type: i64,
    pub identification_variable: String,
    pub field: Option<String>,
}

impl PathExpression {
    pub fn new(expected_type: i64, identification_variable: &str, field: Option<&str>) -> Self {
        Self {
            type_: None,
            expected_type,
            identification_variable: identification_variable.to_owned(),
            field: field.map(str::to_owned),
        }
    }
}

#[derive(Debug, Clone)]
pub struct QuantifiedExpression {
    pub type_: String,
    pub subselect: Subselect,
}

impl QuantifiedExpression {
    pub fn is_all(&self) -> bool {
        self.type_.eq_ignore_ascii_case("ALL")
    }

    pub fn is_any(&self) -> bool {
        self.type_.eq_ignore_ascii_case("ANY")
    }

    pub fn is_some(&self) -> bool {
        self.type_.eq_ignore_ascii_case("SOME")
    }
}

#[derive(Debug, Clone)]
pub struct RangeVariableDeclaration {
    pub abstract_schema_name: String,
    pub alias_identification_variable: String,
    pub is_root: bool,
}

impl RangeVariableDeclaration {
    pub fn new(abstract_schema_name: &str, alias_identification_variable: &str) -> Self {
        Self {
            abstract_schema_name: abstract_schema_name.to_owned(),
            alias_identification_variable: alias_identification_variable.to_owned(),
            is_root: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SelectClause {
    pub select_expressions: Vec<Rc<SelectExpression>>,
    pub is_distinct: bool,
}

#[derive(Debug, Clone)]
pub struct SelectExpression {
    pub expression: Node,
    pub field_identification_variable: Option<String>,
    pub hidden_alias_result_variable: bool,
}

#[derive(Debug, Clone)]
pub struct SelectStatement {
    pub select_clause: SelectClause,
    pub from_clause: FromClause,
    pub where_clause: Option<WhereClause>,
    pub group_by_clause: Option<GroupByClause>,
    pub having_clause: Option<HavingClause>,
    pub order_by_clause: Option<OrderByClause>,
}

#[derive(Debug, Clone)]
pub struct SimpleArithmeticExpression {
    /// Terms, with the operators between them as strings.
    pub arithmetic_terms: Vec<Node>,
}

#[derive(Debug, Clone)]
pub struct SimpleCaseExpression {
    pub case_operand: Option<SharedPathExpression>,
    pub simple_when_clauses: Vec<SimpleWhenClause>,
    pub else_scalar_expression: Node,
}

#[derive(Debug, Clone)]
pub struct SimpleSelectClause {
    pub simple_select_expression: SimpleSelectExpression,
    pub is_distinct: bool,
}

#[derive(Debug, Clone)]
pub struct SimpleSelectExpression {
    pub field_identification_variable: Option<String>,
    pub expression: Node,
}

#[derive(Debug, Clone)]
pub struct SimpleWhenClause {
    pub case_scalar_expression: Node,
    pub then_scalar_expression: Node,
}

#[derive(Debug, Clone)]
pub struct Subselect {
    pub simple_select_clause: SimpleSelectClause,
    pub subselect_from_clause: SubselectFromClause,
    pub where_clause: Option<WhereClause>,
    pub group_by_clause: Option<GroupByClause>,
    pub having_clause: Option<HavingClause>,
    pub order_by_clause: Option<OrderByClause>,
}

#[derive(Debug, Clone)]
pub struct SubselectFromClause {
    pub identification_variable_declarations: Vec<IdentificationVariableDeclaration>,
}

/// `SubselectIdentificationVariableDeclaration` (not a `Node`; the parser
/// does not build it).
#[derive(Debug, Clone)]
pub struct SubselectIdentificationVariableDeclaration {
    pub association_path_expression: SharedPathExpression,
    pub alias_identification_variable: String,
}

#[derive(Debug, Clone)]
pub struct UpdateClause {
    pub abstract_schema_name: String,
    pub update_items: Vec<UpdateItem>,
    pub alias_identification_variable: String,
}

#[derive(Debug, Clone)]
pub struct UpdateItem {
    pub path_expression: SharedPathExpression,
    /// `InputParameter|ArithmeticExpression|null`.
    pub new_value: Option<Node>,
}

#[derive(Debug, Clone)]
pub struct UpdateStatement {
    pub update_clause: UpdateClause,
    pub where_clause: Option<WhereClause>,
}

#[derive(Debug, Clone)]
pub struct WhenClause {
    pub case_condition_expression: Node,
    pub then_scalar_expression: Node,
}

#[derive(Debug, Clone)]
pub struct WhereClause {
    pub conditional_expression: Node,
}
