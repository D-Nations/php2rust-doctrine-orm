//! `Expr\Select`.
//!
//! Upstream: doctrine/orm src/Query/Expr/Select.php

use super::base::{BaseKind, base_class};

const FUNC: &str = "Doctrine\\ORM\\Query\\Expr\\Func";

const KIND: BaseKind = BaseKind {
    class: "Doctrine\\ORM\\Query\\Expr\\Select",
    pre_separator: "",
    separator: ", ",
    post_separator: "",
    allowed_classes: &[FUNC],
    composite: false,
};

base_class! {
    /// Expression class for building DQL select statements.
    Select, KIND, parents: [], get_parts: true
}
