use serde::Serialize;
use std::{
    fmt,
    hash::{Hash, Hasher},
};

use crate::model::{Operator, Value};

/// An attribute path such as `user.department` or `device.deviceOSType`.
///
/// Entra treats property names case-insensitively, so equality and hashing
/// both ignore ASCII case.
#[derive(Debug, Clone, Copy, Serialize, Eq)]
pub struct Property<'src>(pub &'src str);

impl<'src> Property<'src> {
    /// Returns the property if it targets a supported object (`user.` or
    /// `device.`), otherwise `None`.
    pub fn validated(name: &'src str) -> Option<Self> {
        let lower = name.to_lowercase();
        if lower.starts_with("user.") || lower.starts_with("device.") {
            Some(Self(name))
        } else {
            None
        }
    }
}

impl PartialEq for Property<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq_ignore_ascii_case(other.0)
    }
}

impl Hash for Property<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.as_bytes().to_ascii_lowercase().hash(state)
    }
}

/// A single test against one property, e.g. `user.department -eq "Sales"`.
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq, Serialize)]
pub struct Condition<'src> {
    pub property: Property<'src>,
    pub operator: Operator,
    #[serde(rename = "item")]
    pub value: Value<'src>,
}

impl Condition<'_> {
    /// True when every non-wildcard part of `pattern` equals this condition.
    pub fn matches(&self, pattern: &ConditionPattern) -> bool {
        pattern.property.is_none_or(|property| property == self.property)
            && pattern.operator.is_none_or(|operator| operator == self.operator)
            && pattern.value.is_none_or(|value| value == self.value)
    }
}

impl fmt::Display for Condition<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} {}", self.property.0, self.operator.as_str(), self.value)
    }
}

/// A `Condition` where any part may be a wildcard (`None`), used to find or
/// remove conditions rather than to describe one.
#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct ConditionPattern<'src> {
    pub property: Option<Property<'src>>,
    pub operator: Option<Operator>,
    pub value: Option<Value<'src>>,
}

impl ConditionPattern<'static> {
    /// Matches every condition.
    pub const ANY: Self = Self { property: None, operator: None, value: None };
}

/// A fully specified condition is a pattern with no wildcards.
impl<'src> From<Condition<'src>> for ConditionPattern<'src> {
    fn from(condition: Condition<'src>) -> Self {
        Self {
            property: Some(condition.property),
            operator: Some(condition.operator),
            value: Some(condition.value),
        }
    }
}
