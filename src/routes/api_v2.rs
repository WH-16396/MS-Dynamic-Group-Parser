use axum::{
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value::{self, Object}, json};
use crate::{
    condition::{
        Condition, 
        ConditionPart, 
        operator::Operator, 
        property::{
            self, 
            validate_property
        }, 
        value::Value as Item,
    }, 
    parse::parser::parse_rulebuilder, 
    rules::{
        checks::check_rules, 
        reconstruct::reconstruct, 
        tidy::*,
        Dnf,
        Node,
        Rule, // as Nodes,
    },
};

#[derive(Deserialize)]
pub struct ApiRequest {
    // String input of rule syntax
    input: Value,

    // Argument to return JSON tree format for the syntax, use when information 
    // needs to be displayed to a user or modified externally to the API response
    //
    // By default is TRUE if neither 'returnSyntax' or 'returnRules' have arguments
    // are present
    #[serde(rename = "returnJson")]
    return_json: Option<bool>,

    // Argument to return recompiled syntax string, use when automating since it's
    // more efficient
    #[serde(rename = "returnSyntax")]
    return_syntax: Option<bool>,

    // Argument to return individual AND rules for each unique branch of access 
    #[serde(rename = "returnRules")]
    return_rules: Option<bool>,

    // Checks all of the rules for common process errors
    #[serde(rename = "checkRisks")]
    warn_risks: Option<bool>,

    // Checks all of the rules for common process errors
    #[serde(rename = "globalConditions")]
    global_conditions: Option<Value>,
}

#[derive(Serialize)]
pub struct ApiResponse {
    success: bool,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    json: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    syntax: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rules: Option<Value>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    warnings: Vec<String>,
    #[serde(skip_serializing_if = "Value::is_null")]
    error: Value,
}

                                
#[derive(Debug)]
enum InputType<'j> {
    SyntaxAdd(String),
    SyntaxRemove(String),
    RuleAdd(Vec<Condition<'j>>),
    RuleRemoveOnly(Vec<ConditionPart<'j>>),
    RuleRemoveAll(Vec<ConditionPart<'j>>),
}

impl<'j> InputType<'j> {
    fn process_input(val: &'j Value) -> Result<Self, InputErr<'j>> {
        let t = match val {
            Value::Object(obj) => {

                fn syntax<'j>(obj: &'j Value) -> Result<String, InputErr<'j>> {
                    match obj {
                        Value::String(s) => Ok(s.clone()),
                        _ => Err(InputErr::SyntaxParse(obj)),
                    }
                }


                fn condition<'j, T: RuleCondition<'j>>(val: &'j Value) -> Result<Vec<T>, InputErr<'j>> {
                    Ok(match val {
                        Value::Object(_) => Ok(vec![
                            T::from_value(val)?
                        ]),
                        Value::Array(a) => a.into_iter()
                            .map(|x| T::from_value(x))
                            .collect(),
                        _ => Err(InputErr::DataParse(val)),
                    }?)
                }

                
                if let Some(data) = obj.get_key_value("data") {

                    match obj.get_key_value("type")
                        .ok_or(InputErr::TypeExist(&val))?.1 
                    {
                        Value::String(s) => 
                            match s.as_str() {
                                "syntaxAdd" =>      Ok(InputType::SyntaxAdd(syntax(data.1)?)),
                                "syntaxRemove" =>   Ok(InputType::SyntaxRemove(syntax(data.1)?)),
                                "ruleAdd" =>        Ok(InputType::RuleAdd(condition(data.1)?)),
                                "ruleRemoveOnly" => Ok(InputType::RuleRemoveOnly(condition(data.1)?)),
                                "ruleRemoveAll" =>  Ok(InputType::RuleRemoveAll(condition(data.1)?)),
                                _ =>                Err(InputErr::TypeParse(val)),
                            },
                        _ => Err(InputErr::TypeParse(val)),
                    }
                } else { Err(InputErr::DataExist(val)) }
            },
            Value::String(s) => Ok(Self::SyntaxAdd(s.clone())),
            _ => Err(InputErr::InputParse(val)),
        };

        println!("{:#?}", t); t
    }
}

trait RuleCondition<'j> {
    fn from_value(val: &'j Value) -> Result<Self, InputErr<'j>> where Self: Sized;
}

impl<'j> RuleCondition<'j> for ConditionPart<'j> {
    fn from_value(val: &'j Value) -> Result<Self, InputErr<'j>> {
        match val {
            Value::Object(obj) => Ok(Self {
                property: match obj.get_key_value("property").ok_or(InputErr::PropertyParse(val))?.1 {
                    Value::String(property) => if property == "*" { None } else { 
                        Some(validate_property(property).ok_or(InputErr::PropertyParse(val))?) 
                    },
                    _ => return Err(InputErr::PropertyParse(val)),
                },
                operator: match obj.get_key_value("operator").ok_or(InputErr::OperatorParse(val))?.1 {
                    Value::String(operator) => if operator == "*" { None } else { 
                        let parsed = Operator::from_str(operator).ok_or(InputErr::OperatorParse(val))?;

                        if parsed.is_single() {
                            Some(parsed)
                        } else {
                            return Err(InputErr::OperatorArray(val))
                        } 
                    },
                    _ => return Err(InputErr::OperatorParse(val)),
                },
                value: match obj.get_key_value("value").ok_or(InputErr::ItemParse(val))?.1 {
                    Value::String(s) => if s == "*" { None } else { Some(Item::String(s)) },
                    Value::Number(n) => Some(Item::Number(n.as_i64().ok_or(InputErr::ItemParse(val))?)),
                    Value::Bool(b) => Some(Item::Boolean(*b)),
                    Value::Null => Some(Item::Null),
                    _ => return Err(InputErr::ItemParse(val)),
                } 
            }),
            _ => Err(InputErr::ConditionParse(val)),
        }
    }
}

impl<'j> RuleCondition<'j> for Condition<'j> {
    fn from_value(val: &'j Value) -> Result<Self, InputErr<'j>>  {
        match ConditionPart::from_value(val) {
            Ok(part) => Ok(Self {
                property: part.property.ok_or(InputErr::ConditionBlank(val))?,
                operator: part.operator.ok_or(InputErr::ConditionBlank(val))?,
                value: part.value.ok_or(InputErr::ConditionBlank(val))?,
            }),
            Err(e) => Err(e),
        }
    }
}

impl Operator {
    fn from_str(str: &str) -> Option<Self> {
        let arg = if &str[..1] == "-" {&str[1..]} else {&str};

        let matched =
        if arg.eq_ignore_ascii_case("plus")             { Self::Add }                   else 
        if arg.eq_ignore_ascii_case("all")              { Self::All }                   else 
        if arg.eq_ignore_ascii_case("any")              { Self::Any }                   else 
        if arg.eq_ignore_ascii_case("contains")         { Self::Contains }              else 
        if arg.eq_ignore_ascii_case("endsWith")         { Self::EndsWith }              else 
        if arg.eq_ignore_ascii_case("eq")               { Self::Equals }                else 
        if arg.eq_ignore_ascii_case("ge")               { Self::GreaterThanOrEqual }    else 
        if arg.eq_ignore_ascii_case("in")               { Self::In }                    else 
        if arg.eq_ignore_ascii_case("le")               { Self::LessThanOrEqual }       else 
        if arg.eq_ignore_ascii_case("match")            { Self::Match }                 else 
        if arg.eq_ignore_ascii_case("notContains")      { Self::NotContains }           else 
        if arg.eq_ignore_ascii_case("notEndsWith")      { Self::NotEndsWith }           else 
        if arg.eq_ignore_ascii_case("ne")               { Self::NotEquals }             else 
        if arg.eq_ignore_ascii_case("notIn")            { Self::NotIn }                 else 
        if arg.eq_ignore_ascii_case("notMatch")         { Self::NotMatch }              else 
        if arg.eq_ignore_ascii_case("notStartsWith")    { Self::NotStartsWith }         else 
        if arg.eq_ignore_ascii_case("startsWith")       { Self::StartsWith }            else 
        if arg.eq_ignore_ascii_case("minus")            { Self::Subtract }              else 
        { return None };
        
        Some(matched)
    }
}



#[derive(Debug)]
enum InputErr<'j> {
    TypeParse(&'j Value),
    TypeExist(&'j Value),
    DataParse(&'j Value),
    DataExist(&'j Value),
    SyntaxParse(&'j Value),
    ConditionParse(&'j Value),
    ConditionBlank(&'j Value),
    PropertyParse(&'j Value),
    OperatorParse(&'j Value),
    OperatorArray(&'j Value),
    ItemParse(&'j Value),
    InputParse(&'j Value),
    InputExist(&'j Value),
}
impl<'j> InputErr<'j> {
    fn as_message(&self) -> &str {
        match self {
            Self::TypeParse(_) => "Invalid value for the key 'type'.",
            Self::TypeExist(_) => "Could not find the key 'type' within the input object.",
            Self::DataParse(_) => "Invalid value for the key 'data', please ensure rule inputs are an object or object array.",
            Self::DataExist(_) => "Could not find the key 'data' within the input object.",
            Self::SyntaxParse(_) => "Failed to parse 'data', please ensure direct syntax inputs are a string.",
            Self::ConditionParse(_) => "Failed to parse rule condition.",
            Self::ConditionBlank(_) => "Failed to parse rule condition, please ensure that there are no wildcard values (*) for 'ruleAdd' types.",
            Self::PropertyParse(_) => "Failed to parse property.",
            Self::OperatorParse(_) => "Failed to parse operator.",
            Self::OperatorArray(_) => "Invalid operator input, cannot use an array operator.",
            Self::ItemParse(_) => "Failed to parse input value.",
            Self::InputParse(_) => "Invalid value for the key 'input'.",
            Self::InputExist(_) => "Could not find the key 'input' within the request.",
        }
    }
    fn into_value(self) -> Value {
        match self {
            Self::TypeParse(j) |
            Self::TypeExist(j) |
            Self::DataParse(j) |
            Self::DataExist(j) |
            Self::SyntaxParse(j) |
            Self::ConditionParse(j) |
            Self::ConditionBlank(j) |
            Self::PropertyParse(j) |
            Self::OperatorParse(j) |
            Self::OperatorArray(j) |
            Self::ItemParse(j) |
            Self::InputParse(j) |
            Self::InputExist(j) => j.clone()
        }
    }
}

#[derive(Debug)]
enum ExecErr {
    SyntaxParse,
}

impl<'j> InputType<'j> {
    fn to_executable(&'j self) -> Result<ExecType<'j>, ExecErr> {
        match self {
            Self::SyntaxAdd(s) => {
                match parse_rulebuilder(s) {
                    Ok(o) => {
                        // dnf.append(&mut o.clone()); 
                        // Ok(ExecType::Add(tidy_dnf(dnf.clone())))
                        Ok(ExecType::Add(tidy_dnf(o)))
                    },
                    Err(_) => Err(ExecErr::SyntaxParse),
                }
            },
            // Self::SyntaxRemove(s) => {
                
            // },
            Self::RuleAdd(r) => {
                Ok(ExecType::Add(vec![
                    r.iter().fold(
                        Rule::new(),
                        |mut rule, condition| 
                        {
                            rule.nodes.push(Node::from(condition.clone())); rule 
                        }
                    )
                ]))
            },
            Self::RuleRemoveOnly(r) => { Ok(ExecType::RemoveAll(r.clone())) },
            Self::RuleRemoveAll(r) => { Ok(ExecType::RemoveAll(r.clone())) },
            _ => return Err(ExecErr::SyntaxParse),
        }
    }
}


enum ExecType<'a> {
    Add(Dnf<'a>),
    RemoveOnly(Vec<ConditionPart<'a>>),
    RemoveAll(Vec<ConditionPart<'a>>),
}

impl<'a> ExecType<'a> {
    fn execute(self, mut dnf: Dnf<'a>) -> Dnf<'a> {
        match self {
            Self::Add(a) => {
                dnf.append(&mut a.clone()); 
                tidy_dnf(dnf.clone())
            },
            // Self::RemoveOnly(ro) => {
            //     dnf.into_iter()
            //         .filter(|rule| 
            //             iters_equal_anyorder(
            //                 ro.clone().into_iter(), 
            //                 rule.nodes.clone().into_iter()
            //                     .map(|x| ConditionPart::from(x.condition))
            //             )
            //         ).collect()
            // },
            // Self::RemoveOnly(ro) => {
            //     dnf.into_iter()
            //         .filter(|rule| 
            //             rule.nodes.clone().into_iter()
            //                 .map(|x| ConditionPart::from(x.condition));
            //             ro.clone().into_iter()
            //         ).collect()
            // },
            Self::RemoveOnly(ro) => {
                dnf
            },
            Self::RemoveAll(ra) => {
                dnf.into_iter().filter(|rule| 
                    ra.iter().fold(
                        false,
                        |keep, check| {
                            match keep {
                                false => {
                                    !rule.contains(check)
                                },
                                true => true,
                            }
                        }
                    )
                ).collect()
            },
        }
    }
}


pub fn order_test<'a>(mut parts: Vec<ConditionPart<'a>>) -> Vec<ConditionPart<'a>> {
    use std::collections::HashMap;
    let mut map: HashMap<ConditionPart<'_>, i32> = HashMap::new();

    for part in parts.iter() {
        *map.entry(part.clone()).or_insert(0) += 1;
    }
    parts.sort_by(|a, b| 
        map.get(&b.clone()).unwrap()
        .cmp(map.get(&a.clone()).unwrap())
    ); 
    parts
}

// https://users.rust-lang.org/t/assert-vectors-equal-in-any-order/38716/10
use std::{hash::Hash,collections::{HashMap,hash_map::Entry}};
fn iters_equal_anyorder<T: Eq + Hash>(i1:impl Iterator<Item = T>, i2: impl Iterator<Item = T>) -> bool {
    fn get_lookup<T: Eq + Hash>(iter:impl Iterator<Item = T>) -> HashMap<T, usize> {
        let mut lookup = HashMap::<T, usize>::new();
        for value in iter {
            match lookup.entry(value) {
                Entry::Occupied(entry) => { *entry.into_mut() += 1; },
                Entry::Vacant(entry) => { entry.insert(0); }
            }
        }
        lookup
    }
    get_lookup(i1) == get_lookup(i2)
}

pub async fn api_handler(Json(req): Json<ApiRequest>) -> Response {

    let input_processed: Result<Vec<InputType<'_>>, InputErr<'_>> = match &req.input {
        Value::Array(a) => if a.len() != 0 { a.iter().map(InputType::process_input).collect() } else { Err(InputErr::InputExist(&req.input)) },
        Value::String(_) |
        Value::Object(_) => vec![InputType::process_input(&req.input)].into_iter().collect(),
        _ => {println!("{:#?}", req.input); Ok(Vec::new())},
    };

    match input_processed {
        Ok(o) => { 
            let executed: Result<Dnf<'_>, ExecErr> = o.iter()
                .fold(
                    Ok(Vec::new()), 
                    |result, x| 
                    {
                        match result {
                            Ok(dnf) => match x.to_executable() {
                                Ok(exec) => Ok(exec.execute(dnf)),
                                Err(e) => Err(e),
                            },
                            Err(e) => Err(e),
                        }
                    }
                );
            
            Json(ApiResponse {
                success: true,
                message: String::from("Success"),
                json: None,
                syntax: None, // Some(format!("{:#?}", executed)),
                rules: None,
                warnings: Vec::new(),
                error: serde_json::to_value(tidy_dnf(executed.unwrap())).unwrap(), // Value::Null,
            }).into_response()},


        Err(e) => {
            println!("{:#?}", e);
            Json(ApiResponse {
                success: false,
                message: String::from(e.as_message()),
                json: None,
                syntax: None,
                rules: None,
                warnings: Vec::new(),
                error: e.into_value(),
            }).into_response()
        },
    }
}

fn parse_err_message<R>(error: pest::error::Error<R>, message: &str) -> Value {
    use pest::error::InputLocation;
    let location: (usize, usize) = match error.location {
        InputLocation::Pos(p) => (p, p),
        InputLocation::Span(s) => s,
    };
    json!({
        "message": String::from(message),
        "location": location,
    })
}