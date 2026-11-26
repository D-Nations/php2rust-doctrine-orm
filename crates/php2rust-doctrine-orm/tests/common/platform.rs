//! A test platform standing in for PHPUnit's `createStub(AbstractPlatform::class)`
//! and for the DBAL platforms the port does not have: it reports a class
//! name of the test's choice, can quote identifiers its own way, and
//! delegates the rest to `SQLitePlatform`.

#![allow(dead_code, reason = "each test uses a part of the helpers")]

use php_value::{PhpArray, PhpString};
use php2rust_doctrine_dbal::Exception;
use php2rust_doctrine_dbal::platforms::{AbstractPlatform, SQLitePlatform};

pub const POSTGRESQL: &str = "Doctrine\\DBAL\\Platforms\\PostgreSQLPlatform";
pub const ORACLE: &str = "Doctrine\\DBAL\\Platforms\\OraclePlatform";
pub const MYSQL: &str = "Doctrine\\DBAL\\Platforms\\MySQLPlatform";

pub struct TestPlatform {
    class: String,
    /// `quoteSingleIdentifier()`'s format, `%s` for the identifier.
    quote_format: Option<String>,
    max_identifier_length: Option<i64>,
    inner: SQLitePlatform,
}

impl TestPlatform {
    pub fn new(class: &str) -> Self {
        Self {
            class: class.to_owned(),
            quote_format: None,
            max_identifier_length: None,
            inner: SQLitePlatform::new(),
        }
    }

    #[must_use]
    pub fn quoting(mut self, format: &str) -> Self {
        self.quote_format = Some(format.to_owned());
        self
    }

    #[must_use]
    pub fn max_identifier_length(mut self, length: i64) -> Self {
        self.max_identifier_length = Some(length);
        self
    }
}

impl AbstractPlatform for TestPlatform {
    fn class(&self) -> &str {
        &self.class
    }

    fn quote_single_identifier(&self, str: &str) -> String {
        match &self.quote_format {
            Some(format) => format.replace("%s", str),
            None => self.inner.quote_single_identifier(str),
        }
    }

    fn get_max_identifier_length(&self) -> i64 {
        self.max_identifier_length
            .unwrap_or_else(|| self.inner.get_max_identifier_length())
    }

    fn get_boolean_type_declaration_sql(&self, column: &PhpArray) -> Result<PhpString, Exception> {
        self.inner.get_boolean_type_declaration_sql(column)
    }

    fn get_integer_type_declaration_sql(&self, column: &PhpArray) -> Result<PhpString, Exception> {
        self.inner.get_integer_type_declaration_sql(column)
    }

    fn get_big_int_type_declaration_sql(&self, column: &PhpArray) -> Result<PhpString, Exception> {
        self.inner.get_big_int_type_declaration_sql(column)
    }

    fn get_small_int_type_declaration_sql(
        &self,
        column: &PhpArray,
    ) -> Result<PhpString, Exception> {
        self.inner.get_small_int_type_declaration_sql(column)
    }

    fn get_clob_type_declaration_sql(&self, column: &PhpArray) -> Result<PhpString, Exception> {
        self.inner.get_clob_type_declaration_sql(column)
    }

    fn get_blob_type_declaration_sql(&self, column: &PhpArray) -> Result<PhpString, Exception> {
        self.inner.get_blob_type_declaration_sql(column)
    }

    fn get_date_time_type_declaration_sql(
        &self,
        column: &PhpArray,
    ) -> Result<PhpString, Exception> {
        self.inner.get_date_time_type_declaration_sql(column)
    }

    fn get_date_type_declaration_sql(&self, column: &PhpArray) -> Result<PhpString, Exception> {
        self.inner.get_date_type_declaration_sql(column)
    }

    fn get_time_type_declaration_sql(&self, column: &PhpArray) -> Result<PhpString, Exception> {
        self.inner.get_time_type_declaration_sql(column)
    }
}
