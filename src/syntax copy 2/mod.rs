pub mod parser;
pub mod process_rules;
pub mod checks;
pub mod reconstruct;
// pub mod syntax_blocks;
// pub mod tree;
// pub mod structure;

use serde::Serialize;

// Struct for a rule
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Node<'a> {
    pub condition: Condition<'a>,
    pub warnings: Vec<&'a Warning>,
}
// impl<'a> Node<'a> {
//     pub fn from(p: &'a str, o: &'a str, v: Value<'a>) -> Node<'a> {
//         Node {
//             condition: Condition { property: p, operator: o, value: v },
//             warnings: Vec::new(),
//         }
//     } 
// }
impl<'a> From<Condition<'a>> for Node<'a> {
    fn from(item: Condition<'a>) -> Self {
        Node {
            condition: item,
            warnings: Vec::new(),
        }
    } 
}
impl<'a> From<(&'a str, &'a str, Value<'a>)> for Node<'a> {
    fn from(item: (&'a str, &'a str, Value<'a>)) -> Self {
        Node {
            condition: Condition { property: item.0, operator: item.1, value: item.2 },
            warnings: Vec::new(),
        }
    } 
}

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq, Serialize)]
pub struct Condition<'a> {
    pub property: &'a str,
    pub operator: &'a str,
    #[serde(rename = "item")]
    pub value: Value<'a>,
}
impl<'a> Condition<'a> {
    pub fn to_string(self) -> String {
        format!("{} {} {}", self.property, self.operator, self.value.to_string())
    } 
}

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub enum Value<'a> {
    String(&'a str),
    Boolean(bool),
    Number(i32),
    // Array(Vec<Value<'a>>),
}
impl<'a> Value<'a> {
    pub fn to_string(&self) -> String {
        match self {
            Self::String(s) => format!("{}", s),
            Self::Boolean(b) => b.to_string(),
            Self::Number(n) => n.to_string(),
            // Self::Array(a) => format!("[\"{}\"]", a.into_iter()
            //     .map(|x| x.to_string())
            //     .collect::<Vec<String>>()
            //     .join("\", \"")),
        }
    }
    pub fn get_type(&self) -> &'a str {
        match self {
            Self::String(_) => "string",
            Self::Boolean(_) => "boolean",
            Self::Number(_) => "number",
            // Self::Array(_) => "array",
        }
    }
    // pub fn into_inner<T>(&self) -> <T> {
        
    // } 
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
            // Self::Array(o) => s.serialize_field("value", o)?,
            Self::Boolean(o) => s.serialize_field("value", o)?,
            Self::String(o) => s.serialize_field("value", o)?,
            Self::Number(o) => s.serialize_field("value", o)?,
        }
        s.end()
    }
}



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