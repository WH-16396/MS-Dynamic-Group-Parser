use axum::{
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use crate::{
    parse::{
        parser::parse_rulebuilder,
    },
    rules::{
        checks::check_rules, 
        tidy::*,
        reconstruct::reconstruct,
    }
};

#[derive(Deserialize)]
pub struct ApiRequest {
    // String input of rule syntax
    input: String,

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

    // A second input for rule syntax string which gets merged with the main rules
    // This should be used to add conditions
    #[serde(rename = "mergeRules", default)]
    merge_rules: Option<String>,
    
    // // A extra rule syntax string which gets parsed into individual rules, any existing
    // // rules that match the input criteria will be removed
    // // This should be used to remove users and will run before the merge rules
    // #[serde(rename = "removeRules", default)]
    // remove_rules: Option<String>,
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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    errors: Vec<Value>,
}

pub async fn syntax_api(Json(req): Json<ApiRequest>) -> Response {
    match parse_rulebuilder(req.input.as_str()) 
    {
        Ok(mut dnf) => {
            let mut res = ApiResponse {
                success: true,
                message: String::from("Successfully parsed input"),
                // json: serde_json::from_str(&serialize_syntax(&tree)).unwrap(),
                json: None,
                syntax: None,
                rules: None,
                warnings: Vec::new(),
                errors: Vec::new(),
            };

            // Parses the input syntax string separately and merges into the main.
            // Handled first so that other arguments build off of merged rules
            if let Some(rules) = &req.merge_rules {
                match parse_rulebuilder(rules.as_str()) {
                    Ok(mut o) => dnf.append(&mut o),
                    Err(e) => res.errors.push(parse_err_message(e, "Failed to parse merge rules")),
                }
            }
            
            // // If the mergeRules argument is passed then add it to the dnf.
            // // Handled first so that other arguments build off of merged rules
            // if let Some(rules) = &req.modify_users {
            //     match parse_rulebuilder(rules.as_str()) {
            //         Ok(mut o) => dnf.append(&mut o),
            //         Err(_) => res.errors.push(String::from("Failed to parse merge rules.")),
            //     }
            // }

            // Cleans the rules e.g. duplicate rules
            dnf = remove_duplicates(dnf);
            dnf = order_rules(dnf);


            // If warnings are on then warn on the 
            if Some(true) == req.warn_risks {
                dnf = check_rules(dnf);
            }


            // Defines the conditions for returning JSON as a default when no other conditions are selected
            let return_json = 
                req.return_json != Some(true) && req.return_syntax != Some(true) && req.return_rules != Some(true) ||
                req.return_json == Some(true);

            if return_json || req.return_syntax == Some(true) {
                let tree = reconstruct(&dnf);
                
                if return_json {
                    res.json = tree.as_ref().and_then(|b| serde_json::to_value(b).ok());
                }

                if req.return_syntax == Some(true) {
                    res.syntax = match tree {
                        Some(s) => Some(s.to_syntax_string()),
                        None => None,
                    };
                }

                

            }

            if req.return_rules == Some(true) {
                res.rules = match serde_json::to_value(&dnf) {
                    Ok(o) => Some(o),
                    Err(_) => None,
                };
            }

            Json(res).into_response()
        }
        Err(e) => {
            println!("{}", e);
            Json(ApiResponse {
                success: false,
                message: String::from("Error parsing input"),
                json: None,
                syntax: None,
                rules: None,
                warnings: Vec::new(),
                errors: vec![parse_err_message(e, "Failed to parse rules syntax.")],
            }).into_response()
        }
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