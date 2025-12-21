#![allow(clippy::unwrap_used, clippy::expect_used)]

use lib_common::robot::{
    CommandSchema, Example, ParameterSchema, ROBOT_HELP_SCHEMA_VERSION, RobotArg, RobotCommand,
    RobotFlagSchema, RobotHelp, RobotHelpOutput,
};
use serde_json::json;
use std::collections::HashMap;

#[test]
fn test_robot_help_serialization() {
    let help = RobotHelp {
        program: "test-app".into(),
        version: "0.1.0".into(),
        description: "A test application".into(),
        commands: vec![RobotCommand {
            name: "start".into(),
            about: "Start the application".into(),
            args: vec![RobotArg {
                name: "port".into(),
                long: Some("--port".into()),
                short: Some('p'),
                help: "Port to listen on".into(),
                required: false,
                possible_values: vec![],
            }],
            subcommands: vec![],
        }],
    };

    let json = serde_json::to_value(&help).unwrap();

    assert_eq!(json["program"], "test-app");
    assert_eq!(json["commands"][0]["name"], "start");
    assert_eq!(json["commands"][0]["args"][0]["long"], "--port");
}

#[test]
fn test_robot_help_output_schema_version_constant() {
    assert_eq!(ROBOT_HELP_SCHEMA_VERSION, "1.0.0");

    let parts: Vec<&str> = ROBOT_HELP_SCHEMA_VERSION.split('.').collect();
    assert_eq!(parts.len(), 3);
    assert!(parts[0].parse::<u32>().is_ok());
    assert!(parts[1].parse::<u32>().is_ok());
    assert!(parts[2].parse::<u32>().is_ok());
}

#[test]
fn test_robot_help_output_minimal() {
    let output = RobotHelpOutput {
        schema_version: ROBOT_HELP_SCHEMA_VERSION.to_string(),
        tool: "mcp-agent-mail".to_string(),
        version: "0.1.0".to_string(),
        description: "Gmail for coding agents".to_string(),
        commands: vec![],
        robot_flags: vec![],
    };

    let json = serde_json::to_value(&output).expect("serialization should succeed");

    assert_eq!(json["schema_version"], "1.0.0");
    assert_eq!(json["tool"], "mcp-agent-mail");
    assert_eq!(json["version"], "0.1.0");
    assert_eq!(json["description"], "Gmail for coding agents");
    assert!(json["commands"].as_array().unwrap().is_empty());
    assert!(json.get("robot_flags").is_none());
}

#[test]
fn test_robot_help_output_with_commands() {
    let mut exit_codes = HashMap::new();
    exit_codes.insert(0, "Success".to_string());
    exit_codes.insert(1, "General error".to_string());
    exit_codes.insert(2, "Invalid arguments".to_string());

    let output = RobotHelpOutput {
        schema_version: ROBOT_HELP_SCHEMA_VERSION.to_string(),
        tool: "mcp-agent-mail".to_string(),
        version: "0.1.0".to_string(),
        description: "Gmail for coding agents".to_string(),
        commands: vec![CommandSchema {
            name: "serve".to_string(),
            description: "Start a server (HTTP or MCP)".to_string(),
            parameters: vec![ParameterSchema {
                name: "port".to_string(),
                long: Some("--port".to_string()),
                short: Some('p'),
                description: "Server port number".to_string(),
                param_type: Some("u16".to_string()),
                default: Some("8765".to_string()),
                required: false,
                possible_values: vec![],
                env_var: Some("PORT".to_string()),
            }],
            exit_codes: exit_codes.clone(),
            subcommands: vec![],
            examples: vec![],
        }],
        robot_flags: vec![],
    };

    let json = serde_json::to_value(&output).expect("serialization should succeed");

    let cmd = &json["commands"][0];
    assert_eq!(cmd["name"], "serve");
    assert_eq!(cmd["description"], "Start a server (HTTP or MCP)");

    let param = &cmd["parameters"][0];
    assert_eq!(param["name"], "port");
    assert_eq!(param["long"], "--port");
    assert_eq!(param["short"], "p");
    assert_eq!(param["param_type"], "u16");
    assert_eq!(param["default"], "8765");
    assert_eq!(param["env_var"], "PORT");
    assert!(!param["required"].as_bool().unwrap());

    let codes = cmd["exit_codes"].as_object().unwrap();
    assert_eq!(codes.get("0").unwrap(), "Success");
    assert_eq!(codes.get("1").unwrap(), "General error");
    assert_eq!(codes.get("2").unwrap(), "Invalid arguments");
}

#[test]
fn test_robot_help_output_with_robot_flags() {
    let output = RobotHelpOutput {
        schema_version: ROBOT_HELP_SCHEMA_VERSION.to_string(),
        tool: "mcp-agent-mail".to_string(),
        version: "0.1.0".to_string(),
        description: "Gmail for coding agents".to_string(),
        commands: vec![],
        robot_flags: vec![
            RobotFlagSchema {
                name: "--robot-help".to_string(),
                description: "AI-optimized capability discovery".to_string(),
                output_format: "json".to_string(),
                examples: vec![
                    Example {
                        invocation: "am --robot-help".to_string(),
                        description: "Show all capabilities as JSON".to_string(),
                    },
                    Example {
                        invocation: "am --robot-help --format yaml".to_string(),
                        description: "Show capabilities as YAML".to_string(),
                    },
                ],
            },
            RobotFlagSchema {
                name: "--robot-examples".to_string(),
                description: "Show usage examples for flags/subcommands".to_string(),
                output_format: "json".to_string(),
                examples: vec![Example {
                    invocation: "am --robot-examples serve http".to_string(),
                    description: "Examples for serve http subcommand".to_string(),
                }],
            },
        ],
    };

    let json = serde_json::to_value(&output).expect("serialization should succeed");

    let flags = json["robot_flags"].as_array().unwrap();
    assert_eq!(flags.len(), 2);

    let flag1 = &flags[0];
    assert_eq!(flag1["name"], "--robot-help");
    assert_eq!(flag1["description"], "AI-optimized capability discovery");
    assert_eq!(flag1["output_format"], "json");

    let examples = flag1["examples"].as_array().unwrap();
    assert_eq!(examples.len(), 2);
    assert_eq!(examples[0]["invocation"], "am --robot-help");
    assert_eq!(examples[0]["description"], "Show all capabilities as JSON");
}

#[test]
fn test_robot_help_output_deserialization() {
    let json = json!({
        "schema_version": "1.0.0",
        "tool": "mcp-agent-mail",
        "version": "0.2.0",
        "description": "Test tool",
        "commands": [
            {
                "name": "health",
                "description": "Check server health",
                "parameters": [
                    {
                        "name": "url",
                        "long": "--url",
                        "short": "u",
                        "description": "Server URL",
                        "param_type": "String",
                        "default": "http://localhost:8765",
                        "required": false,
                        "env_var": "MCP_AGENT_MAIL_URL"
                    }
                ],
                "exit_codes": {
                    "0": "Healthy",
                    "1": "Unhealthy"
                }
            }
        ],
        "robot_flags": [
            {
                "name": "--robot-status",
                "description": "System health JSON",
                "output_format": "json",
                "examples": []
            }
        ]
    });

    let output: RobotHelpOutput =
        serde_json::from_value(json).expect("deserialization should succeed");

    assert_eq!(output.schema_version, "1.0.0");
    assert_eq!(output.tool, "mcp-agent-mail");
    assert_eq!(output.version, "0.2.0");
    assert_eq!(output.commands.len(), 1);
    assert_eq!(output.commands[0].name, "health");
    assert_eq!(output.commands[0].parameters.len(), 1);
    assert_eq!(
        output.commands[0].parameters[0].param_type,
        Some("String".to_string())
    );
    assert_eq!(
        output.commands[0].exit_codes.get(&0),
        Some(&"Healthy".to_string())
    );
    assert_eq!(output.robot_flags.len(), 1);
    assert_eq!(output.robot_flags[0].name, "--robot-status");
}

#[test]
fn test_parameter_schema_optional_fields_omitted_when_none() {
    let param = ParameterSchema {
        name: "verbose".to_string(),
        long: None,
        short: Some('v'),
        description: "Enable verbose output".to_string(),
        param_type: None,
        default: None,
        required: false,
        possible_values: vec![],
        env_var: None,
    };

    let json = serde_json::to_value(&param).expect("serialization should succeed");

    assert_eq!(json["name"], "verbose");
    assert_eq!(json["short"], "v");
    assert!(json.get("long").is_none());
    assert!(json.get("param_type").is_none());
    assert!(json.get("default").is_none());
    assert!(json.get("env_var").is_none());
}

#[test]
fn test_command_schema_nested_subcommands() {
    let cmd = CommandSchema {
        name: "archive".to_string(),
        description: "Archive management".to_string(),
        parameters: vec![],
        exit_codes: HashMap::new(),
        subcommands: vec![
            CommandSchema {
                name: "save".to_string(),
                description: "Create a restorable snapshot".to_string(),
                parameters: vec![ParameterSchema {
                    name: "label".to_string(),
                    long: Some("--label".to_string()),
                    short: Some('l'),
                    description: "Label for the archive".to_string(),
                    param_type: Some("String".to_string()),
                    default: None,
                    required: false,
                    possible_values: vec![],
                    env_var: None,
                }],
                exit_codes: HashMap::new(),
                subcommands: vec![],
                examples: vec![],
            },
            CommandSchema {
                name: "restore".to_string(),
                description: "Restore from backup".to_string(),
                parameters: vec![],
                exit_codes: HashMap::new(),
                subcommands: vec![],
                examples: vec![],
            },
        ],
        examples: vec![],
    };

    let json = serde_json::to_value(&cmd).expect("serialization should succeed");

    assert_eq!(json["name"], "archive");
    let subs = json["subcommands"].as_array().unwrap();
    assert_eq!(subs.len(), 2);
    assert_eq!(subs[0]["name"], "save");
    assert_eq!(subs[1]["name"], "restore");
    assert_eq!(subs[0]["parameters"][0]["name"], "label");
}

#[test]
fn test_example_serialization() {
    let example = Example {
        invocation: "am serve http --port 9000".to_string(),
        description: "Start server on custom port".to_string(),
    };

    let json = serde_json::to_value(&example).expect("serialization should succeed");

    assert_eq!(json["invocation"], "am serve http --port 9000");
    assert_eq!(json["description"], "Start server on custom port");
}

#[test]
fn test_backward_compatibility_legacy_robot_help_still_works() {
    let legacy = RobotHelp {
        program: "legacy-app".into(),
        version: "1.0.0".into(),
        description: "Legacy application".into(),
        commands: vec![RobotCommand {
            name: "run".into(),
            about: "Run the app".into(),
            args: vec![RobotArg {
                name: "config".into(),
                long: Some("--config".into()),
                short: Some('c'),
                help: "Config file path".into(),
                required: false,
                possible_values: vec![],
            }],
            subcommands: vec![],
        }],
    };

    let json = serde_json::to_value(&legacy).expect("serialization should succeed");

    assert_eq!(json["program"], "legacy-app");
    assert_eq!(json["commands"][0]["about"], "Run the app");
    assert_eq!(json["commands"][0]["args"][0]["help"], "Config file path");
}

// =============================================================================
// Tests for robot-* flag schema validation (TDD RED phase - mcp-agent-mail-rs-vgs4)
// =============================================================================

#[test]
fn test_robot_help_output_has_required_schema_version() {
    // Schema version must be "1.0.0" for the current API
    let output = RobotHelpOutput {
        schema_version: ROBOT_HELP_SCHEMA_VERSION.to_string(),
        tool: "mcp-agent-mail".to_string(),
        version: "0.3.0".to_string(),
        description: "Gmail for coding agents".to_string(),
        commands: vec![],
        robot_flags: vec![],
    };

    assert_eq!(output.schema_version, "1.0.0");

    // Parse as JSON and verify field exists
    let json = serde_json::to_value(&output).unwrap();
    assert!(json.get("schema_version").is_some());
    assert_eq!(json["schema_version"].as_str().unwrap(), "1.0.0");
}

#[test]
fn test_robot_help_output_valid_json_with_all_robot_flags() {
    // A valid RobotHelpOutput should include all three robot-* flags
    let output = RobotHelpOutput {
        schema_version: ROBOT_HELP_SCHEMA_VERSION.to_string(),
        tool: "mcp-agent-mail".to_string(),
        version: "0.3.0".to_string(),
        description: "Gmail for coding agents".to_string(),
        commands: vec![],
        robot_flags: vec![
            RobotFlagSchema {
                name: "--robot-help".to_string(),
                description: "AI-optimized capability discovery".to_string(),
                output_format: "json".to_string(),
                examples: vec![Example {
                    invocation: "mcp-agent-mail --robot-help".to_string(),
                    description: "Show all capabilities as JSON".to_string(),
                }],
            },
            RobotFlagSchema {
                name: "--robot-examples".to_string(),
                description: "Show usage examples".to_string(),
                output_format: "json".to_string(),
                examples: vec![],
            },
            RobotFlagSchema {
                name: "--robot-status".to_string(),
                description: "System health JSON".to_string(),
                output_format: "json".to_string(),
                examples: vec![],
            },
        ],
    };

    // Must serialize to valid JSON
    let json_str = serde_json::to_string_pretty(&output).expect("must serialize to JSON");
    assert!(!json_str.is_empty());

    // Must deserialize back
    let parsed: RobotHelpOutput = serde_json::from_str(&json_str).expect("must deserialize");
    assert_eq!(parsed.robot_flags.len(), 3);

    // Verify all three robot-* flags are present
    let flag_names: Vec<&str> = parsed.robot_flags.iter().map(|f| f.name.as_str()).collect();
    assert!(flag_names.contains(&"--robot-help"));
    assert!(flag_names.contains(&"--robot-examples"));
    assert!(flag_names.contains(&"--robot-status"));
}

#[test]
fn test_robot_examples_must_be_self_documenting() {
    // The robot_flags output should include examples for itself
    let output = RobotHelpOutput {
        schema_version: ROBOT_HELP_SCHEMA_VERSION.to_string(),
        tool: "mcp-agent-mail".to_string(),
        version: "0.3.0".to_string(),
        description: "Gmail for coding agents".to_string(),
        commands: vec![],
        robot_flags: vec![RobotFlagSchema {
            name: "--robot-examples".to_string(),
            description: "Show usage examples for flags/subcommands".to_string(),
            output_format: "json".to_string(),
            examples: vec![
                Example {
                    invocation: "mcp-agent-mail --robot-examples serve".to_string(),
                    description: "Examples for serve command".to_string(),
                },
                Example {
                    invocation: "mcp-agent-mail --robot-examples --robot-examples".to_string(),
                    description: "Self-documenting: show this very output".to_string(),
                },
            ],
        }],
    };

    let robot_examples_flag = output
        .robot_flags
        .iter()
        .find(|f| f.name == "--robot-examples")
        .expect("--robot-examples flag must exist");

    // Must have at least one example that documents itself
    let has_self_doc = robot_examples_flag.examples.iter().any(|ex| {
        ex.invocation.contains("--robot-examples") && ex.description.to_lowercase().contains("self")
    });
    assert!(
        has_self_doc,
        "--robot-examples should be self-documenting with an example for itself"
    );
}
