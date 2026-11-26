//! `Expr\GroupBy`.
//!
//! Upstream: doctrine/orm src/Query/Expr/GroupBy.php

use super::base::{BaseKind, base_class};

const KIND: BaseKind = BaseKind {
    class: "Doctrine\\ORM\\Query\\Expr\\GroupBy",
    pre_separator: "",
    separator: ", ",
    post_separator: "",
    allowed_classes: &[],
    composite: false,
};

base_class! {
    /// Expression class for building DQL Group By parts.
    GroupBy, KIND, parents: [], get_parts: true
}
