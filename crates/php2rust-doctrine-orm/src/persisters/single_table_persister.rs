//! Persister for entities that participate in a hierarchy mapped with the
//! `SINGLE_TABLE` strategy.
//!
//! `BasicEntityPersister` with `PersisterKind::SingleTable` runs these
//! overrides; `getInsertColumnList()` and `getSQLTableAlias()` are in
//! `basic_entity_persister.rs`. Filters are not ported, so there is no
//! `generateFilterConditionSQL()`.
//!
//! @link <https://martinfowler.com/eaaCatalog/singleTableInheritance.html>
//!
//! Upstream: doctrine/orm src/Persisters/Entity/SingleTablePersister.php

use std::sync::Arc;

use php_value::Thrown;

use crate::entity_manager::EntityManager;
use crate::mapping::quote_strategy::get_sql_result_casing;
use crate::query::context::QueryContext;
use crate::utility::{get_type_of_column, to_text};

use super::basic_entity_persister::BasicEntityPersister;

impl BasicEntityPersister {
    /// `SingleTablePersister::getSelectColumnsSQL()`.
    pub(super) fn single_table_get_select_columns_sql(
        &self,
        em: &Arc<EntityManager>,
    ) -> Result<String, Thrown> {
        if let Some(sql) = self.cached_select_column_list_sql(em) {
            return Ok(sql);
        }
        let mut column_list = vec![self.basic_get_select_columns_sql(em)?];

        let platform = em.get_platform();
        let root = self.class.read().root_entity_name.clone();
        let root_class = em.get_class_metadata(&root)?;
        let table_alias = self.get_sql_table_alias(&root_class.read().name.clone(), "");

        // Append discriminator column
        let (discr_column_name, discr_column_type, sub_classes) = {
            let info = self.class.read();
            let discr = info.get_discriminator_column()?;
            (
                discr.name.clone(),
                discr.r#type.clone(),
                info.sub_classes.clone(),
            )
        };

        column_list.push(format!("{table_alias}.{discr_column_name}"));

        let result_column_name = get_sql_result_casing(platform, &discr_column_name);
        {
            let mut st = self.st();
            let rsm = &mut st.ctx().rsm;
            rsm.set_discriminator_column("r", &result_column_name);
            rsm.add_meta_result(
                "r",
                &result_column_name,
                &discr_column_name,
                false,
                Some(&discr_column_type),
            );
        }

        // Append subclass columns
        for sub_class_name in &sub_classes {
            let sub_class = em.get_class_metadata(sub_class_name)?;

            // Regular columns
            let fields: Vec<String> = sub_class
                .read()
                .field_mappings
                .iter()
                .filter(|(_, mapping)| mapping.inherited.is_none())
                .map(|(field, _)| field.to_owned())
                .collect();
            for field_name in fields {
                column_list.push(self.get_select_column_sql(em, &field_name, &sub_class, "r")?);
            }

            // Foreign key columns
            let associations: Vec<_> = sub_class
                .read()
                .association_mappings
                .values()
                .filter(|assoc| assoc.is_to_one_owning_side() && assoc.inherited.is_none())
                .cloned()
                .collect();
            for assoc in associations {
                let target_class = em.get_class_metadata(&assoc.target_entity)?;
                for join_column in &assoc.join_columns {
                    let quoted = self.quote_strategy.get_join_column_name(
                        join_column,
                        &sub_class.read(),
                        platform,
                    );
                    let ty = get_type_of_column(
                        &join_column.referenced_column_name,
                        &target_class.read(),
                        em,
                    )?;
                    column_list.push(self.get_select_join_column_sql(
                        em,
                        &table_alias,
                        &join_column.name,
                        &quoted,
                        &ty,
                    ));
                }
            }
        }

        let sql = column_list.join(", ");
        self.st().ctx().select_column_list_sql = Some(sql.clone());
        self.update_filter_hash(em);
        Ok(sql)
    }

    /// The `getSelectConditionSQL()` and `getSelectConditionCriteriaSQL()`
    /// overrides: the parent's condition, then the discriminator values.
    pub(super) fn single_table_append_discriminator_condition(
        &self,
        em: &Arc<EntityManager>,
        mut condition_sql: String,
    ) -> Result<String, Thrown> {
        if !condition_sql.is_empty() {
            condition_sql.push_str(" AND ");
        }
        condition_sql.push_str(&self.get_select_condition_discriminator_value_sql(em)?);
        Ok(condition_sql)
    }

    /// `getSelectConditionDiscriminatorValueSQL()`.
    fn get_select_condition_discriminator_value_sql(
        &self,
        em: &Arc<EntityManager>,
    ) -> Result<String, Thrown> {
        let (mut discriminator_values, discriminator_value, discr_column_name, class_name) = {
            let info = self.class.read();
            // array_flip(array_intersect($this->class->discriminatorMap, $this->class->subClasses))
            let values: Vec<String> = info
                .discriminator_map
                .iter()
                .filter(|(_, class)| info.sub_classes.iter().any(|s| *s == to_text(class)))
                .map(|(value, _)| crate::utility::key_text(value))
                .collect();
            (
                values,
                info.discriminator_value.clone(),
                info.get_discriminator_column()?.name.clone(),
                info.name.clone(),
            )
        };
        let mut values = Vec::new();
        if !discriminator_value.is_null() {
            // discriminators can be 0
            values.push(QueryContext::quote(&**em, &to_text(&discriminator_value))?);
        }
        for value in discriminator_values.drain(..) {
            values.push(QueryContext::quote(&**em, &value)?);
        }

        let table_alias = self.get_sql_table_alias(&class_name, "");
        Ok(format!(
            "{table_alias}.{discr_column_name} IN ({})",
            values.join(", ")
        ))
    }
}
