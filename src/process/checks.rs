use crate::{
    condition::{
        Property,
        Condition,
        ConditionPart, 
        operator::Operator, 
        value::Value
    },
    rules::{
        Dnf, Rule, Warning,
    }
};


impl<'a> Condition<'a> {
    pub fn contains(&self, check_node: &ConditionPart) -> bool {
        if match check_node.property {
                Some(s) => s == self.property,
                None => true,
            } &&
            (Some(self.operator) == check_node.operator || !check_node.operator.is_some()) &&
            (Some(self.value) == check_node.value || !check_node.value.is_some()) 
        {
            return true
        }
        false
    }
}
impl<'a> Rule<'a> {
    pub fn contains(&self, check_node: &ConditionPart) -> bool {
        for node in self.nodes.iter() {
            if node.condition.contains(check_node) {
                return true
            }
        }
        false
    }
}

pub fn check_rules<'a>(dnf: Dnf<'a>) -> Dnf<'a> {
    let mut out: Dnf<'a> = Vec::new();
    for r in dnf.into_iter() {
        let mut rule: Rule<'a> = r.clone();
        let mut warnings: Vec<(Vec<&ConditionPart<'_>>, &ConditionPart<'_>, &Warning)> = Vec::new(); 

        // CHECKS
        
        // --------------------------
        // Check for enabled accounts
        // --------------------------
        warnings.push((
            vec![
                &ConditionPart {
                    property: Some(Property("user.accountEnabled")),
                    operator: Some(Operator::Equals),
                    value: Some(Value::Boolean(true))
                }, 
            ],
            &ConditionPart {
                property: None,
                operator: None,
                value: None
            },
            &Warning::NotEnabled
        ));
        
        // -------------------------
        // Check for member accounts
        // -------------------------
        warnings.push((
            vec![
                &ConditionPart {
                    property: Some(Property("user.userType")),
                    operator: Some(Operator::Equals),
                    value: Some(Value::String("Member"))
                }, 
            ],
            &ConditionPart {
                property: None,
                operator: None,
                value: None
            },
            &Warning::NotMember
        ));

        // -------------------------------------------------
        // Check for department when a jobtitle is specified
        // -------------------------------------------------
        warnings.push((
            vec![
                &ConditionPart {
                    property: Some(Property("user.department")),
                    operator: Some(Operator::Equals),
                    value: None
                }, 
            ],
            &ConditionPart {
                    property: Some(Property("user.jobTitle")),
                    operator: Some(Operator::Equals),
                    value: None
            },
            &Warning::MissingDept
        ));

        for criteria in warnings {
            rule = match_rule(
                &rule, 
                criteria.0,
                criteria.1,
                criteria.2,
            );
        }
        
        out.push(rule);
    }
    out
}


fn match_rule<'a>(
    rule: &Rule<'a>, 
    criteria: Vec<&ConditionPart>, 
    node_criteria: &ConditionPart, 
    warning: &'a Warning
) -> Rule<'a> {
    let mut out: Rule<'a> = rule.clone();

    let contains_criteria = criteria.into_iter().fold(true, |matched, node| match matched {
        true => rule.contains(node),
        false => false,
    });

    let is_warning = if !contains_criteria {
        out.nodes.iter_mut()
            .fold(false, |matched, node| match node.condition.contains(node_criteria) {
                true => {node.warnings.push(warning); true},
                false => matched,
            })
    } else { 
        false 
    };

    if is_warning {out.warnings.push(*warning);}

    out
}


