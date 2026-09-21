use crate::rules::{
    // SyntaxBlock,
    // Node,
    // Value,
    Dnf,
    Condition,
    // Rule,
};

pub fn remove_duplicates<'a>(mut dnf: Dnf<'a>) -> Dnf<'a> {
    for rule in dnf.iter_mut() {
        let mut seen = Vec::new();
        rule.nodes.retain(|item| match seen.contains(item) {
            true => false,
            _ => {
                seen.push(item.clone());
                true
            }
        })
    }
    dnf
}

pub fn order_rules<'a>(mut dnf: Dnf<'a>) -> Dnf<'a> {
    use std::collections::HashMap;
    let mut map: HashMap<Condition<'_>, i32> = HashMap::new();

    for row in dnf.iter_mut() {
        for rule in row.nodes.iter() {
            *map.entry(rule.clone().condition).or_insert(0) += 1;
        }
        row.nodes.sort_by(|a, b| 
            map.get(&b.clone().condition).unwrap()
            .cmp(map.get(&a.clone().condition).unwrap())
        ); 
    }
    dnf
}