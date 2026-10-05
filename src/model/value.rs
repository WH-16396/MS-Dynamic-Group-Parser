//
// VALUE FOR CONDTIION TO MEET
//
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub enum Value<'a> {
    String(&'a str),
    Boolean(bool),
    Number(i64),
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