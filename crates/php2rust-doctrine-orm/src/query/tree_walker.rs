//! `Doctrine\ORM\Query\TreeWalker`, `TreeWalkerAdapter` and
//! `TreeWalkerChain`: walkers that may modify the AST before the output
//! walker generates SQL. PHP's walkers receive the AST objects and change
//! them in place; here they receive the statement as `&mut`.
//!
//! Upstream: doctrine/orm src/Query/TreeWalker.php src/Query/TreeWalkerAdapter.php src/Query/TreeWalkerChain.php

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use php_value::Thrown;

use crate::mapping::ClassMetadata;
use crate::query::ast::{DeleteStatement, SelectStatement, UpdateStatement};
use crate::query::context::{
    DqlQuery, QueryComponent, QueryComponents, TreeWalkerClass, metadata_for_dql_alias,
};
use crate::query::parser_result::ParserResult;
use crate::query::query_exception;

/// `Doctrine\ORM\Query\TreeWalker`: interface for walkers of DQL ASTs
/// (abstract syntax trees).
pub trait TreeWalker<'a> {
    /// Returns internal queryComponents array.
    fn get_query_components(&self) -> &QueryComponents;

    /// Walks down a `SelectStatement` AST node.
    fn walk_select_statement(
        &mut self,
        select_statement: &mut SelectStatement,
    ) -> Result<(), Thrown>;

    /// Walks down an `UpdateStatement` AST node.
    fn walk_update_statement(
        &mut self,
        update_statement: &mut UpdateStatement,
    ) -> Result<(), Thrown>;

    /// Walks down a `DeleteStatement` AST node.
    fn walk_delete_statement(
        &mut self,
        delete_statement: &mut DeleteStatement,
    ) -> Result<(), Thrown>;
}

/// `Doctrine\ORM\Query\TreeWalkerAdapter`: the state of an adapter; a tree
/// walker holds one and implements [`TreeWalker`] with it (the walk
/// methods of the adapter do nothing).
pub struct TreeWalkerAdapter<'a> {
    query: &'a dyn DqlQuery,
    parser_result: Rc<RefCell<ParserResult>>,
    query_components: QueryComponents,
}

impl<'a> TreeWalkerAdapter<'a> {
    pub fn new(
        query: &'a dyn DqlQuery,
        parser_result: Rc<RefCell<ParserResult>>,
        query_components: QueryComponents,
    ) -> Self {
        Self {
            query,
            parser_result,
            query_components,
        }
    }

    pub fn get_query_components(&self) -> &QueryComponents {
        &self.query_components
    }

    /// Sets or overrides a query component for a given dql alias.
    pub fn set_query_component(
        &mut self,
        dql_alias: &str,
        query_component: QueryComponent,
    ) -> Result<(), Thrown> {
        if !query_component.has_entity_keys {
            return Err(query_exception::invalid_query_component(dql_alias));
        }
        self.query_components
            .insert(dql_alias.to_owned(), query_component);
        Ok(())
    }

    /// `_getQuery()`.
    pub fn get_query(&self) -> &'a dyn DqlQuery {
        self.query
    }

    /// `_getParserResult()`.
    pub fn get_parser_result(&self) -> &Rc<RefCell<ParserResult>> {
        &self.parser_result
    }

    pub fn get_metadata_for_dql_alias(
        &self,
        dql_alias: &str,
    ) -> Result<Arc<ClassMetadata>, Thrown> {
        metadata_for_dql_alias(&self.query_components, dql_alias)
    }
}

/// Represents a chain of tree walkers that modify an AST and finally emit
/// output. Only the last walker in the chain can emit output. Any
/// previous walkers can modify the AST to influence the final output.
pub struct TreeWalkerChain<'a> {
    query: &'a dyn DqlQuery,
    parser_result: Rc<RefCell<ParserResult>>,
    query_components: QueryComponents,
    /// The tree walkers.
    walkers: Vec<TreeWalkerClass>,
}

impl<'a> TreeWalkerChain<'a> {
    pub fn new(
        query: &'a dyn DqlQuery,
        parser_result: Rc<RefCell<ParserResult>>,
        query_components: QueryComponents,
    ) -> Self {
        Self {
            query,
            parser_result,
            query_components,
            walkers: Vec::new(),
        }
    }

    /// Adds a tree walker to the chain.
    pub fn add_tree_walker(&mut self, walker_class: TreeWalkerClass) {
        self.walkers.push(walker_class);
    }

    /// `getWalkers()`: a new walker of each class, with the current query
    /// components.
    fn walker(&self, walker_class: TreeWalkerClass) -> Box<dyn TreeWalker<'a> + 'a> {
        (walker_class.construct)(
            self.query,
            self.parser_result.clone(),
            self.query_components.clone(),
        )
    }
}

impl<'a> TreeWalker<'a> for TreeWalkerChain<'a> {
    fn get_query_components(&self) -> &QueryComponents {
        &self.query_components
    }

    fn walk_select_statement(
        &mut self,
        select_statement: &mut SelectStatement,
    ) -> Result<(), Thrown> {
        for walker_class in self.walkers.clone() {
            let mut walker = self.walker(walker_class);
            walker.walk_select_statement(select_statement)?;
            self.query_components = walker.get_query_components().clone();
        }
        Ok(())
    }

    fn walk_update_statement(
        &mut self,
        update_statement: &mut UpdateStatement,
    ) -> Result<(), Thrown> {
        for walker_class in self.walkers.clone() {
            self.walker(walker_class)
                .walk_update_statement(update_statement)?;
        }
        Ok(())
    }

    fn walk_delete_statement(
        &mut self,
        delete_statement: &mut DeleteStatement,
    ) -> Result<(), Thrown> {
        for walker_class in self.walkers.clone() {
            self.walker(walker_class)
                .walk_delete_statement(delete_statement)?;
        }
        Ok(())
    }
}
