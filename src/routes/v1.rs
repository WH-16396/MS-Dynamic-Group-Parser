//! `POST /api/v1` - parses a rule syntax string, optionally merged with a
//! second one.

use axum::{
    Json,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::{
    parser,
    routes::{Output, OutputOptions, render_output, syntax_error},
};

#[derive(Deserialize)]
pub struct ApiRequest {
    /// Rule syntax to parse.
    input: String,

    /// More rule syntax, OR-ed onto `input` before anything else happens.
    /// Use this to add conditions.
    #[serde(rename = "mergeRules", default)]
    merge_rules: Option<String>,

    #[serde(flatten)]
    options: OutputOptions,
}

#[derive(Serialize)]
pub struct ApiResponse {
    success: bool,
    message: String,
    #[serde(flatten)]
    output: Output,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    warnings: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    errors: Vec<JsonValue>,
}

pub async fn handler(Json(request): Json<ApiRequest>) -> Response {
    let mut dnf = match parser::parse(&request.input) {
        Ok(dnf) => dnf,
        Err(error) => {
            println!("{}", error);
            return Json(ApiResponse {
                success: false,
                message: String::from("Error parsing input"),
                output: Output::default(),
                warnings: Vec::new(),
                errors: vec![syntax_error(error, "Failed to parse rules syntax.")],
            })
            .into_response();
        }
    };

    // Merged first so every output reflects the combined rules. A failure
    // here is reported but doesn't fail the request
    let mut errors = Vec::new();
    if let Some(merge_rules) = &request.merge_rules {
        match parser::parse(merge_rules) {
            Ok(mut merged) => dnf.append(&mut merged),
            Err(error) => errors.push(syntax_error(error, "Failed to parse merge rules")),
        }
    }

    Json(ApiResponse {
        success: true,
        message: String::from("Successfully parsed input"),
        output: render_output(dnf, &request.options),
        warnings: Vec::new(),
        errors,
    })
    .into_response()
}
