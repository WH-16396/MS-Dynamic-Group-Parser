use serde::Serialize;

use crate::syntax::{
    Dnf,
    Node,
};

#[derive(Serialize)]
pub enum Block<'a> {
    Bracket(Vec<Block<'a>>),
    Node(Node<'a>),
    And,
    Or,
}

enum Group<'a> {
    Leaf(Node<'a>),
    All(Vec<Group<'a>>),
    Any(Vec<Group<'a>>),
}

pub fn reconstruct<'a>(dnf: &Dnf<'a>) -> Option<Block<'a>> {
    let branches: Vec<&[Node<'a>]> = dnf.iter().map(|r| r.nodes.as_slice()).collect();
    combine(&branches).map(Block::from)
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
            let leaf = Group::Leaf(node);
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

impl<'a> From<Group<'a>> for Block<'a> {
    fn from(group: Group<'a>) -> Self {
        match group {
            Group::Leaf(node)    => Block::Node(node),
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
    pub fn to_syntax_string(&self) -> String {
        fn walk_tree(block: &Block<'_>) -> String {
            match block {
                Block::And => String::from("and"),
                Block::Or  => String::from("or"),
                Block::Node(n) => n.condition.to_string(),
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
            Block::Node(n) => 
                write!(f, "{}", n.condition.to_string()),
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