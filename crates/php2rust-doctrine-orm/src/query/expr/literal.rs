//! `Expr\Literal`.
//!
//! Upstream: doctrine/orm src/Query/Expr/Literal.php

use super::base::{BaseKind, base_class};

const KIND: BaseKind = BaseKind {
    class: "Doctrine\\ORM\\Query\\Expr\\Literal",
    pre_separator: "",
    separator: ", ",
    post_separator: "",
    allowed_classes: &[],
    composite: false,
};

base_class! {
    /// Expression class for generating DQL functions.
    Literal, KIND, parents: [], get_parts: true
}
