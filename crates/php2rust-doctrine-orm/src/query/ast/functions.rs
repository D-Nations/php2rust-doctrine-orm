//! `Doctrine\ORM\Query\AST\Functions`: the DQL functions. A function node
//! parses its arguments with the [`Parser`] and generates its SQL with the
//! [`SqlWalker`].
//!
//! Upstream: doctrine/orm src/Query/AST/Functions/FunctionNode.php src/Query/AST/Functions/AbsFunction.php src/Query/AST/Functions/AvgFunction.php src/Query/AST/Functions/BitAndFunction.php src/Query/AST/Functions/BitOrFunction.php src/Query/AST/Functions/ConcatFunction.php src/Query/AST/Functions/CountFunction.php src/Query/AST/Functions/CurrentDateFunction.php src/Query/AST/Functions/CurrentTimeFunction.php src/Query/AST/Functions/CurrentTimestampFunction.php src/Query/AST/Functions/DateAddFunction.php src/Query/AST/Functions/DateDiffFunction.php src/Query/AST/Functions/DateSubFunction.php src/Query/AST/Functions/IdentityFunction.php src/Query/AST/Functions/LengthFunction.php src/Query/AST/Functions/LocateFunction.php src/Query/AST/Functions/LowerFunction.php src/Query/AST/Functions/MaxFunction.php src/Query/AST/Functions/MinFunction.php src/Query/AST/Functions/ModFunction.php src/Query/AST/Functions/SizeFunction.php src/Query/AST/Functions/SqrtFunction.php src/Query/AST/Functions/SubstringFunction.php src/Query/AST/Functions/SumFunction.php src/Query/AST/Functions/TrimFunction.php src/Query/AST/Functions/UpperFunction.php

use std::fmt;
use std::fmt::Write as _;
use std::sync::Arc;

use php_value::{Key, Thrown};
use php2rust_doctrine_dbal::platforms::TrimMode;
use php2rust_doctrine_dbal::types::Type;

use super::{AggregateExpression, Node, SharedPathExpression};
use crate::query::parser::Parser;
use crate::query::query_exception;
use crate::query::sql_walker::{SqlWalker, dbal_error, dispatch};
use crate::query::token_type::TokenType;

/// `Doctrine\ORM\Query\AST\Functions\FunctionNode`: an abstract node
/// class whose subclasses parse and walk themselves.
pub trait FunctionNode: fmt::Debug {
    /// The function's class (`$function::class`).
    fn class(&self) -> &str;

    /// `$name`: the name the function was called with.
    fn name(&self) -> &str;

    /// `getSql($sqlWalker)`.
    fn get_sql(&self, sql_walker: &mut dyn SqlWalker<'_>) -> Result<String, Thrown>;

    /// `parse($parser)`.
    fn parse(&mut self, parser: &mut Parser<'_>) -> Result<(), Thrown>;

    /// `getReturnTypeName()`, for a function that implements
    /// `ExpressionWithReturnType`.
    fn return_type_name(&self) -> Option<String> {
        None
    }

    /// `getReturnType()`, for a function that implements the deprecated
    /// `TypedExpression` (and not `ExpressionWithReturnType`).
    fn return_type(&self) -> Option<Result<Arc<dyn Type>, Thrown>> {
        None
    }

    /// `$function->pathExpression`, when the class has that public
    /// property (`isset($resultVariable->pathExpression)`).
    fn path_expression(&self) -> Option<SharedPathExpression> {
        None
    }
}

macro_rules! class_name {
    ($short:literal) => {
        concat!("Doctrine\\ORM\\Query\\AST\\Functions\\", $short)
    };
}

/// The built-in functions (`Parser::$stringFunctions`,
/// `$numericFunctions`, `$datetimeFunctions`).
pub(crate) fn builtin(lower_name: &str) -> Option<Box<dyn FunctionNode>> {
    let name = lower_name.to_owned();
    Some(match lower_name {
        "concat" => Box::new(ConcatFunction::new(name)),
        "substring" => Box::new(SubstringFunction::new(name)),
        "trim" => Box::new(TrimFunction::new(name)),
        "lower" => Box::new(LowerFunction::new(name)),
        "upper" => Box::new(UpperFunction::new(name)),
        "identity" => Box::new(IdentityFunction::new(name)),
        "length" => Box::new(LengthFunction::new(name)),
        "locate" => Box::new(LocateFunction::new(name)),
        "abs" => Box::new(AbsFunction::new(name)),
        "sqrt" => Box::new(SqrtFunction::new(name)),
        "mod" => Box::new(ModFunction::new(name)),
        "size" => Box::new(SizeFunction::new(name)),
        "date_diff" => Box::new(DateDiffFunction::new(name)),
        "bit_and" => Box::new(BitAndFunction::new(name)),
        "bit_or" => Box::new(BitOrFunction::new(name)),
        "min" => Box::new(AggregateFunction::new(name, class_name!("MinFunction"))),
        "max" => Box::new(AggregateFunction::new(name, class_name!("MaxFunction"))),
        "avg" => Box::new(AggregateFunction::new(name, class_name!("AvgFunction"))),
        "sum" => Box::new(AggregateFunction::new(name, class_name!("SumFunction"))),
        "count" => Box::new(AggregateFunction::new(name, class_name!("CountFunction"))),
        "current_date" => Box::new(CurrentFunction::new(name, CurrentKind::Date)),
        "current_time" => Box::new(CurrentFunction::new(name, CurrentKind::Time)),
        "current_timestamp" => Box::new(CurrentFunction::new(name, CurrentKind::Timestamp)),
        "date_add" => Box::new(DateAddFunction::new(name, false)),
        "date_sub" => Box::new(DateAddFunction::new(name, true)),
        _ => return None,
    })
}

/// Whether `name` (lower case) is in `Parser::$stringFunctions`.
pub(crate) fn is_string_function(name: &str) -> bool {
    matches!(
        name,
        "concat" | "substring" | "trim" | "lower" | "upper" | "identity"
    )
}

/// Whether `name` (lower case) is in `Parser::$numericFunctions`.
pub(crate) fn is_numeric_function(name: &str) -> bool {
    matches!(
        name,
        "length"
            | "locate"
            | "abs"
            | "sqrt"
            | "mod"
            | "size"
            | "date_diff"
            | "bit_and"
            | "bit_or"
            | "min"
            | "max"
            | "avg"
            | "sum"
            | "count"
    )
}

/// Whether `name` (lower case) is in `Parser::$datetimeFunctions`.
pub(crate) fn is_datetime_function(name: &str) -> bool {
    matches!(
        name,
        "current_date" | "current_time" | "current_timestamp" | "date_add" | "date_sub"
    )
}

/// A typed property that is read before it is initialized.
fn uninitialized(class: &str, property: &str) -> Thrown {
    Thrown::new(
        "Error",
        format!("Typed property {class}::${property} must not be accessed before initialization"),
        0,
    )
}

fn required<'n>(node: Option<&'n Node>, class: &str, property: &str) -> Result<&'n Node, Thrown> {
    node.ok_or_else(|| uninitialized(class, property))
}

/// `$parser->match(T_IDENTIFIER); $parser->match(T_OPEN_PARENTHESIS);`
fn open(parser: &mut Parser<'_>) -> Result<(), Thrown> {
    parser.match_token(TokenType::T_IDENTIFIER)?;
    parser.match_token(TokenType::T_OPEN_PARENTHESIS)
}

fn close(parser: &mut Parser<'_>) -> Result<(), Thrown> {
    parser.match_token(TokenType::T_CLOSE_PARENTHESIS)
}

/// A function of one simple arithmetic expression whose SQL wraps it:
/// `ABS()`, `SQRT()`.
#[derive(Debug, Clone)]
pub struct AbsFunction {
    pub name: String,
    pub simple_arithmetic_expression: Option<Node>,
}

impl AbsFunction {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            simple_arithmetic_expression: None,
        }
    }
}

impl FunctionNode for AbsFunction {
    fn class(&self) -> &'static str {
        class_name!("AbsFunction")
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn get_sql(&self, sql_walker: &mut dyn SqlWalker<'_>) -> Result<String, Thrown> {
        let expr = required(
            self.simple_arithmetic_expression.as_ref(),
            self.class(),
            "simpleArithmeticExpression",
        )?;
        Ok(format!(
            "ABS({})",
            sql_walker.walk_simple_arithmetic_expression(expr)?
        ))
    }

    fn parse(&mut self, parser: &mut Parser<'_>) -> Result<(), Thrown> {
        open(parser)?;
        self.simple_arithmetic_expression = Some(parser.simple_arithmetic_expression()?);
        close(parser)
    }
}

#[derive(Debug, Clone)]
pub struct SqrtFunction {
    pub name: String,
    pub simple_arithmetic_expression: Option<Node>,
}

impl SqrtFunction {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            simple_arithmetic_expression: None,
        }
    }
}

impl FunctionNode for SqrtFunction {
    fn class(&self) -> &'static str {
        class_name!("SqrtFunction")
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn get_sql(&self, sql_walker: &mut dyn SqlWalker<'_>) -> Result<String, Thrown> {
        let expr = required(
            self.simple_arithmetic_expression.as_ref(),
            self.class(),
            "simpleArithmeticExpression",
        )?;
        Ok(format!(
            "SQRT({})",
            sql_walker.walk_simple_arithmetic_expression(expr)?
        ))
    }

    fn parse(&mut self, parser: &mut Parser<'_>) -> Result<(), Thrown> {
        open(parser)?;
        self.simple_arithmetic_expression = Some(parser.simple_arithmetic_expression()?);
        close(parser)
    }
}

/// `AvgFunction`, `CountFunction`, `MaxFunction`, `MinFunction` and
/// `SumFunction`: an aggregate expression.
#[derive(Debug, Clone)]
pub struct AggregateFunction {
    pub name: String,
    class: &'static str,
    aggregate_expression: Option<AggregateExpression>,
}

impl AggregateFunction {
    pub fn new(name: impl Into<String>, class: &'static str) -> Self {
        Self {
            name: name.into(),
            class,
            aggregate_expression: None,
        }
    }
}

impl FunctionNode for AggregateFunction {
    fn class(&self) -> &str {
        self.class
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn get_sql(&self, sql_walker: &mut dyn SqlWalker<'_>) -> Result<String, Thrown> {
        let expr = self
            .aggregate_expression
            .as_ref()
            .ok_or_else(|| uninitialized(self.class, "aggregateExpression"))?;
        sql_walker.walk_aggregate_expression(expr)
    }

    fn parse(&mut self, parser: &mut Parser<'_>) -> Result<(), Thrown> {
        self.aggregate_expression = Some(parser.aggregate_expression()?);
        Ok(())
    }

    fn return_type_name(&self) -> Option<String> {
        (self.class == class_name!("CountFunction")).then(|| "integer".to_owned())
    }
}

/// `BitAndFunction` and `BitOrFunction`.
#[derive(Debug, Clone)]
pub struct BitAndFunction {
    pub name: String,
    pub first_arithmetic: Option<Node>,
    pub second_arithmetic: Option<Node>,
    or: bool,
}

impl BitAndFunction {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            first_arithmetic: None,
            second_arithmetic: None,
            or: false,
        }
    }
}

/// `BitOrFunction`.
#[derive(Debug, Clone)]
pub struct BitOrFunction;

impl BitOrFunction {
    #[allow(
        clippy::new_ret_no_self,
        reason = "BitOrFunction shares BitAndFunction's node"
    )]
    pub fn new(name: impl Into<String>) -> BitAndFunction {
        BitAndFunction {
            or: true,
            ..BitAndFunction::new(name)
        }
    }
}

impl FunctionNode for BitAndFunction {
    fn class(&self) -> &str {
        if self.or {
            class_name!("BitOrFunction")
        } else {
            class_name!("BitAndFunction")
        }
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn get_sql(&self, sql_walker: &mut dyn SqlWalker<'_>) -> Result<String, Thrown> {
        let first = required(
            self.first_arithmetic.as_ref(),
            self.class(),
            "firstArithmetic",
        )?;
        let second = required(
            self.second_arithmetic.as_ref(),
            self.class(),
            "secondArithmetic",
        )?;
        let first = dispatch(sql_walker, first)?;
        let second = dispatch(sql_walker, second)?;
        let platform = sql_walker.get_entity_manager().get_database_platform();
        Ok(if self.or {
            platform.get_bit_or_comparison_expression(&first, &second)
        } else {
            platform.get_bit_and_comparison_expression(&first, &second)
        })
    }

    fn parse(&mut self, parser: &mut Parser<'_>) -> Result<(), Thrown> {
        open(parser)?;
        self.first_arithmetic = Some(parser.arithmetic_primary()?);
        parser.match_token(TokenType::T_COMMA)?;
        self.second_arithmetic = Some(parser.arithmetic_primary()?);
        close(parser)
    }
}

/// `CONCAT()`.
#[derive(Debug, Clone)]
pub struct ConcatFunction {
    pub name: String,
    pub first_string_primary: Option<Node>,
    pub second_string_primary: Option<Node>,
    pub concat_expressions: Vec<Node>,
}

impl ConcatFunction {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            first_string_primary: None,
            second_string_primary: None,
            concat_expressions: Vec::new(),
        }
    }
}

impl FunctionNode for ConcatFunction {
    fn class(&self) -> &'static str {
        class_name!("ConcatFunction")
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn get_sql(&self, sql_walker: &mut dyn SqlWalker<'_>) -> Result<String, Thrown> {
        let mut args = Vec::new();
        for expression in &self.concat_expressions {
            args.push(sql_walker.walk_string_primary(expression)?);
        }
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        Ok(sql_walker
            .get_entity_manager()
            .get_database_platform()
            .get_concat_expression(&args))
    }

    fn parse(&mut self, parser: &mut Parser<'_>) -> Result<(), Thrown> {
        open(parser)?;
        let first = parser.string_primary()?;
        self.first_string_primary = Some(first.clone());
        self.concat_expressions.push(first);
        parser.match_token(TokenType::T_COMMA)?;
        let second = parser.string_primary()?;
        self.second_string_primary = Some(second.clone());
        self.concat_expressions.push(second);
        while parser.is_next_token(TokenType::T_COMMA) {
            parser.match_token(TokenType::T_COMMA)?;
            let next = parser.string_primary()?;
            self.concat_expressions.push(next);
        }
        close(parser)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CurrentKind {
    Date,
    Time,
    Timestamp,
}

/// `CurrentDateFunction`, `CurrentTimeFunction` and
/// `CurrentTimestampFunction`.
#[derive(Debug, Clone)]
pub struct CurrentFunction {
    pub name: String,
    kind: CurrentKind,
}

impl CurrentFunction {
    fn new(name: String, kind: CurrentKind) -> Self {
        Self { name, kind }
    }
}

impl FunctionNode for CurrentFunction {
    fn class(&self) -> &str {
        match self.kind {
            CurrentKind::Date => class_name!("CurrentDateFunction"),
            CurrentKind::Time => class_name!("CurrentTimeFunction"),
            CurrentKind::Timestamp => class_name!("CurrentTimestampFunction"),
        }
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn get_sql(&self, sql_walker: &mut dyn SqlWalker<'_>) -> Result<String, Thrown> {
        let platform = sql_walker.get_entity_manager().get_database_platform();
        Ok(match self.kind {
            CurrentKind::Date => platform.get_current_date_sql(),
            CurrentKind::Time => platform.get_current_time_sql(),
            CurrentKind::Timestamp => platform.get_current_timestamp_sql(),
        })
    }

    fn parse(&mut self, parser: &mut Parser<'_>) -> Result<(), Thrown> {
        open(parser)?;
        close(parser)
    }
}

/// `DATE_ADD()` and (`sub`) `DATE_SUB()`.
#[derive(Debug, Clone)]
pub struct DateAddFunction {
    pub name: String,
    pub first_date_expression: Option<Node>,
    pub interval_expression: Option<Node>,
    pub unit: Option<Node>,
    sub: bool,
}

impl DateAddFunction {
    pub fn new(name: impl Into<String>, sub: bool) -> Self {
        Self {
            name: name.into(),
            first_date_expression: None,
            interval_expression: None,
            unit: None,
            sub,
        }
    }
}

impl FunctionNode for DateAddFunction {
    fn class(&self) -> &str {
        if self.sub {
            class_name!("DateSubFunction")
        } else {
            class_name!("DateAddFunction")
        }
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn get_sql(&self, sql_walker: &mut dyn SqlWalker<'_>) -> Result<String, Thrown> {
        let class = self.class();
        let unit = required(self.unit.as_ref(), class, "unit")?;
        // (string) $this->unit->value: the unit is a Literal or an
        // InputParameter (whose value is not a property: null).
        let unit = match unit {
            Node::Literal(literal) => literal.value.clone().unwrap_or_default(),
            _ => String::new(),
        }
        .to_lowercase();
        let first = required(
            self.first_date_expression.as_ref(),
            class,
            "firstDateExpression",
        )?;
        let interval = required(
            self.interval_expression.as_ref(),
            class,
            "intervalExpression",
        )?;
        if !matches!(
            unit.as_str(),
            "second" | "minute" | "hour" | "day" | "week" | "month" | "year"
        ) {
            return Err(query_exception::semantical_error(if self.sub {
                "DATE_SUB() only supports units of type second, minute, hour, day, week, month and year."
            } else {
                "DATE_ADD() only supports units of type second, minute, hour, day, week, month and year."
            }));
        }
        let date = dispatch(sql_walker, first)?;
        let interval = dispatch(sql_walker, interval)?;
        let platform = sql_walker.get_entity_manager().get_database_platform();
        let sql = match (unit.as_str(), self.sub) {
            ("second", false) => platform.get_date_add_seconds_expression(&date, &interval),
            ("minute", false) => platform.get_date_add_minutes_expression(&date, &interval),
            ("hour", false) => platform.get_date_add_hour_expression(&date, &interval),
            ("day", false) => platform.get_date_add_days_expression(&date, &interval),
            ("week", false) => platform.get_date_add_weeks_expression(&date, &interval),
            ("month", false) => platform.get_date_add_month_expression(&date, &interval),
            ("second", true) => platform.get_date_sub_seconds_expression(&date, &interval),
            ("minute", true) => platform.get_date_sub_minutes_expression(&date, &interval),
            ("hour", true) => platform.get_date_sub_hour_expression(&date, &interval),
            ("day", true) => platform.get_date_sub_days_expression(&date, &interval),
            ("week", true) => platform.get_date_sub_weeks_expression(&date, &interval),
            ("month", true) => platform.get_date_sub_month_expression(&date, &interval),
            (_, true) => platform.get_date_sub_years_expression(&date, &interval),
            (_, false) => platform.get_date_add_years_expression(&date, &interval),
        };
        sql.map_err(dbal_error)
    }

    fn parse(&mut self, parser: &mut Parser<'_>) -> Result<(), Thrown> {
        open(parser)?;
        self.first_date_expression = Some(parser.arithmetic_primary()?);
        parser.match_token(TokenType::T_COMMA)?;
        self.interval_expression = Some(parser.arithmetic_primary()?);
        parser.match_token(TokenType::T_COMMA)?;
        self.unit = Some(parser.string_primary()?);
        close(parser)
    }
}

/// `DATE_DIFF()`.
#[derive(Debug, Clone)]
pub struct DateDiffFunction {
    pub name: String,
    pub date1: Option<Node>,
    pub date2: Option<Node>,
}

impl DateDiffFunction {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            date1: None,
            date2: None,
        }
    }
}

impl FunctionNode for DateDiffFunction {
    fn class(&self) -> &'static str {
        class_name!("DateDiffFunction")
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn get_sql(&self, sql_walker: &mut dyn SqlWalker<'_>) -> Result<String, Thrown> {
        let date1 = required(self.date1.as_ref(), self.class(), "date1")?;
        let date2 = required(self.date2.as_ref(), self.class(), "date2")?;
        let date1 = dispatch(sql_walker, date1)?;
        let date2 = dispatch(sql_walker, date2)?;
        sql_walker
            .get_entity_manager()
            .get_database_platform()
            .get_date_diff_expression(&date1, &date2)
            .map_err(dbal_error)
    }

    fn parse(&mut self, parser: &mut Parser<'_>) -> Result<(), Thrown> {
        open(parser)?;
        self.date1 = Some(parser.arithmetic_primary()?);
        parser.match_token(TokenType::T_COMMA)?;
        self.date2 = Some(parser.arithmetic_primary()?);
        close(parser)
    }
}

/// `IDENTITY()`.
#[derive(Debug, Clone)]
pub struct IdentityFunction {
    pub name: String,
    pub path_expression: Option<SharedPathExpression>,
    pub field_mapping: Option<String>,
}

impl IdentityFunction {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            path_expression: None,
            field_mapping: None,
        }
    }
}

impl FunctionNode for IdentityFunction {
    fn class(&self) -> &'static str {
        class_name!("IdentityFunction")
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn get_sql(&self, sql_walker: &mut dyn SqlWalker<'_>) -> Result<String, Thrown> {
        let path = self
            .path_expression
            .as_ref()
            .ok_or_else(|| uninitialized(self.class(), "pathExpression"))?
            .borrow()
            .clone();
        let entity_manager = sql_walker.get_entity_manager();
        let dql_alias = path.identification_variable.clone();
        let assoc_field = path.field.clone().unwrap_or_default();
        let class = sql_walker.get_metadata_for_dql_alias(&dql_alias)?;
        let assoc = class
            .read()
            .association_mappings
            .get(&assoc_field)
            .cloned()
            .ok_or_else(|| {
                Thrown::new(
                    "Error",
                    "Attempt to read property \"targetEntity\" on null",
                    0,
                )
            })?;
        let target_entity = entity_manager.get_class_metadata(&assoc.target_entity)?;
        if !assoc.is_to_one_owning_side() {
            return Err(
                query_exception::association_path_inverse_side_not_supported(
                    &path.identification_variable,
                    path.field.as_deref(),
                ),
            );
        }
        let mut join_column = assoc.join_columns.first().cloned();
        if let Some(field_mapping) = &self.field_mapping {
            let target = target_entity.read();
            let Some(field) = target.field_mappings.get(field_mapping) else {
                return Err(query_exception::new(format!(
                    "Undefined reference field mapping \"{field_mapping}\""
                )));
            };
            join_column = assoc
                .join_columns
                .iter()
                .find(|mapping| mapping.referenced_column_name == field.column_name)
                .cloned();
            if join_column.is_none() {
                return Err(query_exception::new(format!(
                    "Unable to resolve the reference field mapping \"{field_mapping}\""
                )));
            }
        }
        // The table with the relation may be a subclass, so get the table name from the association definition
        let table_name = entity_manager
            .get_class_metadata(&assoc.source_entity)?
            .read()
            .get_table_name()?;
        let table_alias = sql_walker.get_sql_table_alias(&table_name, &dql_alias);
        let join_column = join_column.ok_or_else(|| {
            Thrown::new(
                "TypeError",
                "Doctrine\\ORM\\Mapping\\DefaultQuoteStrategy::getJoinColumnName(): Argument #1 ($joinColumnMapping) must be of type Doctrine\\ORM\\Mapping\\JoinColumnMapping, bool given",
                0,
            )
        })?;
        let entity_manager = sql_walker.get_entity_manager();
        let column_name = entity_manager.get_quote_strategy().get_join_column_name(
            &join_column,
            &target_entity.read(),
            entity_manager.get_database_platform(),
        );
        Ok(format!("{table_alias}.{column_name}"))
    }

    fn parse(&mut self, parser: &mut Parser<'_>) -> Result<(), Thrown> {
        open(parser)?;
        self.path_expression = Some(parser.single_valued_association_path_expression()?);
        if parser.is_next_token(TokenType::T_COMMA) {
            parser.match_token(TokenType::T_COMMA)?;
            parser.match_token(TokenType::T_STRING)?;
            self.field_mapping = parser.token_value();
        }
        close(parser)
    }

    fn path_expression(&self) -> Option<SharedPathExpression> {
        self.path_expression.clone()
    }
}

/// A function of one string primary: `LENGTH()`, `LOWER()`, `UPPER()`.
#[derive(Debug, Clone)]
pub struct StringPrimaryFunction {
    pub name: String,
    pub string_primary: Option<Node>,
    class: &'static str,
}

/// `LENGTH()`.
pub struct LengthFunction;
/// `LOWER()`.
pub struct LowerFunction;
/// `UPPER()`.
pub struct UpperFunction;

macro_rules! string_primary_function {
    ($ty:ident, $class:literal) => {
        impl $ty {
            #[allow(clippy::new_ret_no_self, reason = "the functions share one node")]
            pub fn new(name: impl Into<String>) -> StringPrimaryFunction {
                StringPrimaryFunction {
                    name: name.into(),
                    string_primary: None,
                    class: class_name!($class),
                }
            }
        }
    };
}

string_primary_function!(LengthFunction, "LengthFunction");
string_primary_function!(LowerFunction, "LowerFunction");
string_primary_function!(UpperFunction, "UpperFunction");

impl FunctionNode for StringPrimaryFunction {
    fn class(&self) -> &str {
        self.class
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn get_sql(&self, sql_walker: &mut dyn SqlWalker<'_>) -> Result<String, Thrown> {
        let primary = required(self.string_primary.as_ref(), self.class, "stringPrimary")?;
        let sql = sql_walker.walk_simple_arithmetic_expression(primary)?;
        Ok(match self.class {
            class_name!("LengthFunction") => sql_walker
                .get_entity_manager()
                .get_database_platform()
                .get_length_expression(&sql),
            class_name!("LowerFunction") => format!("LOWER({sql})"),
            _ => format!("UPPER({sql})"),
        })
    }

    fn parse(&mut self, parser: &mut Parser<'_>) -> Result<(), Thrown> {
        open(parser)?;
        self.string_primary = Some(parser.string_primary()?);
        close(parser)
    }

    fn return_type_name(&self) -> Option<String> {
        (self.class == class_name!("LengthFunction")).then(|| "integer".to_owned())
    }
}

/// `LOCATE()`.
#[derive(Debug, Clone)]
pub struct LocateFunction {
    pub name: String,
    pub first_string_primary: Option<Node>,
    pub second_string_primary: Option<Node>,
    /// `false` until set.
    pub simple_arithmetic_expression: Option<Node>,
}

impl LocateFunction {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            first_string_primary: None,
            second_string_primary: None,
            simple_arithmetic_expression: None,
        }
    }
}

impl FunctionNode for LocateFunction {
    fn class(&self) -> &'static str {
        class_name!("LocateFunction")
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn get_sql(&self, sql_walker: &mut dyn SqlWalker<'_>) -> Result<String, Thrown> {
        let first = required(
            self.first_string_primary.as_ref(),
            self.class(),
            "firstStringPrimary",
        )?;
        let second = required(
            self.second_string_primary.as_ref(),
            self.class(),
            "secondStringPrimary",
        )?;
        let first_string = sql_walker.walk_string_primary(first)?;
        let second_string = sql_walker.walk_string_primary(second)?;
        let start = match &self.simple_arithmetic_expression {
            Some(expr) if expr.is_truthy() => {
                Some(sql_walker.walk_simple_arithmetic_expression(expr)?)
            }
            _ => None,
        };
        sql_walker
            .get_entity_manager()
            .get_database_platform()
            .get_locate_expression(&second_string, &first_string, start.as_deref())
            .map_err(dbal_error)
    }

    fn parse(&mut self, parser: &mut Parser<'_>) -> Result<(), Thrown> {
        open(parser)?;
        self.first_string_primary = Some(parser.string_primary()?);
        parser.match_token(TokenType::T_COMMA)?;
        self.second_string_primary = Some(parser.string_primary()?);
        if parser.is_next_token(TokenType::T_COMMA) {
            parser.match_token(TokenType::T_COMMA)?;
            self.simple_arithmetic_expression = Some(parser.simple_arithmetic_expression()?);
        }
        close(parser)
    }
}

/// `MOD()`.
#[derive(Debug, Clone)]
pub struct ModFunction {
    pub name: String,
    pub first_simple_arithmetic_expression: Option<Node>,
    pub second_simple_arithmetic_expression: Option<Node>,
}

impl ModFunction {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            first_simple_arithmetic_expression: None,
            second_simple_arithmetic_expression: None,
        }
    }
}

impl FunctionNode for ModFunction {
    fn class(&self) -> &'static str {
        class_name!("ModFunction")
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn get_sql(&self, sql_walker: &mut dyn SqlWalker<'_>) -> Result<String, Thrown> {
        let first = required(
            self.first_simple_arithmetic_expression.as_ref(),
            self.class(),
            "firstSimpleArithmeticExpression",
        )?;
        let second = required(
            self.second_simple_arithmetic_expression.as_ref(),
            self.class(),
            "secondSimpleArithmeticExpression",
        )?;
        let first = sql_walker.walk_simple_arithmetic_expression(first)?;
        let second = sql_walker.walk_simple_arithmetic_expression(second)?;
        Ok(sql_walker
            .get_entity_manager()
            .get_database_platform()
            .get_mod_expression(&first, &second))
    }

    fn parse(&mut self, parser: &mut Parser<'_>) -> Result<(), Thrown> {
        open(parser)?;
        self.first_simple_arithmetic_expression = Some(parser.simple_arithmetic_expression()?);
        parser.match_token(TokenType::T_COMMA)?;
        self.second_simple_arithmetic_expression = Some(parser.simple_arithmetic_expression()?);
        close(parser)
    }
}

/// `SIZE()`.
#[derive(Debug, Clone)]
pub struct SizeFunction {
    pub name: String,
    pub collection_path_expression: Option<SharedPathExpression>,
}

impl SizeFunction {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            collection_path_expression: None,
        }
    }
}

/// `$class->fieldNames[$column]` (an undefined key is `null`, which
/// `getColumnName()` refuses).
fn field_name_of(
    class: &crate::mapping::ClassMetadataInfo,
    column: &str,
) -> Result<String, Thrown> {
    class.field_names.get(column).cloned().ok_or_else(|| {
        Thrown::new(
            "TypeError",
            "Doctrine\\ORM\\Mapping\\DefaultQuoteStrategy::getColumnName(): Argument #1 ($fieldName) must be of type string, null given",
            0,
        )
    })
}

/// The string pairs of a `targetToSourceKeyColumns` array.
pub(crate) fn key_columns(columns: &php_value::PhpArray) -> Vec<(String, String)> {
    columns
        .iter()
        .map(|(key, value)| {
            let key = match key {
                Key::Int(i) => i.to_string(),
                Key::Str(s) => s.to_string_lossy().into_owned(),
            };
            let value = match value {
                php_value::Value::String(s) => s.to_string_lossy().into_owned(),
                php_value::Value::Int(i) => i.to_string(),
                _ => String::new(),
            };
            (key, value)
        })
        .collect()
}

impl FunctionNode for SizeFunction {
    fn class(&self) -> &'static str {
        class_name!("SizeFunction")
    }

    fn name(&self) -> &str {
        &self.name
    }

    #[allow(clippy::too_many_lines, reason = "one PHP method")]
    fn get_sql(&self, sql_walker: &mut dyn SqlWalker<'_>) -> Result<String, Thrown> {
        let path = self
            .collection_path_expression
            .as_ref()
            .ok_or_else(|| uninitialized(self.class(), "collectionPathExpression"))?
            .borrow()
            .clone();
        let entity_manager = sql_walker.get_entity_manager();
        let dql_alias = path.identification_variable.clone();
        let assoc_field = path.field.clone().unwrap_or_default();
        let class = sql_walker.get_metadata_for_dql_alias(&dql_alias)?;
        let assoc = class
            .read()
            .association_mappings
            .get(&assoc_field)
            .cloned()
            .ok_or_else(|| {
                Thrown::new(
                    "Error",
                    "Call to a member function isOneToMany() on null",
                    0,
                )
            })?;
        let mut sql = String::from("SELECT COUNT(*) FROM ");
        if assoc.is_one_to_many() {
            let target_class = entity_manager.get_class_metadata(&assoc.target_entity)?;
            let target_table = target_class.read().get_table_name()?;
            let target_table_alias = sql_walker.get_sql_table_alias(&target_table, "");
            let source_table = class.read().get_table_name()?;
            let source_table_alias = sql_walker.get_sql_table_alias(&source_table, &dql_alias);
            let entity_manager = sql_walker.get_entity_manager();
            let platform = entity_manager.get_database_platform();
            let quote_strategy = entity_manager.get_quote_strategy();
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
                .ok_or_else(|| {
                    Thrown::new(
                        "Error",
                        "Attempt to read property \"targetToSourceKeyColumns\" on null",
                        0,
                    )
                })?;
            let class_info = class.read();
            let mut first = true;
            for (target_column, source_column) in
                key_columns(&owning_assoc.target_to_source_key_columns)
            {
                if first {
                    first = false;
                } else {
                    sql.push_str(" AND ");
                }
                let field = field_name_of(&class_info, &target_column)?;
                let _ = write!(
                    sql,
                    "{target_table_alias}.{source_column} = {source_table_alias}.{}",
                    quote_strategy.get_column_name(&field, &class_info, platform)
                );
            }
        } else {
            // many-to-many
            let owning_assoc = entity_manager.get_owning_side(&assoc)?;
            let join_table = owning_assoc.join_table.clone().ok_or_else(|| {
                Thrown::new("Error", "Attempt to read property \"name\" on null", 0)
            })?;
            // SQL table aliases
            let join_table_alias = sql_walker.get_sql_table_alias(&join_table.name, "");
            let source_table = class.read().get_table_name()?;
            let source_table_alias = sql_walker.get_sql_table_alias(&source_table, &dql_alias);
            // join to target table
            let entity_manager = sql_walker.get_entity_manager();
            let platform = entity_manager.get_database_platform();
            let quote_strategy = entity_manager.get_quote_strategy();
            let target_class = entity_manager.get_class_metadata(&assoc.target_entity)?;
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
            let class_info = class.read();
            let mut first = true;
            for join_column in join_columns {
                if first {
                    first = false;
                } else {
                    sql.push_str(" AND ");
                }
                let field = field_name_of(&class_info, &join_column.referenced_column_name)?;
                let source_column_name =
                    quote_strategy.get_column_name(&field, &class_info, platform);
                let _ = write!(
                    sql,
                    "{join_table_alias}.{} = {source_table_alias}.{source_column_name}",
                    join_column.name
                );
            }
        }
        Ok(format!("({sql})"))
    }

    fn parse(&mut self, parser: &mut Parser<'_>) -> Result<(), Thrown> {
        open(parser)?;
        self.collection_path_expression = Some(parser.collection_valued_path_expression()?);
        close(parser)
    }
}

/// `SUBSTRING()`.
#[derive(Debug, Clone)]
pub struct SubstringFunction {
    pub name: String,
    pub string_primary: Option<Node>,
    pub first_simple_arithmetic_expression: Option<Node>,
    pub second_simple_arithmetic_expression: Option<Node>,
}

impl SubstringFunction {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            string_primary: None,
            first_simple_arithmetic_expression: None,
            second_simple_arithmetic_expression: None,
        }
    }
}

impl FunctionNode for SubstringFunction {
    fn class(&self) -> &'static str {
        class_name!("SubstringFunction")
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn get_sql(&self, sql_walker: &mut dyn SqlWalker<'_>) -> Result<String, Thrown> {
        let mut optional_second = None;
        if let Some(second) = &self.second_simple_arithmetic_expression {
            optional_second = Some(sql_walker.walk_simple_arithmetic_expression(second)?);
        }
        let primary = required(self.string_primary.as_ref(), self.class(), "stringPrimary")?;
        let first = required(
            self.first_simple_arithmetic_expression.as_ref(),
            self.class(),
            "firstSimpleArithmeticExpression",
        )?;
        let string = sql_walker.walk_string_primary(primary)?;
        let start = sql_walker.walk_simple_arithmetic_expression(first)?;
        Ok(sql_walker
            .get_entity_manager()
            .get_database_platform()
            .get_substring_expression(&string, &start, optional_second.as_deref()))
    }

    fn parse(&mut self, parser: &mut Parser<'_>) -> Result<(), Thrown> {
        open(parser)?;
        self.string_primary = Some(parser.string_primary()?);
        parser.match_token(TokenType::T_COMMA)?;
        self.first_simple_arithmetic_expression = Some(parser.simple_arithmetic_expression()?);
        if parser.is_next_token(TokenType::T_COMMA) {
            parser.match_token(TokenType::T_COMMA)?;
            self.second_simple_arithmetic_expression = Some(parser.simple_arithmetic_expression()?);
        }
        close(parser)
    }
}

/// `TRIM()`.
#[derive(Debug, Clone)]
pub struct TrimFunction {
    pub name: String,
    pub leading: bool,
    pub trailing: bool,
    pub both: bool,
    /// `false` until set.
    pub trim_char: Option<String>,
    pub string_primary: Option<Node>,
}

impl TrimFunction {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            leading: false,
            trailing: false,
            both: false,
            trim_char: None,
            string_primary: None,
        }
    }

    fn get_trim_mode(&self) -> TrimMode {
        if self.leading {
            return TrimMode::Leading;
        }
        if self.trailing {
            return TrimMode::Trailing;
        }
        if self.both {
            return TrimMode::Both;
        }
        TrimMode::Unspecified
    }

    fn parse_trim_mode(&mut self, parser: &mut Parser<'_>) -> Result<(), Thrown> {
        let value = parser.lookahead_value().unwrap_or_default();
        if value.eq_ignore_ascii_case("leading") {
            parser.match_token(TokenType::T_LEADING)?;
            self.leading = true;
            return Ok(());
        }
        if value.eq_ignore_ascii_case("trailing") {
            parser.match_token(TokenType::T_TRAILING)?;
            self.trailing = true;
            return Ok(());
        }
        if value.eq_ignore_ascii_case("both") {
            parser.match_token(TokenType::T_BOTH)?;
            self.both = true;
        }
        Ok(())
    }
}

impl FunctionNode for TrimFunction {
    fn class(&self) -> &'static str {
        class_name!("TrimFunction")
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn get_sql(&self, sql_walker: &mut dyn SqlWalker<'_>) -> Result<String, Thrown> {
        let primary = required(self.string_primary.as_ref(), self.class(), "stringPrimary")?;
        let string_primary = sql_walker.walk_string_primary(primary)?;
        let platform = sql_walker.get_entity_manager().get_database_platform();
        let trim_mode = self.get_trim_mode();
        if let Some(trim_char) = &self.trim_char {
            return Ok(platform.get_trim_expression(
                &string_primary,
                trim_mode,
                Some(&platform.quote_string_literal(trim_char)),
            ));
        }
        Ok(platform.get_trim_expression(&string_primary, trim_mode, None))
    }

    fn parse(&mut self, parser: &mut Parser<'_>) -> Result<(), Thrown> {
        open(parser)?;
        self.parse_trim_mode(parser)?;
        if parser.is_next_token(TokenType::T_STRING) {
            parser.match_token(TokenType::T_STRING)?;
            self.trim_char = parser.token_value();
        }
        if self.leading || self.trailing || self.both || self.trim_char.is_some() {
            parser.match_token(TokenType::T_FROM)?;
        }
        self.string_primary = Some(parser.string_primary()?);
        close(parser)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_classes() {
        let names = [
            "concat",
            "count",
            "date_sub",
            "bit_or",
            "upper",
            "current_time",
        ];
        let classes: Vec<String> = names
            .iter()
            .map(|name| {
                builtin(name)
                    .map(|f| f.class().to_owned())
                    .unwrap_or_default()
            })
            .collect();
        assert_eq!(
            classes,
            [
                class_name!("ConcatFunction"),
                class_name!("CountFunction"),
                class_name!("DateSubFunction"),
                class_name!("BitOrFunction"),
                class_name!("UpperFunction"),
                class_name!("CurrentTimeFunction"),
            ]
        );
        assert!(builtin("nope").is_none());
    }
}
