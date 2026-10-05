use serde::{Serializer, ser::SerializeStruct};
use std::fmt;

/// The right-hand side of a condition.
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub enum Value<'src> {
    String(&'src str),
    Boolean(bool),
    Number(i64),
    Null,
}

impl Value<'_> {
    /// Type tag used in the JSON output.
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::String(_) => "string",
            Self::Boolean(_) => "boolean",
            Self::Number(_) => "number",
            Self::Null => "null",
        }
    }
}

/// Renders the value as rule syntax, quoting strings.
impl fmt::Display for Value<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::String(string) => write!(f, "\"{}\"", string),
            Self::Boolean(boolean) => write!(f, "{}", boolean),
            Self::Number(number) => write!(f, "{}", number),
            Self::Null => write!(f, "Null"),
        }
    }
}

/// Serialises as `{"type": "<type_name>", "value": <value>}`.
impl serde::Serialize for Value<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut out = serializer.serialize_struct("Value", 2)?;
        out.serialize_field("type", self.type_name())?;
        match self {
            Self::Boolean(boolean) => out.serialize_field("value", boolean)?,
            Self::String(string) => out.serialize_field("value", string)?,
            Self::Number(number) => out.serialize_field("value", number)?,
            Self::Null => out.serialize_field("value", "null")?,
        }
        out.end()
    }
}
