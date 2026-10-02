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

//
// A SINGLE CONDITION FOR GROUP ACCESS
//
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq, Serialize)]
pub struct Condition<'a> {
    pub property: &'a str,
    // pub operator: &'a str,
    pub operator: Operator,
    #[serde(rename = "item")]
    pub value: Value<'a>,
}
impl<'a> Condition<'a> {
    pub fn to_string(self) -> String {
        format!("{} {} {}", self.property, self.operator.as_str(), self.value.to_string())
    } 
}

#[derive(Debug, Clone, Eq, Hash)]
pub struct ConditionPart<'a> {
    pub property: Option<&'a str>,
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
        (
            match self.property {
                Some(s) => match other.property {
                    Some(o) => s.eq_ignore_ascii_case(o),
                    None => true,
                },
                None => true,
            } &&
            self.operator == other.operator &&
            self.value == other.value
        )

    }
} 