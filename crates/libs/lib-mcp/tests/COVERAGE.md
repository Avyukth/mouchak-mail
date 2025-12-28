# lib-mcp Coverage Reference

## Quick Glance: Hard to Test

| Module | Why | Where Tested |
|--------|-----|--------------|
| `lib.rs` | Server startup (real I/O) | `e2e/`, `mcp-stdio` crate |
| `mod.rs` | Macro-generated dispatch | `tool_dispatch_tests.rs` |
| `resources.rs` | MCP resource:// URIs | `mcp_resource_api_tests.rs` |
| `precommit.rs` | Git hook filesystem ops | `test_precommit_files.rs` |

## Test File → Module Mapping

```
agent_tests.rs       → agent.rs
contacts_tests.rs    → contacts.rs  
files_tests.rs       → files.rs
observability_tests.rs → observability.rs
product_tools_tests.rs → products.rs
messaging_tests.rs   → messaging.rs
builds_tests.rs      → builds.rs
attachments_tests.rs → attachments.rs
tool_dispatch_tests.rs → mod.rs (dispatch logic)
```

## Run Coverage

```bash
cargo llvm-cov --package lib-mcp
cargo llvm-cov --package lib-mcp --html  # open target/llvm-cov/html/
```
