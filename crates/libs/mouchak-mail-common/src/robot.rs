use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const ROBOT_HELP_SCHEMA_VERSION: &str = "1.0.0";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RobotHelp {
    pub program: String,
    pub version: String,
    pub description: String,
    pub commands: Vec<RobotCommand>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RobotCommand {
    pub name: String,
    pub about: String,
    pub args: Vec<RobotArg>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subcommands: Vec<RobotCommand>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RobotArg {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub long: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short: Option<char>,
    #[serde(default)]
    pub help: String,
    pub required: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub possible_values: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RobotHelpOutput {
    pub schema_version: String,
    pub tool: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub commands: Vec<CommandSchema>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub robot_flags: Vec<RobotFlagSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandSchema {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub parameters: Vec<ParameterSchema>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub exit_codes: HashMap<i32, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subcommands: Vec<CommandSchema>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<Example>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterSchema {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub long: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short: Option<char>,
    #[serde(default)]
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub param_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    pub required: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub possible_values: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env_var: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RobotFlagSchema {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub output_format: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<Example>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Example {
    pub invocation: String,
    pub description: String,
}
