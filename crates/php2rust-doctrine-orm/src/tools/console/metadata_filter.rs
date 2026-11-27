//! Port of `Doctrine\ORM\Tools\Console\MetadataFilter`: the class
//! metadata whose class name matches one of the filters, each a regex
//! body (`/<filter>/`). The filter iterator itself is not ported; its
//! `filter()` keeps the order of the metadata and drops the keys
//! (`iterator_to_array()` of a list).
//!
//! Upstream: doctrine/orm src/Tools/Console/MetadataFilter.php

use std::sync::Arc;

use php_regex::Pcre;
use php_value::Thrown;

use crate::mapping::ClassMetadata;

/// `MetadataFilter::filter($metadatas, $filter)`; a string filter is a
/// one-element slice (`(array) $filter`).
pub fn filter<S: AsRef<str>>(
    metadatas: Vec<Arc<ClassMetadata>>,
    filter: &[S],
) -> Result<Vec<Arc<ClassMetadata>>, Thrown> {
    let mut kept = Vec::with_capacity(metadatas.len());
    for metadata in metadatas {
        if accept(&metadata, filter)? {
            kept.push(metadata);
        }
    }
    Ok(kept)
}

/// `accept()`.
fn accept<S: AsRef<str>>(metadata: &ClassMetadata, filter: &[S]) -> Result<bool, Thrown> {
    if filter.is_empty() {
        return Ok(true);
    }

    let name = metadata.read().name.clone();
    for filter in filter {
        let filter = filter.as_ref();
        let error = || {
            Thrown::new(
                "RuntimeException",
                format!("Error while evaluating regex '/{filter}/'."),
                0,
            )
        };
        let pattern = Pcre::new(format!("/{filter}/").as_bytes()).map_err(|_| error())?;
        if pattern.is_match(name.as_bytes()).map_err(|_| error())? {
            return Ok(true);
        }
    }

    Ok(false)
}
