//! Upstream: doctrine/collections src/Order.php

use php_value::{PhpString, Thrown};

/// `Order`, a string-backed enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Order {
    Ascending,
    Descending,
}

impl Order {
    /// The enum's class name.
    pub const CLASS: &'static str = "Doctrine\\Common\\Collections\\Order";

    /// Every case, in declaration order (`Order::cases()`).
    pub const CASES: [Self; 2] = [Self::Ascending, Self::Descending];

    /// The case name (`->name`).
    pub fn name(self) -> &'static str {
        match self {
            Self::Ascending => "Ascending",
            Self::Descending => "Descending",
        }
    }

    /// The backing value (`->value`): `ASC` or `DESC`.
    pub fn value(self) -> &'static str {
        match self {
            Self::Ascending => "ASC",
            Self::Descending => "DESC",
        }
    }

    /// `Order::tryFrom($value)`.
    pub fn try_from_value(value: &[u8]) -> Option<Self> {
        Self::CASES
            .into_iter()
            .find(|case| case.value().as_bytes() == value)
    }

    /// `Order::from($value)`: a `ValueError` for any other value.
    pub fn from_value(value: &[u8]) -> Result<Self, Thrown> {
        Self::try_from_value(value).ok_or_else(|| {
            let mut message = b"\"".to_vec();
            message.extend_from_slice(value);
            message.extend_from_slice(b"\" is not a valid backing value for enum ");
            message.extend_from_slice(Self::CLASS.as_bytes());
            Thrown::new("ValueError", PhpString::from(message), 0)
        })
    }
}
