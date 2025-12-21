use serde::{Deserialize, Serialize};

/// Root structure for robot-friendly help output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RobotHelp {
    /// The name of the program/binary
    pub program: String,
    /// Semantic version of the program
    pub version: String,
    /// Brief description of the program
    pub description: String,
    /// List of available subcommands
    pub commands: Vec<RobotCommand>,
}

/// Description of a single subcommand
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RobotCommand {
    /// The name of the subcommand (e.g., "server", "config")
    pub name: String,
    /// Brief description of what the command does
    pub about: String,
    /// List of arguments accepted by this command
    pub args: Vec<RobotArg>,
    /// List of sub-subcommands (nested commands)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subcommands: Vec<RobotCommand>,
}

/// Description of a command argument
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RobotArg {
    /// The name of the argument (e.g., "port", "config")
    pub name: String,
    /// The long flag version (e.g., "--port")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub long: Option<String>,
    /// The short flag version (e.g., "-p")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short: Option<char>, // Using char for single character short flags
    /// Description of the argument
    #[serde(default)]
    pub help: String,
    /// Whether the argument is required
    pub required: bool,
    /// Possible values if restricted (enum-like)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub possible_values: Vec<String>,
}
