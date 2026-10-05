//! Flags rules that could let unintended users into the group.

use crate::model::{ConditionPattern, Dnf, Operator, Property, Rule, Value, Warning};

/// A risk check: when a rule is missing any of the `required` conditions,
/// every node matching `flagged` gets `warning`, and so does the rule itself
/// if at least one node was flagged.
struct Check {
    required: &'static [ConditionPattern<'static>],
    flagged: ConditionPattern<'static>,
    warning: Warning,
}

const CHECKS: &[Check] = &[
    // Every rule should exclude disabled accounts
    Check {
        required: &[ConditionPattern {
            property: Some(Property("user.accountEnabled")),
            operator: Some(Operator::Equals),
            value: Some(Value::Boolean(true)),
        }],
        flagged: ConditionPattern::ANY,
        warning: Warning::NotEnabled,
    },
    // Every rule should exclude guest / external accounts
    Check {
        required: &[ConditionPattern {
            property: Some(Property("user.userType")),
            operator: Some(Operator::Equals),
            value: Some(Value::String("Member")),
        }],
        flagged: ConditionPattern::ANY,
        warning: Warning::NotMember,
    },
    // Job titles aren't unique across departments, so a job title rule
    // should also pin the department
    Check {
        required: &[ConditionPattern {
            property: Some(Property("user.department")),
            operator: Some(Operator::Equals),
            value: None,
        }],
        flagged: ConditionPattern {
            property: Some(Property("user.jobTitle")),
            operator: Some(Operator::Equals),
            value: None,
        },
        warning: Warning::MissingDept,
    },
];

/// Runs every check against every rule, attaching warnings in place.
pub fn check_rules(mut dnf: Dnf<'_>) -> Dnf<'_> {
    for rule in dnf.iter_mut() {
        for check in CHECKS {
            apply_check(rule, check);
        }
    }
    dnf
}

fn apply_check(rule: &mut Rule<'_>, check: &Check) {
    if check.required.iter().all(|pattern| rule.contains(pattern)) {
        return;
    }

    let mut flagged_any = false;
    for node in rule.nodes.iter_mut() {
        if node.condition.matches(&check.flagged) {
            node.warnings.push(check.warning);
            flagged_any = true;
        }
    }

    if flagged_any {
        rule.warnings.push(check.warning);
    }
}
