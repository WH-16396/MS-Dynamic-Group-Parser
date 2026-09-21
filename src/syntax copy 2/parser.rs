use pest::{Parser, error::Error, iterators::Pair};
use pest_derive::Parser;

use crate::syntax::{
    Node,
    Condition,
    Value,
    Dnf,
    // Imported as 'Nodes' to prevent conflicts with the 'Rules' enum since
    // that gets automatically imported as part of '#[derive(Parser)]'
    Rule as Nodes,
};


#[derive(Parser)]
#[grammar = "syntax/membership_rules.pest"]
struct SyntaxParser;

pub fn parse_rulebuilder<'a>(raw: &'a str) -> Result<Dnf<'a>, Error<Rule>> {

    // Parses the syntax using the pest rules in 'membership_rules.pest' 
    let parse = SyntaxParser::parse(Rule::MembershipRules, raw);

    // println!("TEST: {:#?}", parse);

    let dnf = parse_tree(parse?.next().unwrap(), vec![Nodes::new()]);

    Ok(dnf)
}



fn parse_tree<'a>(pair: Pair<'a, Rule>, mut dnf: Dnf<'a>) -> Dnf<'a> {
    match pair.as_rule() {
        Rule::Segment => {
            let mut tree = Vec::new();
            let mut branch = vec![Nodes::new()];

            for child in pair.into_inner() {
                match child.as_rule() {
                    Rule::And => {},
                    Rule::Or  => {
                        tree.append(&mut branch);
                        branch = vec![Nodes::new()];
                    },
                    Rule::Segment | 
                    // Rule::Node | 
                    Rule::SingleNode | 
                    Rule::ArrayNode => branch = parse_tree(child, branch),
                    _ => { println!("ERROR - parse_tree() failed to handle child of Rule::Segment, {:#?}", child); unreachable!() },
                }
            }
            tree.append(&mut branch);
            dnf = merge_tree(dnf, tree);
        },
        Rule::SingleNode => {
            if let Some(node) = parse_single(pair) {
                for row in dnf.iter_mut() {
                    row.nodes.push(node.clone())
                }
            }
        },
        Rule::ArrayNode => {
            if let Some(nodes) = parse_array(pair) {
                for node in nodes {
                    for row in dnf.iter_mut() {
                        row.nodes.push(node.clone())
                    }
                }
            }
        },
        _ => { println!("ERROR - parse_tree() failed to handle Rule, {:#?}", pair); unreachable!() },
    };
    dnf
}

fn merge_tree<'a>(left: Dnf<'a>, right: Dnf<'a>) -> Dnf<'a> {
    let mut out = Vec::with_capacity(left.len() * right.len());
    for l in &left {
        for r in &right {
            let mut row = l.clone();
            row.nodes.extend(r.nodes.iter().cloned());
            out.push(row);
        }
    }
    out
}



fn parse_single(pair: Pair<Rule>) -> Option<Node> {
    let mut inner = pair.into_inner();
    Some(Node {
        condition: Condition {
            property: inner.next()?.as_str(),
            operator: inner.next()?.as_str(),
            value: parse_value(inner.next()?)?,
        },
        warnings: Vec::new(),
    })
}

fn parse_array(pair: Pair<Rule>) -> Option<Vec<Node>> {
    let mut inner = pair.into_inner();
    fn get_single_operator(op: Pair<Rule>) -> Option<&'static str> {
        Some(match op.as_rule() {
            Rule::In => "-eq",
            Rule::NotIn => "-ne",
            Rule::ArrayOp => get_single_operator(op.into_inner().next()?)?,
            _ => return None,
        })
    }
    
    let mut out: Vec<Node> = Vec::new();
    let property = inner.next()?.as_str();
    let operator = get_single_operator(inner.next()?)?;
    for value in inner.next()?.into_inner() {
        out.push(Node::from(Condition{
            property,
            operator,
            value: parse_value(value)?,
        }));
    }
    Some(out)
}

fn parse_value(value: Pair<Rule>) -> Option<Value> {
    match &value.as_rule() {
        Rule::string => Some(Value::String(value.as_str())),
        // Rule::boolean => Some(Value::Boolean(value.as_str().parse().ok()?)),
        Rule::True => Some(Value::Boolean(true)),
        Rule::False => Some(Value::Boolean(false)),
        Rule::number => Some(Value::Number(value.as_str().parse().ok()?)),
        _ => None,
    }
}
