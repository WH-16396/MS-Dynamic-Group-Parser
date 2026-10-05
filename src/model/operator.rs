/// Comparison operator between a property and its value.
///
/// See https://learn.microsoft.com/en-gb/entra/identity/users/groups-dynamic-membership
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
    /// Parses an operator name case-insensitively, with or without the
    /// leading `-` (e.g. `eq`, `-eq`, `-EQ`).
    pub fn parse(name: &str) -> Option<Self> {
        let name = name.strip_prefix('-').unwrap_or(name);
        let operator = match name.to_ascii_lowercase().as_str() {
            "plus" =>          Self::Add,
            "all" =>           Self::All,
            "any" =>           Self::Any,
            "contains" =>      Self::Contains,
            "endswith" =>      Self::EndsWith,
            "eq" =>            Self::Equals,
            "ge" =>            Self::GreaterThanOrEqual,
            "in" =>            Self::In,
            "le" =>            Self::LessThanOrEqual,
            "match" =>         Self::Match,
            "notcontains" =>   Self::NotContains,
            "notendswith" =>   Self::NotEndsWith,
            "ne" =>            Self::NotEquals,
            "notin" =>         Self::NotIn,
            "notmatch" =>      Self::NotMatch,
            "notstartswith" => Self::NotStartsWith,
            "startswith" =>    Self::StartsWith,
            "minus" =>         Self::Subtract,
            _ => return None,
        };
        Some(operator)
    }

    /// The operator as written in rule syntax.
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

    /// The single-value form of an array operator (`-in` -> `-eq`), used when
    /// an array condition is split into one condition per value.
    pub fn as_single(&self) -> Self {
        match self {
            Self::In =>    Self::Equals,
            Self::NotIn => Self::NotEquals,
            &other => other,
        }
    }

    /// The array form of a single-value operator (`-eq` -> `-in`), used when
    /// sibling conditions are folded back into one.
    pub fn as_multiple(&self) -> Self {
        match self {
            Self::Equals =>    Self::In,
            Self::NotEquals => Self::NotIn,
            &other => other,
        }
    }

    /// False for operators that take an array of values.
    pub fn is_single(&self) -> bool {
        !matches!(self, Self::In | Self::NotIn)
    }
}

/// Serialises as the rule syntax form, e.g. `"-eq"`.
impl serde::Serialize for Operator {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}
