//! Turns rule syntax text into a `Dnf`.
//!
//! Parsing happens in two steps: pest builds a parse tree from
//! `grammar.pest`, then `expand` walks that tree and multiplies the brackets
//! out into a flat OR-list of AND-rules.

use pest::{Parser, error::Error, iterators::Pair};

use crate::model::{Condition, Dnf, Node, Operator, Property, Rule, Value};

/// The pest-generated parser lives in its own module so that its `Rule` enum
/// (one variant per grammar rule) doesn't clash with `model::Rule`.
mod grammar {
    #[derive(pest_derive::Parser)]
    #[grammar = "parser/grammar.pest"]
    pub struct SyntaxParser;
}

use grammar::{Rule as Token, SyntaxParser};

/// Parses rule syntax into a `Dnf`, borrowing property names and string
/// values from `syntax`.
pub fn parse(syntax: &str) -> Result<Dnf<'_>, Error<Token>> {
    let root = SyntaxParser::parse(Token::MembershipRules, syntax)?
        .next()
        .unwrap();

    Ok(expand(root, vec![Rule::new()]))
}

/// Expands a `Segment` or `Node` from the parse tree, AND-ing the result onto
/// every rule in `dnf`.
fn expand<'src>(pair: Pair<'src, Token>, mut dnf: Dnf<'src>) -> Dnf<'src> {
    match pair.as_rule() {
        // A segment is a list of operands separated by 'and' / 'or'. Operands
        // between two 'or's are AND-ed into the same set of rules ('current'),
        // and each 'or' starts a fresh set. 'alternatives' collects the
        // finished sets.
        Token::Segment => {
            let mut alternatives = Vec::new();
            let mut current = vec![Rule::new()];

            for child in pair.into_inner() {
                match child.as_rule() {
                    Token::And => {}
                    Token::Or => {
                        alternatives.append(&mut current);
                        current = vec![Rule::new()];
                    }
                    Token::Segment | Token::Node => current = expand(child, current),
                    _ => {
                        println!("ERROR - expand() failed to handle child of Segment, {:#?}", child);
                        unreachable!()
                    }
                }
            }
            alternatives.append(&mut current);

            // (a or b) and (c or d) -> ac or ad or bc or bd
            dnf = cross_product(dnf, alternatives);
        }

        // A single condition is AND-ed onto every rule
        Token::Node => {
            if let Some(nodes) = parse_node(pair) {
                for node in nodes {
                    for rule in dnf.iter_mut() {
                        rule.nodes.push(node.clone())
                    }
                }
            }
        }

        _ => {
            println!("ERROR - expand() failed to handle token, {:#?}", pair);
            unreachable!()
        }
    };
    dnf
}

/// ANDs two DNFs together by pairing every rule on the left with every rule
/// on the right.
fn cross_product<'src>(left: Dnf<'src>, right: Dnf<'src>) -> Dnf<'src> {
    let mut out = Vec::with_capacity(left.len() * right.len());
    for left_rule in &left {
        for right_rule in &right {
            let mut rule = left_rule.clone();
            rule.nodes.extend(right_rule.nodes.iter().cloned());
            out.push(rule);
        }
    }
    out
}

/// Converts a `property operator value` triple into nodes. An array value is
/// split into one node per element, with the operator narrowed to its
/// single-value form (`-in` -> `-eq`).
fn parse_node(pair: Pair<'_, Token>) -> Option<Vec<Node<'_>>> {
    let mut parts = pair.into_inner();
    let mut out = Vec::new();

    let property = Property(parts.next()?.as_str());

    let operator_pair = parts.next()?;
    let operator = match operator_pair.as_rule() {
        Token::Operator => operator_pair.into_inner().next()?.as_rule().as_operator(),
        _ => {
            println!("ERROR - Tried to parse operator: {:?}", operator_pair.as_rule());
            unreachable!()
        }
    };

    let value_pair = parts.next()?;
    match value_pair.as_rule() {
        Token::String | Token::True | Token::False | Token::Number | Token::Null => {
            out.push(Node::from(Condition {
                property,
                operator,
                value: parse_value(value_pair)?,
            }))
        }
        Token::Array => {
            for element in value_pair.into_inner() {
                out.push(Node::from(Condition {
                    property,
                    operator: operator.as_single(),
                    value: parse_value(element)?,
                }));
            }
        }
        _ => {
            println!("ERROR - Tried to parse value: {:?}", value_pair.as_rule());
            unreachable!()
        }
    }
    Some(out)
}

fn parse_value(pair: Pair<'_, Token>) -> Option<Value<'_>> {
    match pair.as_rule() {
        Token::String => Some(Value::String(pair.as_str())),
        Token::True => Some(Value::Boolean(true)),
        Token::False => Some(Value::Boolean(false)),
        Token::Number => Some(Value::Number(pair.as_str().parse().ok()?)),
        Token::Null => Some(Value::Null),
        _ => None,
    }
}

impl Token {
    /// Maps an operator token to its `Operator`. Only called on the inner
    /// token of `Token::Operator`, so any other token is a grammar bug.
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
