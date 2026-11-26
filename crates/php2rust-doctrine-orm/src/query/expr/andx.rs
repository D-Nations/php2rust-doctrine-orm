//! `Expr\Andx`.
//!
//! Upstream: doctrine/orm src/Query/Expr/Andx.php

use super::base::{BaseKind, base_class};

const COMPARISON: &str = "Doctrine\\ORM\\Query\\Expr\\Comparison";
const FUNC: &str = "Doctrine\\ORM\\Query\\Expr\\Func";
const ORX: &str = "Doctrine\\ORM\\Query\\Expr\\Orx";
const ANDX: &str = "Doctrine\\ORM\\Query\\Expr\\Andx";

const KIND: BaseKind = BaseKind {
    class: "Doctrine\\ORM\\Query\\Expr\\Andx",
    pre_separator: "(",
    separator: " AND ",
    post_separator: ")",
    allowed_classes: &[COMPARISON, FUNC, ORX, ANDX],
    composite: true,
};

base_class! {
    /// Expression class for building DQL and parts.
    Andx, KIND, parents: ["Doctrine\\ORM\\Query\\Expr\\Composite"], get_parts: true
}
