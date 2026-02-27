// Tool parameter types used by the MCP server.
// Each tool's implementation lives in server.rs using #[tool] macros.

use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchConfigParams {
    /// The search query (e.g. "transparent", "font size", "padding")
    pub query: String,
    /// Optional category filter (e.g. "appearance", "font", "window")
    pub category: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetOptionParams {
    /// The exact option name (e.g. "background-opacity", "font-size")
    pub name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReadConfigParams {
    /// Specific option to read. If omitted, returns all set options.
    pub option: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WriteConfigParams {
    /// The option name to set (e.g. "font-size")
    pub option: String,
    /// The value to set (e.g. "14")
    pub value: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RemoveConfigParams {
    /// The option name to remove (will be commented out, not deleted)
    pub option: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ValidateConfigParams {
    /// Specific option to validate. If omitted, validates the entire config file.
    pub option: Option<String>,
    /// Value to validate against the option's type constraints.
    pub value: Option<String>,
}
