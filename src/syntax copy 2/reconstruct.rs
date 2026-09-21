use crate::syntax::{
    Dnf,
    Rule,
    Node,
    Value,
};



pub enum Block<'a> {
    Bracket(Vec<Block<'a>>),
    Node(Node<'a>),
    And,
    Or,
}

// fn construct_tree(dnf: Dnf) {
//     let mut tree: Vec<Block> = Vec::new();
//     for rule in dnf.iter() {
//         for node in rule.nodes.iter() {
//             if tree
//         }

//     }
    
//     ;()
// }