use serde::{Serialize, Serializer, ser::SerializeStruct};

use crate::rules::{
    Dnf,
    Node,
    Operator,
    Value,
    Warning,
};

#[derive(Serialize)]
pub(crate) enum Block<'a> {
    Bracket(Vec<Block<'a>>),
    Node(Leaf<'a>),
    And,
    Or,
}

enum Group<'a> {
    Leaf(Leaf<'a>),
    All(Vec<Group<'a>>),
    Any(Vec<Group<'a>>),
}

// Mirrors 'Node', except that the value side can hold a folded set, which
// 'Condition' cannot - it is 'Copy' and a 'HashMap' key in 'process_rules'
#[derive(Debug, Clone)]
pub(crate) struct Leaf<'a> {
    property: &'a str,
    operator: Operator,
    values: Vec<Value<'a>>,
    warnings: Vec<&'a Warning>,
}

pub(crate) fn reconstruct<'a>(dnf: &Dnf<'a>) -> Option<Block<'a>> {
    let branches: Vec<&[Node<'a>]> = dnf.iter().map(|r| r.nodes.as_slice()).collect();
    combine(&branches).map(compact).map(Block::from)
}

fn combine<'a>(branches: &[&[Node<'a>]]) -> Option<Group<'a>> {
    let mut groups: Vec<(Node<'a>, Vec<&[Node<'a>]>)> = Vec::new();
    let mut ends_here = false;

    for nodes in branches {
        match nodes.split_first() {
            None => ends_here = true,
            Some((head, tail)) => {
                match groups.iter_mut().find(|(n, _)| n.condition == head.condition) {
                    Some((node, members)) => {
                        merge_warnings(node, head);
                        members.push(tail);
                    }
                    None => groups.push((head.clone(), vec![tail])),
                }
            }
        }
    }

    if ends_here { return None }

    let mut children: Vec<Group<'a>> = groups
        .into_iter()
        .map(|(node, members)| {
            let leaf = Group::Leaf(Leaf::from(node));
            match combine(&members) {
                None => leaf,
                Some(Group::All(mut inner)) => {
                    let mut c = vec![leaf];
                    c.append(&mut inner);
                    Group::All(c)
                }
                Some(sub) => Group::All(vec![leaf, sub]),
            }
        })
        .collect();

    match children.len() {
        0 => None,
        1 => children.pop(),
        _ => Some(Group::Any(children)),
    }
}

fn merge_warnings<'a>(into: &mut Node<'a>, from: &Node<'a>) {
    for w in &from.warnings {
        if !into.warnings.contains(w) { into.warnings.push(*w) }
    }
}



//
// COMPACTION
//

// Folds compactable siblings at every 'Any' level. 'All' levels are prefix
// chains rather than siblings, so they are only walked through
fn compact<'a>(group: Group<'a>) -> Group<'a> {
    match group {
        Group::Leaf(_) => group,
        Group::All(children) => Group::All(children.into_iter().map(compact).collect()),
        Group::Any(children) => fold_siblings(children),
    }
}

fn fold_siblings<'a>(children: Vec<Group<'a>>) -> Group<'a> {
    let mut out: Vec<Group<'a>> = Vec::with_capacity(children.len());

    for child in children {
        // Only a bare terminal leaf folds - a node with anything hanging off
        // it arrives as a 'Group::All'
        let leaf = match child {
            Group::Leaf(leaf) if leaf.is_string_set() => leaf,
            other => { out.push(compact(other)); continue }
        };

        // Folding into the first match holds sibling order stable, which the
        // frequency sort in 'order_rules' relies on
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

impl<'a> Group<'a> {
    // True when this sibling is a leaf already collecting the same property
    fn absorbs(&self, leaf: &Leaf<'a>) -> bool {
        matches!(self, Group::Leaf(target)
            if target.property == leaf.property && target.is_string_set())
    }

    fn absorb(&mut self, leaf: Leaf<'a>) {
        if let Group::Leaf(target) = self { target.absorb(leaf) }
    }
}

impl<'a> From<Node<'a>> for Leaf<'a> {
    fn from(node: Node<'a>) -> Self {
        Leaf {
            property: node.condition.property,
            operator: node.condition.operator,
            values: vec![node.condition.value],
            warnings: node.warnings,
        }
    }
}

impl<'a> Leaf<'a> {
    // Tests one or more strings for equality - the only foldable shapes, being
    // '-eq "x"' before a fold and '-in ["x", ...]' after one
    fn is_string_set(&self) -> bool {
        matches!(self.operator, Operator::Equals | Operator::In)
            && self.values.iter().all(|v| matches!(v, Value::String(_)))
    }

    // Folds a sibling in, widening '-eq' to '-in' on the first addition
    fn absorb(&mut self, other: Leaf<'a>) {
        self.values.extend(other.values);
        self.operator = self.operator.as_multiple();
        for w in other.warnings {
            if !self.warnings.contains(&w) { self.warnings.push(w) }
        }
    }

    fn to_string(&self) -> String {
        let value = match self.values.as_slice() {
            [value] => value.to_string(),
            values => format!("[{}]", values
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<String>>()
                .join(", ")
            ),
        };
        format!("{} {} {}", self.property, self.operator.as_str(), value)
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

impl<'a> From<Group<'a>> for Block<'a> {
    fn from(group: Group<'a>) -> Self {
        match group {
            Group::Leaf(leaf)    => Block::Node(leaf),
            Group::All(children) => Block::Bracket(interleave(children, true)),
            Group::Any(children) => Block::Bracket(interleave(children, false)),
        }
    }
}

fn interleave<'a>(children: Vec<Group<'a>>, and: bool) -> Vec<Block<'a>> {
    let mut out = Vec::new();
    for (i, child) in children.into_iter().enumerate() {
        if i > 0 { out.push(if and { Block::And } else { Block::Or }) }
        out.push(Block::from(child));
    }
    out
}

impl Block<'_> {
    pub(crate) fn to_syntax_string(&self) -> String {
        fn walk_tree(block: &Block<'_>) -> String {
            match block {
                Block::And => String::from("and"),
                Block::Or  => String::from("or"),
                Block::Node(n) => n.to_string(),
                Block::Bracket(b) => format!("({})",
                    b.into_iter()
                    .map(|c| walk_tree(c))
                    .collect::<Vec<String>>()
                    .join(" ")
                ),
            }
        }
        match self {
            Block::Bracket(b) => String::from(b.into_iter().map(|c| walk_tree(c)).collect::<Vec<String>>().join(" ")),
            _ => walk_tree(&self),
        }
    }
}

impl std::fmt::Display for Block<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Block::And => write!(f, "and"),
            Block::Or  => write!(f, "or"),
            Block::Node(n) => write!(f, "{}", n.to_string()),
            Block::Bracket(children) => {
                write!(f, "(")?;
                for (i, c) in children.iter().enumerate() {
                    if i > 0 { write!(f, " ")? }
                    write!(f, "{}", c)?;
                }
                write!(f, ")")
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::{Condition, Rule as Nodes};

    fn node<'a>(property: &'a str, operator: Operator, value: Value<'a>) -> Node<'a> {
        Node::from(Condition { property, operator, value })
    }

    fn dnf<'a>(rows: Vec<Vec<Node<'a>>>) -> Dnf<'a> {
        rows.into_iter()
            .map(|nodes| Nodes { nodes, warnings: Vec::new() })
            .collect()
    }

    fn syntax(rows: Vec<Vec<Node<'_>>>) -> String {
        reconstruct(&dnf(rows)).expect("a tree").to_syntax_string()
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
