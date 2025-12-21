use lib_common::robot::{RobotArg, RobotCommand, RobotHelp};
use serde_json::json;

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
fn test_robot_help_deserialization() {
    let json = json!({
        "program": "cli-tool",
        "version": "1.0.0",
        "description": "CLI Tool",
        "commands": [
            {
                "name": "config",
                "about": "Manage configuration",
                "args": [],
                "subcommands": [
                    {
                        "name": "set",
                        "about": "Set a value",
                        "args": [
                            {
                                "name": "key",
                                "help": "Key to set",
                                "required": true
                            }
                        ]
                    }
                ]
            }
        ]
    });

    let help: RobotHelp = serde_json::from_value(json).unwrap();

    assert_eq!(help.program, "cli-tool");
    assert_eq!(help.commands.len(), 1);
    assert_eq!(help.commands[0].name, "config");
    assert_eq!(help.commands[0].subcommands.len(), 1);
    assert_eq!(help.commands[0].subcommands[0].name, "set");
    assert!(help.commands[0].subcommands[0].args[0].required);
}
