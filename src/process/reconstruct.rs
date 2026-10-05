//! Rebuilds a minimal tree from a flat `Dnf`.
//!
//! Three passes:
//!
//! 1. `combine` - factors shared leading nodes out of rules into a tree of
//!    `Group`s, so `a and b` / `a and c` becomes `a and (b or c)`
//! 2. `compact` - folds sibling string equalities on the same property into
//!    one `-in` condition, so `x -eq "1" or x -eq "2"` becomes
//!    `x -in ["1", "2"]`
//! 3. `Block::from` - renders the tree as a flat token list with explicit
//!    `And` / `Or` separators for the JSON output and syntax string

use serde::{Serialize, Serializer, ser::SerializeStruct};
use std::fmt;

use crate::model::{Dnf, Node, Operator, Value, Warning};

/// Output tree: what the API serialises as `json`.
#[derive(Serialize)]
pub(crate) enum Block<'src> {
    Bracket(Vec<Block<'src>>),
    Node(Leaf<'src>),
    And,
    Or,
}

/// Intermediate tree used while combining and compacting.
enum Group<'src> {
    Leaf(Leaf<'src>),
    /// Every child must match
    All(Vec<Group<'src>>),
    /// Any child may match
    Any(Vec<Group<'src>>),
}

/// Mirrors `Node`, except that the value side can hold a folded set, which
/// `Condition` cannot - it is `Copy` and a `HashMap` key in `order_nodes`.
#[derive(Debug, Clone)]
pub(crate) struct Leaf<'src> {
    property: &'src str,
    operator: Operator,
    values: Vec<Value<'src>>,
    warnings: Vec<Warning>,
}

/// Builds the output tree, or `None` when `dnf` has no conditions to show.
pub(crate) fn reconstruct<'src>(dnf: &Dnf<'src>) -> Option<Block<'src>> {
    let branches: Vec<&[Node<'src>]> = dnf.iter().map(|rule| rule.nodes.as_slice()).collect();
    combine(&branches).map(compact).map(Block::from)
}



//
// COMBINING
//

/// Groups `branches` by their first node and recurses into the remainders.
///
/// Returns `None` if any branch is empty: an empty branch matches everyone,
/// so the siblings next to it add nothing.
fn combine<'src>(branches: &[&[Node<'src>]]) -> Option<Group<'src>> {
    // Each distinct first node, with the remainders of every branch that
    // started with it
    let mut heads: Vec<(Node<'src>, Vec<&[Node<'src>]>)> = Vec::new();
    let mut ends_here = false;

    for nodes in branches {
        match nodes.split_first() {
            None => ends_here = true,
            Some((head, tail)) => {
                match heads.iter_mut().find(|(node, _)| node.condition == head.condition) {
                    Some((node, tails)) => {
                        merge_warnings(&mut node.warnings, &head.warnings);
                        tails.push(tail);
                    }
                    None => heads.push((head.clone(), vec![tail])),
                }
            }
        }
    }

    if ends_here { return None }

    let mut children: Vec<Group<'src>> = heads
        .into_iter()
        .map(|(node, tails)| {
            let leaf = Group::Leaf(Leaf::from(node));
            match combine(&tails) {
                None => leaf,
                // Flatten 'a and (b and c)' into 'a and b and c'
                Some(Group::All(mut rest)) => {
                    let mut chain = vec![leaf];
                    chain.append(&mut rest);
                    Group::All(chain)
                }
                Some(rest) => Group::All(vec![leaf, rest]),
            }
        })
        .collect();

    match children.len() {
        0 => None,
        1 => children.pop(),
        _ => Some(Group::Any(children)),
    }
}

/// Adds any warnings from `from` that `into` doesn't already have.
fn merge_warnings(into: &mut Vec<Warning>, from: &[Warning]) {
    for warning in from {
        if !into.contains(warning) { into.push(*warning) }
    }
}



//
// COMPACTION
//

/// Folds compactable siblings at every `Any` level. `All` levels are prefix
/// chains rather than siblings, so they are only walked through.
fn compact<'src>(group: Group<'src>) -> Group<'src> {
    match group {
        Group::Leaf(_) => group,
        Group::All(children) => Group::All(children.into_iter().map(compact).collect()),
        Group::Any(children) => fold_siblings(children),
    }
}

fn fold_siblings<'src>(children: Vec<Group<'src>>) -> Group<'src> {
    let mut out: Vec<Group<'src>> = Vec::with_capacity(children.len());

    for child in children {
        // Only a bare terminal leaf folds - a node with anything hanging off
        // it arrives as a 'Group::All'
        let leaf = match child {
            Group::Leaf(leaf) if leaf.is_string_set() => leaf,
            other => { out.push(compact(other)); continue }
        };

        // Folding into the first match holds sibling order stable, which the
        // frequency sort in 'order_nodes' relies on
        match out.iter_mut().find(|sibling| sibling.absorbs(&leaf)) {
            Some(sibling) => sibling.absorb(leaf),
            None => out.push(Group::Leaf(leaf)),
        }
    }

    match out.len() {
        1 => out.pop().unwrap(),
        _ => Group::Any(out),
    }
}

impl<'src> Group<'src> {
    /// True when this sibling is a leaf already collecting the same property.
    fn absorbs(&self, leaf: &Leaf<'src>) -> bool {
        matches!(self, Group::Leaf(target)
            if target.property.eq_ignore_ascii_case(leaf.property) && target.is_string_set())
    }

    fn absorb(&mut self, leaf: Leaf<'src>) {
        if let Group::Leaf(target) = self { target.absorb(leaf) }
    }
}

impl<'src> From<Node<'src>> for Leaf<'src> {
    fn from(node: Node<'src>) -> Self {
        Leaf {
            property: node.condition.property.0,
            operator: node.condition.operator,
            values: vec![node.condition.value],
            warnings: node.warnings,
        }
    }
}

impl<'src> Leaf<'src> {
    /// Tests one or more strings for equality - the only foldable shapes,
    /// being `-eq "x"` before a fold and `-in ["x", ...]` after one.
    fn is_string_set(&self) -> bool {
        matches!(self.operator, Operator::Equals | Operator::In)
            && self.values.iter().all(|value| matches!(value, Value::String(_)))
    }

    /// Folds a sibling in, widening `-eq` to `-in` on the first addition.
    fn absorb(&mut self, other: Leaf<'src>) {
        self.values.extend(other.values);
        self.operator = self.operator.as_multiple();
        merge_warnings(&mut self.warnings, &other.warnings);
    }
}

/// Renders as rule syntax, e.g. `user.city -in ["Leeds", "York"]`.
impl fmt::Display for Leaf<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} ", self.property, self.operator.as_str())?;
        match self.values.as_slice() {
            [value] => write!(f, "{}", value),
            values => {
                write!(f, "[")?;
                for (i, value) in values.iter().enumerate() {
                    if i > 0 { write!(f, ", ")? }
                    write!(f, "{}", value)?;
                }
                write!(f, "]")
            }
        }
    }
}

// Both impls keep the wire format identical to 'Node', so a folded leaf
// differs only by its 'item' carrying '{"type": "array", ...}'
impl Serialize for Leaf<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Condition<'a> {
            property: &'a str,
            operator: Operator,
            #[serde(rename = "item")]
            value: Item<'a>,
        }

        let mut node = serializer.serialize_struct("Node", 2)?;
        node.serialize_field("condition", &Condition {
            property: self.property,
            operator: self.operator,
            value: Item(&self.values),
        })?;
        node.serialize_field("warnings", &self.warnings)?;
        node.end()
    }
}

/// A leaf's values: serialises as a plain `Value` when there's one, or as
/// `{"type": "array", "value": [...]}` when there are several.
struct Item<'a>(&'a [Value<'a>]);

impl Serialize for Item<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            [value] => value.serialize(serializer),
            values => {
                let mut set = serializer.serialize_struct("Value", 2)?;
                set.serialize_field("type", "array")?;
                set.serialize_field("value", values)?;
                set.end()
            }
        }
    }
}



//
// RENDERING
//

impl<'src> From<Group<'src>> for Block<'src> {
    fn from(group: Group<'src>) -> Self {
        match group {
            Group::Leaf(leaf)    => Block::Node(leaf),
            Group::All(children) => Block::Bracket(interleave(children, true)),
            Group::Any(children) => Block::Bracket(interleave(children, false)),
        }
    }
}

/// Converts `children` to blocks, separated by `And` if `and` is set and by
/// `Or` otherwise.
fn interleave<'src>(children: Vec<Group<'src>>, and: bool) -> Vec<Block<'src>> {
    let mut out = Vec::new();
    for (i, child) in children.into_iter().enumerate() {
        if i > 0 { out.push(if and { Block::And } else { Block::Or }) }
        out.push(Block::from(child));
    }
    out
}

impl Block<'_> {
    /// Renders as rule syntax. Same as `Display`, minus the redundant
    /// brackets around the outermost level.
    pub(crate) fn to_syntax_string(&self) -> String {
        match self {
            Block::Bracket(children) => join(children),
            _ => self.to_string(),
        }
    }
}

/// Renders each block and joins them with spaces.
fn join(blocks: &[Block<'_>]) -> String {
    blocks.iter().map(Block::to_string).collect::<Vec<String>>().join(" ")
}

impl fmt::Display for Block<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Block::And => write!(f, "and"),
            Block::Or  => write!(f, "or"),
            Block::Node(leaf) => write!(f, "{}", leaf),
            Block::Bracket(children) => write!(f, "({})", join(children)),
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Condition, Property, Rule};

    fn node<'src>(property: &'src str, operator: Operator, value: Value<'src>) -> Node<'src> {
        Node::from(Condition { property: Property(property), operator, value })
    }

    fn dnf<'src>(rules: Vec<Vec<Node<'src>>>) -> Dnf<'src> {
        rules.into_iter()
            .map(|nodes| Rule { nodes, warnings: Vec::new() })
            .collect()
    }

    fn syntax(rules: Vec<Vec<Node<'_>>>) -> String {
        reconstruct(&dnf(rules)).expect("a tree").to_syntax_string()
    }

    #[test]
    fn folds_sibling_string_equality() {
        assert_eq!(
            syntax(vec![
                vec![node("user.department", Operator::Equals, Value::String("Sales"))],
                vec![node("user.department", Operator::Equals, Value::String("Legal"))],
            ]),
            "user.department -in [\"Sales\", \"Legal\"]",
        );
    }

    #[test]
    fn folds_under_a_shared_parent() {
        let parent = node("user.accountEnabled", Operator::Equals, Value::Boolean(true));
        assert_eq!(
            syntax(vec![
                vec![parent.clone(), node("user.department", Operator::Equals, Value::String("Sales"))],
                vec![parent.clone(), node("user.department", Operator::Equals, Value::String("Legal"))],
                vec![parent, node("user.department", Operator::Equals, Value::String("IT"))],
            ]),
            "user.accountEnabled -eq true and user.department -in [\"Sales\", \"Legal\", \"IT\"]",
        );
    }

    #[test]
    fn leaves_non_string_siblings_alone() {
        assert_eq!(
            syntax(vec![
                vec![node("user.extension_x", Operator::Equals, Value::Number(1))],
                vec![node("user.extension_x", Operator::Equals, Value::Number(2))],
            ]),
            "user.extension_x -eq 1 or user.extension_x -eq 2",
        );
    }

    #[test]
    fn leaves_non_equality_siblings_alone() {
        assert_eq!(
            syntax(vec![
                vec![node("user.mail", Operator::StartsWith, Value::String("a"))],
                vec![node("user.mail", Operator::StartsWith, Value::String("b"))],
            ]),
            "user.mail -startsWith \"a\" or user.mail -startsWith \"b\"",
        );
    }

    #[test]
    fn will_not_fold_a_node_with_children_past_it() {
        assert_eq!(
            syntax(vec![
                vec![
                    node("user.department", Operator::Equals, Value::String("Sales")),
                    node("user.city", Operator::Equals, Value::String("Leeds")),
                ],
                vec![node("user.department", Operator::Equals, Value::String("Legal"))],
            ]),
            "(user.department -eq \"Sales\" and user.city -eq \"Leeds\") or user.department -eq \"Legal\"",
        );
    }

    #[test]
    fn folds_each_property_separately_and_keeps_order() {
        assert_eq!(
            syntax(vec![
                vec![node("user.department", Operator::Equals, Value::String("Sales"))],
                vec![node("user.city", Operator::Equals, Value::String("Leeds"))],
                vec![node("user.department", Operator::Equals, Value::String("Legal"))],
                vec![node("user.city", Operator::Equals, Value::String("York"))],
            ]),
            "user.department -in [\"Sales\", \"Legal\"] or user.city -in [\"Leeds\", \"York\"]",
        );
    }

    #[test]
    fn folded_leaf_keeps_the_node_wire_shape() {
        let tree = reconstruct(&dnf(vec![
            vec![node("user.department", Operator::Equals, Value::String("Sales"))],
            vec![node("user.department", Operator::Equals, Value::String("Legal"))],
        ])).expect("a tree");

        assert_eq!(
            serde_json::to_value(&tree).expect("serialises"),
            serde_json::json!({"Node": {
                "condition": {
                    "property": "user.department",
                    "operator": "-in",
                    "item": {"type": "array", "value": [
                        {"type": "string", "value": "Sales"},
                        {"type": "string", "value": "Legal"},
                    ]},
                },
                "warnings": [],
            }}),
        );
    }
}
