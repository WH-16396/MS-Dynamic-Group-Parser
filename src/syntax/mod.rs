pub mod parser;
pub mod parser_remove_temp;
pub mod process_rules;
pub mod checks;
pub mod reconstruct;
// pub mod syntax;
// pub mod tree;
// pub mod structure;
use serde::Serialize;



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



//
// OPERATOR FOR CONDTIION TO MATCH THE VALUE
//
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub enum Operator {
    Add,
    All,
    Any,
    Contains,
    EndsWith,
    Equals,
    GreaterThanOrEqual,
    In,
    LessThanOrEqual,
    Match,
    NotContains,
    NotEndsWith,
    NotEquals,
    NotIn,
    NotMatch,
    NotStartsWith,
    StartsWith,
    Subtract,
}
impl Operator {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Add =>                "-plus",
            Self::All =>                "-all",
            Self::Any =>                "-any",
            Self::Contains =>           "-contains",
            Self::EndsWith =>           "-endsWith",
            Self::Equals =>             "-eq",
            Self::GreaterThanOrEqual => "-ge",
            Self::In =>                 "-in",
            Self::LessThanOrEqual =>    "-le",
            Self::Match =>              "-match",
            Self::NotContains =>        "-notContains",
            Self::NotEndsWith =>        "-notEndsWith",
            Self::NotEquals =>          "-ne",
            Self::NotIn =>              "-notIn",
            Self::NotMatch =>           "-notMatch",
            Self::NotStartsWith =>      "-notStartsWith",
            Self::StartsWith =>         "-startsWith",
            Self::Subtract =>           "-minus",
        }
    }
    fn as_single(&self) -> Self {
        match self {
            Self::In =>                 Self::Equals,
            Self::NotIn =>              Self::NotEquals,
            &s => s,
        }
    }
}
impl serde::ser::Serialize for Operator {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}




//
// VALUE FOR CONDTIION TO MEET
//
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub enum Value<'a> {
    String(&'a str),
    Boolean(bool),
    Number(i32),
    Null,
}
impl<'a> Value<'a> {
    pub fn to_string(&self) -> String {
        match self {
            Self::String(s) => format!("\"{}\"", s),
            Self::Boolean(b) => b.to_string(),
            Self::Number(n) => n.to_string(),
            Self::Null => String::from("Null"),
        }
    }
    pub fn get_type(&self) -> &'a str {
        match self {
            Self::String(_) => "string",
            Self::Boolean(_) => "boolean",
            Self::Number(_) => "number",
            Self::Null => "null",
        }
    }
}
impl serde::ser::Serialize for Value<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("Value", 3)?;
        s.serialize_field("type", &self.get_type())?;
        match self {
            Self::Boolean(o) => s.serialize_field("value", o)?,
            Self::String(o) => s.serialize_field("value", o)?,
            Self::Number(o) => s.serialize_field("value", o)?,
            Self::Null => s.serialize_field("value", "null")?,
        }
        s.end()
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
    MissingDept,
}
impl Warning {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NotEnabled => "notEnabled",
            Self::MissingDept => "missingDept",
        }
    }
    pub fn message(&self) -> String {
        match self {
            Self::NotEnabled => String::from("'user.accountEnabled -eq True' is \
             not universally applied to all rules, disabled accounts can still \
             match these rules."),
            Self::MissingDept => String::from("'user.jobTitle' rule is missing a \
             'user.department' condition, unintended users might match these rules."),
        }
    }
}