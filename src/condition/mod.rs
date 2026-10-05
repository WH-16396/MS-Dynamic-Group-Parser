pub mod property;
pub mod operator;
pub mod value;

use serde::Serialize;
use crate::{
    routes::api_v2::*,
    condition::{
        operator::Operator, 
        value::Value
    }
};

#[derive(Debug, Clone, Copy, Serialize, Eq)]
pub struct Property<'a>(pub &'a str);
impl<'a> PartialEq for Property<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq_ignore_ascii_case(other.0)
    }
}
use std::hash::{Hash, Hasher};
impl<'a> Hash for Property<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.as_bytes().to_ascii_lowercase().hash(state)
    }
}

pub fn validate_property<'j>(property: Property<'j>) -> Option<Property<'j>> {
    let cmp = property.0.to_lowercase();
    if cmp.starts_with("user.") || cmp.starts_with("device.") {
        Some(property)
    } else {
        None
    }
}

//
// A SINGLE CONDITION FOR GROUP ACCESS
//
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq, Serialize)]
pub struct Condition<'a> {
    pub property: Property<'a>,
    // pub operator: &'a str,
    pub operator: Operator,
    #[serde(rename = "item")]
    pub value: Value<'a>,
}
impl<'a> Condition<'a> {
    pub fn to_string(self) -> String {
        format!("{} {} {}", self.property.0, self.operator.as_str(), self.value.to_string())
    } 
}

#[derive(Debug, Clone, Eq, Hash)]
pub struct ConditionPart<'a> {
    pub property: Option<Property<'a>>,
    pub operator: Option<Operator>,
    pub value: Option<Value<'a>>,
}
impl<'a> From<Condition<'a>> for ConditionPart<'a> {
    fn from(item: Condition<'a>) -> Self {
        Self {
            property: Some(item.property),
            operator: Some(item.operator),
            value: Some(item.value),
        }
    } 
}
impl<'a> PartialEq for ConditionPart<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.property == other.property &&
        self.operator == other.operator &&
        self.value == other.value
    }
} 