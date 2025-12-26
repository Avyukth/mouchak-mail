use lib_mcp::docs::generate_markdown_docs;
use lib_mcp::tools::{ParameterSchema, ToolSchema};

#[test]
fn test_generate_markdown_docs_empty_schemas() {
    let schemas: Vec<ToolSchema> = vec![];
    let result = generate_markdown_docs(&schemas);

    assert!(result.contains("# MCP Agent Mail - Tool Reference"));
    assert!(result.contains("Total tools: 0"));
    assert!(result.contains("## Table of Contents"));
}

#[test]
fn test_generate_markdown_docs_single_tool_no_params() {
    let schemas = vec![ToolSchema {
        name: "test_tool".to_string(),
        description: "A test tool description".to_string(),
        parameters: vec![],
    }];

    let result = generate_markdown_docs(&schemas);

    assert!(result.contains("# MCP Agent Mail - Tool Reference"));
    assert!(result.contains("Total tools: 1"));
    assert!(result.contains("- [test_tool](#test-tool)"));
    assert!(result.contains("## test_tool"));
    assert!(result.contains("A test tool description"));
    assert!(!result.contains("### Parameters"));
}

#[test]
fn test_generate_markdown_docs_with_parameters() {
    let schemas = vec![ToolSchema {
        name: "send_message".to_string(),
        description: "Send a message to an agent".to_string(),
        parameters: vec![
            ParameterSchema {
                name: "recipient".to_string(),
                param_type: "string".to_string(),
                required: true,
                description: "The message recipient".to_string(),
            },
            ParameterSchema {
                name: "priority".to_string(),
                param_type: "integer".to_string(),
                required: false,
                description: "Message priority level".to_string(),
            },
        ],
    }];

    let result = generate_markdown_docs(&schemas);

    assert!(result.contains("### Parameters"));
    assert!(result.contains("| Name | Type | Required | Description |"));
    assert!(result.contains("| `recipient` | string | Yes | The message recipient |"));
    assert!(result.contains("| `priority` | integer | No | Message priority level |"));
}

#[test]
fn test_generate_markdown_docs_multiple_tools() {
    let schemas = vec![
        ToolSchema {
            name: "tool_one".to_string(),
            description: "First tool".to_string(),
            parameters: vec![],
        },
        ToolSchema {
            name: "tool_two".to_string(),
            description: "Second tool".to_string(),
            parameters: vec![ParameterSchema {
                name: "arg".to_string(),
                param_type: "boolean".to_string(),
                required: true,
                description: "An argument".to_string(),
            }],
        },
    ];

    let result = generate_markdown_docs(&schemas);

    assert!(result.contains("Total tools: 2"));
    assert!(result.contains("- [tool_one](#tool-one)"));
    assert!(result.contains("- [tool_two](#tool-two)"));
    assert!(result.contains("## tool_one"));
    assert!(result.contains("## tool_two"));
}

#[test]
fn test_generate_markdown_docs_output_not_empty() {
    let schemas = vec![ToolSchema {
        name: "any_tool".to_string(),
        description: "Any description".to_string(),
        parameters: vec![],
    }];

    let result = generate_markdown_docs(&schemas);

    assert!(!result.is_empty());
    assert!(result.len() > 50);
}

#[test]
fn test_generate_markdown_docs_underscore_to_dash_in_anchor() {
    let schemas = vec![ToolSchema {
        name: "my_complex_tool_name".to_string(),
        description: "Tool with underscores".to_string(),
        parameters: vec![],
    }];

    let result = generate_markdown_docs(&schemas);

    assert!(result.contains("- [my_complex_tool_name](#my-complex-tool-name)"));
}
