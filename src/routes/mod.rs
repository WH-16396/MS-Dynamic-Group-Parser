//! HTTP handlers, plus the request options and output steps both API
//! versions share.

pub mod v1;
pub mod v2;

use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, json};

use crate::{
    model::Dnf,
    process::{
        checks::check_rules,
        reconstruct::reconstruct,
        tidy::{dedup_nodes, order_nodes},
    },
};

/// Request flags that choose what goes in the response.
#[derive(Deserialize)]
pub struct OutputOptions {
    /// Return the rules as a JSON tree - use when the result is shown to a
    /// user or edited outside the API.
    ///
    /// Defaults to true when none of the `return*` flags are set.
    #[serde(rename = "returnJson")]
    return_json: Option<bool>,

    /// Return the rules as recompiled syntax - use when automating, since
    /// it's more compact.
    #[serde(rename = "returnSyntax")]
    return_syntax: Option<bool>,

    /// Return the individual AND-rules for each unique branch of access.
    #[serde(rename = "returnRules")]
    return_rules: Option<bool>,

    /// Check every rule for common process mistakes and attach warnings.
    #[serde(rename = "checkRisks")]
    check_risks: Option<bool>,
}

impl OutputOptions {
    fn wants_json(&self) -> bool {
        let nothing_requested = self.return_json != Some(true)
            && self.return_syntax != Some(true)
            && self.return_rules != Some(true);
        nothing_requested || self.return_json == Some(true)
    }

    fn wants_syntax(&self) -> bool {
        self.return_syntax == Some(true)
    }

    fn wants_rules(&self) -> bool {
        self.return_rules == Some(true)
    }

    fn wants_checks(&self) -> bool {
        self.check_risks == Some(true)
    }
}

/// The parts of a successful response chosen by `OutputOptions`.
#[derive(Serialize, Default)]
pub struct Output {
    #[serde(skip_serializing_if = "Option::is_none")]
    json: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    syntax: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rules: Option<JsonValue>,
}

/// The final steps shared by every API version: tidy the rules, run the risk
/// checks if asked to, then render whichever outputs were requested.
pub fn render_output(dnf: Dnf<'_>, options: &OutputOptions) -> Output {
    let mut dnf = order_nodes(dedup_nodes(dnf));

    if options.wants_checks() {
        dnf = check_rules(dnf);
    }

    let mut output = Output::default();

    if options.wants_json() || options.wants_syntax() {
        let tree = reconstruct(&dnf);

        if options.wants_json() {
            output.json = tree.as_ref().and_then(|tree| serde_json::to_value(tree).ok());
        }
        if options.wants_syntax() {
            output.syntax = tree.map(|tree| tree.to_syntax_string());
        }
    }

    if options.wants_rules() {
        output.rules = serde_json::to_value(&dnf).ok();
    }

    output
}

/// Describes a syntax error as `{"message": ..., "location": [start, end]}`,
/// where `location` is a byte range into the input.
pub fn syntax_error<R>(error: pest::error::Error<R>, message: &str) -> JsonValue {
    use pest::error::InputLocation;
    let location: (usize, usize) = match error.location {
        InputLocation::Pos(position) => (position, position),
        InputLocation::Span(span) => span,
    };
    json!({
        "message": message,
        "location": location,
    })
}
