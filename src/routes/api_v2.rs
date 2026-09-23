use axum::{
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value::{self, Object}, json};
use crate::{
    condition::{
        Condition, ConditionPart, operator::Operator, property::{self, validate_property}, value::Value as Item,
    }, parse::parser::parse_rulebuilder, rules::{
        checks::check_rules, 
        reconstruct::reconstruct, 
        tidy::*,
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
    errors: Value,
}

// Syntax input
// {
//     "type": "syntaxAdd",
//     "data": "syntax rules"
// }

// Rule input
// {
//     "type": "syntaxAdd",
//     "data": [
//         {
//             "property": "user.property",
//             "operator": "-eq",
//             "value": "Value"
//         }
//     ]
// }

#[derive(Debug)]
enum InputType<'j> {
    SyntaxAdd(String),
    SyntaxRemove(String),
    RuleAdd(Vec<Condition<'j>>),
    RuleRemoveOnly(Vec<ConditionPart<'j>>),
    RuleRemoveAll(Vec<ConditionPart<'j>>),
}

// pub struct InputErr {
//     message: &'static str,
//     json: Value,
// }
#[derive(Debug)]
enum InputErr<'j> {
    TypeParse(&'j Value),
    TypeExist(&'j Value),
    DataParse(&'j Value),
    DataExist(&'j Value),
    SyntaxParse(&'j Value),
    ConditionParse(&'j Value),
    PropertyParse(&'j Value),
    OperatorParse(&'j Value),
    ItemParse(&'j Value),
    InputParse(&'j Value),
    InputExist(&'j Value),
}
impl<'j> InputErr<'j> {
    fn into_value(self) -> Value {
        match self {
            Self::TypeParse(j) |
            Self::TypeExist(j) |
            Self::DataParse(j) |
            Self::DataExist(j) |
            Self::SyntaxParse(j) |
            Self::ConditionParse(j) |
            Self::PropertyParse(j) |
            Self::OperatorParse(j) |
            Self::ItemParse(j) |
            Self::InputParse(j) |
            Self::InputExist(j) => j.clone()
        }
    }
}
// enum InputErr {
//     TypeParse(Value),
//     TypeExist(Value),
//     DataParse(Value),
//     DataExist(Value),
//     SyntaxParse(Value),
//     ConditionParse(Value),
//     PropertyParse(Value),
//     OperatorParse(Value),
//     ItemParse(Value),
//     InputParse(Value),
// }


// Err(InputErr {
//     message: "REPLACE ERROR",
//     json: &val,
// })


                                // .ok_or(InputErr {
                                //     message: "REPLACE ERROR",
                                //     json: obj.clone(),
                                // })

impl<'j> InputType<'j> {
    fn from_value(val: &'j Value) -> Result<Self, InputErr<'j>> {
        match val {
            Value::Object(obj) => {

                // Conditions are used for 'ruleAdd' type since it needs defined data
                fn condition<'j>(obj: &'j Value) -> Result<Vec<Condition<'j>>, InputErr<'j>> {
                    Ok(match obj {
                        Value::Object(_) => Ok(vec![
                            Condition::from_value(obj).ok_or(InputErr::ConditionParse(obj))?
                        ]),
                        Value::Array(a) => a.into_iter()
                            .map(
                                |x| 
                                Condition::from_value(x)
                                .ok_or(InputErr::ConditionParse(obj))
                            ).collect(),
                        _ => Err(InputErr::ConditionParse(obj)),
                    }?
                    )
                }

                // // Condition parts are used for 'ruleRemove' types since they need optional data
                // fn condition_part<'a>(data: &'a Value) -> Result<ReqInputType<'a>, InputErr> {
                //     Ok(ReqInputType::RuleAdd(match data {
                //         Value::Object(_) => Ok(vec![
                //             Condition::from_value(&data).ok_or(InputErr)?
                //         ]),
                //         Value::Array(a) => a.into_iter()
                //             .map(
                //                 |x| 
                //                 Condition::from_value(&x)
                //                     .ok_or(InputErr)
                //             ).collect(),
                //         _ => Err(InputErr),
                //     }?
                //     ))
                // }
                
                match obj.get_key_value("type")
                    .ok_or(InputErr::TypeExist(&val))?.1 
                {
                    Value::String(s) => 
                        match s.as_str() {
                            // "syntaxAdd" => rule_add(obj.get_key_value("data").ok_or(InputErr)?.1),
                            "ruleAdd" => Ok(InputType::RuleAdd(condition(obj.get_key_value("data")
                                .ok_or(InputErr::DataExist(val))?
                                .1
                            )?)),
                            // "ruleAdd" => rule_add(obj.get_key_value("data").ok_or(InputErr)?.1),
                            _ => Err(InputErr::TypeParse(val)),
                        },
                    _ => Err(InputErr::TypeParse(val)),
                }
            },
            Value::String(s) => Ok(Self::SyntaxAdd(s.to_string())),
            _ => Err(InputErr::InputParse(val)),
        }
    }
    // fn match_type(obj: &'a Map<String, Value>) -> Result<Self, InputErr> {

        
    // }
}


// impl<'a> ConditionPart<'a> {
//     pub fn from_value(val: &'a Value) -> Option<Self> {
//         match val {
//             Value::Object(obj) => Some(Self {
//                 property: match obj.get_key_value("property")?.1 {
//                     Value::String(property) => if property == "*" { None } else { Some(validate_property(property)?) },
//                     _ => return None,
//                 },
//                 operator: match obj.get_key_value("property")?.1 {
//                     Value::String(operator) => if operator == "*" { None } else { Some(Operator::from_str(operator)?) },
//                     _ => return None,
//                 }, //Operator::from_str(&String::from("-eq"))?,
//                 value: Item::Null,
//             }),
//             _ => None,
//         }
//     }
// }
impl<'j> Condition<'j> {
    pub fn from_value(val: &'j Value) -> Option<Self> {
        match val {
            Value::Object(obj) => Some(Self {
                property: match obj.get_key_value("property")?.1 {
                    Value::String(property) => validate_property(property)?,
                    _ => return None,
                },
                operator: match obj.get_key_value("operator")?.1 {
                    Value::String(operator) => Operator::from_str(operator)?,
                    _ => return None,
                },
                // operator: Operator::from_str(&String::from("-eq"))?,
                value: Item::Null,
            }),
            _ => None,
        }
    }
}
impl Operator {
    fn from_str(str: &str) -> Option<Self> {
        match if &str[..1] == "-" {&str[1..]} else {&str} {
            a => println!("'{a}'"),
        }
        ;Some(Self::Add)
    }
}
impl<'j> Item<'j> {
    pub fn from_value(val: &'j Value) -> Option<Self> {
        match val {
            Value::String(s) => Some(Self::String(s)),
            Value::Number(n) => Some(Self::Number(n.as_i64()?)),
            Value::Bool(b) => Some(Self::Boolean(*b)),
            Value::Null => Some(Self::Null),
            _ => None,
        }
    }
}

// struct 

// Syntax Add               String
// Vec Syntax Add/Remove    Vec<String>
// Vec RuleAdd              Vec<Condition>
// Vec RuleRemovePrecise    Vec<ConditionPart>
// Vec RuleRemoveAll        Vec<ConditionPart>

pub async fn api_handler(Json(req): Json<ApiRequest>) -> Response {

    // println!("{:#?}\n\n\n----------------------------------------------\n\n\n", req.input);

    // enum RequestInput {
    //     Rule {
    //         remove: bool,
    //         input_type: Vec<>,
    //     },

    // }

    


    // let f: Vec<Result<InputType<'_>, InputErr>> = if let Value::Array(a) = &req.input {
    //     a.iter().map(InputType::from_value).collect()
    // } else { Vec::new() };

    // let q = match &req.input {
    //     Value::Array(a) => a.iter().map(InputType::from_value)
    //         .collect::<Vec<Result<InputType<'_>, InputErr>>>(),
    //     Value::String(_) |
    //     Value::Object(_) => vec![InputType::from_value(&req.input)],
    //     _ => {println!("{:#?}", req.input); Vec::new()},
    // };
    let input_processed: Result<Vec<InputType<'_>>, InputErr<'_>> = match &req.input {
        Value::Array(a) => a.iter().map(InputType::from_value).collect(),
        Value::String(_) |
        Value::Object(_) => vec![InputType::from_value(&req.input)].into_iter().collect(),
        _ => {println!("{:#?}", req.input); Ok(Vec::new())},
    };

    match input_processed {
        Ok(o) => { ;
            Json(ApiResponse {
                success: true,
                message: String::from("Success"),
                json: None,
                syntax: None,
                rules: None,
                warnings: Vec::new(),
                errors: Value::Null,
            }).into_response()},
        Err(e) => {
            println!("{:#?}", e);
            Json(ApiResponse {
                success: false,
                message: String::from("Error parsing input"),
                json: None,
                syntax: None,
                rules: None,
                warnings: Vec::new(),
                errors: e.into_value(),
            }).into_response()
        },
    }



    // for item in q {
    //     match item {
    //         Ok(o) => {
    //             println!("\nSUCCESS: {:#?}\n", o);
    //         },
    //         Err(e) => {
    //             println!("\nERROR: {:#?}\n", e);
    //             Json(ApiResponse {
    //                 success: false,
    //                 message: String::from("Error parsing input"),
    //                 json: None,
    //                 syntax: None,
    //                 rules: None,
    //                 warnings: Vec::new(),
    //                 errors: vec![],
    //             }).into_response()
    //         },
    //     }
    // }

    // Json(ApiResponse {
    //     success: false,
    //     message: String::from("Error parsing input"),
    //     json: None,
    //     syntax: None,
    //     rules: None,
    //     warnings: Vec::new(),
    //     errors: vec![],
    // }).into_response()

    // let mut q = Vec::new();
    // let mut w: Vec<Result<InputType, InputErr>> = Vec::new();

    // match req.input {
    //     Value::String(s) => (),
    //     Value::Array(a) => 
    //     // {
    //     //     for child in a.into_iter() {
    //     //         println!("{:#?}\n\n\n\n\n", child);
                
    //     //         // match child {
    //     //         //     Value::Object(o) => println!("{:#?}", o),
    //     //         //     _ => (),
    //     //         // };

    //     //         q.push(InputType::from_value(&child));
    //     //     }

    //     // },
    //     _ => (),
    // }


    

    // If warnings are on then warn on the 
    // if Some(true) == req.warn_risks {
    //     dnf = check_rules(dnf);
    // }

    // // Defines the conditions for returning JSON as a default when no other conditions are selected
    // let return_json = req.return_json != Some(false) && req.return_syntax != Some(true) && req.return_rules != Some(true);

    // if return_json || req.return_syntax == Some(true) {
    //     let tree = reconstruct(&dnf);
        
    //     if return_json {
    //         res.json = tree.as_ref().and_then(|b| serde_json::to_value(b).ok());
    //     }

    //     if req.return_syntax == Some(true) {
    //         res.syntax = match tree {
    //             Some(s) => Some(s.to_syntax_string()),
    //             None => None,
    //         };
    //     }

    // }

    // if req.return_rules == Some(true) {
    //     res.rules = match serde_json::to_value(&dnf) {
    //         Ok(o) => Some(o),
    //         Err(_) => None,
    //     };
    // }
    // match parse_rulebuilder(req.input.as_str()) 
    // {
    //     Ok(mut dnf) => {
    //         let mut res = ApiResponse {
    //             success: true,
    //             message: String::from("Successfully parsed input"),
    //             // json: serde_json::from_str(&serialize_syntax(&tree)).unwrap(),
    //             json: None,
    //             syntax: None,
    //             rules: None,
    //             warnings: Vec::new(),
    //             errors: Vec::new(),
    //         };

    //         // Parses the input syntax string separately and merges into the main.
    //         // Handled first so that other arguments build off of merged rules
    //         if let Some(rules) = &req.merge_rules {
    //             match parse_rulebuilder(rules.as_str()) {
    //                 Ok(mut o) => dnf.append(&mut o),
    //                 Err(e) => res.errors.push(parse_err_message(e, "Failed to parse merge rules")),
    //             }
    //         }
            
    //         // // If the mergeRules argument is passed then add it to the dnf.
    //         // // Handled first so that other arguments build off of merged rules
    //         // if let Some(rules) = &req.modify_users {
    //         //     match parse_rulebuilder(rules.as_str()) {
    //         //         Ok(mut o) => dnf.append(&mut o),
    //         //         Err(_) => res.errors.push(String::from("Failed to parse merge rules.")),
    //         //     }
    //         // }

    //         // Cleans the rules e.g. duplicate rules
    //         dnf = remove_duplicates(dnf);
    //         dnf = order_rules(dnf);


            

    //         Json(res).into_response()
    //     }
        
    // }
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