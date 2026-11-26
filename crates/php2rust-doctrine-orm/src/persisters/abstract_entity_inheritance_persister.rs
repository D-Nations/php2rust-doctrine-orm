//! Base class for entity persisters that implement a certain inheritance
//! mapping strategy. All these persisters are assumed to use a
//! discriminator column to discriminate entity types in the hierarchy.
//!
//! `BasicEntityPersister` with a `PersisterKind` other than `Basic` runs
//! these overrides.
//!
//! Upstream: doctrine/orm src/Persisters/Entity/AbstractEntityInheritancePersister.php

use std::sync::Arc;

use php_value::{Key, Object, PhpArray, Thrown, Value};
use php2rust_doctrine_dbal::types;

use crate::entity_manager::EntityManager;
use crate::mapping::ClassMetadata;

use super::basic_entity_persister::{BasicEntityPersister, PersisterKind};

impl BasicEntityPersister {
    /// `prepareInsertData()`: the parent's data, with the discriminator
    /// column populated.
    pub(super) fn inheritance_prepare_insert_data(
        &self,
        em: &Arc<EntityManager>,
        entity: &Object,
    ) -> Result<PhpArray, Thrown> {
        let mut data = self.prepare_update_data(em, entity, true)?;

        // Populate the discriminator column
        let (discr_name, discr_type, discr_value) = {
            let info = self.class.read();
            let discr = info.get_discriminator_column()?;
            (
                discr.name.clone(),
                discr.r#type.clone(),
                info.discriminator_value.clone(),
            )
        };
        self.st()
            .column_types
            .insert(discr_name.clone(), discr_type);
        let table = Key::from(self.get_discriminator_column_table_name(em)?.as_str());
        let mut table_data = match data.get(&table) {
            Some(Value::Array(table_data)) => table_data.clone(),
            _ => PhpArray::new(),
        };
        table_data.insert(discr_name.as_str(), discr_value);
        data.insert(table, Value::Array(table_data));
        Ok(data)
    }

    /// Gets the name of the table that contains the discriminator column.
    pub(super) fn get_discriminator_column_table_name(
        &self,
        em: &EntityManager,
    ) -> Result<String, Thrown> {
        match self.kind {
            PersisterKind::JoinedSubclass => {
                let (name, root) = {
                    let info = self.class.read();
                    (info.name.clone(), info.root_entity_name.clone())
                };
                if name == root {
                    self.class.read().get_table_name()
                } else {
                    em.get_class_metadata(&root)?.read().get_table_name()
                }
            }
            _ => self.class.read().get_table_name(),
        }
    }

    /// `AbstractEntityInheritancePersister::getSelectColumnSQL()`.
    pub(super) fn inheritance_get_select_column_sql(
        &self,
        em: &Arc<EntityManager>,
        field: &str,
        class: &ClassMetadata,
        alias: &str,
    ) -> Result<String, Thrown> {
        let platform = em.get_platform();
        let table_alias = if alias == "r" { "" } else { alias };
        let (class_name, column_name, ty, quoted) = {
            let info = class.read();
            let mapping = info.get_field_mapping(field)?;
            (
                info.name.clone(),
                mapping.column_name.clone(),
                mapping.r#type.clone(),
                self.quote_strategy.get_column_name(field, &info, platform),
            )
        };
        let sql = format!(
            "{}.{quoted}",
            self.get_sql_table_alias(&class_name, table_alias)
        );

        // Reuse the alias if the field was already selected before; the
        // SQL is regenerated when the filters change, and a second alias
        // for the same field breaks indexBy(). getColumnAliasByField() is
        // of no use here, as it misses fields inherited from a parent
        // class - the map is keyed by the declaring class.
        let known = {
            let mut st = self.st();
            st.ctx()
                .rsm
                .column_alias_mappings
                .get(&class_name)
                .and_then(|by_alias| by_alias.get(alias))
                .and_then(|fields| fields.get(field))
                .cloned()
        };
        let column_alias = match known {
            Some(column_alias) => column_alias,
            None => self.get_sql_column_alias(em, &column_name),
        };
        self.st()
            .ctx()
            .rsm
            .add_field_result(alias, &column_alias, field, Some(&class_name));

        let ty = types::get_type(&ty).map_err(|e| Thrown::new(e.class(), e.message(), 0))?;
        let sql = ty.convert_to_php_value_sql(sql.as_bytes(), platform);
        Ok(format!("{} AS {column_alias}", sql.to_string_lossy()))
    }

    /// `getSelectJoinColumnSQL()`.
    pub(super) fn get_select_join_column_sql(
        &self,
        em: &EntityManager,
        table_alias: &str,
        join_column_name: &str,
        quoted_column_name: &str,
        ty: &str,
    ) -> String {
        let column_alias = self.get_sql_column_alias(em, join_column_name);
        self.st()
            .ctx()
            .rsm
            .add_meta_result("r", &column_alias, join_column_name, false, Some(ty));
        format!("{table_alias}.{quoted_column_name} AS {column_alias}")
    }
}
