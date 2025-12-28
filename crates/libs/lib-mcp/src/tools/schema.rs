//! Auto-generation of MCP Tool schemas from JsonSchema-deriving parameter structs.
//!
//! This module generates `ToolSchema` from Rust structs that derive `schemars::JsonSchema`,
//! eliminating manual schema definitions by keeping params.rs as the single source of truth.
//!
//! # Example
//!
//! ```ignore
//! let schema = schema_from_params::<SendMessageParams>(
//!     "send_message",
//!     "Send a message from one agent to others."
//! );
//! ```

use schemars::{JsonSchema, schema_for};
use serde_json::Value;

use super::{ParameterSchema, ToolSchema};

/// Generate a `ToolSchema` from a type implementing `JsonSchema`.
///
/// Extracts parameter names, types, required status, and descriptions from the struct.
/// `Option<T>` fields become optional parameters; all others are required.
pub fn schema_from_params<T: JsonSchema>(name: &str, description: &str) -> ToolSchema {
    let schema = schema_for!(T);
    let json_value = serde_json::to_value(schema).unwrap_or(Value::Null);

    let mut parameters = Vec::new();

    if let Value::Object(root) = &json_value {
        let properties = root.get("properties").and_then(|v| v.as_object());
        let required = root
            .get("required")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .collect::<std::collections::HashSet<_>>()
            })
            .unwrap_or_default();

        if let Some(props) = properties {
            for (field_name, field_schema) in props {
                let param_type = extract_type(field_schema);
                let desc = extract_description(field_schema);
                let is_required = required.contains(field_name.as_str());

                parameters.push(ParameterSchema {
                    name: field_name.clone(),
                    param_type,
                    required: is_required,
                    description: desc,
                });
            }
        }
    }

    parameters.sort_by(|a, b| a.name.cmp(&b.name));

    ToolSchema {
        name: name.into(),
        description: description.into(),
        parameters,
    }
}

fn extract_type(schema: &Value) -> String {
    if let Value::Object(obj) = schema {
        if let Some(Value::String(t)) = obj.get("type") {
            return t.clone();
        }

        if let Some(Value::Array(types)) = obj.get("type") {
            for t in types {
                if let Value::String(s) = t {
                    if s != "null" {
                        return s.clone();
                    }
                }
            }
        }

        if let Some(Value::Array(any_of)) = obj.get("anyOf") {
            for variant in any_of {
                if let Value::Object(v) = variant {
                    if let Some(Value::String(t)) = v.get("type") {
                        if t != "null" {
                            return t.clone();
                        }
                    }
                }
            }
        }

        if obj.contains_key("$ref") {
            if let Some(Value::String(r)) = obj.get("$ref") {
                if let Some(name) = r.rsplit('/').next() {
                    return name.to_lowercase();
                }
            }
        }
    }

    "string".into()
}

fn extract_description(schema: &Value) -> String {
    if let Value::Object(obj) = schema {
        if let Some(Value::String(desc)) = obj.get("description") {
            return desc.clone();
        }

        if let Some(Value::Array(any_of)) = obj.get("anyOf") {
            for variant in any_of {
                if let Value::Object(v) = variant {
                    if let Some(Value::String(desc)) = v.get("description") {
                        return desc.clone();
                    }
                }
            }
        }
    }

    String::new()
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use schemars::JsonSchema;

    #[allow(dead_code)]
    #[derive(Debug, JsonSchema)]
    struct TestParams {
        /// A required string field
        pub required_field: String,
        /// An optional string field
        pub optional_field: Option<String>,
        /// A required integer field
        pub count: i64,
        /// An optional boolean field
        pub enabled: Option<bool>,
    }

    #[test]
    fn test_schema_from_params_extracts_fields() {
        let schema = schema_from_params::<TestParams>("test_tool", "A test tool description");

        assert_eq!(schema.name, "test_tool");
        assert_eq!(schema.description, "A test tool description");
        assert_eq!(schema.parameters.len(), 4);
    }

    #[test]
    fn test_schema_from_params_required_fields() {
        let schema = schema_from_params::<TestParams>("test_tool", "A test tool");

        let required_field = schema
            .parameters
            .iter()
            .find(|p| p.name == "required_field");
        assert!(required_field.is_some());
        assert!(required_field.unwrap().required);

        let optional_field = schema
            .parameters
            .iter()
            .find(|p| p.name == "optional_field");
        assert!(optional_field.is_some());
        assert!(!optional_field.unwrap().required);
    }

    #[test]
    fn test_schema_from_params_types() {
        let schema = schema_from_params::<TestParams>("test_tool", "A test tool");

        let required_field = schema
            .parameters
            .iter()
            .find(|p| p.name == "required_field")
            .unwrap();
        assert_eq!(required_field.param_type, "string");

        let count = schema
            .parameters
            .iter()
            .find(|p| p.name == "count")
            .unwrap();
        assert_eq!(count.param_type, "integer");

        let enabled = schema
            .parameters
            .iter()
            .find(|p| p.name == "enabled")
            .unwrap();
        assert_eq!(enabled.param_type, "boolean");
    }

    #[test]
    fn test_schema_from_params_descriptions() {
        let schema = schema_from_params::<TestParams>("test_tool", "A test tool");

        let required_field = schema
            .parameters
            .iter()
            .find(|p| p.name == "required_field")
            .unwrap();
        assert_eq!(required_field.description, "A required string field");

        let optional_field = schema
            .parameters
            .iter()
            .find(|p| p.name == "optional_field")
            .unwrap();
        assert_eq!(optional_field.description, "An optional string field");
    }

    #[test]
    fn test_parameters_sorted_by_name() {
        let schema = schema_from_params::<TestParams>("test_tool", "A test tool");

        let names: Vec<&str> = schema.parameters.iter().map(|p| p.name.as_str()).collect();
        let mut sorted_names = names.clone();
        sorted_names.sort();
        assert_eq!(names, sorted_names);
    }
}
