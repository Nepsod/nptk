use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use strum::EnumString;

#[derive(
    Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, EnumString,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum ReasoningEffort {
    None,
    Minimal,
    Low,
    Medium,
    High,
    XHigh,
}

#[derive(
    Clone, Copy, Default, Debug, Serialize, Deserialize, PartialEq, Eq, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Speed {
    #[default]
    Standard,
    Fast,
}
