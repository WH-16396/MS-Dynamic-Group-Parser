//! `POST /api/v2` - applies a list of actions to an initially empty rule set.
//!
//! `input` is either a syntax string, a single action object or an array of
//! action objects. Each action object looks like `{"type": ..., "data": ...}`:
//!
//! | `type`               | `data`                                              |
//! | -------------------- | --------------------------------------------------- |
//! | `syntaxAdd`          | rule syntax string, OR-ed onto the rules            |
//! | `ruleAdd`            | condition(s), AND-ed into one new rule              |
//! | `ruleRemoveMatching` | condition(s); removes rules made of exactly these   |
//! | `ruleRemoveContains` | condition(s); removes rules that contain all these  |
//!
//! A condition is `{"property": ..., "operator": ..., "value": ...}`. The
//! remove actions accept `"*"` for any part as a wildcard.

use axum::{
    Json,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::{collections::HashMap, hash::Hash};

use crate::{
    model::{Condition, ConditionPattern, Dnf, Node, Operator, Property, Rule, Value},
    parser,
    process::tidy::tidy,
    routes::{Output, OutputOptions, render_output},
};

#[derive(Deserialize)]
pub struct ApiRequest {
    /// The action(s) to apply - see the module docs.
    input: JsonValue,

    // TODO: not implemented yet
    #[allow(dead_code)]
    #[serde(rename = "globalConditions")]
    global_conditions: Option<JsonValue>,

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
    /// The piece of `input` that caused the failure, if any.
    #[serde(skip_serializing_if = "JsonValue::is_null")]
    error: JsonValue,
}

impl ApiResponse {
    fn failure(message: &str, error: JsonValue) -> Response {
        Json(ApiResponse {
            success: false,
            message: String::from(message),
            output: Output::default(),
            warnings: Vec::new(),
            error,
        })
        .into_response()
    }
}

pub async fn handler(Json(request): Json<ApiRequest>) -> Response {
    let actions = match parse_input(&request.input) {
        Ok(actions) => actions,
        Err(error) => {
            println!("{:#?}", error);
            return ApiResponse::failure(error.kind.message(), error.json.clone());
        }
    };

    // Apply each action in order, stopping at the first failure
    let dnf = actions
        .into_iter()
        .try_fold(Vec::new(), |dnf, action| action.apply(dnf));

    match dnf {
        Ok(dnf) => Json(ApiResponse {
            success: true,
            message: String::from("Successfully parsed input"),
            output: render_output(dnf, &request.options),
            warnings: Vec::new(),
            error: JsonValue::Null,
        })
        .into_response(),
        Err(error) => {
            println!("{:#?}", error);
            ApiResponse::failure("Failed to parse syntax input", JsonValue::Null)
        }
    }
}

/// Reads `input` into a list of actions.
fn parse_input(input: &JsonValue) -> Result<Vec<Action<'_>>, InputError<'_>> {
    match input {
        JsonValue::Array(items) if items.is_empty() => Err(InputError::new(InputErrorKind::EmptyInput, input)),
        JsonValue::Array(items) => items.iter().map(Action::from_json).collect(),
        JsonValue::String(_) | JsonValue::Object(_) => Ok(vec![Action::from_json(input)?]),
        _ => {
            println!("{:#?}", input);
            Ok(Vec::new())
        }
    }
}



//
// ACTIONS
//

/// One step requested in `input`.
#[derive(Debug)]
enum Action<'src> {
    SyntaxAdd(&'src str),
    RuleAdd(Vec<Condition<'src>>),
    RuleRemoveMatching(Vec<ConditionPattern<'src>>),
    RuleRemoveContains(Vec<ConditionPattern<'src>>),
}

/// The only way an action can fail once it has been read.
#[derive(Debug)]
enum ActionError {
    InvalidSyntax,
}

impl<'src> Action<'src> {
    /// Reads a bare syntax string or a `{"type": ..., "data": ...}` object.
    fn from_json(json: &'src JsonValue) -> Result<Self, InputError<'src>> {
        use InputErrorKind::*;

        let action = match json {
            JsonValue::String(syntax) => Self::SyntaxAdd(syntax),
            JsonValue::Object(object) => {
                let data = object.get("data").ok_or(InputError::new(MissingData, json))?;
                let action_type = object.get("type").ok_or(InputError::new(MissingType, json))?;

                match action_type.as_str() {
                    Some("syntaxAdd") => Self::SyntaxAdd(
                        data.as_str().ok_or(InputError::new(InvalidSyntaxData, data))?,
                    ),
                    Some("ruleAdd") => Self::RuleAdd(conditions_from_json(data)?),
                    Some("ruleRemoveMatching") => Self::RuleRemoveMatching(conditions_from_json(data)?),
                    Some("ruleRemoveContains") => Self::RuleRemoveContains(conditions_from_json(data)?),
                    _ => return Err(InputError::new(InvalidType, json)),
                }
            }
            _ => return Err(InputError::new(InvalidInput, json)),
        };

        println!("{:#?}", action);
        Ok(action)
    }

    /// Applies this action to `dnf`, returning the updated rules.
    fn apply(self, dnf: Dnf<'src>) -> Result<Dnf<'src>, ActionError> {
        Ok(match self {
            Self::SyntaxAdd(syntax) => {
                let parsed = parser::parse(syntax).map_err(|_| ActionError::InvalidSyntax)?;
                add_rules(dnf, tidy(parsed))
            }

            Self::RuleAdd(conditions) => {
                let mut rule = Rule::new();
                rule.nodes.extend(conditions.into_iter().map(Node::from));
                add_rules(dnf, vec![rule])
            }

            // Drop rules whose conditions are exactly the given set, in any
            // order. Wildcards never match here, since every condition in a
            // rule is fully specified
            Self::RuleRemoveMatching(patterns) => dnf
                .into_iter()
                .filter(|rule| {
                    let conditions = rule.nodes.iter().map(|node| ConditionPattern::from(node.condition));
                    !same_elements(patterns.iter().cloned(), conditions)
                })
                .collect(),

            // Drop rules that contain a match for every given pattern
            Self::RuleRemoveContains(patterns) => dnf
                .into_iter()
                .filter(|rule| !patterns.iter().all(|pattern| rule.contains(pattern)))
                .collect(),
        })
    }
}

fn add_rules<'src>(mut dnf: Dnf<'src>, mut rules: Dnf<'src>) -> Dnf<'src> {
    dnf.append(&mut rules);
    tidy(dnf)
}

/// True when both iterators yield the same items, in any order.
///
/// https://users.rust-lang.org/t/assert-vectors-equal-in-any-order/38716/10
fn same_elements<T: Eq + Hash>(left: impl Iterator<Item = T>, right: impl Iterator<Item = T>) -> bool {
    fn count<T: Eq + Hash>(items: impl Iterator<Item = T>) -> HashMap<T, usize> {
        let mut counts = HashMap::new();
        for item in items {
            *counts.entry(item).or_insert(0) += 1;
        }
        counts
    }
    count(left) == count(right)
}



//
// CONDITIONS
//

/// Reads a single condition object or an array of them.
fn conditions_from_json<'src, T: FromJson<'src>>(data: &'src JsonValue) -> Result<Vec<T>, InputError<'src>> {
    match data {
        JsonValue::Object(_) => Ok(vec![T::from_json(data)?]),
        JsonValue::Array(items) => items.iter().map(T::from_json).collect(),
        _ => Err(InputError::new(InputErrorKind::InvalidData, data)),
    }
}

trait FromJson<'src>: Sized {
    fn from_json(json: &'src JsonValue) -> Result<Self, InputError<'src>>;
}

/// Reads `{"property": ..., "operator": ..., "value": ...}`, where `"*"` in
/// any position is a wildcard.
impl<'src> FromJson<'src> for ConditionPattern<'src> {
    fn from_json(json: &'src JsonValue) -> Result<Self, InputError<'src>> {
        use InputErrorKind::*;
        let error = |kind| InputError::new(kind, json);

        let JsonValue::Object(object) = json else {
            return Err(error(InvalidCondition));
        };

        let property = match object.get("property").ok_or(error(InvalidProperty))? {
            JsonValue::String(property) if property == "*" => None,
            JsonValue::String(property) => {
                Some(Property::validated(property).ok_or(error(InvalidProperty))?)
            }
            _ => return Err(error(InvalidProperty)),
        };

        let operator = match object.get("operator").ok_or(error(InvalidOperator))? {
            JsonValue::String(operator) if operator == "*" => None,
            JsonValue::String(operator) => {
                let operator = Operator::parse(operator).ok_or(error(InvalidOperator))?;
                // Conditions hold a single value, so array operators can't apply
                if !operator.is_single() {
                    return Err(error(ArrayOperator));
                }
                Some(operator)
            }
            _ => return Err(error(InvalidOperator)),
        };

        let value = match object.get("value").ok_or(error(InvalidValue))? {
            JsonValue::String(string) if string == "*" => None,
            JsonValue::String(string) => Some(Value::String(string)),
            JsonValue::Number(number) => Some(Value::Number(number.as_i64().ok_or(error(InvalidValue))?)),
            JsonValue::Bool(boolean) => Some(Value::Boolean(*boolean)),
            JsonValue::Null => Some(Value::Null),
            _ => return Err(error(InvalidValue)),
        };

        Ok(Self { property, operator, value })
    }
}

/// Same as `ConditionPattern`, but wildcards are rejected.
impl<'src> FromJson<'src> for Condition<'src> {
    fn from_json(json: &'src JsonValue) -> Result<Self, InputError<'src>> {
        let pattern = ConditionPattern::from_json(json)?;
        let wildcard = || InputError::new(InputErrorKind::Wildcard, json);
        Ok(Self {
            property: pattern.property.ok_or_else(wildcard)?,
            operator: pattern.operator.ok_or_else(wildcard)?,
            value: pattern.value.ok_or_else(wildcard)?,
        })
    }
}



//
// ERRORS
//

/// A problem with the shape of `input`, along with the part of it at fault.
#[derive(Debug)]
struct InputError<'src> {
    kind: InputErrorKind,
    json: &'src JsonValue,
}

impl<'src> InputError<'src> {
    fn new(kind: InputErrorKind, json: &'src JsonValue) -> Self {
        Self { kind, json }
    }
}

#[derive(Debug, Clone, Copy)]
enum InputErrorKind {
    InvalidType,
    MissingType,
    InvalidData,
    MissingData,
    InvalidSyntaxData,
    InvalidCondition,
    Wildcard,
    InvalidProperty,
    InvalidOperator,
    ArrayOperator,
    InvalidValue,
    InvalidInput,
    EmptyInput,
}

impl InputErrorKind {
    fn message(&self) -> &'static str {
        match self {
            Self::InvalidType => "Invalid value for the key 'type'.",
            Self::MissingType => "Could not find the key 'type' within the input object.",
            Self::InvalidData => "Invalid value for the key 'data', please ensure rule inputs are an object or object array.",
            Self::MissingData => "Could not find the key 'data' within the input object.",
            Self::InvalidSyntaxData => "Failed to parse 'data', please ensure direct syntax inputs are a string.",
            Self::InvalidCondition => "Failed to parse rule condition.",
            Self::Wildcard => "Failed to parse rule condition, please ensure that there are no wildcard values (*) for 'ruleAdd' types.",
            Self::InvalidProperty => "Failed to parse property.",
            Self::InvalidOperator => "Failed to parse operator.",
            Self::ArrayOperator => "Invalid operator input, cannot use an array operator.",
            Self::InvalidValue => "Failed to parse input value.",
            Self::InvalidInput => "Invalid value for the key 'input'.",
            Self::EmptyInput => "Could not find the key 'input' within the request.",
        }
    }
}
