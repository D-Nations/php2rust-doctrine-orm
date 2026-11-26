//! Upstream: doctrine/inflector src/CachedWordInflector.php

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use php_value::PhpString;

use crate::word_inflector::WordInflector;

/// `Doctrine\Inflector\CachedWordInflector`: each word inflected once.
pub struct CachedWordInflector {
    word_inflector: Arc<dyn WordInflector>,
    cache: Mutex<HashMap<Vec<u8>, PhpString>>,
}

impl CachedWordInflector {
    pub fn new(word_inflector: Arc<dyn WordInflector>) -> Self {
        Self {
            word_inflector,
            cache: Mutex::new(HashMap::new()),
        }
    }
}

impl WordInflector for CachedWordInflector {
    fn inflect(&self, word: &[u8]) -> PhpString {
        let cached = self
            .cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(word)
            .cloned();
        if let Some(inflected) = cached {
            return inflected;
        }
        let inflected = self.word_inflector.inflect(word);
        self.cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(word.to_vec(), inflected.clone());
        inflected
    }
}
