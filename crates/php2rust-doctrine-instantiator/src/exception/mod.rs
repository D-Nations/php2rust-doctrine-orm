//! The instantiator's exceptions, and the error `instantiate()` returns.
//!
//! Upstream: none

mod exception_interface;
mod invalid_argument_exception;
mod unexpected_value_exception;

use std::fmt;

use php_value::Thrown;

pub use exception_interface::ExceptionInterface;
pub use invalid_argument_exception::InvalidArgumentException;
pub use unexpected_value_exception::UnexpectedValueException;

/// What `instantiate()` throws: one of the instantiator's exceptions, or
/// what PHP itself threw (an `Error` or `ReflectionException`, or an
/// exception a cached un-serialization factory raised later).
#[derive(Debug, Clone, PartialEq)]
pub enum InstantiatorError {
    InvalidArgument(InvalidArgumentException),
    UnexpectedValue(UnexpectedValueException),
    Thrown(Thrown),
}

impl InstantiatorError {
    /// The thrown object as `php_value` carries it (class, message, code).
    pub fn to_thrown(&self) -> Thrown {
        match self {
            Self::InvalidArgument(e) => e.to_thrown(),
            Self::UnexpectedValue(e) => e.to_thrown(),
            Self::Thrown(t) => t.clone(),
        }
    }

    /// `$e instanceof ExceptionInterface`.
    pub fn as_exception_interface(&self) -> Option<&dyn ExceptionInterface> {
        match self {
            Self::InvalidArgument(e) => Some(e),
            Self::UnexpectedValue(e) => Some(e),
            Self::Thrown(_) => None,
        }
    }
}

impl fmt::Display for InstantiatorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidArgument(e) => e.fmt(f),
            Self::UnexpectedValue(e) => e.fmt(f),
            Self::Thrown(t) => f.write_str(&String::from_utf8_lossy(t.message.as_bytes())),
        }
    }
}

impl std::error::Error for InstantiatorError {}

impl From<InvalidArgumentException> for InstantiatorError {
    fn from(e: InvalidArgumentException) -> Self {
        Self::InvalidArgument(e)
    }
}

impl From<UnexpectedValueException> for InstantiatorError {
    fn from(e: UnexpectedValueException) -> Self {
        Self::UnexpectedValue(e)
    }
}

impl From<Thrown> for InstantiatorError {
    fn from(t: Thrown) -> Self {
        Self::Thrown(t)
    }
}
