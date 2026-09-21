use crate::syntax::parser::{
    Syntax, 
    // Syntax::*, 
    // DNF, 
    Row
};

// pub struct 

// pub fn serialize_dnf(dnf: &DNF) -> String {

//     use crate::syntax::parser::Syntax::*;
//     let a = match dnf {
//         Brackets(b) => {
//             let contents: Vec<_> = b.iter().map(serialize_syntax).collect();
//             format!("{{\"nodeType\": \"bracket\", \"nodeContents\": [{}]}}", contents.join(","))
//         },
//         Rule(r) => {
//             let contents: Vec<_> = r.iter().map(serialize_syntax).collect();
//             format!("{{\"nodeType\": \"rule\", \"nodeContents\": [{}]}}", contents.join(","))
//         },
//         Array(a) => {
//             let contents: Vec<_> = a.iter().map(serialize_syntax).collect();
//             format!("{{\"nodeType\": \"array\", \"nodeContents\": [{}]}}", contents.join(","))
//         },
//         Comparator(c) => format!("{{\"nodeType\": \"comparator\", \"nodeContents\": \"{}\"}}", c),
//         Operator(op) => format!("{{\"nodeType\": \"operator\", \"nodeContents\": \"{}\"}}", op),
//         Object(ob) => format!("{{\"nodeType\": \"object\", \"nodeContents\": \"{}\"}}", ob),
//         String(s) => format!("{{\"nodeType\": \"string\", \"nodeContents\": \"{}\"}}", s),
//         Boolean(b) => format!("{{\"nodeType\": \"boolean\", \"nodeContents\": \"{}\"}}", b),
//         Null => format!("null"),
//     };

//     return a
// }

#[derive(Debug)]
pub struct RuleParts<'a> {
    pub object: Option<Syntax<'a>>,
    pub operator: Option<Syntax<'a>>,
    pub value: Option<Syntax<'a>>,
}

pub fn validate_rules(row: &Row, if_contains: Option<Vec<RuleParts>>, check_rule: RuleParts) -> bool {
    for syntax in row.iter() {
        let rule = if let Syntax::Rule(rule) = syntax {};
        if let Syntax::Rule(rule) = syntax {
            if match if_contains {
                Some(conditions) => {
                    for condition in conditions {

                    };
                    // if rule == String::from(condition) {
                    //     return if_validate_rules(row, None, check_rule)
                    // }
                    true
                },
                None => match_rule(rule, check_rule),
            } {return true}
        }
        return false
    }
    return false
}

fn match_rule(rule: &Vec<Syntax>, check_rule: RuleParts) -> bool {
    if let Some(check) = check_rule.object {
        println!("1");
        if rule[0] != check {false} else { 
            if let Some(check) = check_rule.operator {
                println!("2");
                if rule[1] != check {false} else {
                    if let Some(check) = check_rule.value {
                        println!("3");
                        if rule[2] == check {true} else {false}
                    } else {true}
                }
            } else {true}
        }
    } else {true}
}