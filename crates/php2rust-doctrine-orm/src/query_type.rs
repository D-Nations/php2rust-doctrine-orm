//! `QueryType`: the kind of DQL statement a `QueryBuilder` builds.
//!
//! Upstream: doctrine/orm src/QueryType.php

/// `enum QueryType { case Select; case Delete; case Update; }`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QueryType {
    Select,
    Delete,
    Update,
}

impl QueryType {
    pub const CLASS: &'static str = "Doctrine\\ORM\\QueryType";
}
