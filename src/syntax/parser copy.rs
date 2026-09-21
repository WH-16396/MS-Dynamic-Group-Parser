use pest::{Parser, error::Error, iterators::Pair};
use pest_derive::Parser;

use crate::syntax::{
    // SyntaxBlock,
    SyntaxRule,
    RuleValue,
    Dnf,
    Row,
};

enum Rules<'a> {
    Single(SingleRule<'a>),
    Array(ArrayRule<'a>),
}
struct SingleRule<'a> {
    pub property: &'a str,
    pub operator: &'a str,
    pub value: Value<'a>,
}

enum Value<'a> {
    String(&'a str),
    Bool(bool),
}
struct ArrayRule<'a> {
    pub property: &'a str,
    pub operator: &'a str,
    pub value: Vec<String>,
}


#[derive(Parser)]
#[grammar = "syntax/membership_rules.pest"]
struct SyntaxParser;

pub fn parse_rulebuilder<'a>(raw: &'a str) -> Result<Dnf<'a>, Error<Rule>> {

    // Parses the syntax using the pest rules in 'membership_rules.pest' 
    let mut rules = SyntaxParser::parse(Rule::MembershipRules, raw)?;

    let dnf = parse_tree(rules.next().unwrap(), vec![Row::new()]);

    Ok(dnf)
}



fn parse_tree<'a>(pair: Pair<'a, Rule>, mut dnf: Dnf<'a>) -> Dnf<'a> {
    match pair.as_rule() {
        Rule::Segment => {
            let mut tree = Vec::new();
            let mut branch = vec![Row::new()];

            for child in pair.into_inner() {
                match child.as_rule() {
                    Rule::Comparator if child.as_str() == "and" => {},
                    Rule::Comparator if child.as_str() == "or"  => {
                        tree.append(&mut branch);
                        branch = vec![Row::new()];
                    },
                    // Rule::Comparator => {},
                    Rule::Segment | 
                    Rule::Rule => branch = parse_tree(child, branch),
                    _ => { println!("ERROR - parse_tree() failed to handle child of Rule::Segment, {:#?}", child); unreachable!() },
                }
            }
            tree.append(&mut branch);
            dnf = merge_tree(dnf, tree);
        },
        Rule::Rule => {
            match parse_rule(pair) {
                Some(rule) => {
                    for row in dnf.iter_mut() {
                        row.rules.push(rule.clone())
                    }
                },
                _ => (),
            }
        },
        _ => { println!("ERROR - parse_tree() failed to handle Rule, {:#?}", pair); unreachable!() },
    };
    dnf
}



fn parse_rule(pair: Pair<Rule>) -> Option<SyntaxRule> {
    
    let v: Vec<Pair<Rule>> = pair.into_inner().collect();

    let property = match v[0].as_rule() {
        Rule::Property => v[0].as_str(),
        _ => return None,
    };

    let operator = match v[1].as_rule() {
        Rule::Operator => v[1].as_str(),
        _ => return None,
    };

    fn parse_value(value: Pair<Rule>) -> Option<RuleValue> {
        match &value.as_rule() {
            Rule::string => Some(RuleValue::String(value.into_inner().next().unwrap().as_str())),
            // Rule::array => Some(RuleValue::Array(value.into_inner().filter_map(parse_value).collect())),
            Rule::boolean => Some(RuleValue::Boolean(value.as_str().to_lowercase().parse().unwrap())),
            _ => None,
        }
    }

    let value = match parse_value(v[2].clone()) {
        Some(o) => o,
        _ => return None,
    };

    Some(SyntaxRule {
        property,
        operator,
        value,
    })
}



fn merge_tree<'a>(left: Dnf<'a>, right: Dnf<'a>) -> Dnf<'a> {
    let mut out = Vec::with_capacity(left.len() * right.len());
    for l in &left {
        for r in &right {
            let mut row = l.clone();
            row.rules.extend(r.rules.iter().cloned());
            out.push(row);
        }
    }
    out
}



// pub fn order_rules<'a>(mut dnf: Dnf<'a>) -> Dnf<'a> {
//     use std::collections::HashMap;
//     let mut map = HashMap::new();

//     for branch in dnf.iter_mut() {
//         for rule in branch.rules.iter() {
//             *map.entry(rule.clone().to_string()).or_insert(0) += 1;
//         }
//         branch.rules.sort_by(|a, b| 
//             map.get(&b.clone().to_string()).unwrap()
//             .cmp(map.get(&a.clone().to_string()).unwrap())
//         ); 
//     }
//     dnf
// }

// pub fn serialize_dnf(dnf: &DNF) -> String {

//     use crate::syntax::parser::Syntax::*;
//     let a = match dnf {
//         Brackets(b) => {
//             let contents: Vec<_> = b.iter().map(serialize_syntax).collect();
//             format!("{{\"type\": \"bracket\", \"value\": [{}]}}", contents.join(","))
//         },
//         Rule(r) => {
//             let contents: Vec<_> = r.iter().map(serialize_syntax).collect();
//             format!("{{\"type\": \"rule\", \"value\": [{}]}}", contents.join(","))
//         },
//         Array(a) => {
//             let contents: Vec<_> = a.iter().map(serialize_syntax).collect();
//             format!("{{\"type\": \"array\", \"value\": [{}]}}", contents.join(","))
//         },
//         Comparator(c) => format!("{{\"type\": \"comparator\", \"value\": \"{}\"}}", c),
//         Operator(op) => format!("{{\"type\": \"operator\", \"value\": \"{}\"}}", op),
//         Object(ob) => format!("{{\"type\": \"object\", \"value\": \"{}\"}}", ob),
//         String(s) => format!("{{\"type\": \"string\", \"value\": \"{}\"}}", s),
//         Boolean(b) => format!("{{\"type\": \"boolean\", \"value\": \"{}\"}}", b),
//         Null => format!("null"),
//     };

//     return a
// }



pub fn serialize_rules<'a>(dnf: Dnf) -> String {
    
    let mut root = Vec::new();
    
    for row in dnf.iter() {
        if !root.contains(&row) {
            root.push(row)
        }
    }
    
    ;String::new()
}
