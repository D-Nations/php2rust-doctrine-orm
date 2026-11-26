//! `SortDirection`, the enum PHP 8.6 adds and symfony/polyfill-php86
//! declares for older versions; the ORM's order-by expressions take it.
//!
//! Upstream: none

/// `enum SortDirection { case Ascending; case Descending; }`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SortDirection {
    Ascending,
    Descending,
}

impl SortDirection {
    pub const CLASS: &'static str = "SortDirection";
    pub const CASES: [Self; 2] = [Self::Ascending, Self::Descending];

    /// The case's name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Ascending => "Ascending",
            Self::Descending => "Descending",
        }
    }
}
