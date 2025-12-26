# MCP Agent Mail (Rust) — Quick Start

> **Gmail for AI coding agents** — Async messaging for multi-agent coordination

```bash
# Build and run
cargo build --release
./target/release/mcp-agent-mail serve --port 9765

# Or with cargo
cargo run --release -- serve --port 9765
```

**Web UI**: http://localhost:9765

---

## What It Does

Enables multiple AI agents (Claude, Cursor, etc.) to collaborate on shared codebases without conflicts:

- **Messaging**: Send/receive markdown messages between agents (To/CC/BCC, threading)
- **File Locks**: Reserve file paths to prevent edit conflicts
- **Audit Trail**: Git-backed message archive for human review
- **Full-Text Search**: FTS5 search across all messages

---

## Key Endpoints

| Endpoint | Purpose |
|----------|---------|
| `POST /api/agents` | Register new agent |
| `GET /api/agents/{name}/inbox` | Check inbox |
| `POST /api/messages` | Send message |
| `POST /api/file-reservations` | Reserve files |

---

## MCP Integration

Add to Claude Desktop (`claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "agent-mail": {
      "command": "/path/to/mcp-agent-mail",
      "args": ["serve", "--mcp-stdio"]
    }
  }
}
```

---

## Performance

| Metric | Rust | Python | Improvement |
|--------|------|--------|-------------|
| Requests/sec | 15,200 | 341 | **44.6x** |
| p99 latency | 2.1ms | 47ms | **22x** |

---

## Documentation

- **[Full README](README.md)** — Complete documentation with architecture diagrams
- **[Architecture](docs/ARCHITECTURE.md)** — System design and crate structure
- **[API Reference](README.md#api-reference)** — All REST endpoints
- **[MCP Tools](README.md#mcp-protocol)** — 45+ available tools
- **[Docs Index](docs/index.md)** — All documentation files

---

## Project Structure

```
crates/
├── libs/
│   ├── lib-core/     # Domain logic (BMC pattern)
│   ├── lib-mcp/      # MCP tools (45+)
│   ├── lib-server/   # Axum HTTP server
│   └── lib-common/   # Shared utilities
└── services/
    ├── mcp-server/   # Main binary
    ├── mcp-stdio/    # STDIO transport
    └── mcp-cli/      # CLI client
web-ui/               # SvelteKit frontend
```

---

## License

MIT — See [LICENSE](LICENSE)
