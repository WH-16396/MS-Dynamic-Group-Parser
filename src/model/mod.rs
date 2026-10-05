//! Core data types shared by every stage of the pipeline.
//!
//! The lifetime `'src` used throughout is the lifetime of the request body
//! the rules were parsed from - properties and string values borrow straight
//! out of it rather than being copied.

pub mod condition;
pub mod operator;
pub mod rule;
pub mod value;

pub use condition::{Condition, ConditionPattern, Property};
pub use operator::Operator;
pub use rule::{Dnf, Node, Rule, Warning};
pub use value::Value;
