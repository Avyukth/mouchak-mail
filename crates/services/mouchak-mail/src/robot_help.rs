use mouchak_mail_common::robot::Example;
use once_cell::sync::Lazy;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Clone, Serialize)]
pub(crate) struct ExampleEntry {
    pub description: &'static str,
    pub target_type: &'static str,        // "flag" | "subcommand"
    pub param_type: Option<&'static str>, // "u16", "String", etc.
    pub default: Option<&'static str>,
    pub examples: Vec<Example>,
}

// Helper function to create Example with String fields
fn example(invocation: &str, description: &str) -> Example {
    Example {
        invocation: invocation.to_string(),
        description: description.to_string(),
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct RobotStatusOutput {
    pub schema_version: String,
    pub tool: String,
    pub version: String,
    pub timestamp: String,
    pub status: String,
    pub checks: HashMap<String, CheckResult>,
    pub exit_code: u8,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct CheckResult {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct RobotExamplesOutput {
    pub schema_version: String,
    pub target: String,
    pub target_type: String, // "flag" | "subcommand"
    pub examples: Vec<Example>,
}

pub(crate) static EXAMPLE_REGISTRY: Lazy<HashMap<&'static str, ExampleEntry>> = Lazy::new(|| {
    let mut m = HashMap::new();

    // ===== GLOBAL FLAGS =====
    m.insert(
        "--robot-help",
        ExampleEntry {
            description: "AI-optimized capability discovery",
            target_type: "flag",
            param_type: None,
            default: None,
            examples: vec![
                example("mouchak-mail --robot-help", "Show all capabilities as JSON"),
                example("mouchak-mail --robot-help --format yaml", "YAML output"),
            ],
        },
    );

    m.insert(
        "--robot-examples",
        ExampleEntry {
            description: "Show usage examples for flags/subcommands",
            target_type: "flag",
            param_type: Some("String"),
            default: None,
            examples: vec![
                example(
                    "mouchak-mail --robot-examples serve",
                    "Examples for serve command",
                ),
                example(
                    "mouchak-mail --robot-examples --port",
                    "Examples for --port flag",
                ),
            ],
        },
    );

    m.insert(
        "--robot-status",
        ExampleEntry {
            description: "Show system status and health checks",
            target_type: "flag",
            param_type: None,
            default: None,
            examples: vec![
                example("mouchak-mail --robot-status", "Show status in JSON format"),
                example(
                    "mouchak-mail --robot-status --format yaml",
                    "YAML status output",
                ),
            ],
        },
    );

    m.insert(
        "--format",
        ExampleEntry {
            description: "Output format for status/examples",
            target_type: "flag",
            param_type: Some("String"),
            default: Some("json"),
            examples: vec![
                example(
                    "mouchak-mail --robot-status --format yaml",
                    "YAML status output",
                ),
                example(
                    "mouchak-mail schema --format markdown",
                    "Markdown schema docs",
                ),
            ],
        },
    );

    m.insert(
        "--log-format",
        ExampleEntry {
            description: "Log output format",
            target_type: "flag",
            param_type: Some("String"),
            default: Some("pretty"),
            examples: vec![
                example(
                    "mouchak-mail serve --log-format json",
                    "Structured JSON logs",
                ),
                example(
                    "mouchak-mail serve --log-format pretty",
                    "Human-readable logs",
                ),
            ],
        },
    );

    // ===== SUBCOMMANDS =====
    m.insert(
        "serve",
        ExampleEntry {
            description: "Start server (HTTP or MCP mode)",
            target_type: "subcommand",
            param_type: None,
            default: None,
            examples: vec![
                example("mouchak-mail serve http", "Start HTTP REST API server"),
                example(
                    "mouchak-mail serve mcp --transport stdio",
                    "Start MCP stdio server",
                ),
                example(
                    "mouchak-mail serve mcp --transport sse --port 3000",
                    "Start MCP SSE server",
                ),
            ],
        },
    );

    m.insert(
        "serve http",
        ExampleEntry {
            description: "Start HTTP REST API server",
            target_type: "subcommand",
            param_type: None,
            default: None,
            examples: vec![
                example(
                    "mouchak-mail serve http --port 8765",
                    "Start on custom port",
                ),
                example("mouchak-mail serve http --no-ui", "Headless API server"),
            ],
        },
    );

    m.insert(
        "serve mcp",
        ExampleEntry {
            description: "Start MCP protocol server",
            target_type: "subcommand",
            param_type: None,
            default: None,
            examples: vec![
                example(
                    "mouchak-mail serve mcp --transport stdio",
                    "For Claude Desktop",
                ),
                example("mouchak-mail serve mcp --transport sse", "For web clients"),
            ],
        },
    );

    m.insert(
        "health",
        ExampleEntry {
            description: "Check server health and connectivity",
            target_type: "subcommand",
            param_type: None,
            default: None,
            examples: vec![
                example("mouchak-mail health", "Check localhost:8765"),
                example(
                    "mouchak-mail health --url http://prod.example.com",
                    "Check remote server",
                ),
            ],
        },
    );

    m.insert(
        "config",
        ExampleEntry {
            description: "Manage configuration settings",
            target_type: "subcommand",
            param_type: None,
            default: None,
            examples: vec![
                example("mouchak-mail config set-port 9000", "Set custom port"),
                example("mouchak-mail config show-port", "Show current port"),
            ],
        },
    );

    m.insert(
        "config set-port",
        ExampleEntry {
            description: "Set the server port",
            target_type: "subcommand",
            param_type: Some("u16"),
            default: None,
            examples: vec![example(
                "mouchak-mail config set-port 3000",
                "Set port to 3000",
            )],
        },
    );

    m.insert(
        "config show-port",
        ExampleEntry {
            description: "Show current server port setting",
            target_type: "subcommand",
            param_type: None,
            default: None,
            examples: vec![example(
                "mouchak-mail config show-port",
                "Display current port",
            )],
        },
    );

    m.insert(
        "schema",
        ExampleEntry {
            description: "Export JSON schemas for all tools",
            target_type: "subcommand",
            param_type: None,
            default: None,
            examples: vec![
                example("mouchak-mail schema", "Export all tool schemas as JSON"),
                example(
                    "mouchak-mail schema --format markdown --output docs/tools.md",
                    "Generate markdown docs",
                ),
            ],
        },
    );

    m.insert(
        "tools",
        ExampleEntry {
            description: "List all available MCP tools",
            target_type: "subcommand",
            param_type: None,
            default: None,
            examples: vec![example("mouchak-mail tools", "List all 45 MCP tools")],
        },
    );

    m.insert(
        "install",
        ExampleEntry {
            description: "Install shell alias and configuration",
            target_type: "subcommand",
            param_type: None,
            default: None,
            examples: vec![example(
                "mouchak-mail install alias",
                "Install 'am' shell alias",
            )],
        },
    );

    m.insert(
        "install alias",
        ExampleEntry {
            description: "Install shell alias for easier access",
            target_type: "subcommand",
            param_type: None,
            default: None,
            examples: vec![example(
                "mouchak-mail install alias",
                "Create 'am' command alias",
            )],
        },
    );

    m.insert(
        "service",
        ExampleEntry {
            description: "Manage background service",
            target_type: "subcommand",
            param_type: None,
            default: None,
            examples: vec![
                example("mouchak-mail service status", "Check service status"),
                example("mouchak-mail service start", "Start background service"),
                example("mouchak-mail service stop", "Stop background service"),
            ],
        },
    );

    m.insert(
        "service status",
        ExampleEntry {
            description: "Check background service status",
            target_type: "subcommand",
            param_type: None,
            default: None,
            examples: vec![example(
                "mouchak-mail service status --port 8765",
                "Check specific port",
            )],
        },
    );

    m.insert(
        "service start",
        ExampleEntry {
            description: "Start background service",
            target_type: "subcommand",
            param_type: None,
            default: None,
            examples: vec![example(
                "mouchak-mail service start --port 8765",
                "Start on specific port",
            )],
        },
    );

    m.insert(
        "service stop",
        ExampleEntry {
            description: "Stop background service",
            target_type: "subcommand",
            param_type: None,
            default: None,
            examples: vec![example(
                "mouchak-mail service stop --port 8765",
                "Stop specific instance",
            )],
        },
    );

    m.insert(
        "service restart",
        ExampleEntry {
            description: "Restart background service",
            target_type: "subcommand",
            param_type: None,
            default: None,
            examples: vec![example(
                "mouchak-mail service restart --port 8765",
                "Restart with new config",
            )],
        },
    );

    m.insert(
        "share",
        ExampleEntry {
            description: "Export sharing utilities (encryption/signing)",
            target_type: "subcommand",
            param_type: None,
            default: None,
            examples: vec![
                example("mouchak-mail share keypair", "Generate signing keypair"),
                example(
                    "mouchak-mail share verify --manifest manifest.json",
                    "Verify signed manifest",
                ),
            ],
        },
    );

    m.insert(
        "share keypair",
        ExampleEntry {
            description: "Generate signing keypair for manifests",
            target_type: "subcommand",
            param_type: None,
            default: None,
            examples: vec![
                example("mouchak-mail share keypair", "Create new keypair"),
                example(
                    "mouchak-mail share keypair --output keys.json",
                    "Save to file",
                ),
            ],
        },
    );

    m.insert(
        "share verify",
        ExampleEntry {
            description: "Verify signed manifest integrity",
            target_type: "subcommand",
            param_type: None,
            default: None,
            examples: vec![example(
                "mouchak-mail share verify --manifest export.json",
                "Verify manifest signature",
            )],
        },
    );

    m.insert(
        "share encrypt",
        ExampleEntry {
            description: "Encrypt data with passphrase",
            target_type: "subcommand",
            param_type: None,
            default: None,
            examples: vec![example(
                "mouchak-mail share encrypt --project myproj --passphrase",
                "Encrypt project data",
            )],
        },
    );

    m.insert(
        "share decrypt",
        ExampleEntry {
            description: "Decrypt age-encrypted data",
            target_type: "subcommand",
            param_type: None,
            default: None,
            examples: vec![example(
                "mouchak-mail share decrypt --input data.age",
                "Decrypt with passphrase",
            )],
        },
    );

    m.insert(
        "share deploy",
        ExampleEntry {
            description: "Deploy exported data to hosting platforms",
            target_type: "subcommand",
            param_type: None,
            default: None,
            examples: vec![
                example(
                    "mouchak-mail share deploy github-pages --repo my-archive --bundle export.zip",
                    "Deploy to GitHub Pages",
                ),
                example(
                    "mouchak-mail share deploy github-pages --repo my-archive --bundle export.zip --private",
                    "Deploy to private GitHub Pages",
                ),
            ],
        },
    );

    m.insert(
        "share deploy github-pages",
        ExampleEntry {
            description: "Deploy archive to GitHub Pages for static hosting",
            target_type: "subcommand",
            param_type: None,
            default: None,
            examples: vec![
                example(
                    "mouchak-mail share deploy github-pages --repo agent-archive --bundle data/export.zip",
                    "Basic deployment (uses GITHUB_TOKEN env var)",
                ),
                example(
                    "mouchak-mail share deploy github-pages --repo my-archive --owner myorg --bundle export.zip",
                    "Deploy to organization repo",
                ),
                example(
                    "mouchak-mail share deploy github-pages --repo archive --bundle export.zip --custom-domain archive.example.com",
                    "Deploy with custom domain",
                ),
                example(
                    "mouchak-mail share deploy github-pages --repo archive --bundle export.zip --private --token ghp_xxx",
                    "Private repo with explicit token",
                ),
            ],
        },
    );

    m.insert(
        "archive",
        ExampleEntry {
            description: "Archive management for disaster recovery",
            target_type: "subcommand",
            param_type: None,
            default: None,
            examples: vec![
                example(
                    "mouchak-mail archive save --label backup-2024",
                    "Create timestamped backup",
                ),
                example("mouchak-mail archive list", "List available archives"),
                example(
                    "mouchak-mail archive restore archive.zip",
                    "Restore from backup",
                ),
            ],
        },
    );

    m.insert(
        "archive save",
        ExampleEntry {
            description: "Create restorable snapshot archive",
            target_type: "subcommand",
            param_type: None,
            default: None,
            examples: vec![
                example("mouchak-mail archive save", "Create timestamped archive"),
                example(
                    "mouchak-mail archive save --include-git",
                    "Include git history",
                ),
            ],
        },
    );

    m.insert(
        "archive list",
        ExampleEntry {
            description: "List available restore points",
            target_type: "subcommand",
            param_type: None,
            default: None,
            examples: vec![
                example("mouchak-mail archive list", "Show all archives"),
                example("mouchak-mail archive list --json", "JSON format output"),
            ],
        },
    );

    m.insert(
        "archive restore",
        ExampleEntry {
            description: "Restore from archive snapshot",
            target_type: "subcommand",
            param_type: None,
            default: None,
            examples: vec![
                example(
                    "mouchak-mail archive restore backup.zip",
                    "Restore from file",
                ),
                example(
                    "mouchak-mail archive restore backup.zip --yes",
                    "Skip confirmation",
                ),
            ],
        },
    );

    m.insert(
        "archive clear-and-reset",
        ExampleEntry {
            description: "Clear all data (creates backup first)",
            target_type: "subcommand",
            param_type: None,
            default: None,
            examples: vec![example(
                "mouchak-mail archive clear-and-reset --archive",
                "Backup then clear",
            )],
        },
    );

    m.insert(
        "summarize",
        ExampleEntry {
            description: "Summarize conversation threads",
            target_type: "subcommand",
            param_type: None,
            default: None,
            examples: vec![
                example(
                    "mouchak-mail summarize --project myproj --thread-id task-123",
                    "Summarize single thread",
                ),
                example(
                    "mouchak-mail summarize --project myproj --thread-id task-123,task-456",
                    "Summarize multiple threads",
                ),
            ],
        },
    );

    m.insert(
        "version",
        ExampleEntry {
            description: "Show version information",
            target_type: "subcommand",
            param_type: None,
            default: None,
            examples: vec![example(
                "mouchak-mail version",
                "Display version and build info",
            )],
        },
    );

    m.insert(
        "products",
        ExampleEntry {
            description: "Multi-repo coordination management",
            target_type: "subcommand",
            param_type: None,
            default: None,
            examples: vec![
                example(
                    "mouchak-mail products ensure --uid cross-platform --name 'Cross-Platform'",
                    "Create product",
                ),
                example(
                    "mouchak-mail products link --uid cross-platform --project /path/to/repo",
                    "Link project to product",
                ),
            ],
        },
    );

    m.insert(
        "products ensure",
        ExampleEntry {
            description: "Create or get existing product",
            target_type: "subcommand",
            param_type: None,
            default: None,
            examples: vec![example(
                "mouchak-mail products ensure --uid my-product --name 'My Product'",
                "Create new product",
            )],
        },
    );

    m.insert(
        "products link",
        ExampleEntry {
            description: "Link project to product",
            target_type: "subcommand",
            param_type: None,
            default: None,
            examples: vec![example(
                "mouchak-mail products link --uid my-product --project /repo/path",
                "Add project to product",
            )],
        },
    );

    m.insert(
        "products unlink",
        ExampleEntry {
            description: "Remove project from product",
            target_type: "subcommand",
            param_type: None,
            default: None,
            examples: vec![example(
                "mouchak-mail products unlink --uid my-product --project /repo/path",
                "Remove project link",
            )],
        },
    );

    m.insert(
        "products status",
        ExampleEntry {
            description: "Show product status and linked projects",
            target_type: "subcommand",
            param_type: None,
            default: None,
            examples: vec![example(
                "mouchak-mail products status --uid my-product",
                "Show product details",
            )],
        },
    );

    m.insert(
        "products search",
        ExampleEntry {
            description: "Search across product projects",
            target_type: "subcommand",
            param_type: None,
            default: None,
            examples: vec![example(
                "mouchak-mail products search --uid my-product --query 'error'",
                "Search product messages",
            )],
        },
    );

    m.insert(
        "products inbox",
        ExampleEntry {
            description: "View product-wide inbox",
            target_type: "subcommand",
            param_type: None,
            default: None,
            examples: vec![example(
                "mouchak-mail products inbox --uid my-product",
                "Show product inbox",
            )],
        },
    );

    m.insert(
        "products summarize-thread",
        ExampleEntry {
            description: "Summarize thread across product",
            target_type: "subcommand",
            param_type: None,
            default: None,
            examples: vec![example(
                "mouchak-mail products summarize-thread --uid my-product --thread-id task-123",
                "Cross-project summary",
            )],
        },
    );

    m.insert(
        "guard",
        ExampleEntry {
            description: "Pre-commit guard management",
            target_type: "subcommand",
            param_type: None,
            default: None,
            examples: vec![
                example("mouchak-mail guard status", "Check guard status"),
                example(
                    "mouchak-mail guard check --stdin-nul",
                    "Validate file reservations",
                ),
            ],
        },
    );

    m.insert(
        "guard status",
        ExampleEntry {
            description: "Show pre-commit guard status",
            target_type: "subcommand",
            param_type: None,
            default: None,
            examples: vec![example(
                "mouchak-mail guard status",
                "Display current settings",
            )],
        },
    );

    m.insert(
        "guard check",
        ExampleEntry {
            description: "Check file reservation conflicts",
            target_type: "subcommand",
            param_type: None,
            default: None,
            examples: vec![
                example("mouchak-mail guard check --stdin-nul", "Check from stdin"),
                example(
                    "mouchak-mail guard check --advisory",
                    "Warn instead of fail",
                ),
            ],
        },
    );

    m.insert(
        "mail",
        ExampleEntry {
            description: "Direct mail operations",
            target_type: "subcommand",
            param_type: None,
            default: None,
            examples: vec![example(
                "mouchak-mail mail status",
                "Show mail system status",
            )],
        },
    );

    m.insert(
        "mail status",
        ExampleEntry {
            description: "Show mail system status",
            target_type: "subcommand",
            param_type: None,
            default: None,
            examples: vec![example("mouchak-mail mail status", "Display system health")],
        },
    );

    m
});
