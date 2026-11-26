//! `Doctrine\ORM\Query\Parser`: an LL(*) recursive-descent parser for the
//! context-free grammar of the Doctrine Query Language. Parses a DQL query,
//! reports any errors in it, and generates an SQL finalizer (or executor)
//! with the output walker.
//!
//! The grammar methods keep upstream's names in snake case
//! (`SelectStatement()` is [`Parser::select_statement`]); a method that
//! throws in PHP returns the `Thrown` as `Err`.
//!
//! Upstream: doctrine/orm src/Query/Parser.php

use std::cell::RefCell;
use std::fmt::Write as _;
use std::rc::Rc;

use php_value::{Key, PhpString, Thrown, Value};
use php2rust_doctrine_lexer::{AbstractLexer, Token};
use php2rust_doctrine_persistence::php_reflection::{self, ClassKind};

use crate::deprecations;
use crate::mapping::ordered_map::OrderedMap;
use crate::query::ast::{
    self, AggregateExpression, ArithmeticExpression, ArithmeticFactor, ArithmeticTerm,
    BetweenExpression, CoalesceExpression, CollectionMemberExpression, ComparisonExpression,
    ConditionalExpression, ConditionalFactor, ConditionalPrimary, ConditionalTerm, DeleteClause,
    DeleteStatement, EmptyCollectionComparisonExpression, EntityAsDtoArgumentExpression,
    ExistsExpression, FromClause, FunctionNode, GeneralCaseExpression, GroupByClause, HavingClause,
    IdentificationVariableDeclaration, InListExpression, InSubselectExpression, IndexBy,
    InputParameter, InstanceOfExpression, Join, JoinAssociationDeclaration,
    JoinAssociationPathExpression, LikeExpression, Literal, NewObjectExpression, Node,
    NullComparisonExpression, NullIfExpression, OrderByClause, OrderByItem, ParenthesisExpression,
    PartialObjectExpression, PathExpression, QuantifiedExpression, RangeVariableDeclaration,
    SelectClause, SelectExpression, SelectStatement, SharedNewObjectExpression,
    SharedPathExpression, SimpleArithmeticExpression, SimpleCaseExpression, SimpleSelectClause,
    SimpleSelectExpression, SimpleWhenClause, Statement, Subselect, SubselectFromClause,
    UpdateClause, UpdateItem, UpdateStatement, WhenClause, WhereClause, functions,
};
use crate::query::context::{
    DqlQuery, DqlToken, HINT_CUSTOM_OUTPUT_WALKER, HINT_CUSTOM_TREE_WALKERS, Hint,
    OutputWalkerClass, QueryComponent, QueryComponents, QueryContext, TreeWalkerClass,
    has_metadata, has_result_variable, metadata_for_dql_alias,
};
use crate::query::lexer::Lexer;
use crate::query::parser_result::ParserResult;
use crate::query::query_exception;
use crate::query::sql_walker::SqlOutputWalker;
use crate::query::token_type::TokenType;
use crate::query::tree_walker::{TreeWalker, TreeWalkerChain};

/// An item the parser checks after parsing (`$deferred*[]`).
#[derive(Debug, Clone)]
struct Deferred<T> {
    expression: T,
    nesting_level: i64,
    token: Option<DqlToken>,
}

fn text(value: &PhpString) -> String {
    value.to_string_lossy().into_owned()
}

fn type_of(token: Option<&DqlToken>) -> Option<TokenType> {
    token.and_then(|t| t.type_)
}

fn value_of(token: Option<&DqlToken>) -> Option<String> {
    token.map(|t| text(&t.value))
}

/// `$token->value === $value` for a token that may be `null`.
fn value_is(token: Option<&DqlToken>, value: &str) -> bool {
    token.is_some_and(|t| t.value.as_bytes() == value.as_bytes())
}

/// `$tokenType->value >= TokenType::T_IDENTIFIER->value`.
fn is_identifier_or_keyword(token_type: Option<TokenType>) -> bool {
    token_type.is_some_and(|t| t.value() >= TokenType::T_IDENTIFIER.value())
}

/// `class_exists($name) || interface_exists($name)` (PHP drops a leading
/// `\` when it looks a class up).
fn class_or_interface_exists(name: &str) -> bool {
    let name = name.strip_prefix('\\').unwrap_or(name);
    php_reflection::class_exists(name) || php_reflection::interface_exists(name)
}

fn class_exists(name: &str) -> bool {
    php_reflection::class_exists(name.strip_prefix('\\').unwrap_or(name))
}

/// `substr($string, $start, $length)` on bytes.
fn substr(string: &str, start: usize, length: usize) -> String {
    let bytes = string.as_bytes();
    let start = start.min(bytes.len());
    let end = start.saturating_add(length).min(bytes.len());
    String::from_utf8_lossy(&bytes[start..end]).into_owned()
}

/// `Doctrine\ORM\Query\Parser`.
#[allow(clippy::struct_field_names, reason = "upstream's property names")]
pub struct Parser<'a> {
    query: &'a dyn DqlQuery,
    em: &'a dyn QueryContext,
    lexer: Lexer,
    parser_result: Rc<RefCell<ParserResult>>,
    deferred_identification_variables: Vec<Deferred<String>>,
    deferred_partial_object_expressions: Vec<Deferred<PartialObjectExpression>>,
    deferred_path_expressions: Vec<Deferred<SharedPathExpression>>,
    deferred_result_variables: Vec<Deferred<String>>,
    deferred_new_object_expressions: Vec<Deferred<SharedNewObjectExpression>>,
    /// Map of declared query components in the parsed query.
    query_components: QueryComponents,
    /// Keeps the nesting level of defined `ResultVariables`.
    nesting_level: i64,
    /// Keeps the nesting level of WITH join conditions.
    with_join_condition_nesting_level: i64,
    /// Any additional custom tree walkers that modify the AST.
    custom_tree_walkers: Vec<TreeWalkerClass>,
    /// The custom last tree walker, if any, that is responsible for
    /// producing the output.
    custom_output_walker: Option<OutputWalkerClass>,
    ident_variable_expressions: OrderedMap<Rc<SelectExpression>>,
}

impl<'a> Parser<'a> {
    /// Creates a new query parser object.
    pub fn new(query: &'a dyn DqlQuery) -> Result<Self, Thrown> {
        let lexer =
            Lexer::new(query.get_dql()).map_err(|e| Thrown::new("Error", format!("{e:?}"), 0))?;
        Ok(Self {
            query,
            em: query.get_entity_manager(),
            lexer,
            parser_result: Rc::new(RefCell::new(ParserResult::new())),
            deferred_identification_variables: Vec::new(),
            deferred_partial_object_expressions: Vec::new(),
            deferred_path_expressions: Vec::new(),
            deferred_result_variables: Vec::new(),
            deferred_new_object_expressions: Vec::new(),
            query_components: QueryComponents::new(),
            nesting_level: 0,
            with_join_condition_nesting_level: 0,
            custom_tree_walkers: Vec::new(),
            custom_output_walker: None,
            ident_variable_expressions: OrderedMap::new(),
        })
    }

    /// Sets a custom tree walker that produces output.
    /// This tree walker will be run last over the AST, after any other
    /// walkers.
    pub fn set_custom_output_tree_walker(&mut self, class: OutputWalkerClass) {
        deprecations::trigger("https://github.com/doctrine/orm/pull/11641", || {
            "Doctrine\\ORM\\Query\\Parser::setCustomOutputTreeWalker is deprecated, set the output walker class with the \\Doctrine\\ORM\\Query::HINT_CUSTOM_OUTPUT_WALKER query hint instead".to_owned()
        });
        self.custom_output_walker = Some(class);
    }

    /// Adds a custom tree walker for modifying the AST.
    pub fn add_custom_tree_walker(&mut self, class: TreeWalkerClass) {
        self.custom_tree_walkers.push(class);
    }

    /// Gets the lexer used by the parser.
    pub fn get_lexer(&mut self) -> &mut Lexer {
        &mut self.lexer
    }

    /// Gets the `ParserResult` that is being filled with information during
    /// parsing.
    pub fn get_parser_result(&self) -> Rc<RefCell<ParserResult>> {
        self.parser_result.clone()
    }

    /// Gets the `EntityManager` used by the parser.
    pub fn get_entity_manager(&self) -> &'a dyn QueryContext {
        self.em
    }

    /// The query components declared so far.
    pub fn get_query_components(&self) -> &QueryComponents {
        &self.query_components
    }

    /// `$lexer->isNextToken($token)`.
    pub fn is_next_token(&self, token: TokenType) -> bool {
        self.lexer.is_next_token(&token)
    }

    /// `$lexer->lookahead->value`.
    pub fn lookahead_value(&self) -> Option<String> {
        value_of(self.lexer.lookahead())
    }

    /// `$lexer->token->value`.
    pub fn token_value(&self) -> Option<String> {
        value_of(self.lexer.token())
    }

    fn lookahead_type(&self) -> Option<TokenType> {
        type_of(self.lexer.lookahead())
    }

    fn lookahead(&self) -> Option<DqlToken> {
        self.lexer.lookahead().cloned()
    }

    fn token(&self) -> Option<DqlToken> {
        self.lexer.token().cloned()
    }

    /// `$lexer->token->value` after a match (the match guarantees it).
    fn matched_value(&self) -> String {
        self.token_value().unwrap_or_default()
    }

    /// Parses and builds AST for the given Query.
    pub fn get_ast(&mut self) -> Result<Statement, Thrown> {
        // Parse & build AST
        let mut ast = self.query_language()?;

        // Process any deferred validations of some nodes in the AST.
        // This also allows post-processing of the AST for modification purposes.
        self.process_deferred_identification_variables()?;

        if !self.deferred_partial_object_expressions.is_empty() {
            self.process_deferred_partial_object_expressions()?;
        }

        if !self.deferred_path_expressions.is_empty() {
            self.process_deferred_path_expressions()?;
        }

        if !self.deferred_result_variables.is_empty() {
            self.process_deferred_result_variables()?;
        }

        if !self.deferred_new_object_expressions.is_empty() {
            self.process_deferred_new_object_expressions(&ast)?;
        }

        self.process_root_entity_alias_selected()?;

        // TODO: Is there a way to remove this? It may impact the mixed hydration resultset a lot!
        self.fix_identification_variable_order(&mut ast);

        Ok(ast)
    }

    /// Attempts to match the given token with the current lookahead token.
    ///
    /// If they match, updates the lookahead token; otherwise raises a
    /// syntax error.
    pub fn match_token(&mut self, token: TokenType) -> Result<(), Thrown> {
        let lookahead_type = self.lookahead_type();

        // Short-circuit on first condition, usually types match
        if lookahead_type == Some(token) {
            self.lexer.move_next();
            return Ok(());
        }

        // If parameter is not identifier (1-99) must be exact match
        if token.value() < TokenType::T_IDENTIFIER.value() {
            return Err(self.syntax_error(&self.token_literal(token), None));
        }

        // If parameter is keyword (200+) must be exact match
        if token.value() > TokenType::T_IDENTIFIER.value() {
            return Err(self.syntax_error(&self.token_literal(token), None));
        }

        // If parameter is T_IDENTIFIER, then matches T_IDENTIFIER (100) and keywords (200+)
        if token == TokenType::T_IDENTIFIER && !is_identifier_or_keyword(lookahead_type) {
            return Err(self.syntax_error(&self.token_literal(token), None));
        }

        self.lexer.move_next();
        Ok(())
    }

    /// `$lexer->getLiteral($token)`.
    fn token_literal(&self, token: TokenType) -> String {
        match self.lexer.get_literal(&token) {
            Value::String(s) => text(&s),
            Value::Int(i) => i.to_string(),
            _ => String::new(),
        }
    }

    /// Frees this parser, enabling it to be reused.
    pub fn free(&mut self, deep: bool, position: i64) {
        // WARNING! Use this method with care. It resets the scanner!
        self.lexer.reset_position(position);

        // Deep = true cleans peek and also any previously defined errors
        if deep {
            self.lexer.reset_peek();
        }

        let state = self.lexer.lexer_mut();
        state.token = None;
        state.lookahead = None;
    }

    /// Parses a query string.
    pub fn parse(&mut self) -> Result<Rc<RefCell<ParserResult>>, Thrown> {
        let mut ast = self.get_ast()?;

        if let Hint::TreeWalkers(custom_walkers) = self.query.get_hint(HINT_CUSTOM_TREE_WALKERS) {
            self.custom_tree_walkers = custom_walkers;
        }

        if let Hint::OutputWalker(custom_output_walker) =
            self.query.get_hint(HINT_CUSTOM_OUTPUT_WALKER)
        {
            self.custom_output_walker = Some(custom_output_walker);
        }

        // Run any custom tree walkers over the AST
        if !self.custom_tree_walkers.is_empty() {
            let mut tree_walker_chain = TreeWalkerChain::new(
                self.query,
                self.parser_result.clone(),
                std::mem::take(&mut self.query_components),
            );

            for walker in &self.custom_tree_walkers {
                tree_walker_chain.add_tree_walker(*walker);
            }

            match &mut ast {
                Statement::Update(ast) => tree_walker_chain.walk_update_statement(ast)?,
                Statement::Delete(ast) => tree_walker_chain.walk_delete_statement(ast)?,
                Statement::Select(ast) => tree_walker_chain.walk_select_statement(ast)?,
            }

            self.query_components = tree_walker_chain.get_query_components().clone();
        }

        let output_walker_class = self.custom_output_walker.unwrap_or(SqlOutputWalker::CLASS);
        let mut output_walker = (output_walker_class.construct)(
            self.query,
            self.parser_result.clone(),
            self.query_components.clone(),
        );

        if let Some(output_walker) = output_walker.as_output_walker() {
            let finalizer = output_walker.get_finalizer(&ast)?;
            self.parser_result.borrow_mut().set_sql_finalizer(finalizer);
        } else {
            let class = output_walker_class.name;
            deprecations::trigger("https://github.com/doctrine/orm/pull/11188/", || {
                format!(
                    "Your output walker class {class} should implement Doctrine\\ORM\\Query\\OutputWalker in order to provide a Doctrine\\ORM\\Query\\Exec\\SqlFinalizer. This also means the output walker should not use the query firstResult/maxResult values, which should be read from the query by the SqlFinalizer only."
                )
            });
            let executor = output_walker.get_executor(&ast)?;
            self.parser_result.borrow_mut().set_sql_executor(executor);
        }

        Ok(self.parser_result.clone())
    }

    /// Fixes order of identification variables.
    ///
    /// They have to appear in the select clause in the same order as the
    /// declarations (from ... x join ... y join ... z ...) appear in the
    /// query as the hydration process relies on that order for proper
    /// operation.
    fn fix_identification_variable_order(&mut self, ast: &mut Statement) {
        if self.ident_variable_expressions.len() <= 1 {
            return;
        }

        let Statement::Select(ast) = ast else {
            return;
        };

        for (dql_alias, _) in self.query_components.iter() {
            let Some(expr) = self.ident_variable_expressions.get(dql_alias) else {
                continue;
            };
            let expressions = &mut ast.select_clause.select_expressions;
            let key = expressions
                .iter()
                .position(|e| Rc::ptr_eq(e, expr))
                .unwrap_or(0);
            if key < expressions.len() {
                expressions.remove(key);
            }
            expressions.push(expr.clone());
        }
    }

    /// Generates a new syntax error.
    pub fn syntax_error(&self, expected: &str, token: Option<&DqlToken>) -> Thrown {
        let token = token.or(self.lexer.lookahead());
        let token_pos = token.map_or(-1, |t| i64::try_from(t.position).unwrap_or(-1));

        let mut message = format!("line 0, col {token_pos}: Error: ");
        if expected.is_empty() {
            message.push_str("Unexpected ");
        } else {
            let _ = write!(message, "Expected {expected}, got ");
        }
        match (self.lexer.lookahead(), token) {
            (Some(_), Some(token)) => {
                let _ = write!(message, "'{}'", text(&token.value));
            }
            (Some(_), None) => message.push_str("''"),
            (None, _) => message.push_str("end of string."),
        }

        query_exception::syntax_error(&message)
    }

    /// Generates a new semantical error.
    pub fn semantical_error(&self, message: &str, token: Option<&DqlToken>) -> Thrown {
        let fake = Token::new(PhpString::from("fake token"), None, 0);
        let token = token.or(self.lexer.lookahead()).unwrap_or(&fake);

        // Minimum exposed chars ahead of token
        let distance = 12;

        // Find a position of a final word to display in error string
        let dql = self.query.get_dql();
        let length = dql.len();
        let pos = token.position + distance;
        let offset = if length > pos { pos } else { length };
        let pos = dql.as_bytes()[offset..]
            .iter()
            .position(|b| *b == b' ')
            .map(|p| p + offset);
        let length = match pos {
            Some(pos) => pos.saturating_sub(token.position),
            None => distance,
        };

        // Building informative message
        let token_pos = if token.position > 0 {
            token.position.to_string()
        } else {
            "-1".to_owned()
        };
        let token_str = substr(&dql, token.position, length);
        let message = format!("line 0, col {token_pos} near '{token_str}': Error: {message}");

        query_exception::semantical_error(&message)
    }

    /// Peeks beyond the matched closing parenthesis and returns the first
    /// token after that one.
    fn peek_beyond_closing_parenthesis(&mut self, reset_peek: bool) -> Option<DqlToken> {
        let mut token = self.lexer.peek();
        let mut num_unmatched = 1;

        while num_unmatched > 0 {
            let Some(current) = &token else {
                break;
            };
            match current.type_ {
                Some(TokenType::T_OPEN_PARENTHESIS) => num_unmatched += 1,
                Some(TokenType::T_CLOSE_PARENTHESIS) => num_unmatched -= 1,
                _ => {}
            }

            token = self.lexer.peek();
        }

        if reset_peek {
            self.lexer.reset_peek();
        }

        token
    }

    /// Checks if the given token indicates a mathematical operator.
    fn is_math_operator(token: Option<&DqlToken>) -> bool {
        matches!(
            type_of(token),
            Some(
                TokenType::T_PLUS
                    | TokenType::T_MINUS
                    | TokenType::T_DIVIDE
                    | TokenType::T_MULTIPLY
            )
        )
    }

    /// Checks if the next-next (after lookahead) token starts a function.
    fn is_function(&mut self) -> bool {
        let lookahead_type = self.lookahead_type();
        let peek = self.lexer.peek();

        self.lexer.reset_peek();

        is_identifier_or_keyword(lookahead_type)
            && type_of(peek.as_ref()) == Some(TokenType::T_OPEN_PARENTHESIS)
    }

    /// Checks whether the given token type indicates an aggregate function.
    fn is_aggregate_function(token_type: Option<TokenType>) -> bool {
        matches!(
            token_type,
            Some(
                TokenType::T_AVG
                    | TokenType::T_MIN
                    | TokenType::T_MAX
                    | TokenType::T_SUM
                    | TokenType::T_COUNT
            )
        )
    }

    /// Checks whether the current lookahead token of the lexer has the
    /// type `T_ALL`, `T_ANY` or `T_SOME`.
    fn is_next_all_any_some(&self) -> bool {
        matches!(
            self.lookahead_type(),
            Some(TokenType::T_ALL | TokenType::T_ANY | TokenType::T_SOME)
        )
    }

    /// Validates that the given `IdentificationVariable` is semantically
    /// correct. It must exist in query components list.
    fn process_deferred_identification_variables(&self) -> Result<(), Thrown> {
        for deferred_item in &self.deferred_identification_variables {
            let ident_variable = &deferred_item.expression;

            // Check if IdentificationVariable exists in queryComponents
            let Some(q_comp) = self.query_components.get(ident_variable) else {
                return Err(self.semantical_error(
                    &format!("'{ident_variable}' is not defined."),
                    deferred_item.token.as_ref(),
                ));
            };

            // Check if queryComponent points to an AbstractSchemaName or a ResultVariable
            if q_comp.metadata.is_none() {
                return Err(self.semantical_error(
                    &format!("'{ident_variable}' does not point to a Class."),
                    deferred_item.token.as_ref(),
                ));
            }

            // Validate if identification variable nesting level is lower or equal than the current one
            if q_comp.nesting_level > deferred_item.nesting_level {
                return Err(self.semantical_error(
                    &format!("'{ident_variable}' is used outside the scope of its declaration."),
                    deferred_item.token.as_ref(),
                ));
            }
        }
        Ok(())
    }

    /// Validates that the given `NewObjectExpression`.
    fn process_deferred_new_object_expressions(&self, ast: &Statement) -> Result<(), Thrown> {
        let from_class_name = match ast {
            Statement::Select(select) => select
                .from_clause
                .identification_variable_declarations
                .first()
                .and_then(|d| d.range_variable_declaration.as_ref())
                .map(|r| r.abstract_schema_name.clone()),
            _ => None,
        };

        for deferred_item in &self.deferred_new_object_expressions {
            let expression = &deferred_item.expression;
            let token = deferred_item.token.as_ref();
            let mut class_name = expression.borrow().class_name.clone();
            let args = expression.borrow().args.len();

            // If the className is a relative name, try to resolve it from the FROM clause namespace
            if !class_name.contains('\\')
                && !class_exists(&class_name)
                && let Some(from_class_name) = &from_class_name
                && let Some(namespace_end) = from_class_name.rfind('\\')
            {
                let namespace = &from_class_name[..namespace_end];
                let fqcn = format!("{namespace}\\{class_name}");

                if class_exists(&fqcn) {
                    expression.borrow_mut().class_name.clone_from(&fqcn);
                    class_name = fqcn;
                }
            }

            // If the className is not a valid class, throw an exception
            if !class_exists(&class_name) {
                return Err(self
                    .semantical_error(&format!("Class \"{class_name}\" is not defined."), token));
            }

            let info =
                php_reflection::class_info(class_name.strip_prefix('\\').unwrap_or(&class_name));
            let constructor = info.as_ref().and_then(|info| find_constructor(info));
            let instantiable = info.as_ref().is_some_and(|info| {
                info.kind() == ClassKind::Class
                    && !info.is_abstract()
                    && constructor.as_ref().is_none_or(
                        php2rust_doctrine_persistence::php_reflection::MethodInfo::is_public,
                    )
            });

            if !instantiable {
                return Err(self.semantical_error(
                    &format!("Class \"{class_name}\" can not be instantiated."),
                    token,
                ));
            }

            let Some(constructor) = constructor else {
                return Err(self.semantical_error(
                    &format!("Class \"{class_name}\" has not a valid constructor."),
                    token,
                ));
            };

            if constructor.get_number_of_required_parameters() > args {
                return Err(self.semantical_error(
                    &format!(
                        "Number of arguments does not match with \"{class_name}\" constructor declaration."
                    ),
                    token,
                ));
            }
        }
        Ok(())
    }

    /// Validates that the given `PartialObjectExpression` is semantically
    /// correct. It must exist in query components list.
    fn process_deferred_partial_object_expressions(&self) -> Result<(), Thrown> {
        for deferred_item in &self.deferred_partial_object_expressions {
            let expr = &deferred_item.expression;
            let class = self.get_metadata_for_dql_alias(&expr.identification_variable)?;
            let class = class.read();

            for field in &expr.partial_field_set {
                if class.field_mappings.contains_key(field) {
                    continue;
                }

                if class
                    .association_mappings
                    .get(field)
                    .is_some_and(super::super::mapping::association_mapping::AssociationMapping::is_to_one_owning_side)
                {
                    continue;
                }

                return Err(self.semantical_error(
                    &format!(
                        "There is no mapped field named '{field}' on class {}.",
                        class.name
                    ),
                    deferred_item.token.as_ref(),
                ));
            }

            // array_intersect($class->identifier, $expr->partialFieldSet) !== $class->identifier
            let intersect: Vec<&String> = class
                .identifier
                .iter()
                .filter(|id| expr.partial_field_set.contains(id))
                .collect();
            if intersect.len() != class.identifier.len() {
                return Err(self.semantical_error(
                    &format!(
                        "The partial field selection of class {} must contain the identifier.",
                        class.name
                    ),
                    deferred_item.token.as_ref(),
                ));
            }
        }
        Ok(())
    }

    /// Validates that the given `ResultVariable` is semantically correct.
    /// It must exist in query components list.
    fn process_deferred_result_variables(&self) -> Result<(), Thrown> {
        for deferred_item in &self.deferred_result_variables {
            let result_variable = &deferred_item.expression;

            // Check if ResultVariable exists in queryComponents
            let Some(q_comp) = self.query_components.get(result_variable) else {
                return Err(self.semantical_error(
                    &format!("'{result_variable}' is not defined."),
                    deferred_item.token.as_ref(),
                ));
            };

            // Check if queryComponent points to an AbstractSchemaName or a ResultVariable
            if q_comp.result_variable.is_none() {
                return Err(self.semantical_error(
                    &format!("'{result_variable}' does not point to a ResultVariable."),
                    deferred_item.token.as_ref(),
                ));
            }

            // Validate if identification variable nesting level is lower or equal than the current one
            if q_comp.nesting_level > deferred_item.nesting_level {
                return Err(self.semantical_error(
                    &format!("'{result_variable}' is used outside the scope of its declaration."),
                    deferred_item.token.as_ref(),
                ));
            }
        }
        Ok(())
    }

    /// Validates that the given `PathExpression` is semantically correct
    /// for grammar rules.
    fn process_deferred_path_expressions(&self) -> Result<(), Thrown> {
        for deferred_item in &self.deferred_path_expressions {
            let path_expression = &deferred_item.expression;
            let (identification_variable, field) = {
                let p = path_expression.borrow();
                (p.identification_variable.clone(), p.field.clone())
            };

            let class = self.get_metadata_for_dql_alias(&identification_variable)?;
            let class = class.read();

            let field = if let Some(field) = field {
                field
            } else {
                let field = class.identifier.first().cloned().unwrap_or_default();
                path_expression.borrow_mut().field = Some(field.clone());
                field
            };

            // Check if field or association exists
            if !class.association_mappings.contains_key(&field)
                && !class.field_mappings.contains_key(&field)
            {
                return Err(self.semantical_error(
                    &format!(
                        "Class {} has no field or association named {field}",
                        class.name
                    ),
                    deferred_item.token.as_ref(),
                ));
            }

            let mut field_type = ast::TYPE_STATE_FIELD;

            if let Some(assoc) = class.association_mappings.get(&field) {
                field_type = if assoc.is_to_one() {
                    ast::TYPE_SINGLE_VALUED_ASSOCIATION
                } else {
                    ast::TYPE_COLLECTION_VALUED_ASSOCIATION
                };
            }

            // Validate if PathExpression is one of the expected types
            let expected_type = path_expression.borrow().expected_type;

            if expected_type & field_type == 0 {
                // We need to recognize which was expected type(s)
                let mut expected_string_types = Vec::new();

                // Validate state field type
                if expected_type & ast::TYPE_STATE_FIELD != 0 {
                    expected_string_types.push("StateFieldPathExpression");
                }

                // Validate single valued association (*-to-one)
                if expected_type & ast::TYPE_SINGLE_VALUED_ASSOCIATION != 0 {
                    expected_string_types.push("SingleValuedAssociationField");
                }

                // Validate single valued association (*-to-many)
                if expected_type & ast::TYPE_COLLECTION_VALUED_ASSOCIATION != 0 {
                    expected_string_types.push("CollectionValuedAssociationField");
                }

                // Build the error message
                let mut semantical_error = String::from("Invalid PathExpression. ");
                if expected_string_types.len() == 1 {
                    let _ = write!(semantical_error, "Must be a {}.", expected_string_types[0]);
                } else {
                    let _ = write!(
                        semantical_error,
                        "{} expected.",
                        expected_string_types.join(" or ")
                    );
                }

                return Err(self.semantical_error(&semantical_error, deferred_item.token.as_ref()));
            }

            // We need to force the type in PathExpression
            path_expression.borrow_mut().type_ = Some(field_type);
        }
        Ok(())
    }

    fn process_root_entity_alias_selected(&self) -> Result<(), Thrown> {
        if self.ident_variable_expressions.is_empty() {
            return Ok(());
        }

        for (dql_alias, _) in self.ident_variable_expressions.iter() {
            if self
                .query_components
                .get(dql_alias)
                .is_some_and(|c| c.parent.is_none())
            {
                return Ok(());
            }
        }

        Err(self.semantical_error(
            "Cannot select entity through identification variables without choosing at least one root entity alias.",
            None,
        ))
    }

    /// ```text
    /// QueryLanguage ::= SelectStatement | UpdateStatement | DeleteStatement
    /// ```
    pub fn query_language(&mut self) -> Result<Statement, Thrown> {
        self.lexer.move_next();

        let statement = match self.lookahead_type() {
            Some(TokenType::T_SELECT) => Statement::Select(self.select_statement()?),
            Some(TokenType::T_UPDATE) => Statement::Update(self.update_statement()?),
            Some(TokenType::T_DELETE) => Statement::Delete(self.delete_statement()?),
            _ => return Err(self.syntax_error("SELECT, UPDATE or DELETE", None)),
        };

        // Check for end of string
        if self.lexer.lookahead().is_some() {
            return Err(self.syntax_error("end of string", None));
        }

        Ok(statement)
    }

    /// ```text
    /// SelectStatement ::= SelectClause FromClause [WhereClause] [GroupByClause] [HavingClause] [OrderByClause]
    /// ```
    pub fn select_statement(&mut self) -> Result<SelectStatement, Thrown> {
        let select_clause = self.select_clause()?;
        let from_clause = self.from_clause()?;
        let where_clause = if self.is_next_token(TokenType::T_WHERE) {
            Some(self.where_clause()?)
        } else {
            None
        };
        let group_by_clause = if self.is_next_token(TokenType::T_GROUP) {
            Some(self.group_by_clause()?)
        } else {
            None
        };
        let having_clause = if self.is_next_token(TokenType::T_HAVING) {
            Some(self.having_clause()?)
        } else {
            None
        };
        let order_by_clause = if self.is_next_token(TokenType::T_ORDER) {
            Some(self.order_by_clause()?)
        } else {
            None
        };

        Ok(SelectStatement {
            select_clause,
            from_clause,
            where_clause,
            group_by_clause,
            having_clause,
            order_by_clause,
        })
    }

    /// ```text
    /// UpdateStatement ::= UpdateClause [WhereClause]
    /// ```
    pub fn update_statement(&mut self) -> Result<UpdateStatement, Thrown> {
        let update_clause = self.update_clause()?;
        let where_clause = if self.is_next_token(TokenType::T_WHERE) {
            Some(self.where_clause()?)
        } else {
            None
        };
        Ok(UpdateStatement {
            update_clause,
            where_clause,
        })
    }

    /// ```text
    /// DeleteStatement ::= DeleteClause [WhereClause]
    /// ```
    pub fn delete_statement(&mut self) -> Result<DeleteStatement, Thrown> {
        let delete_clause = self.delete_clause()?;
        let where_clause = if self.is_next_token(TokenType::T_WHERE) {
            Some(self.where_clause()?)
        } else {
            None
        };
        Ok(DeleteStatement {
            delete_clause,
            where_clause,
        })
    }

    /// ```text
    /// IdentificationVariable ::= identifier
    /// ```
    pub fn identification_variable(&mut self) -> Result<String, Thrown> {
        self.match_token(TokenType::T_IDENTIFIER)?;

        let ident_variable = self.matched_value();

        self.deferred_identification_variables.push(Deferred {
            expression: ident_variable.clone(),
            nesting_level: self.nesting_level,
            token: self.token(),
        });

        Ok(ident_variable)
    }

    /// `AliasIdentificationVariable` = identifier
    pub fn alias_identification_variable(&mut self) -> Result<String, Thrown> {
        self.match_token(TokenType::T_IDENTIFIER)?;

        let alias_ident_variable = self.matched_value();
        let exists = self.query_components.contains_key(&alias_ident_variable);

        if exists {
            return Err(self.semantical_error(
                &format!("'{alias_ident_variable}' is already defined."),
                self.lexer.token(),
            ));
        }

        Ok(alias_ident_variable)
    }

    /// ```text
    /// AbstractSchemaName ::= fully_qualified_name | identifier
    /// ```
    pub fn abstract_schema_name(&mut self) -> Result<String, Thrown> {
        if self.is_next_token(TokenType::T_FULLY_QUALIFIED_NAME) {
            self.match_token(TokenType::T_FULLY_QUALIFIED_NAME)?;

            return Ok(self.matched_value());
        }

        self.match_token(TokenType::T_IDENTIFIER)?;

        Ok(self.matched_value())
    }

    /// Validates an `AbstractSchemaName`, making sure the class exists.
    fn validate_abstract_schema_name(&self, schema_name: &str) -> Result<(), Thrown> {
        if !class_or_interface_exists(schema_name) {
            return Err(self.semantical_error(
                &format!("Class '{schema_name}' is not defined."),
                self.lexer.token(),
            ));
        }
        Ok(())
    }

    /// ```text
    /// AliasResultVariable ::= identifier
    /// ```
    pub fn alias_result_variable(&mut self) -> Result<String, Thrown> {
        self.match_token(TokenType::T_IDENTIFIER)?;

        let result_variable = self.matched_value();
        let exists = self.query_components.contains_key(&result_variable);

        if exists {
            return Err(self.semantical_error(
                &format!("'{result_variable}' is already defined."),
                self.lexer.token(),
            ));
        }

        Ok(result_variable)
    }

    /// ```text
    /// ResultVariable ::= identifier
    /// ```
    pub fn result_variable(&mut self) -> Result<String, Thrown> {
        self.match_token(TokenType::T_IDENTIFIER)?;

        let result_variable = self.matched_value();

        // Defer ResultVariable validation
        self.deferred_result_variables.push(Deferred {
            expression: result_variable.clone(),
            nesting_level: self.nesting_level,
            token: self.token(),
        });

        Ok(result_variable)
    }

    /// ```text
    /// JoinAssociationPathExpression ::= IdentificationVariable "." (CollectionValuedAssociationField | SingleValuedAssociationField)
    /// ```
    pub fn join_association_path_expression(
        &mut self,
    ) -> Result<JoinAssociationPathExpression, Thrown> {
        let ident_variable = self.identification_variable()?;

        if !self.query_components.contains_key(&ident_variable) {
            return Err(self.semantical_error(
                &format!(
                    "Identification Variable {ident_variable} used in join path expression but was not defined before."
                ),
                None,
            ));
        }

        self.match_token(TokenType::T_DOT)?;
        self.match_token(TokenType::T_IDENTIFIER)?;

        let field = self.matched_value();

        // Validate association field
        let class = self.get_metadata_for_dql_alias(&ident_variable)?;

        if !class.read().has_association(&field) {
            let name = class.read().name.clone();
            return Err(self.semantical_error(
                &format!("Class {name} has no association named {field}"),
                None,
            ));
        }

        Ok(JoinAssociationPathExpression {
            identification_variable: ident_variable,
            association_field: field,
        })
    }

    /// Parses an arbitrary path expression and defers semantical validation
    /// based on expected types.
    ///
    /// ```text
    /// PathExpression ::= IdentificationVariable {"." identifier}*
    /// ```
    pub fn path_expression(&mut self, expected_types: i64) -> Result<SharedPathExpression, Thrown> {
        let ident_variable = self.identification_variable()?;
        let mut field = None;

        if self.is_next_token(TokenType::T_DOT) {
            self.match_token(TokenType::T_DOT)?;
            self.match_token(TokenType::T_IDENTIFIER)?;

            let mut f = self.matched_value();

            while self.is_next_token(TokenType::T_DOT) {
                self.match_token(TokenType::T_DOT)?;
                self.match_token(TokenType::T_IDENTIFIER)?;
                f.push('.');
                f.push_str(&self.matched_value());
            }
            field = Some(f);
        }

        // Creating AST node
        let path_expr = Rc::new(RefCell::new(PathExpression::new(
            expected_types,
            &ident_variable,
            field.as_deref(),
        )));

        // Defer PathExpression validation if requested to be deferred
        self.deferred_path_expressions.push(Deferred {
            expression: path_expr.clone(),
            nesting_level: self.nesting_level,
            token: self.token(),
        });

        Ok(path_expr)
    }

    /// ```text
    /// AssociationPathExpression ::= CollectionValuedPathExpression | SingleValuedAssociationPathExpression
    /// ```
    pub fn association_path_expression(&mut self) -> Result<SharedPathExpression, Thrown> {
        self.path_expression(
            ast::TYPE_SINGLE_VALUED_ASSOCIATION | ast::TYPE_COLLECTION_VALUED_ASSOCIATION,
        )
    }

    /// ```text
    /// SingleValuedPathExpression ::= StateFieldPathExpression | SingleValuedAssociationPathExpression
    /// ```
    pub fn single_valued_path_expression(&mut self) -> Result<SharedPathExpression, Thrown> {
        self.path_expression(ast::TYPE_STATE_FIELD | ast::TYPE_SINGLE_VALUED_ASSOCIATION)
    }

    /// ```text
    /// StateFieldPathExpression ::= IdentificationVariable "." StateField
    /// ```
    pub fn state_field_path_expression(&mut self) -> Result<SharedPathExpression, Thrown> {
        self.path_expression(ast::TYPE_STATE_FIELD)
    }

    /// ```text
    /// SingleValuedAssociationPathExpression ::= IdentificationVariable "." SingleValuedAssociationField
    /// ```
    pub fn single_valued_association_path_expression(
        &mut self,
    ) -> Result<SharedPathExpression, Thrown> {
        self.path_expression(ast::TYPE_SINGLE_VALUED_ASSOCIATION)
    }

    /// ```text
    /// CollectionValuedPathExpression ::= IdentificationVariable "." CollectionValuedAssociationField
    /// ```
    pub fn collection_valued_path_expression(&mut self) -> Result<SharedPathExpression, Thrown> {
        self.path_expression(ast::TYPE_COLLECTION_VALUED_ASSOCIATION)
    }

    /// ```text
    /// EntityAsDtoArgumentExpression ::= IdentificationVariable
    /// ```
    pub fn entity_as_dto_argument_expression(
        &mut self,
    ) -> Result<EntityAsDtoArgumentExpression, Thrown> {
        let expression = self.identification_variable()?;
        let ident_variable = expression.clone();

        // [["AS"] AliasResultVariable]
        let mut must_have_alias_result_variable = false;

        if self.is_next_token(TokenType::T_AS) {
            self.match_token(TokenType::T_AS)?;
            must_have_alias_result_variable = true;
        }

        let mut alias_result_variable = None;

        if must_have_alias_result_variable || self.is_next_token(TokenType::T_IDENTIFIER) {
            let token = self.lookahead();
            let alias = self.alias_result_variable()?;

            // Include AliasResultVariable in query components.
            self.query_components.insert(
                alias.clone(),
                QueryComponent::result(Node::str(expression.clone()), self.nesting_level, token),
            );
            alias_result_variable = Some(alias);
        }

        Ok(EntityAsDtoArgumentExpression::new(
            expression,
            Some(ident_variable),
            alias_result_variable,
        ))
    }

    /// ```text
    /// SelectClause ::= "SELECT" ["DISTINCT"] SelectExpression {"," SelectExpression}
    /// ```
    pub fn select_clause(&mut self) -> Result<SelectClause, Thrown> {
        let mut is_distinct = false;
        self.match_token(TokenType::T_SELECT)?;

        // Check for DISTINCT
        if self.is_next_token(TokenType::T_DISTINCT) {
            self.match_token(TokenType::T_DISTINCT)?;

            is_distinct = true;
        }

        // Process SelectExpressions (1..N)
        let mut select_expressions = vec![self.select_expression()?];

        while self.is_next_token(TokenType::T_COMMA) {
            self.match_token(TokenType::T_COMMA)?;

            select_expressions.push(self.select_expression()?);
        }

        Ok(SelectClause {
            select_expressions,
            is_distinct,
        })
    }

    /// ```text
    /// SimpleSelectClause ::= "SELECT" ["DISTINCT"] SimpleSelectExpression
    /// ```
    pub fn simple_select_clause(&mut self) -> Result<SimpleSelectClause, Thrown> {
        let mut is_distinct = false;
        self.match_token(TokenType::T_SELECT)?;

        if self.is_next_token(TokenType::T_DISTINCT) {
            self.match_token(TokenType::T_DISTINCT)?;

            is_distinct = true;
        }

        Ok(SimpleSelectClause {
            simple_select_expression: self.simple_select_expression()?,
            is_distinct,
        })
    }

    /// ```text
    /// UpdateClause ::= "UPDATE" AbstractSchemaName ["AS"] AliasIdentificationVariable "SET" UpdateItem {"," UpdateItem}*
    /// ```
    pub fn update_clause(&mut self) -> Result<UpdateClause, Thrown> {
        self.match_token(TokenType::T_UPDATE)?;

        let token = self.lookahead();
        let abstract_schema_name = self.abstract_schema_name()?;

        self.validate_abstract_schema_name(&abstract_schema_name)?;

        if self.is_next_token(TokenType::T_AS) {
            self.match_token(TokenType::T_AS)?;
        }

        let alias_identification_variable = self.alias_identification_variable()?;

        let class = self.em.get_class_metadata(&abstract_schema_name)?;

        // Building queryComponent
        let query_component = QueryComponent::entity(class, None, None, self.nesting_level, token);

        self.query_components
            .insert(alias_identification_variable.clone(), query_component);

        self.match_token(TokenType::T_SET)?;

        let mut update_items = vec![self.update_item()?];

        while self.is_next_token(TokenType::T_COMMA) {
            self.match_token(TokenType::T_COMMA)?;

            update_items.push(self.update_item()?);
        }

        Ok(UpdateClause {
            abstract_schema_name,
            update_items,
            alias_identification_variable,
        })
    }

    /// ```text
    /// DeleteClause ::= "DELETE" ["FROM"] AbstractSchemaName ["AS"] AliasIdentificationVariable
    /// ```
    pub fn delete_clause(&mut self) -> Result<DeleteClause, Thrown> {
        self.match_token(TokenType::T_DELETE)?;

        if self.is_next_token(TokenType::T_FROM) {
            self.match_token(TokenType::T_FROM)?;
        }

        let token = self.lookahead();
        let abstract_schema_name = self.abstract_schema_name()?;

        self.validate_abstract_schema_name(&abstract_schema_name)?;

        if self.is_next_token(TokenType::T_AS) {
            self.match_token(TokenType::T_AS)?;
        }

        let alias_identification_variable = if self.is_next_token(TokenType::T_IDENTIFIER) {
            self.alias_identification_variable()?
        } else {
            "alias_should_have_been_set".to_owned()
        };

        let class = self.em.get_class_metadata(&abstract_schema_name)?;

        // Building queryComponent
        let query_component = QueryComponent::entity(class, None, None, self.nesting_level, token);

        self.query_components
            .insert(alias_identification_variable.clone(), query_component);

        Ok(DeleteClause {
            abstract_schema_name,
            alias_identification_variable,
        })
    }

    /// ```text
    /// FromClause ::= "FROM" IdentificationVariableDeclaration {"," IdentificationVariableDeclaration}*
    /// ```
    pub fn from_clause(&mut self) -> Result<FromClause, Thrown> {
        self.match_token(TokenType::T_FROM)?;

        let mut identification_variable_declarations =
            vec![self.identification_variable_declaration()?];

        while self.is_next_token(TokenType::T_COMMA) {
            self.match_token(TokenType::T_COMMA)?;

            identification_variable_declarations.push(self.identification_variable_declaration()?);
        }

        Ok(FromClause {
            identification_variable_declarations,
        })
    }

    /// ```text
    /// SubselectFromClause ::= "FROM" SubselectIdentificationVariableDeclaration {"," SubselectIdentificationVariableDeclaration}*
    /// ```
    pub fn subselect_from_clause(&mut self) -> Result<SubselectFromClause, Thrown> {
        self.match_token(TokenType::T_FROM)?;

        let mut identification_variables =
            vec![self.subselect_identification_variable_declaration()?];

        while self.is_next_token(TokenType::T_COMMA) {
            self.match_token(TokenType::T_COMMA)?;

            identification_variables.push(self.subselect_identification_variable_declaration()?);
        }

        Ok(SubselectFromClause {
            identification_variable_declarations: identification_variables,
        })
    }

    /// ```text
    /// WhereClause ::= "WHERE" ConditionalExpression
    /// ```
    pub fn where_clause(&mut self) -> Result<WhereClause, Thrown> {
        self.match_token(TokenType::T_WHERE)?;

        Ok(WhereClause {
            conditional_expression: self.conditional_expression()?,
        })
    }

    /// ```text
    /// HavingClause ::= "HAVING" ConditionalExpression
    /// ```
    pub fn having_clause(&mut self) -> Result<HavingClause, Thrown> {
        self.match_token(TokenType::T_HAVING)?;

        Ok(HavingClause {
            conditional_expression: self.conditional_expression()?,
        })
    }

    /// ```text
    /// GroupByClause ::= "GROUP" "BY" GroupByItem {"," GroupByItem}*
    /// ```
    pub fn group_by_clause(&mut self) -> Result<GroupByClause, Thrown> {
        self.match_token(TokenType::T_GROUP)?;
        self.match_token(TokenType::T_BY)?;

        let mut group_by_items = vec![self.group_by_item()?];

        while self.is_next_token(TokenType::T_COMMA) {
            self.match_token(TokenType::T_COMMA)?;

            group_by_items.push(self.group_by_item()?);
        }

        Ok(GroupByClause { group_by_items })
    }

    /// ```text
    /// OrderByClause ::= "ORDER" "BY" OrderByItem {"," OrderByItem}*
    /// ```
    pub fn order_by_clause(&mut self) -> Result<OrderByClause, Thrown> {
        self.match_token(TokenType::T_ORDER)?;
        self.match_token(TokenType::T_BY)?;

        let mut order_by_items = vec![self.order_by_item()?];

        while self.is_next_token(TokenType::T_COMMA) {
            self.match_token(TokenType::T_COMMA)?;

            order_by_items.push(self.order_by_item()?);
        }

        Ok(OrderByClause {
            order_by_items,
            include_collection_order_by_items: self.with_join_condition_nesting_level == 0,
        })
    }

    /// ```text
    /// Subselect ::= SimpleSelectClause SubselectFromClause [WhereClause] [GroupByClause] [HavingClause] [OrderByClause]
    /// ```
    pub fn subselect(&mut self) -> Result<Subselect, Thrown> {
        // Increase query nesting level
        self.nesting_level += 1;

        let simple_select_clause = self.simple_select_clause()?;
        let subselect_from_clause = self.subselect_from_clause()?;
        let where_clause = if self.is_next_token(TokenType::T_WHERE) {
            Some(self.where_clause()?)
        } else {
            None
        };
        let group_by_clause = if self.is_next_token(TokenType::T_GROUP) {
            Some(self.group_by_clause()?)
        } else {
            None
        };
        let having_clause = if self.is_next_token(TokenType::T_HAVING) {
            Some(self.having_clause()?)
        } else {
            None
        };
        let order_by_clause = if self.is_next_token(TokenType::T_ORDER) {
            Some(self.order_by_clause()?)
        } else {
            None
        };

        // Decrease query nesting level
        self.nesting_level -= 1;

        Ok(Subselect {
            simple_select_clause,
            subselect_from_clause,
            where_clause,
            group_by_clause,
            having_clause,
            order_by_clause,
        })
    }

    /// ```text
    /// UpdateItem ::= SingleValuedPathExpression "=" NewValue
    /// ```
    pub fn update_item(&mut self) -> Result<UpdateItem, Thrown> {
        let path_expr = self.single_valued_path_expression()?;

        self.match_token(TokenType::T_EQUALS)?;

        Ok(UpdateItem {
            path_expression: path_expr,
            new_value: self.new_value()?,
        })
    }

    /// ```text
    /// GroupByItem ::= IdentificationVariable | ResultVariable | SingleValuedPathExpression
    /// ```
    pub fn group_by_item(&mut self) -> Result<Node, Thrown> {
        // We need to check if we are in a IdentificationVariable or SingleValuedPathExpression
        let glimpse = self.lexer.glimpse();

        if type_of(glimpse.as_ref()) == Some(TokenType::T_DOT) {
            return Ok(Node::PathExpression(self.single_valued_path_expression()?));
        }

        let lookahead_value = self.lookahead_value().unwrap_or_default();

        // Validate if any of the following is defined: IdentificationVariable or ResultVariable
        if !self.query_components.contains_key(&lookahead_value) {
            return Err(self.semantical_error(
                "Cannot group by undefined identification or result variable.",
                None,
            ));
        }

        Ok(Node::str(
            if has_metadata(&self.query_components, &lookahead_value) {
                self.identification_variable()?
            } else {
                self.result_variable()?
            },
        ))
    }

    /// ```text
    /// OrderByItem ::= (
    ///      SimpleArithmeticExpression | SingleValuedPathExpression | CaseExpression |
    ///      ScalarExpression | ResultVariable | FunctionDeclaration
    /// ) ["ASC" | "DESC"]
    /// ```
    pub fn order_by_item(&mut self) -> Result<OrderByItem, Thrown> {
        self.lexer.peek(); // lookahead => '.'
        self.lexer.peek(); // lookahead => token after '.'

        let peek = self.lexer.peek(); // lookahead => token after the token after the '.'

        self.lexer.reset_peek();

        let glimpse = self.lexer.glimpse();

        let expr =
            if Self::is_math_operator(peek.as_ref()) || Self::is_math_operator(glimpse.as_ref()) {
                self.simple_arithmetic_expression()?
            } else if type_of(glimpse.as_ref()) == Some(TokenType::T_DOT) {
                Node::PathExpression(self.single_valued_path_expression()?)
            } else if self.lexer.peek().is_some() && {
                let beyond = self.peek_beyond_closing_parenthesis(true);
                Self::is_math_operator(beyond.as_ref())
            } {
                self.scalar_expression()?
            } else if self.lookahead_type() == Some(TokenType::T_CASE) {
                self.case_expression()?
            } else if self.is_function() {
                self.function_declaration()?
            } else {
                Node::str(self.result_variable()?)
            };

        let mut type_ = "ASC";

        if self.is_next_token(TokenType::T_DESC) {
            self.match_token(TokenType::T_DESC)?;
            type_ = "DESC";
        } else if self.is_next_token(TokenType::T_ASC) {
            self.match_token(TokenType::T_ASC)?;
        }

        Ok(OrderByItem {
            expression: expr,
            type_: type_.to_owned(),
        })
    }

    /// ```text
    /// NewValue ::= SimpleArithmeticExpression | "NULL"
    /// ```
    ///
    /// `SimpleArithmeticExpression` covers all *Primary grammar rules and also `SimpleEntityExpression`
    pub fn new_value(&mut self) -> Result<Option<Node>, Thrown> {
        if self.is_next_token(TokenType::T_NULL) {
            self.match_token(TokenType::T_NULL)?;

            return Ok(None);
        }

        if self.is_next_token(TokenType::T_INPUT_PARAMETER) {
            self.match_token(TokenType::T_INPUT_PARAMETER)?;

            return Ok(Some(Node::InputParameter(Box::new(InputParameter::new(
                &self.matched_value(),
            )?))));
        }

        Ok(Some(Node::ArithmeticExpression(Box::new(
            self.arithmetic_expression()?,
        ))))
    }

    /// ```text
    /// IdentificationVariableDeclaration ::= RangeVariableDeclaration [IndexBy] {Join}*
    /// ```
    pub fn identification_variable_declaration(
        &mut self,
    ) -> Result<IdentificationVariableDeclaration, Thrown> {
        let mut joins = Vec::new();
        let mut range_variable_declaration = self.range_variable_declaration()?;
        let index_by = if self.is_next_token(TokenType::T_INDEX) {
            Some(self.index_by()?)
        } else {
            None
        };

        range_variable_declaration.is_root = true;

        while self.is_next_token(TokenType::T_LEFT)
            || self.is_next_token(TokenType::T_INNER)
            || self.is_next_token(TokenType::T_JOIN)
        {
            joins.push(self.join()?);
        }

        Ok(IdentificationVariableDeclaration {
            range_variable_declaration: Some(range_variable_declaration),
            index_by,
            joins,
        })
    }

    /// ```text
    /// SubselectIdentificationVariableDeclaration ::= IdentificationVariableDeclaration
    /// ```
    ///
    /// {Internal note: WARNING: Solution is harder than a bare implementation.
    /// Desired EBNF support:
    ///
    /// ```text
    /// SubselectIdentificationVariableDeclaration ::= IdentificationVariableDeclaration | (AssociationPathExpression ["AS"] AliasIdentificationVariable)
    /// ```
    ///
    /// It demands that entire SQL generation to become programmatical. This is
    /// needed because association based subselect requires "WHERE" conditional
    /// expressions to be injected, but there is no scope to do that. Only scope
    /// accessible is "FROM", prohibiting an easy implementation without larger
    /// changes.}
    pub fn subselect_identification_variable_declaration(
        &mut self,
    ) -> Result<IdentificationVariableDeclaration, Thrown> {
        self.identification_variable_declaration()
    }

    /// ```text
    /// Join ::= ["LEFT" ["OUTER"] | "INNER"] "JOIN"
    ///          (JoinAssociationDeclaration | RangeVariableDeclaration)
    ///          ["WITH" ConditionalExpression]
    /// ```
    pub fn join(&mut self) -> Result<Join, Thrown> {
        // Check Join type
        let mut join_type = ast::JOIN_TYPE_INNER;

        if self.is_next_token(TokenType::T_LEFT) {
            self.match_token(TokenType::T_LEFT)?;

            join_type = ast::JOIN_TYPE_LEFT;

            // Possible LEFT OUTER join
            if self.is_next_token(TokenType::T_OUTER) {
                self.match_token(TokenType::T_OUTER)?;

                join_type = ast::JOIN_TYPE_LEFTOUTER;
            }
        } else if self.is_next_token(TokenType::T_INNER) {
            self.match_token(TokenType::T_INNER)?;
        }

        self.match_token(TokenType::T_JOIN)?;

        let next = self.lexer.glimpse();
        let mut conditional_expression = None;

        let join_declaration = if type_of(next.as_ref()) == Some(TokenType::T_DOT) {
            let join_declaration = self.join_association_declaration()?;

            // Check for ad-hoc Join conditions
            if self.is_next_token(TokenType::T_WITH) {
                self.match_token(TokenType::T_WITH)?;

                self.with_join_condition_nesting_level += 1;
                let result = self.conditional_expression();
                self.with_join_condition_nesting_level -= 1;
                conditional_expression = Some(result?);
            }
            Node::JoinAssociationDeclaration(Box::new(join_declaration))
        } else {
            let mut join_declaration = self.range_variable_declaration()?;
            join_declaration.is_root = false;

            // Check for ad-hoc Join conditions
            if self.is_next_token(TokenType::T_ON) {
                self.match_token(TokenType::T_ON)?;
                conditional_expression = Some(self.conditional_expression()?);
            } else if self.is_next_token(TokenType::T_WITH) {
                self.match_token(TokenType::T_WITH)?;
                conditional_expression = Some(self.conditional_expression()?);
                deprecations::trigger("https://github.com/doctrine/orm/issues/12192", || {
                    "Using WITH for the join condition of arbitrary joins is deprecated. Use ON instead.".to_owned()
                });
            }
            Node::RangeVariableDeclaration(Box::new(join_declaration))
        };

        let mut join = Join::new(join_type, Some(join_declaration));
        join.conditional_expression = conditional_expression;

        Ok(join)
    }

    /// ```text
    /// RangeVariableDeclaration ::= AbstractSchemaName ["AS"] AliasIdentificationVariable
    /// ```
    pub fn range_variable_declaration(&mut self) -> Result<RangeVariableDeclaration, Thrown> {
        if self.is_next_token(TokenType::T_OPEN_PARENTHESIS)
            && type_of(self.lexer.glimpse().as_ref()) == Some(TokenType::T_SELECT)
        {
            return Err(self.semantical_error("Subquery is not supported here", self.lexer.token()));
        }

        let abstract_schema_name = self.abstract_schema_name()?;

        self.validate_abstract_schema_name(&abstract_schema_name)?;

        if self.is_next_token(TokenType::T_AS) {
            self.match_token(TokenType::T_AS)?;
        }

        let token = self.lookahead();
        let alias_identification_variable = self.alias_identification_variable()?;
        let class_metadata = self.em.get_class_metadata(&abstract_schema_name)?;

        // Building queryComponent
        let query_component =
            QueryComponent::entity(class_metadata, None, None, self.nesting_level, token);

        self.query_components
            .insert(alias_identification_variable.clone(), query_component);

        Ok(RangeVariableDeclaration::new(
            &abstract_schema_name,
            &alias_identification_variable,
        ))
    }

    /// ```text
    /// JoinAssociationDeclaration ::= JoinAssociationPathExpression ["AS"] AliasIdentificationVariable [IndexBy]
    /// ```
    pub fn join_association_declaration(&mut self) -> Result<JoinAssociationDeclaration, Thrown> {
        let join_association_path_expression = self.join_association_path_expression()?;

        if self.is_next_token(TokenType::T_AS) {
            self.match_token(TokenType::T_AS)?;
        }

        let alias_identification_variable = self.alias_identification_variable()?;
        let index_by = if self.is_next_token(TokenType::T_INDEX) {
            Some(self.index_by()?)
        } else {
            None
        };

        let identification_variable = &join_association_path_expression.identification_variable;
        let field = &join_association_path_expression.association_field;

        let class = self.get_metadata_for_dql_alias(identification_variable)?;
        let relation = class.read().get_association_mapping(field)?.clone();
        let target_class = self.em.get_class_metadata(&relation.target_entity)?;

        // Building queryComponent
        let join_query_component = QueryComponent::entity(
            target_class,
            Some(identification_variable.clone()),
            Some(relation),
            self.nesting_level,
            self.lookahead(),
        );

        self.query_components
            .insert(alias_identification_variable.clone(), join_query_component);

        Ok(JoinAssociationDeclaration {
            join_association_path_expression,
            alias_identification_variable,
            index_by,
        })
    }

    /// ```text
    /// PartialObjectExpression ::= "PARTIAL" IdentificationVariable "." PartialFieldSet
    /// PartialFieldSet ::= "{" SimpleStateField {"," SimpleStateField}* "}"
    /// ```
    pub fn partial_object_expression(&mut self) -> Result<PartialObjectExpression, Thrown> {
        self.match_token(TokenType::T_PARTIAL)?;

        let mut partial_field_set = Vec::new();

        let identification_variable = self.identification_variable()?;

        self.match_token(TokenType::T_DOT)?;
        self.match_token(TokenType::T_OPEN_CURLY_BRACE)?;
        self.match_token(TokenType::T_IDENTIFIER)?;

        let mut field = self.matched_value();

        // First field in partial expression might be embeddable property
        while self.is_next_token(TokenType::T_DOT) {
            self.match_token(TokenType::T_DOT)?;
            self.match_token(TokenType::T_IDENTIFIER)?;
            field.push('.');
            field.push_str(&self.matched_value());
        }

        partial_field_set.push(field);

        while self.is_next_token(TokenType::T_COMMA) {
            self.match_token(TokenType::T_COMMA)?;
            self.match_token(TokenType::T_IDENTIFIER)?;

            let mut field = self.matched_value();

            while self.is_next_token(TokenType::T_DOT) {
                self.match_token(TokenType::T_DOT)?;
                self.match_token(TokenType::T_IDENTIFIER)?;
                field.push('.');
                field.push_str(&self.matched_value());
            }

            partial_field_set.push(field);
        }

        self.match_token(TokenType::T_CLOSE_CURLY_BRACE)?;

        let partial_object_expression = PartialObjectExpression {
            identification_variable,
            partial_field_set,
        };

        // Defer PartialObjectExpression validation
        self.deferred_partial_object_expressions.push(Deferred {
            expression: partial_object_expression.clone(),
            nesting_level: self.nesting_level,
            token: self.token(),
        });

        Ok(partial_object_expression)
    }

    /// ```text
    /// NewObjectExpression ::= "NEW" AbstractSchemaName "(" NewObjectArg {"," NewObjectArg}* ")"
    /// ```
    pub fn new_object_expression(&mut self) -> Result<SharedNewObjectExpression, Thrown> {
        let mut use_named_arguments = false;
        let mut args = Vec::new();

        self.match_token(TokenType::T_NEW)?;

        if self.is_next_token(TokenType::T_NAMED) {
            self.match_token(TokenType::T_NAMED)?;
            use_named_arguments = true;
        }

        let class_name = self.abstract_schema_name()?; // note that this is not yet validated
        let token = self.token();

        self.match_token(TokenType::T_OPEN_PARENTHESIS)?;

        self.add_argument(&mut args, use_named_arguments)?;

        while self.is_next_token(TokenType::T_COMMA) {
            self.match_token(TokenType::T_COMMA)?;

            self.add_argument(&mut args, use_named_arguments)?;
        }

        self.match_token(TokenType::T_CLOSE_PARENTHESIS)?;

        let expression = Rc::new(RefCell::new(NewObjectExpression { class_name, args }));

        // Defer NewObjectExpression validation
        self.deferred_new_object_expressions.push(Deferred {
            token,
            expression: expression.clone(),
            nesting_level: self.nesting_level,
        });

        Ok(expression)
    }

    /// `substr($dql, $start, $lookahead->position - $start)`, trimmed.
    fn dql_since(&self, start_token: usize) -> String {
        let end = self.lexer.lookahead().map_or(0, |t| t.position);
        let length = i64::try_from(end).unwrap_or(0) - i64::try_from(start_token).unwrap_or(0);
        let dql = self.query.get_dql();
        let start = start_token.min(dql.len());
        // A negative length leaves that many bytes off the end.
        let end = if length < 0 {
            dql.len()
                .saturating_sub(usize::try_from(length.unsigned_abs()).unwrap_or(usize::MAX))
        } else {
            start.saturating_add(usize::try_from(length).unwrap_or(0))
        }
        .min(dql.len());
        let part = if end > start {
            String::from_utf8_lossy(&dql.as_bytes()[start..end]).into_owned()
        } else {
            String::new()
        };
        part.trim_matches([' ', '\t', '\n', '\r', '\0', '\x0B'])
            .to_owned()
    }

    pub fn add_argument(
        &mut self,
        args: &mut Vec<(Key, Node)>,
        use_named_arguments: bool,
    ) -> Result<(), Thrown> {
        let mut field_alias = None;

        if use_named_arguments {
            let start_token = self.lexer.lookahead().map_or(0, |t| t.position);

            let new_arg = self.new_object_arg(&mut field_alias)?;

            let key = field_alias.or_else(|| match &new_arg {
                Node::PathExpression(path) => path.borrow().field.clone(),
                _ => None,
            });

            let Some(key) = key else {
                return Err(query_exception::no_matching_property(
                    &self.dql_since(start_token),
                ));
            };

            let key = Key::from(key.as_str());
            if args.iter().any(|(k, _)| k == &key) {
                let key_text = String::from_utf8_lossy(&key.bytes()).into_owned();
                return Err(query_exception::duplicate_field(
                    &key_text,
                    &self.dql_since(start_token),
                ));
            }

            args.push((key, new_arg));
        } else {
            let new_arg = self.new_object_arg(&mut field_alias)?;
            let next = args
                .iter()
                .filter_map(|(k, _)| k.as_int())
                .max()
                .map_or(0, |max| max + 1);
            args.push((Key::Int(next), new_arg));
        }
        Ok(())
    }

    /// ```text
    /// NewObjectArg ::= (ScalarExpression | "(" Subselect ")" | NewObjectExpression | EntityAsDtoArgumentExpression) ["AS" AliasResultVariable]
    /// ```
    pub fn new_object_arg(&mut self, field_alias: &mut Option<String>) -> Result<Node, Thrown> {
        *field_alias = None;

        let token = self.lookahead();
        let peek = self.lexer.glimpse();
        let token_type = type_of(token.as_ref());
        let peek_type = type_of(peek.as_ref());

        let expression = if token_type == Some(TokenType::T_OPEN_PARENTHESIS)
            && peek_type == Some(TokenType::T_SELECT)
        {
            self.match_token(TokenType::T_OPEN_PARENTHESIS)?;
            let expression = self.subselect()?;
            self.match_token(TokenType::T_CLOSE_PARENTHESIS)?;
            Node::Subselect(Box::new(expression))
        } else if token_type == Some(TokenType::T_NEW) {
            Node::NewObjectExpression(self.new_object_expression()?)
        } else if token_type == Some(TokenType::T_IDENTIFIER)
            && peek_type != Some(TokenType::T_DOT)
            && peek_type != Some(TokenType::T_OPEN_PARENTHESIS)
        {
            let expression = self.entity_as_dto_argument_expression()?;
            field_alias.clone_from(&expression.alias_variable);
            Node::EntityAsDtoArgumentExpression(Box::new(expression))
        } else {
            self.scalar_expression()?
        };

        if self.is_next_token(TokenType::T_AS) {
            self.match_token(TokenType::T_AS)?;
            self.match_token(TokenType::T_IDENTIFIER)?;

            *field_alias = Some(self.matched_value());
        }

        Ok(expression)
    }

    /// ```text
    /// IndexBy ::= "INDEX" "BY" SingleValuedPathExpression
    /// ```
    pub fn index_by(&mut self) -> Result<IndexBy, Thrown> {
        self.match_token(TokenType::T_INDEX)?;
        self.match_token(TokenType::T_BY)?;
        let path_expr = self.single_valued_path_expression()?;

        // Add the INDEX BY info to the query component
        let (alias, field) = {
            let p = path_expr.borrow();
            (p.identification_variable.clone(), p.field.clone())
        };
        match self.query_components.get_mut(&alias) {
            Some(component) => component.map = field,
            None => self.query_components.insert(
                alias,
                QueryComponent {
                    map: field,
                    ..QueryComponent::default()
                },
            ),
        }

        Ok(IndexBy {
            single_valued_path_expression: path_expr,
        })
    }

    /// ```text
    /// ScalarExpression ::= SimpleArithmeticExpression | StringPrimary | DateTimePrimary |
    ///                      StateFieldPathExpression | BooleanPrimary | CaseExpression |
    ///                      InstanceOfExpression
    /// ```
    pub fn scalar_expression(&mut self) -> Result<Node, Thrown> {
        let lookahead = self.lookahead_type();
        let peek = self.lexer.glimpse();

        match lookahead {
            Some(
                TokenType::T_INTEGER
                | TokenType::T_FLOAT
                | TokenType::T_MINUS
                | TokenType::T_PLUS
                | TokenType::T_OPEN_PARENTHESIS,
            ) => return self.simple_arithmetic_expression(),
            Some(TokenType::T_STRING) => return self.string_primary(),
            Some(lookahead @ (TokenType::T_TRUE | TokenType::T_FALSE)) => {
                self.match_token(lookahead)?;

                return Ok(Node::Literal(Box::new(Literal {
                    type_: ast::LITERAL_BOOLEAN,
                    value: self.token_value(),
                })));
            }
            Some(TokenType::T_NULL) => {
                self.match_token(TokenType::T_NULL)?;

                return Ok(Node::Literal(Box::new(Literal {
                    type_: ast::LITERAL_NULL,
                    value: None,
                })));
            }
            Some(TokenType::T_INPUT_PARAMETER) => {
                return if Self::is_math_operator(peek.as_ref()) {
                    // :param + u.value
                    self.simple_arithmetic_expression()
                } else {
                    Ok(Node::InputParameter(Box::new(self.input_parameter()?)))
                };
            }
            Some(TokenType::T_CASE | TokenType::T_COALESCE | TokenType::T_NULLIF) => {
                // Since NULLIF and COALESCE can be identified as a function,
                // we need to check these before checking for FunctionDeclaration
                return self.case_expression();
            }
            _ => {}
        }

        if self.is_function() {
            self.lexer.peek(); // "("

            let beyond = self.peek_beyond_closing_parenthesis(true);
            return if Self::is_math_operator(beyond.as_ref()) {
                self.simple_arithmetic_expression()
            } else {
                self.function_declaration()
            };
        }

        if lookahead == Some(TokenType::T_IDENTIFIER) {
            self.lexer.peek(); // lookahead => '.'
            self.lexer.peek(); // lookahead => token after '.'
            let peek = self.lexer.peek(); // lookahead => token after the token after the '.'
            self.lexer.reset_peek();

            if Self::is_math_operator(peek.as_ref()) {
                return self.simple_arithmetic_expression();
            }

            return Ok(Node::PathExpression(self.state_field_path_expression()?));
        }

        Err(self.syntax_error("", None))
    }

    /// ```text
    /// CaseExpression ::= GeneralCaseExpression | SimpleCaseExpression | CoalesceExpression | NullifExpression
    /// GeneralCaseExpression ::= "CASE" WhenClause {WhenClause}* "ELSE" ScalarExpression "END"
    /// WhenClause ::= "WHEN" ConditionalExpression "THEN" ScalarExpression
    /// SimpleCaseExpression ::= "CASE" CaseOperand SimpleWhenClause {SimpleWhenClause}* "ELSE" ScalarExpression "END"
    /// CaseOperand ::= StateFieldPathExpression | TypeDiscriminator
    /// SimpleWhenClause ::= "WHEN" ScalarExpression "THEN" ScalarExpression
    /// CoalesceExpression ::= "COALESCE" "(" ScalarExpression {"," ScalarExpression}* ")"
    /// NullifExpression ::= "NULLIF" "(" ScalarExpression "," ScalarExpression ")"
    /// ```
    pub fn case_expression(&mut self) -> Result<Node, Thrown> {
        match self.lookahead_type() {
            Some(TokenType::T_NULLIF) => {
                return Ok(Node::NullIfExpression(Box::new(self.null_if_expression()?)));
            }
            Some(TokenType::T_COALESCE) => {
                return Ok(Node::CoalesceExpression(Box::new(
                    self.coalesce_expression()?,
                )));
            }
            Some(TokenType::T_CASE) => {
                self.lexer.reset_peek();
                let peek = self.lexer.peek();

                if type_of(peek.as_ref()) == Some(TokenType::T_WHEN) {
                    return Ok(Node::GeneralCaseExpression(Box::new(
                        self.general_case_expression()?,
                    )));
                }

                return Ok(Node::SimpleCaseExpression(Box::new(
                    self.simple_case_expression()?,
                )));
            }
            _ => {}
        }

        Err(self.syntax_error("", None))
    }

    /// ```text
    /// CoalesceExpression ::= "COALESCE" "(" ScalarExpression {"," ScalarExpression}* ")"
    /// ```
    pub fn coalesce_expression(&mut self) -> Result<CoalesceExpression, Thrown> {
        self.match_token(TokenType::T_COALESCE)?;
        self.match_token(TokenType::T_OPEN_PARENTHESIS)?;

        // Process ScalarExpressions (1..N)
        let mut scalar_expressions = vec![self.scalar_expression()?];

        while self.is_next_token(TokenType::T_COMMA) {
            self.match_token(TokenType::T_COMMA)?;

            scalar_expressions.push(self.scalar_expression()?);
        }

        self.match_token(TokenType::T_CLOSE_PARENTHESIS)?;

        Ok(CoalesceExpression { scalar_expressions })
    }

    /// ```text
    /// NullIfExpression ::= "NULLIF" "(" ScalarExpression "," ScalarExpression ")"
    /// ```
    pub fn null_if_expression(&mut self) -> Result<NullIfExpression, Thrown> {
        self.match_token(TokenType::T_NULLIF)?;
        self.match_token(TokenType::T_OPEN_PARENTHESIS)?;

        let first_expression = self.scalar_expression()?;
        self.match_token(TokenType::T_COMMA)?;
        let second_expression = self.scalar_expression()?;

        self.match_token(TokenType::T_CLOSE_PARENTHESIS)?;

        Ok(NullIfExpression {
            first_expression,
            second_expression,
        })
    }

    /// ```text
    /// GeneralCaseExpression ::= "CASE" WhenClause {WhenClause}* "ELSE" ScalarExpression "END"
    /// ```
    pub fn general_case_expression(&mut self) -> Result<GeneralCaseExpression, Thrown> {
        self.match_token(TokenType::T_CASE)?;

        // Process WhenClause (1..N)
        let mut when_clauses = Vec::new();

        loop {
            when_clauses.push(self.when_clause()?);
            if !self.is_next_token(TokenType::T_WHEN) {
                break;
            }
        }

        self.match_token(TokenType::T_ELSE)?;
        let scalar_expression = self.scalar_expression()?;
        self.match_token(TokenType::T_END)?;

        Ok(GeneralCaseExpression {
            when_clauses,
            else_scalar_expression: scalar_expression,
        })
    }

    /// ```text
    /// SimpleCaseExpression ::= "CASE" CaseOperand SimpleWhenClause {SimpleWhenClause}* "ELSE" ScalarExpression "END"
    /// CaseOperand ::= StateFieldPathExpression | TypeDiscriminator
    /// ```
    pub fn simple_case_expression(&mut self) -> Result<SimpleCaseExpression, Thrown> {
        self.match_token(TokenType::T_CASE)?;
        let case_operand = self.state_field_path_expression()?;

        // Process SimpleWhenClause (1..N)
        let mut simple_when_clauses = Vec::new();

        loop {
            simple_when_clauses.push(self.simple_when_clause()?);
            if !self.is_next_token(TokenType::T_WHEN) {
                break;
            }
        }

        self.match_token(TokenType::T_ELSE)?;
        let scalar_expression = self.scalar_expression()?;
        self.match_token(TokenType::T_END)?;

        Ok(SimpleCaseExpression {
            case_operand: Some(case_operand),
            simple_when_clauses,
            else_scalar_expression: scalar_expression,
        })
    }

    /// ```text
    /// WhenClause ::= "WHEN" ConditionalExpression "THEN" ScalarExpression
    /// ```
    pub fn when_clause(&mut self) -> Result<WhenClause, Thrown> {
        self.match_token(TokenType::T_WHEN)?;
        let conditional_expression = self.conditional_expression()?;
        self.match_token(TokenType::T_THEN)?;

        Ok(WhenClause {
            case_condition_expression: conditional_expression,
            then_scalar_expression: self.scalar_expression()?,
        })
    }

    /// ```text
    /// SimpleWhenClause ::= "WHEN" ScalarExpression "THEN" ScalarExpression
    /// ```
    pub fn simple_when_clause(&mut self) -> Result<SimpleWhenClause, Thrown> {
        self.match_token(TokenType::T_WHEN)?;
        let conditional_expression = self.scalar_expression()?;
        self.match_token(TokenType::T_THEN)?;

        Ok(SimpleWhenClause {
            case_scalar_expression: conditional_expression,
            then_scalar_expression: self.scalar_expression()?,
        })
    }

    /// ```text
    /// SelectExpression ::= (
    ///     IdentificationVariable | ScalarExpression | AggregateExpression | FunctionDeclaration |
    ///     PartialObjectExpression | "(" Subselect ")" | CaseExpression | NewObjectExpression
    /// ) [["AS"] ["HIDDEN"] AliasResultVariable]
    /// ```
    pub fn select_expression(&mut self) -> Result<Rc<SelectExpression>, Thrown> {
        let mut ident_variable = None;
        let peek = self.lexer.glimpse();
        let lookahead_type = self.lookahead_type();
        let peek_type = type_of(peek.as_ref());

        let expression = if lookahead_type == Some(TokenType::T_IDENTIFIER)
            && peek_type == Some(TokenType::T_DOT)
        {
            // ScalarExpression (u.name)
            self.scalar_expression()?
        } else if lookahead_type == Some(TokenType::T_IDENTIFIER)
            && peek_type != Some(TokenType::T_OPEN_PARENTHESIS)
        {
            // IdentificationVariable (u)
            let variable = self.identification_variable()?;
            ident_variable = Some(variable.clone());
            Node::str(variable)
        } else if lookahead_type == Some(TokenType::T_NULL) {
            self.scalar_expression()?
        } else if matches!(
            lookahead_type,
            Some(TokenType::T_CASE | TokenType::T_COALESCE | TokenType::T_NULLIF)
        ) {
            // CaseExpression (CASE ... or NULLIF(...) or COALESCE(...))
            self.case_expression()?
        } else if self.is_function() {
            // DQL Function (SUM(u.value) or SUM(u.value) + 1)
            self.lexer.peek(); // "("

            let beyond = self.peek_beyond_closing_parenthesis(true);
            if Self::is_math_operator(beyond.as_ref()) {
                self.scalar_expression()?
            } else {
                self.function_declaration()?
            }
        } else if lookahead_type == Some(TokenType::T_PARTIAL) {
            // PartialObjectExpression (PARTIAL u.{id, name})
            let expression = self.partial_object_expression()?;
            ident_variable = Some(expression.identification_variable.clone());
            Node::PartialObjectExpression(Box::new(expression))
        } else if lookahead_type == Some(TokenType::T_OPEN_PARENTHESIS)
            && peek_type == Some(TokenType::T_SELECT)
        {
            // Subselect
            self.match_token(TokenType::T_OPEN_PARENTHESIS)?;
            let expression = self.subselect()?;
            self.match_token(TokenType::T_CLOSE_PARENTHESIS)?;
            Node::Subselect(Box::new(expression))
        } else if matches!(
            lookahead_type,
            Some(
                TokenType::T_OPEN_PARENTHESIS
                    | TokenType::T_INTEGER
                    | TokenType::T_STRING
                    | TokenType::T_FLOAT
                    | TokenType::T_MINUS
                    | TokenType::T_PLUS
            )
        ) {
            // Shortcut: ScalarExpression => SimpleArithmeticExpression
            self.simple_arithmetic_expression()?
        } else if lookahead_type == Some(TokenType::T_NEW) {
            // NewObjectExpression (New ClassName(id, name))
            Node::NewObjectExpression(self.new_object_expression()?)
        } else {
            return Err(self.syntax_error(
                "IdentificationVariable | ScalarExpression | AggregateExpression | FunctionDeclaration | PartialObjectExpression | \"(\" Subselect \")\" | CaseExpression",
                self.lexer.lookahead(),
            ));
        };

        // [["AS"] ["HIDDEN"] AliasResultVariable]
        let mut must_have_alias_result_variable = false;

        if self.is_next_token(TokenType::T_AS) {
            self.match_token(TokenType::T_AS)?;

            must_have_alias_result_variable = true;
        }

        let mut hidden_alias_result_variable = false;

        if self.is_next_token(TokenType::T_HIDDEN) {
            self.match_token(TokenType::T_HIDDEN)?;

            hidden_alias_result_variable = true;
        }

        let mut alias_result_variable = None;

        if must_have_alias_result_variable || self.is_next_token(TokenType::T_IDENTIFIER) {
            let token = self.lookahead();
            let alias = self.alias_result_variable()?;

            // Include AliasResultVariable in query components.
            self.query_components.insert(
                alias.clone(),
                QueryComponent::result(expression.clone(), self.nesting_level, token),
            );
            alias_result_variable = Some(alias);
        }

        // AST
        let expr = Rc::new(SelectExpression {
            expression,
            field_identification_variable: alias_result_variable,
            hidden_alias_result_variable,
        });

        if let Some(ident_variable) = ident_variable.filter(|v| !(v.is_empty() || v == "0")) {
            self.ident_variable_expressions
                .insert(ident_variable, expr.clone());
        }

        Ok(expr)
    }

    /// ```text
    /// SimpleSelectExpression ::= (
    ///      StateFieldPathExpression | IdentificationVariable | FunctionDeclaration |
    ///      AggregateExpression | "(" Subselect ")" | ScalarExpression
    /// ) [["AS"] AliasResultVariable]
    /// ```
    pub fn simple_select_expression(&mut self) -> Result<SimpleSelectExpression, Thrown> {
        let peek = self.lexer.glimpse();
        let peek_type = type_of(peek.as_ref());

        match self.lookahead_type() {
            Some(TokenType::T_IDENTIFIER) => {
                if peek_type == Some(TokenType::T_DOT) {
                    // SingleValuedPathExpression | IdentificationVariable
                    let expression = self.state_field_path_expression()?;

                    return Ok(SimpleSelectExpression {
                        field_identification_variable: None,
                        expression: Node::PathExpression(expression),
                    });
                }
                if peek_type != Some(TokenType::T_OPEN_PARENTHESIS) {
                    // SingleValuedPathExpression | IdentificationVariable
                    let expression = self.identification_variable()?;

                    return Ok(SimpleSelectExpression {
                        field_identification_variable: None,
                        expression: Node::str(expression),
                    });
                }
                if self.is_function() {
                    // SUM(u.id) + COUNT(u.id)
                    let beyond = self.peek_beyond_closing_parenthesis(true);
                    if Self::is_math_operator(beyond.as_ref()) {
                        return Ok(SimpleSelectExpression {
                            field_identification_variable: None,
                            expression: self.scalar_expression()?,
                        });
                    }

                    // COUNT(u.id)
                    if Self::is_aggregate_function(self.lookahead_type()) {
                        return Ok(SimpleSelectExpression {
                            field_identification_variable: None,
                            expression: Node::AggregateExpression(Box::new(
                                self.aggregate_expression()?,
                            )),
                        });
                    }

                    // IDENTITY(u)
                    return Ok(SimpleSelectExpression {
                        field_identification_variable: None,
                        expression: self.function_declaration()?,
                    });
                }
            }
            Some(TokenType::T_OPEN_PARENTHESIS) => {
                if peek_type != Some(TokenType::T_SELECT) {
                    // Shortcut: ScalarExpression => SimpleArithmeticExpression
                    let expression = self.simple_arithmetic_expression()?;

                    return Ok(SimpleSelectExpression {
                        field_identification_variable: None,
                        expression,
                    });
                }

                // Subselect
                self.match_token(TokenType::T_OPEN_PARENTHESIS)?;
                let expression = self.subselect()?;
                self.match_token(TokenType::T_CLOSE_PARENTHESIS)?;

                return Ok(SimpleSelectExpression {
                    field_identification_variable: None,
                    expression: Node::Subselect(Box::new(expression)),
                });
            }
            _ => {}
        }

        self.lexer.peek();

        let expression = self.scalar_expression()?;
        let mut expr = SimpleSelectExpression {
            field_identification_variable: None,
            expression,
        };

        if self.is_next_token(TokenType::T_AS) {
            self.match_token(TokenType::T_AS)?;
        }

        if self.is_next_token(TokenType::T_IDENTIFIER) {
            let token = self.lookahead();
            let result_variable = self.alias_result_variable()?;
            expr.field_identification_variable = Some(result_variable.clone());

            // Include AliasResultVariable in query components.
            self.query_components.insert(
                result_variable,
                QueryComponent {
                    lowercase_result_variable: Some(Node::SimpleSelectExpression(Box::new(
                        expr.clone(),
                    ))),
                    nesting_level: self.nesting_level,
                    token,
                    ..QueryComponent::default()
                },
            );
        }

        Ok(expr)
    }

    /// ```text
    /// ConditionalExpression ::= ConditionalTerm {"OR" ConditionalTerm}*
    /// ```
    pub fn conditional_expression(&mut self) -> Result<Node, Thrown> {
        let mut conditional_terms = vec![self.conditional_term()?];

        while self.is_next_token(TokenType::T_OR) {
            self.match_token(TokenType::T_OR)?;

            conditional_terms.push(self.conditional_term()?);
        }

        // Phase 1 AST optimization: Prevent AST\ConditionalExpression
        // if only one AST\ConditionalTerm is defined
        if conditional_terms.len() == 1 {
            return Ok(conditional_terms.remove(0));
        }

        Ok(Node::ConditionalExpression(Box::new(
            ConditionalExpression { conditional_terms },
        )))
    }

    /// ```text
    /// ConditionalTerm ::= ConditionalFactor {"AND" ConditionalFactor}*
    /// ```
    pub fn conditional_term(&mut self) -> Result<Node, Thrown> {
        let mut conditional_factors = vec![self.conditional_factor()?];

        while self.is_next_token(TokenType::T_AND) {
            self.match_token(TokenType::T_AND)?;

            conditional_factors.push(self.conditional_factor()?);
        }

        // Phase 1 AST optimization: Prevent AST\ConditionalTerm
        // if only one AST\ConditionalFactor is defined
        if conditional_factors.len() == 1 {
            return Ok(conditional_factors.remove(0));
        }

        Ok(Node::ConditionalTerm(Box::new(ConditionalTerm {
            conditional_factors,
        })))
    }

    /// ```text
    /// ConditionalFactor ::= ["NOT"] ConditionalPrimary
    /// ```
    pub fn conditional_factor(&mut self) -> Result<Node, Thrown> {
        let mut not = false;

        if self.is_next_token(TokenType::T_NOT) {
            self.match_token(TokenType::T_NOT)?;

            not = true;
        }

        let conditional_primary = self.conditional_primary()?;

        // Phase 1 AST optimization: Prevent AST\ConditionalFactor
        // if only one AST\ConditionalPrimary is defined
        if !not {
            return Ok(Node::ConditionalPrimary(Box::new(conditional_primary)));
        }

        Ok(Node::ConditionalFactor(Box::new(ConditionalFactor {
            conditional_primary,
            not,
        })))
    }

    /// ```text
    /// ConditionalPrimary ::= SimpleConditionalExpression | "(" ConditionalExpression ")"
    /// ```
    pub fn conditional_primary(&mut self) -> Result<ConditionalPrimary, Thrown> {
        let mut cond_primary = ConditionalPrimary::default();

        if !self.is_next_token(TokenType::T_OPEN_PARENTHESIS) {
            cond_primary.simple_conditional_expression =
                Some(self.simple_conditional_expression()?);

            return Ok(cond_primary);
        }

        // Peek beyond the matching closing parenthesis ')'
        let peek = self.peek_beyond_closing_parenthesis(true);

        if let Some(peek) = &peek
            && (["=", "<", "<=", "<>", ">", ">=", "!="]
                .iter()
                .any(|op| peek.value.as_bytes() == op.as_bytes())
                || matches!(
                    peek.type_,
                    Some(
                        TokenType::T_NOT
                            | TokenType::T_BETWEEN
                            | TokenType::T_LIKE
                            | TokenType::T_IN
                            | TokenType::T_IS
                            | TokenType::T_EXISTS
                    )
                )
                || Self::is_math_operator(Some(peek)))
        {
            cond_primary.simple_conditional_expression =
                Some(self.simple_conditional_expression()?);

            return Ok(cond_primary);
        }

        self.match_token(TokenType::T_OPEN_PARENTHESIS)?;
        cond_primary.conditional_expression = Some(self.conditional_expression()?);
        self.match_token(TokenType::T_CLOSE_PARENTHESIS)?;

        Ok(cond_primary)
    }

    /// ```text
    /// SimpleConditionalExpression ::=
    ///      ComparisonExpression | BetweenExpression | LikeExpression |
    ///      InExpression | NullComparisonExpression | ExistsExpression |
    ///      EmptyCollectionComparisonExpression | CollectionMemberExpression |
    ///      InstanceOfExpression
    /// ```
    pub fn simple_conditional_expression(&mut self) -> Result<Node, Thrown> {
        if self.is_next_token(TokenType::T_EXISTS) {
            return Ok(Node::ExistsExpression(Box::new(self.exists_expression()?)));
        }

        let mut token = self.lookahead();
        let peek = self.lexer.glimpse();
        let mut lookahead = token.clone();

        if self.is_next_token(TokenType::T_NOT) {
            token = self.lexer.glimpse();
        }

        let token_type = type_of(token.as_ref());
        if token_type == Some(TokenType::T_IDENTIFIER)
            || token_type == Some(TokenType::T_INPUT_PARAMETER)
            || self.is_function()
        {
            // Peek beyond the matching closing parenthesis.
            let beyond = self.lexer.peek();

            if value_is(peek.as_ref(), "(") {
                // Peeks beyond the matched closing parenthesis.
                token = self.peek_beyond_closing_parenthesis(false);

                if type_of(token.as_ref()) == Some(TokenType::T_NOT) {
                    token = self.lexer.peek();
                }

                if type_of(token.as_ref()) == Some(TokenType::T_IS) {
                    lookahead = self.lexer.peek();
                }
            } else {
                // Peek beyond the PathExpression or InputParameter.
                token = beyond;

                while value_is(token.as_ref(), ".") {
                    self.lexer.peek();

                    token = self.lexer.peek();
                }

                // Also peek beyond a NOT if there is one.
                if type_of(token.as_ref()) == Some(TokenType::T_NOT) {
                    token = self.lexer.peek();
                }

                // We need to go even further in case of IS (differentiate between NULL and EMPTY)
                lookahead = self.lexer.peek();
            }

            // Also peek beyond a NOT if there is one.
            if type_of(lookahead.as_ref()) == Some(TokenType::T_NOT) {
                lookahead = self.lexer.peek();
            }

            self.lexer.reset_peek();
        }

        let token_type = type_of(token.as_ref());

        if token_type == Some(TokenType::T_BETWEEN) {
            return Ok(Node::BetweenExpression(Box::new(
                self.between_expression()?,
            )));
        }

        if token_type == Some(TokenType::T_LIKE) {
            return Ok(Node::LikeExpression(Box::new(self.like_expression()?)));
        }

        if token_type == Some(TokenType::T_IN) {
            return self.in_expression();
        }

        if token_type == Some(TokenType::T_INSTANCE) {
            return Ok(Node::InstanceOfExpression(Box::new(
                self.instance_of_expression()?,
            )));
        }

        if token_type == Some(TokenType::T_MEMBER) {
            return Ok(Node::CollectionMemberExpression(Box::new(
                self.collection_member_expression()?,
            )));
        }

        let lookahead_type = type_of(lookahead.as_ref());

        if token_type == Some(TokenType::T_IS) && lookahead_type == Some(TokenType::T_NULL) {
            return Ok(Node::NullComparisonExpression(Box::new(
                self.null_comparison_expression()?,
            )));
        }

        if token_type == Some(TokenType::T_IS) && lookahead_type == Some(TokenType::T_EMPTY) {
            return Ok(Node::EmptyCollectionComparisonExpression(Box::new(
                self.empty_collection_comparison_expression()?,
            )));
        }

        Ok(Node::ComparisonExpression(Box::new(
            self.comparison_expression()?,
        )))
    }

    /// ```text
    /// EmptyCollectionComparisonExpression ::= CollectionValuedPathExpression "IS" ["NOT"] "EMPTY"
    /// ```
    pub fn empty_collection_comparison_expression(
        &mut self,
    ) -> Result<EmptyCollectionComparisonExpression, Thrown> {
        let path_expression = self.collection_valued_path_expression()?;
        self.match_token(TokenType::T_IS)?;

        let mut not = false;
        if self.is_next_token(TokenType::T_NOT) {
            self.match_token(TokenType::T_NOT)?;
            not = true;
        }

        self.match_token(TokenType::T_EMPTY)?;

        Ok(EmptyCollectionComparisonExpression {
            expression: path_expression,
            not,
        })
    }

    /// ```text
    /// CollectionMemberExpression ::= EntityExpression ["NOT"] "MEMBER" ["OF"] CollectionValuedPathExpression
    /// ```
    ///
    /// ```text
    /// EntityExpression ::= SingleValuedAssociationPathExpression | SimpleEntityExpression
    /// SimpleEntityExpression ::= IdentificationVariable | InputParameter
    /// ```
    pub fn collection_member_expression(&mut self) -> Result<CollectionMemberExpression, Thrown> {
        let mut not = false;
        let entity_expr = self.entity_expression()?;

        if self.is_next_token(TokenType::T_NOT) {
            self.match_token(TokenType::T_NOT)?;

            not = true;
        }

        self.match_token(TokenType::T_MEMBER)?;

        if self.is_next_token(TokenType::T_OF) {
            self.match_token(TokenType::T_OF)?;
        }

        Ok(CollectionMemberExpression {
            entity_expression: entity_expr,
            collection_valued_path_expression: self.collection_valued_path_expression()?,
            not,
        })
    }

    /// ```text
    /// Literal ::= string | char | integer | float | boolean
    /// ```
    pub fn literal(&mut self) -> Result<Literal, Thrown> {
        match self.lookahead_type() {
            Some(TokenType::T_STRING) => {
                self.match_token(TokenType::T_STRING)?;

                Ok(Literal {
                    type_: ast::LITERAL_STRING,
                    value: self.token_value(),
                })
            }
            Some(TokenType::T_INTEGER | TokenType::T_FLOAT) => {
                self.match_token(if self.is_next_token(TokenType::T_INTEGER) {
                    TokenType::T_INTEGER
                } else {
                    TokenType::T_FLOAT
                })?;

                Ok(Literal {
                    type_: ast::LITERAL_NUMERIC,
                    value: self.token_value(),
                })
            }
            Some(TokenType::T_TRUE | TokenType::T_FALSE) => {
                self.match_token(if self.is_next_token(TokenType::T_TRUE) {
                    TokenType::T_TRUE
                } else {
                    TokenType::T_FALSE
                })?;

                Ok(Literal {
                    type_: ast::LITERAL_BOOLEAN,
                    value: self.token_value(),
                })
            }
            _ => Err(self.syntax_error("Literal", None)),
        }
    }

    /// ```text
    /// InParameter ::= ArithmeticExpression | InputParameter
    /// ```
    pub fn in_parameter(&mut self) -> Result<Node, Thrown> {
        if self.lookahead_type() == Some(TokenType::T_INPUT_PARAMETER) {
            return Ok(Node::InputParameter(Box::new(self.input_parameter()?)));
        }

        Ok(Node::ArithmeticExpression(Box::new(
            self.arithmetic_expression()?,
        )))
    }

    /// ```text
    /// InputParameter ::= PositionalParameter | NamedParameter
    /// ```
    pub fn input_parameter(&mut self) -> Result<InputParameter, Thrown> {
        self.match_token(TokenType::T_INPUT_PARAMETER)?;

        InputParameter::new(&self.matched_value())
    }

    /// ```text
    /// ArithmeticExpression ::= SimpleArithmeticExpression | "(" Subselect ")"
    /// ```
    pub fn arithmetic_expression(&mut self) -> Result<ArithmeticExpression, Thrown> {
        let mut expr = ArithmeticExpression::default();

        if self.is_next_token(TokenType::T_OPEN_PARENTHESIS) {
            let peek = self.lexer.glimpse();

            if type_of(peek.as_ref()) == Some(TokenType::T_SELECT) {
                self.match_token(TokenType::T_OPEN_PARENTHESIS)?;
                expr.subselect = Some(self.subselect()?);
                self.match_token(TokenType::T_CLOSE_PARENTHESIS)?;

                return Ok(expr);
            }
        }

        expr.simple_arithmetic_expression = Some(self.simple_arithmetic_expression()?);

        Ok(expr)
    }

    /// ```text
    /// SimpleArithmeticExpression ::= ArithmeticTerm {("+" | "-") ArithmeticTerm}*
    /// ```
    pub fn simple_arithmetic_expression(&mut self) -> Result<Node, Thrown> {
        let mut terms = vec![self.arithmetic_term()?];

        loop {
            let is_plus = self.is_next_token(TokenType::T_PLUS);
            if !(is_plus || self.is_next_token(TokenType::T_MINUS)) {
                break;
            }
            self.match_token(if is_plus {
                TokenType::T_PLUS
            } else {
                TokenType::T_MINUS
            })?;

            terms.push(Node::str(self.matched_value()));
            terms.push(self.arithmetic_term()?);
        }

        // Phase 1 AST optimization: Prevent AST\SimpleArithmeticExpression
        // if only one AST\ArithmeticTerm is defined
        if terms.len() == 1 {
            return Ok(terms.remove(0));
        }

        Ok(Node::SimpleArithmeticExpression(Box::new(
            SimpleArithmeticExpression {
                arithmetic_terms: terms,
            },
        )))
    }

    /// ```text
    /// ArithmeticTerm ::= ArithmeticFactor {("*" | "/") ArithmeticFactor}*
    /// ```
    pub fn arithmetic_term(&mut self) -> Result<Node, Thrown> {
        let mut factors = vec![self.arithmetic_factor()?];

        loop {
            let is_mult = self.is_next_token(TokenType::T_MULTIPLY);
            if !(is_mult || self.is_next_token(TokenType::T_DIVIDE)) {
                break;
            }
            self.match_token(if is_mult {
                TokenType::T_MULTIPLY
            } else {
                TokenType::T_DIVIDE
            })?;

            factors.push(Node::str(self.matched_value()));
            factors.push(self.arithmetic_factor()?);
        }

        // Phase 1 AST optimization: Prevent AST\ArithmeticTerm
        // if only one AST\ArithmeticFactor is defined
        if factors.len() == 1 {
            return Ok(factors.remove(0));
        }

        Ok(Node::ArithmeticTerm(Box::new(ArithmeticTerm {
            arithmetic_factors: factors,
        })))
    }

    /// ```text
    /// ArithmeticFactor ::= [("+" | "-")] ArithmeticPrimary
    /// ```
    pub fn arithmetic_factor(&mut self) -> Result<Node, Thrown> {
        let mut sign = None;

        let is_plus = self.is_next_token(TokenType::T_PLUS);
        if is_plus || self.is_next_token(TokenType::T_MINUS) {
            self.match_token(if is_plus {
                TokenType::T_PLUS
            } else {
                TokenType::T_MINUS
            })?;
            sign = Some(is_plus);
        }

        let primary = self.arithmetic_primary()?;

        // Phase 1 AST optimization: Prevent AST\ArithmeticFactor
        // if only one AST\ArithmeticPrimary is defined
        if sign.is_none() {
            return Ok(primary);
        }

        Ok(Node::ArithmeticFactor(Box::new(ArithmeticFactor {
            arithmetic_primary: primary,
            sign,
        })))
    }

    /// ```text
    /// ArithmeticPrimary ::= SingleValuedPathExpression | Literal | ParenthesisExpression
    ///          | FunctionsReturningNumerics | AggregateExpression | FunctionsReturningStrings
    ///          | FunctionsReturningDatetime | IdentificationVariable | ResultVariable
    ///          | InputParameter | CaseExpression
    /// ```
    pub fn arithmetic_primary(&mut self) -> Result<Node, Thrown> {
        if self.is_next_token(TokenType::T_OPEN_PARENTHESIS) {
            self.match_token(TokenType::T_OPEN_PARENTHESIS)?;

            let expr = self.simple_arithmetic_expression()?;

            self.match_token(TokenType::T_CLOSE_PARENTHESIS)?;

            return Ok(Node::ParenthesisExpression(Box::new(
                ParenthesisExpression { expression: expr },
            )));
        }

        let Some(lookahead) = self.lookahead() else {
            return Err(self.syntax_error("ArithmeticPrimary", None));
        };

        match lookahead.type_ {
            Some(TokenType::T_COALESCE | TokenType::T_NULLIF | TokenType::T_CASE) => {
                self.case_expression()
            }
            Some(TokenType::T_IDENTIFIER) => {
                let peek = self.lexer.glimpse();

                if value_is(peek.as_ref(), "(") {
                    return self.function_declaration();
                }

                if value_is(peek.as_ref(), ".") {
                    return Ok(Node::PathExpression(self.single_valued_path_expression()?));
                }

                if has_result_variable(&self.query_components, &text(&lookahead.value)) {
                    return Ok(Node::str(self.result_variable()?));
                }

                Ok(Node::PathExpression(self.state_field_path_expression()?))
            }
            Some(TokenType::T_INPUT_PARAMETER) => {
                Ok(Node::InputParameter(Box::new(self.input_parameter()?)))
            }
            _ => {
                let peek = self.lexer.glimpse();

                if value_is(peek.as_ref(), "(") {
                    return self.function_declaration();
                }

                Ok(Node::Literal(Box::new(self.literal()?)))
            }
        }
    }

    /// ```text
    /// StringExpression ::= StringPrimary | ResultVariable | "(" Subselect ")"
    /// ```
    pub fn string_expression(&mut self) -> Result<Node, Thrown> {
        let peek = self.lexer.glimpse();

        // Subselect
        if self.is_next_token(TokenType::T_OPEN_PARENTHESIS)
            && type_of(peek.as_ref()) == Some(TokenType::T_SELECT)
        {
            self.match_token(TokenType::T_OPEN_PARENTHESIS)?;
            let expr = self.subselect()?;
            self.match_token(TokenType::T_CLOSE_PARENTHESIS)?;

            return Ok(Node::Subselect(Box::new(expr)));
        }

        // ResultVariable (string)
        if self.is_next_token(TokenType::T_IDENTIFIER)
            && has_result_variable(
                &self.query_components,
                &self.lookahead_value().unwrap_or_default(),
            )
        {
            return Ok(Node::str(self.result_variable()?));
        }

        self.string_primary()
    }

    /// ```text
    /// StringPrimary ::= StateFieldPathExpression | string | InputParameter | FunctionsReturningStrings | AggregateExpression | CaseExpression
    /// ```
    pub fn string_primary(&mut self) -> Result<Node, Thrown> {
        let lookahead_type = self.lookahead_type();

        match lookahead_type {
            Some(TokenType::T_IDENTIFIER) => {
                let peek = self.lexer.glimpse();

                if value_is(peek.as_ref(), ".") {
                    return Ok(Node::PathExpression(self.state_field_path_expression()?));
                }

                if value_is(peek.as_ref(), "(") {
                    // do NOT directly go to FunctionsReturningString() because it doesn't check for custom functions.
                    return self.function_declaration();
                }

                return Err(self.syntax_error("'.' or '('", None));
            }
            Some(TokenType::T_STRING) => {
                self.match_token(TokenType::T_STRING)?;

                return Ok(Node::Literal(Box::new(Literal {
                    type_: ast::LITERAL_STRING,
                    value: self.token_value(),
                })));
            }
            Some(TokenType::T_INPUT_PARAMETER) => {
                return Ok(Node::InputParameter(Box::new(self.input_parameter()?)));
            }
            Some(TokenType::T_CASE | TokenType::T_COALESCE | TokenType::T_NULLIF) => {
                return self.case_expression();
            }
            _ => {
                if Self::is_aggregate_function(lookahead_type) {
                    return Ok(Node::AggregateExpression(Box::new(
                        self.aggregate_expression()?,
                    )));
                }
            }
        }

        Err(self.syntax_error(
            "StateFieldPathExpression | string | InputParameter | FunctionsReturningStrings | AggregateExpression",
            None,
        ))
    }

    /// ```text
    /// EntityExpression ::= SingleValuedAssociationPathExpression | SimpleEntityExpression
    /// ```
    pub fn entity_expression(&mut self) -> Result<Node, Thrown> {
        let glimpse = self.lexer.glimpse();

        if self.is_next_token(TokenType::T_IDENTIFIER) && value_is(glimpse.as_ref(), ".") {
            return Ok(Node::PathExpression(
                self.single_valued_association_path_expression()?,
            ));
        }

        self.simple_entity_expression()
    }

    /// ```text
    /// SimpleEntityExpression ::= IdentificationVariable | InputParameter
    /// ```
    pub fn simple_entity_expression(&mut self) -> Result<Node, Thrown> {
        if self.is_next_token(TokenType::T_INPUT_PARAMETER) {
            return Ok(Node::InputParameter(Box::new(self.input_parameter()?)));
        }

        Ok(Node::PathExpression(self.state_field_path_expression()?))
    }

    /// ```text
    /// AggregateExpression ::=
    ///  ("AVG" | "MAX" | "MIN" | "SUM" | "COUNT") "(" ["DISTINCT"] SimpleArithmeticExpression ")"
    /// ```
    pub fn aggregate_expression(&mut self) -> Result<AggregateExpression, Thrown> {
        let lookahead_type = self.lookahead_type();
        let mut is_distinct = false;

        let Some(
            lookahead_type @ (TokenType::T_COUNT
            | TokenType::T_AVG
            | TokenType::T_MAX
            | TokenType::T_MIN
            | TokenType::T_SUM),
        ) = lookahead_type
        else {
            return Err(self.syntax_error("One of: MAX, MIN, AVG, SUM, COUNT", None));
        };

        self.match_token(lookahead_type)?;
        let function_name = self.matched_value();
        self.match_token(TokenType::T_OPEN_PARENTHESIS)?;

        if self.is_next_token(TokenType::T_DISTINCT) {
            self.match_token(TokenType::T_DISTINCT)?;
            is_distinct = true;
        }

        let path_exp = self.simple_arithmetic_expression()?;

        self.match_token(TokenType::T_CLOSE_PARENTHESIS)?;

        Ok(AggregateExpression {
            function_name,
            path_expression: path_exp,
            is_distinct,
        })
    }

    /// ```text
    /// QuantifiedExpression ::= ("ALL" | "ANY" | "SOME") "(" Subselect ")"
    /// ```
    pub fn quantified_expression(&mut self) -> Result<QuantifiedExpression, Thrown> {
        let lookahead_type = self.lookahead_type();
        let value = self.lookahead_value().unwrap_or_default();

        let Some(lookahead_type @ (TokenType::T_ALL | TokenType::T_ANY | TokenType::T_SOME)) =
            lookahead_type
        else {
            return Err(self.syntax_error("ALL, ANY or SOME", None));
        };

        self.match_token(lookahead_type)?;
        self.match_token(TokenType::T_OPEN_PARENTHESIS)?;

        let q_expr = QuantifiedExpression {
            type_: value,
            subselect: self.subselect()?,
        };

        self.match_token(TokenType::T_CLOSE_PARENTHESIS)?;

        Ok(q_expr)
    }

    /// ```text
    /// BetweenExpression ::= ArithmeticExpression ["NOT"] "BETWEEN" ArithmeticExpression "AND" ArithmeticExpression
    /// ```
    pub fn between_expression(&mut self) -> Result<BetweenExpression, Thrown> {
        let mut not = false;
        let arith_expr1 = self.arithmetic_expression()?;

        if self.is_next_token(TokenType::T_NOT) {
            self.match_token(TokenType::T_NOT)?;
            not = true;
        }

        self.match_token(TokenType::T_BETWEEN)?;
        let arith_expr2 = self.arithmetic_expression()?;
        self.match_token(TokenType::T_AND)?;
        let arith_expr3 = self.arithmetic_expression()?;

        Ok(BetweenExpression {
            expression: arith_expr1,
            left_between_expression: arith_expr2,
            right_between_expression: arith_expr3,
            not,
        })
    }

    /// ```text
    /// ComparisonExpression ::= ArithmeticExpression ComparisonOperator ( QuantifiedExpression | ArithmeticExpression )
    /// ```
    pub fn comparison_expression(&mut self) -> Result<ComparisonExpression, Thrown> {
        self.lexer.glimpse();

        let left_expr = Node::ArithmeticExpression(Box::new(self.arithmetic_expression()?));
        let operator = self.comparison_operator()?;
        let right_expr = if self.is_next_all_any_some() {
            Node::QuantifiedExpression(Box::new(self.quantified_expression()?))
        } else {
            Node::ArithmeticExpression(Box::new(self.arithmetic_expression()?))
        };

        Ok(ComparisonExpression {
            left_expression: left_expr,
            operator,
            right_expression: right_expr,
        })
    }

    /// ```text
    /// InExpression ::= SingleValuedPathExpression ["NOT"] "IN" "(" (InParameter {"," InParameter}* | Subselect) ")"
    /// ```
    pub fn in_expression(&mut self) -> Result<Node, Thrown> {
        let expression = self.arithmetic_expression()?;

        let mut not = false;
        if self.is_next_token(TokenType::T_NOT) {
            self.match_token(TokenType::T_NOT)?;
            not = true;
        }

        self.match_token(TokenType::T_IN)?;
        self.match_token(TokenType::T_OPEN_PARENTHESIS)?;

        let in_expression = if self.is_next_token(TokenType::T_SELECT) {
            Node::InSubselectExpression(Box::new(InSubselectExpression {
                expression,
                subselect: self.subselect()?,
                not,
            }))
        } else {
            let mut literals = vec![self.in_parameter()?];

            while self.is_next_token(TokenType::T_COMMA) {
                self.match_token(TokenType::T_COMMA)?;
                literals.push(self.in_parameter()?);
            }

            Node::InListExpression(Box::new(InListExpression {
                expression,
                literals,
                not,
            }))
        };

        self.match_token(TokenType::T_CLOSE_PARENTHESIS)?;

        Ok(in_expression)
    }

    /// ```text
    /// InstanceOfExpression ::= IdentificationVariable ["NOT"] "INSTANCE" ["OF"] (InstanceOfParameter | "(" InstanceOfParameter {"," InstanceOfParameter}* ")")
    /// ```
    pub fn instance_of_expression(&mut self) -> Result<InstanceOfExpression, Thrown> {
        let identification_variable = self.identification_variable()?;

        let mut not = false;
        if self.is_next_token(TokenType::T_NOT) {
            self.match_token(TokenType::T_NOT)?;
            not = true;
        }

        self.match_token(TokenType::T_INSTANCE)?;
        self.match_token(TokenType::T_OF)?;

        let expr_values = if self.is_next_token(TokenType::T_OPEN_PARENTHESIS) {
            self.instance_of_parameter_list()?
        } else {
            vec![self.instance_of_parameter()?]
        };

        Ok(InstanceOfExpression {
            identification_variable,
            value: expr_values,
            not,
        })
    }

    pub fn instance_of_parameter_list(&mut self) -> Result<Vec<Node>, Thrown> {
        self.match_token(TokenType::T_OPEN_PARENTHESIS)?;

        let mut expr_values = vec![self.instance_of_parameter()?];

        while self.is_next_token(TokenType::T_COMMA) {
            self.match_token(TokenType::T_COMMA)?;

            expr_values.push(self.instance_of_parameter()?);
        }

        self.match_token(TokenType::T_CLOSE_PARENTHESIS)?;

        Ok(expr_values)
    }

    /// ```text
    /// InstanceOfParameter ::= AbstractSchemaName | InputParameter
    /// ```
    pub fn instance_of_parameter(&mut self) -> Result<Node, Thrown> {
        if self.is_next_token(TokenType::T_INPUT_PARAMETER) {
            self.match_token(TokenType::T_INPUT_PARAMETER)?;

            return Ok(Node::InputParameter(Box::new(InputParameter::new(
                &self.matched_value(),
            )?)));
        }

        let abstract_schema_name = self.abstract_schema_name()?;

        self.validate_abstract_schema_name(&abstract_schema_name)?;

        Ok(Node::str(abstract_schema_name))
    }

    /// ```text
    /// LikeExpression ::= StringExpression ["NOT"] "LIKE" StringPrimary ["ESCAPE" char]
    /// ```
    pub fn like_expression(&mut self) -> Result<LikeExpression, Thrown> {
        let string_expr = self.string_expression()?;
        let mut not = false;

        if self.is_next_token(TokenType::T_NOT) {
            self.match_token(TokenType::T_NOT)?;
            not = true;
        }

        self.match_token(TokenType::T_LIKE)?;

        let string_pattern = if self.is_next_token(TokenType::T_INPUT_PARAMETER) {
            self.match_token(TokenType::T_INPUT_PARAMETER)?;
            Node::InputParameter(Box::new(InputParameter::new(&self.matched_value())?))
        } else {
            self.string_primary()?
        };

        let mut escape_char = None;

        if self.lookahead_type() == Some(TokenType::T_ESCAPE) {
            self.match_token(TokenType::T_ESCAPE)?;
            self.match_token(TokenType::T_STRING)?;

            escape_char = Some(Literal {
                type_: ast::LITERAL_STRING,
                value: self.token_value(),
            });
        }

        Ok(LikeExpression {
            string_expression: string_expr,
            string_pattern,
            escape_char,
            not,
        })
    }

    /// ```text
    /// NullComparisonExpression ::= (InputParameter | NullIfExpression | CoalesceExpression | AggregateExpression | FunctionDeclaration | IdentificationVariable | SingleValuedPathExpression | ResultVariable) "IS" ["NOT"] "NULL"
    /// ```
    pub fn null_comparison_expression(&mut self) -> Result<NullComparisonExpression, Thrown> {
        let expr = if self.is_next_token(TokenType::T_INPUT_PARAMETER) {
            self.match_token(TokenType::T_INPUT_PARAMETER)?;
            Node::InputParameter(Box::new(InputParameter::new(&self.matched_value())?))
        } else if self.is_next_token(TokenType::T_NULLIF) {
            Node::NullIfExpression(Box::new(self.null_if_expression()?))
        } else if self.is_next_token(TokenType::T_COALESCE) {
            Node::CoalesceExpression(Box::new(self.coalesce_expression()?))
        } else if self.is_function() {
            self.function_declaration()?
        } else {
            let glimpse = self.lexer.glimpse();

            if type_of(glimpse.as_ref()) == Some(TokenType::T_DOT) {
                Node::PathExpression(self.single_valued_path_expression()?)
            } else {
                let lookahead_value = self.lookahead_value().unwrap_or_default();

                if !self.query_components.contains_key(&lookahead_value) {
                    return Err(self.semantical_error(
                        "Cannot add having condition on undefined result variable.",
                        None,
                    ));
                }

                // Validate SingleValuedPathExpression (ie.: "product")
                if has_metadata(&self.query_components, &lookahead_value) {
                    Node::PathExpression(self.single_valued_path_expression()?)
                } else {
                    // Validating ResultVariable
                    if !has_result_variable(&self.query_components, &lookahead_value) {
                        return Err(self.semantical_error(
                            "Cannot add having condition on a non result variable.",
                            None,
                        ));
                    }

                    Node::str(self.result_variable()?)
                }
            }
        };

        self.match_token(TokenType::T_IS)?;

        let mut not = false;
        if self.is_next_token(TokenType::T_NOT) {
            self.match_token(TokenType::T_NOT)?;

            not = true;
        }

        self.match_token(TokenType::T_NULL)?;

        Ok(NullComparisonExpression {
            expression: expr,
            not,
        })
    }

    /// ```text
    /// ExistsExpression ::= ["NOT"] "EXISTS" "(" Subselect ")"
    /// ```
    pub fn exists_expression(&mut self) -> Result<ExistsExpression, Thrown> {
        let mut not = false;

        if self.is_next_token(TokenType::T_NOT) {
            self.match_token(TokenType::T_NOT)?;
            not = true;
        }

        self.match_token(TokenType::T_EXISTS)?;
        self.match_token(TokenType::T_OPEN_PARENTHESIS)?;

        let subselect = self.subselect()?;

        self.match_token(TokenType::T_CLOSE_PARENTHESIS)?;

        Ok(ExistsExpression { subselect, not })
    }

    /// ```text
    /// ComparisonOperator ::= "=" | "<" | "<=" | "<>" | ">" | ">=" | "!="
    /// ```
    pub fn comparison_operator(&mut self) -> Result<String, Thrown> {
        match self.lookahead_value().as_deref() {
            Some("=") => {
                self.match_token(TokenType::T_EQUALS)?;

                Ok("=".to_owned())
            }
            Some("<") => {
                self.match_token(TokenType::T_LOWER_THAN)?;
                let mut operator = String::from("<");

                if self.is_next_token(TokenType::T_EQUALS) {
                    self.match_token(TokenType::T_EQUALS)?;
                    operator.push('=');
                } else if self.is_next_token(TokenType::T_GREATER_THAN) {
                    self.match_token(TokenType::T_GREATER_THAN)?;
                    operator.push('>');
                }

                Ok(operator)
            }
            Some(">") => {
                self.match_token(TokenType::T_GREATER_THAN)?;
                let mut operator = String::from(">");

                if self.is_next_token(TokenType::T_EQUALS) {
                    self.match_token(TokenType::T_EQUALS)?;
                    operator.push('=');
                }

                Ok(operator)
            }
            Some("!") => {
                self.match_token(TokenType::T_NEGATE)?;
                self.match_token(TokenType::T_EQUALS)?;

                Ok("<>".to_owned())
            }
            _ => Err(self.syntax_error("=, <, <=, <>, >, >=, !=", None)),
        }
    }

    /// ```text
    /// FunctionDeclaration ::= FunctionsReturningStrings | FunctionsReturningNumerics | FunctionsReturningDatetime
    /// ```
    pub fn function_declaration(&mut self) -> Result<Node, Thrown> {
        let token = self.lookahead();
        let func_name = value_of(token.as_ref())
            .unwrap_or_default()
            .to_ascii_lowercase();

        if let Some(custom_function_declaration) = self.custom_function_declaration()? {
            return Ok(custom_function_declaration);
        }

        if functions::is_string_function(&func_name)
            || functions::is_numeric_function(&func_name)
            || functions::is_datetime_function(&func_name)
        {
            return self.builtin_function();
        }

        Err(self.syntax_error("known function", token.as_ref()))
    }

    /// Helper function for `FunctionDeclaration` grammar rule.
    fn custom_function_declaration(&mut self) -> Result<Option<Node>, Thrown> {
        let func_name = self
            .lookahead_value()
            .unwrap_or_default()
            .to_ascii_lowercase();

        if self.em.get_custom_string_function(&func_name).is_some() {
            return self.custom_functions_returning_strings().map(Some);
        }
        if self.em.get_custom_numeric_function(&func_name).is_some() {
            return self.custom_functions_returning_numerics().map(Some);
        }
        if self.em.get_custom_datetime_function(&func_name).is_some() {
            return self.custom_functions_returning_datetime().map(Some);
        }
        Ok(None)
    }

    /// `FunctionsReturningStrings`, `FunctionsReturningNumerics` and
    /// `FunctionsReturningDatetime`: `new $funcClass($funcNameLower)`, parsed.
    fn builtin_function(&mut self) -> Result<Node, Thrown> {
        let func_name_lower = self
            .lookahead_value()
            .unwrap_or_default()
            .to_ascii_lowercase();
        let Some(function) = functions::builtin(&func_name_lower) else {
            return Err(self.syntax_error("known function", None));
        };
        self.parse_function(function)
    }

    fn parse_function(&mut self, mut function: Box<dyn FunctionNode>) -> Result<Node, Thrown> {
        function.parse(self)?;
        Ok(Node::Function(Rc::from(function)))
    }

    /// ```text
    /// FunctionsReturningNumerics ::=
    ///      "LENGTH" "(" StringPrimary ")" |
    ///      "LOCATE" "(" StringPrimary "," StringPrimary ["," SimpleArithmeticExpression]")" |
    ///      "ABS" "(" SimpleArithmeticExpression ")" |
    ///      "SQRT" "(" SimpleArithmeticExpression ")" |
    ///      "MOD" "(" SimpleArithmeticExpression "," SimpleArithmeticExpression ")" |
    ///      "SIZE" "(" CollectionValuedPathExpression ")" |
    ///      "DATE_DIFF" "(" ArithmeticPrimary "," ArithmeticPrimary ")" |
    ///      "BIT_AND" "(" ArithmeticPrimary "," ArithmeticPrimary ")" |
    ///      "BIT_OR" "(" ArithmeticPrimary "," ArithmeticPrimary ")"
    /// ```
    pub fn functions_returning_numerics(&mut self) -> Result<Node, Thrown> {
        self.builtin_function()
    }

    pub fn custom_functions_returning_numerics(&mut self) -> Result<Node, Thrown> {
        // getCustomNumericFunction is case-insensitive
        let function_name = self
            .lookahead_value()
            .unwrap_or_default()
            .to_ascii_lowercase();
        let Some(function_class) = self.em.get_custom_numeric_function(&function_name) else {
            return Err(self.syntax_error("known function", None));
        };

        let function = function_class(&function_name);
        self.parse_function(function)
    }

    /// ```text
    /// FunctionsReturningDateTime ::=
    ///     "CURRENT_DATE" |
    ///     "CURRENT_TIME" |
    ///     "CURRENT_TIMESTAMP" |
    ///     "DATE_ADD" "(" ArithmeticPrimary "," ArithmeticPrimary "," StringPrimary ")" |
    ///     "DATE_SUB" "(" ArithmeticPrimary "," ArithmeticPrimary "," StringPrimary ")"
    /// ```
    pub fn functions_returning_datetime(&mut self) -> Result<Node, Thrown> {
        self.builtin_function()
    }

    pub fn custom_functions_returning_datetime(&mut self) -> Result<Node, Thrown> {
        // getCustomDatetimeFunction is case-insensitive
        let function_name = self.lookahead_value().unwrap_or_default();
        let Some(function_class) = self.em.get_custom_datetime_function(&function_name) else {
            return Err(self.syntax_error("known function", None));
        };

        let function = function_class(&function_name);
        self.parse_function(function)
    }

    /// ```text
    /// FunctionsReturningStrings ::=
    ///   "CONCAT" "(" StringPrimary "," StringPrimary {"," StringPrimary}* ")" |
    ///   "SUBSTRING" "(" StringPrimary "," SimpleArithmeticExpression "," SimpleArithmeticExpression ")" |
    ///   "TRIM" "(" [["LEADING" | "TRAILING" | "BOTH"] [char] "FROM"] StringPrimary ")" |
    ///   "LOWER" "(" StringPrimary ")" |
    ///   "UPPER" "(" StringPrimary ")" |
    ///   "IDENTITY" "(" SingleValuedAssociationPathExpression {"," string} ")"
    /// ```
    pub fn functions_returning_strings(&mut self) -> Result<Node, Thrown> {
        self.builtin_function()
    }

    pub fn custom_functions_returning_strings(&mut self) -> Result<Node, Thrown> {
        // getCustomStringFunction is case-insensitive
        let function_name = self.lookahead_value().unwrap_or_default();
        let Some(function_class) = self.em.get_custom_string_function(&function_name) else {
            return Err(self.syntax_error("known function", None));
        };

        let function = function_class(&function_name);
        self.parse_function(function)
    }

    fn get_metadata_for_dql_alias(
        &self,
        dql_alias: &str,
    ) -> Result<std::sync::Arc<crate::mapping::ClassMetadata>, Thrown> {
        metadata_for_dql_alias(&self.query_components, dql_alias)
    }
}

/// `ReflectionClass::getConstructor()`: the `__construct` method the
/// class declares or inherits.
fn find_constructor(info: &php_reflection::ClassInfo) -> Option<php_reflection::MethodInfo> {
    let mut current = Some(std::sync::Arc::new(info.clone()));
    let mut depth = 0;
    while let Some(class) = current {
        if let Some(method) = class
            .methods()
            .iter()
            .find(|m| m.name().eq_ignore_ascii_case("__construct"))
        {
            return Some(method.clone());
        }
        depth += 1;
        if depth > 64 {
            return None;
        }
        current = class.parent().and_then(php_reflection::class_info);
    }
    None
}
