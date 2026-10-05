//! Normalises a `Dnf` so equivalent inputs produce the same output.

use std::collections::HashMap;

use crate::model::{Condition, Dnf};

/// Runs every tidy step: drops repeated nodes, orders nodes by frequency and
/// then drops repeated rules (which the first two steps can expose).
pub fn tidy(dnf: Dnf<'_>) -> Dnf<'_> {
    dedup_rules(order_nodes(dedup_nodes(dnf)))
}

/// Removes repeated nodes within each rule, keeping the first occurrence.
pub fn dedup_nodes(mut dnf: Dnf<'_>) -> Dnf<'_> {
    for rule in dnf.iter_mut() {
        let mut seen = Vec::new();
        rule.nodes.retain(|node| {
            if seen.contains(node) {
                false
            } else {
                seen.push(node.clone());
                true
            }
        })
    }
    dnf
}

/// Sorts each rule's nodes so the most common conditions come first.
///
/// Putting shared conditions at the front of every rule is what lets
/// `reconstruct` factor them out as common parents. Counts accumulate as the
/// rules are walked, so each rule is sorted by the counts seen so far. The
/// sort is stable, so ties keep their input order.
pub fn order_nodes(mut dnf: Dnf<'_>) -> Dnf<'_> {
    let mut frequency: HashMap<Condition<'_>, i32> = HashMap::new();

    for rule in dnf.iter_mut() {
        for node in rule.nodes.iter() {
            *frequency.entry(node.condition).or_insert(0) += 1;
        }
        rule.nodes.sort_by(|a, b| frequency[&b.condition].cmp(&frequency[&a.condition]));
    }
    dnf
}

/// Removes repeated rules, keeping the first occurrence.
pub fn dedup_rules(dnf: Dnf<'_>) -> Dnf<'_> {
    let mut out = Vec::new();
    for rule in dnf {
        if !out.contains(&rule) {
            out.push(rule)
        }
    }
    out
}
