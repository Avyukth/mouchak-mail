# Documentation Index

> Complete documentation for MCP Agent Mail (Rust)

**Quick Links**: [README](../README.md) | [Quick Start](../README-QUICK.md) | [Architecture](ARCHITECTURE.md)

---

## Getting Started

| Document | Description |
|----------|-------------|
| [README](../README.md) | Full project documentation with diagrams |
| [README-QUICK](../README-QUICK.md) | Essential quickstart (1 page) |
| [SUMMARY](SUMMARY.md) | Project overview and reading order |
| [WALKTHROUGH](WALKTHROUGH.md) | Detailed usage walkthrough |

---

## Architecture & Design

| Document | Description |
|----------|-------------|
| [ARCHITECTURE](ARCHITECTURE.md) | System layers, crate boundaries, data flow |
| [AGENT_COMMUNICATION_PROTOCOL](AGENT_COMMUNICATION_PROTOCOL.md) | Message format and agent interaction patterns |
| [MCP_OPERATIONS_COMPARISON](MCP_OPERATIONS_COMPARISON.md) | MCP tools vs REST endpoints |

---

## Project Plans

| Document | Description |
|----------|-------------|
| [PROJECT_PLAN](PROJECT_PLAN.md) | High-level roadmap and milestones |
| [PYTHON_PORT_PLAN_v2](PYTHON_PORT_PLAN_v2.md) | Python-to-Rust migration strategy |
| [GAP_ANALYSIS](GAP_ANALYSIS.md) | Feature gaps vs Python implementation |
| [CRITICAL_GAP_ANALYSIS](CRITICAL_GAP_ANALYSIS.md) | Priority gaps for production |

---

## Production & Quality

| Document | Description |
|----------|-------------|
| [PRODUCTION_HARDENING_PLAN](PRODUCTION_HARDENING_PLAN.md) | Security, auth, observability, CI/CD |
| [E2E_TEST_PLAN](E2E_TEST_PLAN.md) | End-to-end testing strategy |
| [BENCHMARKING_PLAN](BENCHMARKING_PLAN.md) | Performance testing approach |
| [UNIFIED_BENCHMARKING_PLAN](UNIFIED_BENCHMARKING_PLAN.md) | Comprehensive benchmark suite |
| [api-quality-audit-report](api-quality-audit-report.md) | API quality assessment |

---

## Frontend & UI

| Document | Description |
|----------|-------------|
| [LEPTOS_MIGRATION_PLAN](LEPTOS_MIGRATION_PLAN.md) | SvelteKit to Leptos migration |
| [LEPTOS_PARITY_CHECKLIST](LEPTOS_PARITY_CHECKLIST.md) | UI feature parity tracking |
| [github-pages-deployment](github-pages-deployment.md) | Static site deployment guide |

---

## Infrastructure

| Document | Description |
|----------|-------------|
| [SINGLE_BINARY_PLAN](SINGLE_BINARY_PLAN.md) | Single binary distribution strategy |
| [BEADS_ENV](BEADS_ENV.md) | Beads task tracking integration |

---

## Reference Material

| Document | Description |
|----------|-------------|
| [mcp-agent-mail-python-tree](mcp-agent-mail-python-tree.md) | Python codebase structure |
| [mcp-agent-mail-python-beads-diff](mcp-agent-mail-python-beads-diff.md) | Detailed Python comparison |
| [Completed_task](Completed_task.md) | Completed work log |

---

## Crate Documentation

```bash
# Generate and view Rust docs
cargo doc --workspace --no-deps --open
```

| Crate | Purpose |
|-------|---------|
| `lib-core` | Domain models, BMC controllers, strong types |
| `lib-mcp` | MCP tool implementations (45+ tools) |
| `lib-server` | Axum HTTP server, middleware, routes |
| `lib-common` | Shared utilities, config, error types |
| `mcp-server` | Main server binary |
| `mcp-stdio` | STDIO transport for MCP |
| `mcp-cli` | Command-line client |

---

## External Resources

- [Model Context Protocol](https://modelcontextprotocol.io/) — MCP specification
- [Original Python Implementation](https://github.com/Dicklesworthstone/mcp_agent_mail) — Reference implementation
- [Axum](https://github.com/tokio-rs/axum) — HTTP framework
- [libsql](https://github.com/tursodatabase/libsql) — SQLite fork with extensions
