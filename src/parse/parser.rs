use pest::{Parser, error::Error, iterators::Pair};
use pest_derive::Parser;
use crate::condition::Property;
use crate::rules::{
    Node,
    Dnf,
    // Imported as 'Nodes' to prevent conflicts with the 'Rules' enum since
    // that gets automatically imported as part of '#[derive(Parser)]'
    Rule as Nodes,
};
use crate::condition::{
    Condition, 
    operator::Operator, 
    value::Value
};

impl Rule {
    fn as_operator(&self) -> Operator {
        match self {
            Self::Add =>                Operator::Add,
            Self::All =>                Operator::All,
            Self::Any =>                Operator::Any,
            Self::Contains =>           Operator::Contains,
            Self::EndsWith =>           Operator::EndsWith,
            Self::Equals =>             Operator::Equals,
            Self::GreaterThanOrEqual => Operator::GreaterThanOrEqual,
            Self::In =>                 Operator::In,
            Self::LessThanOrEqual =>    Operator::LessThanOrEqual,
            Self::Match =>              Operator::Match,
            Self::NotContains =>        Operator::NotContains,
            Self::NotEndsWith =>        Operator::NotEndsWith,
            Self::NotEquals =>          Operator::NotEquals,
            Self::NotIn =>              Operator::NotIn,
            Self::NotMatch =>           Operator::NotMatch,
            Self::NotStartsWith =>      Operator::NotStartsWith,
            Self::StartsWith =>         Operator::StartsWith,
            Self::Subtract =>           Operator::Subtract,
            _ => unreachable!(),
        }
    }
}

#[derive(Parser)]
#[grammar = "parse/syntax.pest"]
struct SyntaxParser;

pub fn parse_rulebuilder<'a>(raw: &'a str) -> Result<Dnf<'a>, Error<Rule>> {

    // Parses the syntax using the pest rules in 'membership_rules.pest' 
    let parse = SyntaxParser::parse(Rule::MembershipRules, raw);

    // println!("Rules: {:#?}", parse);

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
                    Rule::Node 
                    => branch = parse_tree(child, branch),
                    _ => { println!("ERROR - parse_tree() failed to handle child of Rule::Segment, {:#?}", child); unreachable!() },
                }
            }
            tree.append(&mut branch);
            dnf = merge_tree(dnf, tree);
        },
        Rule::Node => {
            if let Some(nodes) = parse_node(pair) {
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

fn parse_node(pair: Pair<Rule>) -> Option<Vec<Node>> {
    let mut inner = pair.into_inner();
    let mut out: Vec<Node> = Vec::new();

    let property = inner.next()?.as_str();
    let operator_pair = inner.next()?;

    let operator = match operator_pair.as_rule() {
        Rule::Operator => {
            operator_pair.into_inner().next()?.as_rule()
        },
        _ => { println!("ERROR - Tried to parse operator: {:?}", operator_pair.as_rule()); unreachable!() },
    };

    let value_pair = inner.next()?;

    match value_pair.as_rule() {
        Rule::String |
        Rule::True |
        Rule::False |
        Rule::Number |
        Rule::Null => out.push(
            Node::from(Condition {
                property: Property(property),
                operator: operator.as_operator(),
                value: parse_value(value_pair)?,
            })
        ),
        Rule::Array => {
            for value in value_pair.into_inner() {
                out.push(Node::from(Condition{
                    property: Property(property),
                    operator: operator.as_operator().as_single(),
                    value: parse_value(value)?,
                }));
            }
        },
        _ => { println!("ERROR - Tried to parse value: {:?}", value_pair.as_rule()); unreachable!() },
    }
    Some(out)
}

fn parse_value(value: Pair<Rule>) -> Option<Value> {
    match &value.as_rule() {
        Rule::String => Some(Value::String(value.as_str())),
        Rule::True => Some(Value::Boolean(true)),
        Rule::False => Some(Value::Boolean(false)),
        Rule::Number => Some(Value::Number(value.as_str().parse().ok()?)),
        Rule::Null => Some(Value::Null),
        _ => None,
    }
}
