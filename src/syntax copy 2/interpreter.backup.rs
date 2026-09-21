// use crate::syntax::parser::Syntax;

// pub fn clean_rules<'a>(segment: &Syntax<'a>) -> Option<Syntax<'a>> {
//     use crate::syntax::parser::Syntax::*;

//     match segment {
//         Brackets(b) => {
//             let mut contents: Vec<Syntax<'a>> = b.iter().filter_map(clean_rules).collect();
//             match contents.len() {
//                 0 => None,
//                 1 => match contents.pop().unwrap() {
//                     Brackets(inner) => Some(Brackets(inner)),
//                     rule => Some(Brackets(vec![rule])),
//                 },
//                 _ => Some(Brackets(contents)),
//             }
//         }
//         Rule(r) => {
//             let contents: Vec<Syntax<'a>> = r.iter().filter_map(clean_rules).collect();
//             Some(Rule(contents))
//         },
//         Array(a) => {
//             let contents: Vec<Syntax<'a>> = a.iter().filter_map(clean_rules).collect();
//             Some(Array(contents))
//         },
//         Comparator(c) => Some(Comparator(c)),
//         Operator(op) => Some(Operator(op)),
//         Object(ob) => Some(Object(ob)),
//         String(s) => Some(String(s)),
//         Boolean(b) => Some(Boolean(*b)),
//         Null => Some(Null),
//     }
// }

// /// Recursively determines whether `user.accountEnabled -eq true` is guaranteed
// /// to hold on every path through the rule's and/or tree. For an `and`, the
// /// combined guarantee holds if *either* side already guarantees it (both must
// /// be true for the `and` to be true, so one guaranteeing party is enough). For
// /// an `or`, the guarantee only holds if *both* sides guarantee it, since
// /// either side alone could satisfy the `or` without the other.
// pub fn check_account_enabled(node: &Syntax) -> bool {
//     use crate::syntax::parser::Syntax::*;

//     match node {
//         Rule(parts) => matches!(
//             parts.as_slice(),
//             [Object(obj), Operator(op), Boolean(true)]
//                 if obj.trim_start_matches("user.").eq_ignore_ascii_case("accountEnabled")
//                     && op.eq_ignore_ascii_case("-eq")
//         ),
//         Brackets(items) => {
//             let mut items = items.iter();
//             let Some(first) = items.next() else { return false };
//             let mut guaranteed = check_account_enabled(first);
//             while let (Some(Comparator(c)), Some(next)) = (items.next(), items.next()) {
//                 let next_guaranteed = check_account_enabled(next);
//                 guaranteed = if c.eq_ignore_ascii_case("and") {
//                     guaranteed || next_guaranteed
//                 } else {
//                     guaranteed && next_guaranteed
//                 };
//             }
//             guaranteed
//         }
//         _ => false,
//     }
// }

// pub fn check_job_requires_department(node: &Syntax) -> bool {
//     use crate::syntax::parser::Syntax::*;

//     println!("Checking job title status: {:#?}", node);

//     let res = match node {
//         Rule(parts) => matches!(
//             parts.as_slice(),
//             [Object(obj), Operator(op), String(_)]
//                 if obj.trim_start_matches("user.").eq_ignore_ascii_case("department")
//                     && op.eq_ignore_ascii_case("-eq")
//         ),
//         Brackets(items) => {
//             let mut items = items.iter();
//             let Some(first) = items.next() else { return false };
//             let mut guaranteed = check_job_requires_department(first);
//             while let (Some(Comparator(c)), Some(next)) = (items.next(), items.next()) {
//                 let next_guaranteed = check_job_requires_department(next);
//                 guaranteed = if c.eq_ignore_ascii_case("and") {
//                     guaranteed || next_guaranteed
//                 } else {
//                     guaranteed && next_guaranteed
//                 };
//             }
//             guaranteed
//         }
//         _ => false,
//     };

//     println!("result: {}", res);

//     res
// }

// /// ANDs a flat list of `object operator value` criteria onto an existing rule
// /// tree, e.g. `add_user_criteria(tree, &[("user.department", "-eq", "d1")])`
// /// produces `(tree) and (user.department -eq "d1")`. Criteria within the
// /// added group are themselves ANDed together. A no-op if `criteria` is empty.
// pub fn add_user_criteria<'a>(tree: Syntax<'a>, criteria: &[(&'a str, &'a str, &'a str)]) -> Syntax<'a> {
//     use crate::syntax::parser::Syntax::*;

//     if criteria.is_empty() {
//         return tree;
//     }

//     let mut items = Vec::with_capacity(criteria.len() * 2 - 1);
//     for (i, (object, operator, value)) in criteria.iter().enumerate() {
//         if i > 0 {
//             items.push(Comparator("and"));
//         }
//         items.push(Rule(vec![Object(object), Operator(operator), String(value)]));
//     }

//     Brackets(vec![tree, Comparator("and"), Brackets(items)])
// }

// pub fn syntax_warnings(tree: &Syntax) -> Vec<String> {
//     let mut warnings = Vec::new();
//     if !check_account_enabled(tree) {
//         warnings.push(String::from(
//             "\"user.accountEnabled -eq true\" is not applied universally, \
//              disabled accounts can still match this rule.",
//         ))
//     }
//     if !check_job_requires_department(tree) {
//         warnings.push(String::from(
//             "\"user.jobTitle\" is applied without \"user.department\", \
//              unintended users could match this rule.",
//         ))
//     }
//     warnings
// }

// /// Merges maximal runs of `or`-joined `object -eq value` / `object -in [...]`
// /// rules that share the same object into a single `object -in [...]` rule,
// /// e.g. `user.jobTitle -eq "j1" or user.jobTitle -eq "j2"` becomes
// /// `user.jobTitle -in ["j1", "j2"]`. Rules joined by `and`, or on different
// /// objects, are left untouched.
// pub fn compress_rules<'a>(node: &Syntax<'a>) -> Syntax<'a> {
//     use crate::syntax::parser::Syntax::*;

//     match node {
//         Brackets(items) => Brackets(compress_segment(items)),
//         Rule(r) => Rule(r.iter().map(compress_rules).collect()),
//         Array(a) => Array(a.iter().map(compress_rules).collect()),
//         Comparator(c) => Comparator(c),
//         Operator(op) => Operator(op),
//         Object(ob) => Object(ob),
//         String(s) => String(s),
//         Boolean(b) => Boolean(*b),
//         Null => Null,
//     }
// }

// /// If `node` is (or is a single-wrapped bracket around) an `object -eq value`
// /// or `object -in [values]` rule, returns the object name and its value(s) so
// /// it can take part in a merge.
// fn as_mergeable<'a>(node: &Syntax<'a>) -> Option<(&'a str, Vec<Syntax<'a>>)> {
//     use crate::syntax::parser::Syntax::*;

//     match node {
//         Rule(parts) => match parts.as_slice() {
//             [Object(obj), Operator(op), value] if op.eq_ignore_ascii_case("-eq") => {
//                 Some((obj, vec![value.clone()]))
//             }
//             [Object(obj), Operator(op), Array(values)] if op.eq_ignore_ascii_case("-in") => {
//                 Some((obj, values.clone()))
//             }
//             _ => None,
//         },
//         Brackets(inner) if inner.len() == 1 => as_mergeable(&inner[0]),
//         _ => None,
//     }
// }

// fn compress_segment<'a>(items: &[Syntax<'a>]) -> Vec<Syntax<'a>> {
//     use crate::syntax::parser::Syntax::*;

//     let items: Vec<Syntax<'a>> = items.iter().map(compress_rules).collect();

//     let mut result: Vec<Syntax<'a>> = Vec::new();
//     let mut i = 0;
//     while i < items.len() {
//         let Some((obj, first_values)) = as_mergeable(&items[i]) else {
//             result.push(items[i].clone());
//             i += 1;
//             continue;
//         };

//         let mut values = first_values;
//         let mut merged_count = 1;
//         let mut j = i + 1;
//         while j + 1 < items.len() {
//             let is_or = matches!(&items[j], Comparator(c) if c.eq_ignore_ascii_case("or"));
//             if !is_or {
//                 break;
//             }
//             let Some((next_obj, next_values)) = as_mergeable(&items[j + 1]) else {
//                 break;
//             };
//             if !next_obj.eq_ignore_ascii_case(obj) {
//                 break;
//             }
//             values.extend(next_values);
//             merged_count += 1;
//             j += 2;
//         }

//         if merged_count > 1 {
//             result.push(Rule(vec![Object(obj), Operator("-in"), Array(values)]));
//             i = j;
//         } else {
//             result.push(items[i].clone());
//             i += 1;
//         }
//     }
//     result
// }

// pub fn serialize_syntax(val: &Syntax) -> String {
//     use crate::syntax::parser::Syntax::*;

//     match val {
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
//     }
// }