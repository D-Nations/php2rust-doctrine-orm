//! A test platform standing in for PHPUnit's `createStub(AbstractPlatform::class)`:
//! it reports a class name of the test's choice, can quote identifiers its
//! own way and answer `supportsSequences()`, and delegates the rest to
//! `SQLitePlatform`.
//!
//! Upstream: none

#![allow(dead_code, reason = "each test uses a part of the helpers")]

use php_value::{PhpArray, PhpString};
use php2rust_doctrine_dbal::Exception;
use php2rust_doctrine_dbal::platforms::{AbstractPlatform, SQLitePlatform};

/// The class of a `createStub(AbstractPlatform::class)`.
pub const ABSTRACT_PLATFORM: &str = "Doctrine\\DBAL\\Platforms\\AbstractPlatform";

pub struct TestPlatform {
    class: String,
    /// `quoteSingleIdentifier()`'s format, `%s` for the identifier.
    quote_format: Option<String>,
    max_identifier_length: Option<i64>,
    supports_sequences: Option<bool>,
    sequence_next_val_sql: Option<String>,
    inner: SQLitePlatform,
}

impl TestPlatform {
    pub fn new(class: &str) -> Self {
        Self {
            class: class.to_owned(),
            quote_format: None,
            max_identifier_length: None,
            supports_sequences: None,
            sequence_next_val_sql: None,
            inner: SQLitePlatform::new(),
        }
    }

    #[must_use]
    pub fn quoting(mut self, format: &str) -> Self {
        self.quote_format = Some(format.to_owned());
        self
    }

    /// `$platform->method('supportsSequences')->willReturn($supports)`.
    #[must_use]
    pub fn supporting_sequences(mut self, supports: bool) -> Self {
        self.supports_sequences = Some(supports);
        self
    }

    /// `$platform->method('getSequenceNextValSQL')->willReturn($sql)`.
    #[must_use]
    pub fn sequence_next_val_sql(mut self, sql: &str) -> Self {
        self.sequence_next_val_sql = Some(sql.to_owned());
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

    fn supports_sequences(&self) -> bool {
        self.supports_sequences
            .unwrap_or_else(|| self.inner.supports_sequences())
    }

    fn get_sequence_next_val_sql(&self, sequence: &str) -> Result<String, Exception> {
        match &self.sequence_next_val_sql {
            Some(sql) => Ok(sql.clone()),
            None => self.inner.get_sequence_next_val_sql(sequence),
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
