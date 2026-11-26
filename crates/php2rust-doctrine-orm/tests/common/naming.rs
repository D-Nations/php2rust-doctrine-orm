//! The naming strategies of the tests.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Mapping/NamingStrategy/CustomPascalNamingStrategy.php tests/Tests/ORM/Mapping/NamingStrategy/JoinColumnClassNamingStrategy.php

use php2rust_doctrine_orm::mapping::{DefaultNamingStrategy, NamingStrategy};

fn short_name(class_name: &str) -> &str {
    class_name
        .rfind('\\')
        .map_or(class_name, |at| &class_name[at + 1..])
}

/// `ucfirst()`.
fn ucfirst(s: &str) -> String {
    let mut chars = s.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_ascii_uppercase().to_string() + chars.as_str()
    })
}

/// `CustomPascalNamingStrategy`: a naming strategy implementing the
/// pascal case.
#[derive(Debug, Default)]
pub struct CustomPascalNamingStrategy;

impl NamingStrategy for CustomPascalNamingStrategy {
    fn class_to_table_name(&self, class_name: &str) -> String {
        short_name(class_name).to_owned()
    }

    fn property_to_column_name(&self, property_name: &str, class_name: &str) -> String {
        if property_name.to_ascii_lowercase()
            == self.class_to_table_name(class_name).to_ascii_lowercase() + "id"
        {
            return "Id".to_owned();
        }
        ucfirst(property_name)
    }

    fn embedded_field_to_column_name(&self, _: &str, _: &str, _: &str, _: &str) -> String {
        // PHP throws a LogicException ('Method %s is not implemented'); the
        // tests never call it.
        String::new()
    }

    fn reference_column_name(&self) -> String {
        "Id".to_owned()
    }

    fn join_column_name(&self, property_name: &str, _class_name: &str) -> String {
        ucfirst(property_name) + &self.reference_column_name()
    }

    fn join_table_name(&self, source_entity: &str, target_entity: &str, _: &str) -> String {
        self.class_to_table_name(source_entity) + &self.class_to_table_name(target_entity)
    }

    fn join_key_column_name(
        &self,
        entity_name: &str,
        referenced_column_name: Option<&str>,
    ) -> String {
        let referenced = match referenced_column_name {
            Some(name) if !name.is_empty() && name != "0" => name.to_owned(),
            _ => self.reference_column_name(),
        };
        self.class_to_table_name(entity_name) + &referenced
    }
}

/// `JoinColumnClassNamingStrategy`: the class name in the join column.
#[derive(Debug, Default)]
pub struct JoinColumnClassNamingStrategy(DefaultNamingStrategy);

impl NamingStrategy for JoinColumnClassNamingStrategy {
    fn class_to_table_name(&self, class_name: &str) -> String {
        self.0.class_to_table_name(class_name)
    }

    fn property_to_column_name(&self, property_name: &str, class_name: &str) -> String {
        self.0.property_to_column_name(property_name, class_name)
    }

    fn embedded_field_to_column_name(&self, a: &str, b: &str, c: &str, d: &str) -> String {
        self.0.embedded_field_to_column_name(a, b, c, d)
    }

    fn reference_column_name(&self) -> String {
        self.0.reference_column_name()
    }

    fn join_column_name(&self, property_name: &str, class_name: &str) -> String {
        format!(
            "{}_{property_name}_{}",
            self.class_to_table_name(class_name).to_ascii_lowercase(),
            self.reference_column_name()
        )
    }

    fn join_table_name(&self, a: &str, b: &str, c: &str) -> String {
        self.0.join_table_name(a, b, c)
    }

    fn join_key_column_name(&self, a: &str, b: Option<&str>) -> String {
        self.0.join_key_column_name(a, b)
    }
}
