//! Upstream: doctrine/inflector src/WordInflector.php

use php_value::PhpString;

/// `Doctrine\Inflector\WordInflector`.
pub trait WordInflector: Send + Sync {
    fn inflect(&self, word: &[u8]) -> PhpString;
}
