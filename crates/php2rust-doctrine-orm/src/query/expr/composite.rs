//! `Expr\Composite`.
//!
//! Upstream: doctrine/orm src/Query/Expr/Composite.php

use super::base::{BaseKind, base_class};

const KIND: BaseKind = BaseKind {
    class: "Doctrine\\ORM\\Query\\Expr\\Composite",
    pre_separator: "(",
    separator: ", ",
    post_separator: ")",
    allowed_classes: &[],
    composite: true,
};

base_class! {
    /// Expression class for building DQL and parts.
    Composite, KIND, parents: [], get_parts: false
}
