use crate::tools::ToolSchema;

/// Generate Markdown documentation for tools
pub fn generate_markdown_docs(schemas: &[ToolSchema]) -> String {
    let mut md = String::from("# Mouchak Mail - Tool Reference\n\n");
    md.push_str(&format!("Total tools: {}\n\n", schemas.len()));
    md.push_str("## Table of Contents\n\n");

    for schema in schemas {
        md.push_str(&format!(
            "- [{}](#{})\n",
            schema.name,
            schema.name.replace('_', "-")
        ));
    }

    md.push_str("\n---\n\n");

    for schema in schemas {
        md.push_str(&format!("## {}\n\n", schema.name));
        md.push_str(&format!("{}\n\n", schema.description));

        if !schema.parameters.is_empty() {
            md.push_str("### Parameters\n\n");
            md.push_str("| Name | Type | Required | Description |\n");
            md.push_str("|------|------|----------|-------------|\n");
            for param in &schema.parameters {
                md.push_str(&format!(
                    "| `{}` | {} | {} | {} |\n",
                    param.name,
                    param.param_type,
                    if param.required { "Yes" } else { "No" },
                    param.description
                ));
            }
        }
    }
    md
}
