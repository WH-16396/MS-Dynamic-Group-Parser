//! Re-folds a flat `Dnf` (an OR of AND-rows) into a factored tree that can be
//! serialised straight to JSON for the API.
//!
//! The rows share long prefixes once `order_rules` has sorted them by rule
//! frequency, so grouping rows by their leading rule (a trie) pulls the shared
//! conditions out into single nodes:
//!
//! ```text
//! [enabled, member, dept d1, job d1j1]      enabled
//! [enabled, member, dept d1, job d1j2]  ->  member
//! [enabled, member, dept d2, job d2j1]      or ┬ dept d1 and (job d1j1 or job d1j2)
//!                                             └ dept d2 and job d2j1
//! ```

use serde::Serialize;
use serde_json::Value;

use crate::syntax::{Dnf, RuleValue, SyntaxRule, Warning};

/// A node of the factored rule tree.
///
/// Serialises with a `type` discriminator, e.g.
/// `{"type":"rule","property":"user.department","operator":"-eq","value":"d1"}`.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Node<'a> {
    /// Every child must match.
    And { children: Vec<Node<'a>> },
    /// At least one child must match.
    Or { children: Vec<Node<'a>> },
    /// A single condition. `warnings` is populated when a DNF row ends here.
    Rule {
        property: &'a str,
        operator: &'a str,
        value: Value,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        warnings: Vec<&'static str>,
    },
}

/// Builds the tree for `dnf`. `None` when there is nothing to describe, which
/// serialises to JSON `null`.
pub fn build_tree<'a>(dnf: &Dnf<'a>) -> Option<Node<'a>> {
    // Borrow the rows as (remaining rules, row warnings) cursors, de-duplicating
    // repeated rules inside a row so `A and A` does not become two nodes.
    let owned: Vec<(Vec<SyntaxRule<'a>>, &[Warning])> = dnf
        .iter()
        .map(|row| (dedup_rules(&row.rules), row.warnings.as_slice()))
        .collect();

    let rows: Vec<Row<'a, '_>> = owned
        .iter()
        .map(|(rules, warnings)| (rules.as_slice(), *warnings))
        .collect();

    build_or(&rows).0
}

/// A row part-way through the walk: the rules not yet consumed, plus the
/// warnings raised against the row as a whole.
type Row<'a, 'r> = (&'r [SyntaxRule<'a>], &'r [Warning]);

/// Groups `rows` by their leading rule and returns the OR of those groups,
/// along with the warnings of any row that terminated at this level (they
/// belong to the caller's rule node).
fn build_or<'a, 'r>(rows: &[Row<'a, 'r>]) -> (Option<Node<'a>>, Vec<Warning>) {
    let mut ends_here = false;
    let mut terminal: Vec<Warning> = Vec::new();
    // Insertion-ordered grouping: key -> (leading rule, tails of that group).
    let mut groups: Vec<(String, &'r SyntaxRule<'a>, Vec<Row<'a, 'r>>)> = Vec::new();

    for (rules, warnings) in rows {
        match rules.split_first() {
            // Row exhausted: it is satisfied by the prefix alone.
            None => {
                ends_here = true;
                for w in warnings.iter() {
                    if !terminal.contains(w) {
                        terminal.push(w.clone())
                    }
                }
            }
            Some((head, tail)) => {
                let key = rule_key(head);
                match groups.iter_mut().find(|(k, _, _)| *k == key) {
                    Some((_, _, tails)) => tails.push((tail, warnings)),
                    None => groups.push((key, head, vec![(tail, warnings)])),
                }
            }
        }
    }

    // `A or (A and B)` is just `A`, so a row ending here absorbs its siblings.
    if ends_here {
        return (None, terminal);
    }

    let mut children: Vec<Node<'a>> = groups
        .into_iter()
        .map(|(_, head, tails)| {
            let (subtree, warnings) = build_or(&tails);
            let rule = Node::Rule {
                property: head.property,
                operator: head.operator,
                value: value_to_json(&head.value),
                warnings: warnings.iter().map(Warning::as_str).collect(),
            };
            match subtree {
                None => rule,
                // Keep AND chains flat: `A and B and (…)`, not `A and (B and (…))`.
                Some(Node::And { children: mut inner }) => {
                    let mut c = vec![rule];
                    c.append(&mut inner);
                    Node::And { children: c }
                }
                Some(other) => Node::And {
                    children: vec![rule, other],
                },
            }
        })
        .collect();

    match children.len() {
        0 => (None, Vec::new()),
        1 => (children.pop(), Vec::new()),
        _ => (Some(Node::Or { children }), Vec::new()),
    }
}

fn dedup_rules<'a>(rules: &[SyntaxRule<'a>]) -> Vec<SyntaxRule<'a>> {
    let mut seen: Vec<String> = Vec::with_capacity(rules.len());
    let mut out = Vec::with_capacity(rules.len());
    for rule in rules {
        let key = rule_key(rule);
        if !seen.contains(&key) {
            seen.push(key);
            out.push(rule.clone());
        }
    }
    out
}

/// Identity of a rule for grouping. `RuleValue::to_string` consumes `self`,
/// hence the clone.
fn rule_key(rule: &SyntaxRule) -> String {
    format!(
        "{} {} {}",
        rule.property,
        rule.operator,
        rule.value.clone().to_string()
    )
}

fn value_to_json(value: &RuleValue) -> Value {
    match value {
        RuleValue::String(s) => Value::String((*s).to_string()),
        RuleValue::Integer(i) => Value::from(*i),
        RuleValue::Boolean(b) => Value::Bool(*b),
        RuleValue::Array(a) => Value::Array(a.iter().map(value_to_json).collect()),
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::syntax::parser::{order_rules, parse_rulebuilder};

//     fn tree_of(rules: &str) -> Value {
//         let dnf = order_rules(parse_rulebuilder(rules).expect("parses"));
//         serde_json::to_value(build_tree(&dnf)).expect("serialises")
//     }

//     #[test]
//     fn factors_shared_prefixes() {
//         let json = tree_of(
//             r#"user.accountEnabled -eq true and (
//                  (user.department -eq "d1" and (user.jobTitle -eq "a" or user.jobTitle -eq "b"))
//                  or (user.department -eq "d2" and user.jobTitle -eq "c")
//                )"#,
//         );
//         println!("{}", serde_json::to_string_pretty(&json).unwrap());

//         // accountEnabled is shared by every row, so it is hoisted to a single
//         // node at the top of an AND, with the departments as an OR beneath it.
//         assert_eq!(json["type"], "and");
//         assert_eq!(json["children"][0]["property"], "user.accountEnabled");
//         assert_eq!(json["children"][1]["type"], "or");
//         assert_eq!(json["children"][1]["children"].as_array().unwrap().len(), 2);
//     }

//     #[test]
//     fn absorbs_rows_subsumed_by_a_prefix() {
//         // `A or (A and B)` == `A`
//         let json = tree_of(r#"user.userType -eq "Member" or (user.userType -eq "Member" and user.accountEnabled -eq true)"#);
//         assert_eq!(json["type"], "rule");
//         assert_eq!(json["property"], "user.userType");
//     }

//     #[test]
//     fn empty_dnf_is_null() {
//         assert!(build_tree(&Vec::new()).is_none());
//     }
// }
