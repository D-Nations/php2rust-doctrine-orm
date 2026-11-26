//! PHP 8.6's `\SortDirection` enum, which doctrine/collections 3.1 takes
//! from symfony/polyfill-php86 (a pure enum: `Ascending`, `Descending`).
//!
//! Upstream: none

/// `\SortDirection`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SortDirection {
    Ascending,
    Descending,
}

impl SortDirection {
    /// The enum's class name.
    pub const CLASS: &'static str = "SortDirection";

    /// The case name (`->name`).
    pub fn name(self) -> &'static str {
        match self {
            Self::Ascending => "Ascending",
            Self::Descending => "Descending",
        }
    }
}
