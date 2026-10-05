use serde::Serialize;

use crate::model::{Condition, ConditionPattern};

/// A condition plus any warnings raised against it by `process::checks`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Node<'src> {
    pub condition: Condition<'src>,
    pub warnings: Vec<Warning>,
}

impl<'src> From<Condition<'src>> for Node<'src> {
    fn from(condition: Condition<'src>) -> Self {
        Node { condition, warnings: Vec::new() }
    }
}

/// One branch of access: a user matches the rule when they match *every*
/// node in it (the nodes are implicitly AND-ed together).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Rule<'src> {
    pub nodes: Vec<Node<'src>>,
    pub warnings: Vec<Warning>,
}

impl<'src> Rule<'src> {
    pub fn new() -> Self {
        Self { nodes: Vec::new(), warnings: Vec::new() }
    }

    /// True when any node in the rule matches `pattern`.
    pub fn contains(&self, pattern: &ConditionPattern) -> bool {
        self.nodes.iter().any(|node| node.condition.matches(pattern))
    }
}

/// Disjunctive normal form: a user is a member when they match *any* rule.
///
/// Every input, however it was bracketed, is flattened into this shape so
/// that rules can be compared, merged and removed independently of how they
/// were originally written. `process::reconstruct` turns it back into a tree.
pub type Dnf<'src> = Vec<Rule<'src>>;

/// A risk flagged by `process::checks`.
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

    pub fn message(&self) -> &'static str {
        match self {
            Self::NotEnabled => "'user.accountEnabled -eq True' is not universally \
                applied to all rules, disabled accounts can still match these rules.",
            Self::NotMember => "'user.userType -eq \"Member\"' is not universally \
                applied to all rules, external accounts can still match these rules.",
            Self::MissingDept => "'user.jobTitle' rule is missing a 'user.department' \
                condition, unintended users might match these rules.",
        }
    }
}
