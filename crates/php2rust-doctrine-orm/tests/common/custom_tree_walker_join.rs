//! `Doctrine\Tests\Mocks\CustomTreeWalkerJoin`: a tree walker that joins
//! the address of each `CmsUser` and selects it.
//!
//! Upstream: doctrine/orm tests/Tests/Mocks/CustomTreeWalkerJoin.php

use std::cell::RefCell;
use std::rc::Rc;

use php_value::Thrown;
use php2rust_doctrine_orm::query::ast::{
    DeleteStatement, IdentificationVariableDeclaration, JOIN_TYPE_LEFT, Join,
    JoinAssociationDeclaration, JoinAssociationPathExpression, Node, SelectExpression,
    SelectStatement, UpdateStatement,
};
use php2rust_doctrine_orm::query::context::{
    DqlQuery, QueryComponent, QueryComponents, TreeWalkerClass,
};
use php2rust_doctrine_orm::query::{ParserResult, TreeWalker, TreeWalkerAdapter};

const CMS_USER: &str = r"Doctrine\Tests\Models\CMS\CmsUser";
const CMS_ADDRESS: &str = r"Doctrine\Tests\Models\CMS\CmsAddress";

pub struct CustomTreeWalkerJoin<'a> {
    adapter: TreeWalkerAdapter<'a>,
}

impl CustomTreeWalkerJoin<'_> {
    pub const CLASS: TreeWalkerClass = TreeWalkerClass {
        name: "Doctrine\\Tests\\Mocks\\CustomTreeWalkerJoin",
        construct,
    };

    fn modify_select_statement(
        &mut self,
        select_expressions: &mut Vec<Rc<SelectExpression>>,
        identification_variable_decl: &mut IdentificationVariableDeclaration,
    ) -> Result<(), Thrown> {
        let alias = identification_variable_decl
            .range_variable_declaration
            .as_ref()
            .map(|decl| decl.alias_identification_variable.clone())
            .unwrap_or_default();
        let join_assoc_path_expression = JoinAssociationPathExpression {
            identification_variable: alias.clone(),
            association_field: "address".to_owned(),
        };
        let join_assoc_declaration = JoinAssociationDeclaration {
            join_association_path_expression: join_assoc_path_expression,
            alias_identification_variable: format!("{alias}a"),
            index_by: None,
        };
        let join = Join::new(
            JOIN_TYPE_LEFT,
            Some(Node::JoinAssociationDeclaration(Box::new(
                join_assoc_declaration,
            ))),
        );
        let select_expression = SelectExpression {
            expression: Node::str(format!("{alias}a")),
            field_identification_variable: None,
            hidden_alias_result_variable: false,
        };

        identification_variable_decl.joins.push(join);
        select_expressions.push(Rc::new(select_expression));

        let entity_manager = self.adapter.get_query().get_entity_manager();
        let user_metadata = entity_manager.get_class_metadata(CMS_USER)?;
        let address_metadata = entity_manager.get_class_metadata(CMS_ADDRESS)?;

        self.adapter.set_query_component(
            &format!("{alias}a"),
            QueryComponent::entity(
                address_metadata,
                Some(alias),
                Some(
                    user_metadata
                        .read()
                        .get_association_mapping("address")?
                        .clone(),
                ),
                0,
                None,
            ),
        )
    }
}

fn construct<'a>(
    query: &'a dyn DqlQuery,
    parser_result: Rc<RefCell<ParserResult>>,
    query_components: QueryComponents,
) -> Box<dyn TreeWalker<'a> + 'a> {
    Box::new(CustomTreeWalkerJoin {
        adapter: TreeWalkerAdapter::new(query, parser_result, query_components),
    })
}

impl<'a> TreeWalker<'a> for CustomTreeWalkerJoin<'a> {
    fn get_query_components(&self) -> &QueryComponents {
        self.adapter.get_query_components()
    }

    fn walk_select_statement(
        &mut self,
        select_statement: &mut SelectStatement,
    ) -> Result<(), Thrown> {
        let SelectStatement {
            select_clause,
            from_clause,
            ..
        } = select_statement;
        for identification_variable_declaration in
            &mut from_clause.identification_variable_declarations
        {
            let is_user = identification_variable_declaration
                .range_variable_declaration
                .as_ref()
                .is_some_and(|decl| decl.abstract_schema_name == CMS_USER);
            if !is_user {
                continue;
            }

            self.modify_select_statement(
                &mut select_clause.select_expressions,
                identification_variable_declaration,
            )?;
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
