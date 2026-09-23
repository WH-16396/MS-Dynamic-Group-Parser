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
    pub fn as_str(&self) -> &'static str {
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
    pub fn as_multiple(&self) -> Self {
        match self {
            Self::Equals =>             Self::In,
            Self::NotEquals =>          Self::NotIn,
            &s => s,
        }
    }
    pub fn as_single(&self) -> Self {
        match self {
            Self::In =>                 Self::Equals,
            Self::NotIn =>              Self::NotEquals,
            &s => s,
        }
    }
}

// use std::str::FromStr;

// #[derive(Debug, PartialEq, Eq)]
// pub struct TestError;

// impl FromStr for Operator {
//     type Err = TestError;

//     fn from_str(s: &str) -> Result<Self, Self::Err> {
//         match s {
//             " " => {}
//         }
//         ; Ok(Self::In)
//     }
// }

impl serde::ser::Serialize for Operator {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}