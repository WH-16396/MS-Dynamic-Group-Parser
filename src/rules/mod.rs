pub mod tidy;
pub mod checks;
pub mod reconstruct;

use serde::Serialize;

use crate::condition::{
    Condition, 
    operator::Operator, 
    value::Value
};

//
// NODE IS A CONTAINER FOR CONDITIONS AND WARNINGS
//
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Node<'a> {
    pub condition: Condition<'a>,
    pub warnings: Vec<&'a Warning>,
}
impl<'a> From<Condition<'a>> for Node<'a> {
    fn from(item: Condition<'a>) -> Self {
        Node {
            condition: item,
            warnings: Vec::new(),
        }
    } 
}

//
// SYNTAX STRUCTURE
//

// These make up the data structure of the 'Rules' system 

pub type Dnf<'a> = Vec<Rule<'a>>;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Rule<'a> {
    pub nodes: Vec<Node<'a>>, 
    pub warnings: Vec<Warning>
}
impl<'a> Rule<'a> {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(), 
            warnings: Vec::new()
        }
    }
    pub fn into_string(self) -> String {
        self.nodes.into_iter()
            .map(|x| x.condition.to_string())
            .collect::<Vec<String>>()
            .join(" ")
    }
}



//
// WARNINGS
//
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq, Serialize)]
pub enum Warning {
    NotEnabled,
    NotMember,
    MissingDept,
}
impl Warning {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NotEnabled => "notEnabled",
            Self::NotMember => "notMember",
            Self::MissingDept => "missingDept",
        }
    }
    pub fn message(&self) -> String {
        match self {
            Self::NotEnabled => String::from("'user.accountEnabled -eq True' is \
             not universally applied to all rules, disabled accounts can still \
             match these rules."),
            Self::NotMember => String::from("'user.userType -eq \"Member\"' is \
             not universally applied to all rules, external accounts can still \
             match these rules."),
            Self::MissingDept => String::from("'user.jobTitle' rule is missing a \
             'user.department' condition, unintended users might match these rules."),
        }
    }
}




