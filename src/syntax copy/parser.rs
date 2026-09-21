use pest::{Parser, error::Error, iterators::Pair};
use pest_derive::Parser;
use serde::{Serialize, Deserialize};

use crate::syntax::interpreter::{validate_rules, RuleParts};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SyntaxBlock<'a> {
    Brackets(Vec<Syntax<'a>>),
    Rule(Vec<Syntax<'a>>),
    Comparator(&'a str),
    Object(&'a str),
    Operator(&'a str),
    String(&'a str),
    Array(Vec<Syntax<'a>>),
    Boolean(bool),
    Null,
}

pub struct SyntaxRule<'a> {
    object: RuleObject,
    comparator: RuleComparator,
    value: RuleValue,
}

impl<'a> Syntax<'a> {
    pub fn into_inner(&self) -> &Syntax<'a> {
        match self {
            
            a => a,
        }
        // syntax.into_iter()
        //     .map(|x| x.to_string())
        //     .collect::<Vec<String>>()
        //     .join(join)
    }

    pub fn to_string(self) -> String {
        fn get_inner(syntax: Vec<Syntax>, join: &str) -> String {
            syntax.into_iter()
                .map(|x| x.to_string())
                .collect::<Vec<String>>()
                .join(join)
        }

        match self {
            Syntax::Brackets(br) => format!("({})", get_inner(br, " ")),
            Syntax::Rule(r) => get_inner(r, " "),
            Syntax::Comparator(c) => String::from(c),
            Syntax::Object(ob) => String::from(ob),
            Syntax::Operator(op) => String::from(op),
            Syntax::String(s) => format!("\"{}\"", s),
            Syntax::Array(a) => format!("[{}]", get_inner(a, ",")),
            Syntax::Boolean(bo) => bo.to_string(),
            _ => String::new(),
        }
    }
}

pub type Row<'a> = Vec<Syntax<'a>>;
pub type DNF<'a> = Vec<Row<'a>>;


#[derive(Parser)]
#[grammar = "syntax/membership_rules.pest"]
struct SyntaxParser;

pub fn parse_membership_rules(raw: &str) -> Result<DNF<'_>, Error<Rule>> {

    // Attempts to parse the syntax using the pest rules in 'membership_rules.pest' 
    let mut rules = SyntaxParser::parse(Rule::MembershipRules, raw)?;

    let dnf = order_rules(parse_tree(rules.next().unwrap(), vec![Vec::new()]));

    // println!("Before sorting:\n\n{:#?}", dnf.clone()[5]);
    // println!("After sorting:\n\n{:#?}", order_rules(dnf.clone()).clone()[5]);

    // for row in dnf.iter() {
    //     if_validate_rules(row, "if_contains", check_rule)
    // }
    for row in dnf.clone() {

        let conditions = None;
        let check_rule = RuleParts {
            object: Some(Syntax::Object("user.accountEnabled")),
            operator: Some(Syntax::Operator("-eq")),
            value: Some(Syntax::Boolean(true))
        };

        println!("CONDITIONS: {:?}\nCHECK: {:#?}", conditions, check_rule);
        
        // let out = validate_rules(&row, conditions, check_rule);

        println!("ROW: {:?}\nSUCCESS: {:#?}\n\n\n\n\n", row, validate_rules(&row, conditions, check_rule));
    }

    Ok(dnf)
}

fn parse_tree<'a>(pair: Pair<'a, Rule>, mut dnf: DNF<'a>) -> DNF<'a> {
    match pair.as_rule() {
        Rule::Segment => {
            let mut tree = Vec::new();
            let mut branch = vec![Vec::new()];

            for child in pair.into_inner() {
                match child.as_rule() {
                    Rule::Comparator if child.as_str() == "and" => {},
                    Rule::Comparator if child.as_str() == "or"  => {
                        tree.append(&mut branch);
                        branch = vec![Vec::new()];
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
            let rule = Syntax::Rule(pair.into_inner().map(parse_rule).collect());
            for row in dnf.iter_mut() {
                row.push(rule.clone())
            }
        },
        _ => { println!("ERROR - parse_tree() failed to handle Rule, {:#?}", pair); unreachable!() },
    };
    dnf
}

fn parse_rule(pair: Pair<Rule>) -> Syntax {
    let a = match pair.as_rule() {
        Rule::Object => Syntax::Object(pair.as_str()),
        Rule::Operator => Syntax::Operator(pair.as_str()),
        Rule::string => Syntax::String(pair.into_inner().next().unwrap().as_str()),
        Rule::array => Syntax::Array(pair.into_inner().map(parse_rule).collect()),
        Rule::boolean => Syntax::Boolean(pair.as_str().to_lowercase().parse().unwrap()),
        Rule::null => Syntax::Null,
        _ => { println!("ERROR - parse_rule() failed to handle child of Rule::Rule, {:#?}", pair); unreachable!() },
    };
    println!("{}", a.clone().to_string());
    a
}

fn merge_tree<'a>(left: DNF<'a>, right: DNF<'a>) -> DNF<'a> {
    let mut out = Vec::with_capacity(left.len() * right.len());
    for l in &left {
        for r in &right {
            let mut row = l.clone();
            row.extend(r.iter().cloned());
            out.push(row);
        }
    }
    out
}

fn order_rules<'a>(mut dnf: DNF<'a>) -> DNF<'a> {
    use std::collections::HashMap;
    let mut map = HashMap::new();

    for branch in dnf.iter_mut() {
        for rule in branch.iter() {
            *map.entry(rule.clone().to_string()).or_insert(0) += 1;
        }
        branch.sort_by(|a, b| 
            map.get(&b.clone().to_string()).unwrap()
            .cmp(map.get(&a.clone().to_string()).unwrap())
        ); 
    }
    dnf
}