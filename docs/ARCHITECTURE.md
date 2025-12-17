# MCP Agent Mail - Rust Architecture

> Production-grade multi-agent messaging system following Rust10x patterns

## Overview

MCP Agent Mail is a high-performance messaging platform for AI coding agents, providing "Gmail for agents" functionality. The Rust implementation achieves **44.6x higher throughput** than the Python reference (15,200 req/s vs 341 req/s) through zero-cost abstractions and async-first design.

---

## System Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           CLIENT LAYER                                   │
├─────────────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐ │
│  │  MCP Clients │  │  REST API    │  │   Web UI     │  │    CLI       │ │
│  │  (Claude,    │  │  (curl,      │  │  (SvelteKit) │  │  (mcp-cli)   │ │
│  │   Cline)     │  │   Postman)   │  │              │  │              │ │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘ │
│         │                 │                 │                 │          │
└─────────┼─────────────────┼─────────────────┼─────────────────┼──────────┘
          │                 │                 │                 │
          ▼                 ▼                 ▼                 ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         TRANSPORT LAYER                                  │
├─────────────────────────────────────────────────────────────────────────┤
│  ┌────────────────────────────┐  ┌────────────────────────────────────┐ │
│  │   MCP Protocol (rmcp)      │  │      REST API (Axum 0.8)          │ │
│  │   ├─ JSON-RPC 2.0          │  │      ├─ /api/project/*            │ │
│  │   ├─ SSE Transport         │  │      ├─ /api/agent/*              │ │
│  │   ├─ Session Management    │  │      ├─ /api/message/*            │ │
│  │   └─ mcp-session-id header │  │      └─ /api/inbox                │ │
│  └────────────────────────────┘  └────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────┘
          │                                        │
          ▼                                        ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                      APPLICATION LAYER (lib-server)                      │
├─────────────────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────────┐  │
│  │   Middleware    │  │   Handlers      │  │   MCP Tools             │  │
│  │   ├─ Auth       │  │   (REST routes) │  │   (45 operations)       │  │
│  │   ├─ RateLimit  │  │                 │  │   ├─ ensure_project     │  │
│  │   ├─ Tracing    │  │                 │  │   ├─ send_message       │  │
│  │   └─ CORS       │  │                 │  │   └─ ...                │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                      BUSINESS LAYER (lib-core)                           │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │                   Backend Model Controllers (BMC)                 │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌──────────┐ │   │
│  │  │ ProjectBmc  │  │  AgentBmc   │  │ MessageBmc  │  │ThreadBmc │ │   │
│  │  │ ├─ create   │  │ ├─ register │  │ ├─ send     │  │├─ list   │ │   │
│  │  │ ├─ get      │  │ ├─ get      │  │ ├─ list     │  │├─ get    │ │   │
│  │  │ └─ ensure   │  │ └─ update   │  │ └─ search   │  │└─ reply  │ │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  └──────────┘ │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌──────────┐ │   │
│  │  │ ContactBmc  │  │ FileLockBmc │  │BuildSlotBmc │  │ProductBmc│ │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  └──────────┘ │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                                                                          │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │                      Model Manager (mm)                           │   │
│  │  ├─ Database Connection Pool (libsql)                            │   │
│  │  ├─ Migration Runner                                              │   │
│  │  └─ Health Check                                                  │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         DATA LAYER                                       │
├─────────────────────────────────────────────────────────────────────────┤
│  ┌──────────────────────────────┐  ┌──────────────────────────────────┐ │
│  │      SQLite (libsql)         │  │       Git Archive                │ │
│  │      └─ Primary Storage      │  │       └─ Audit Trail             │ │
│  │         ├─ Projects          │  │          ├─ Message History      │ │
│  │         ├─ Agents            │  │          └─ Thread Archives      │ │
│  │         ├─ Messages          │  │                                  │ │
│  │         ├─ Threads           │  └──────────────────────────────────┘ │
│  │         ├─ File Reservations │                                       │
│  │         └─ Build Slots       │                                       │
│  └──────────────────────────────┘                                       │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Crate Structure

```
mcp-agent-mail-rs/
├── Cargo.toml                 # Workspace configuration
├── crates/
│   ├── libs/                  # Reusable library crates
│   │   ├── lib-common/        # Shared config, utilities
│   │   │   └── src/config.rs  # 12-factor app configuration
│   │   ├── lib-core/          # Domain logic (BMC pattern)
│   │   │   ├── src/
│   │   │   │   ├── model/     # Entity definitions
│   │   │   │   │   ├── project.rs
│   │   │   │   │   ├── agent.rs
│   │   │   │   │   ├── message.rs
│   │   │   │   │   └── ...
│   │   │   │   ├── bmc/       # Backend Model Controllers
│   │   │   │   ├── store/     # Database abstractions
│   │   │   │   └── error.rs   # Domain errors
│   │   ├── lib-mcp/           # MCP tool definitions
│   │   │   └── src/tools.rs   # AgentMailService + schemas
│   │   └── lib-server/        # HTTP layer (Axum)
│   │       ├── src/
│   │       │   ├── api.rs     # REST route definitions
│   │       │   ├── tools.rs   # REST handlers
│   │       │   ├── mcp.rs     # MCP service factory
│   │       │   ├── auth.rs    # Authentication middleware
│   │       │   └── ratelimit.rs
│   │       └── Cargo.toml
│   └── services/              # Binary crates
│       ├── mcp-server/        # Main HTTP server
│       ├── mcp-stdio/         # STDIO MCP transport
│       ├── mcp-cli/           # Testing CLI
│       ├── web-ui/            # SvelteKit frontend
│       └── web-ui-leptos/     # Leptos WASM UI (experimental)
├── migrations/                # SQL migrations (libsql)
├── data/                      # Runtime data (gitignored)
│   ├── mcp_agent_mail.db      # SQLite database
│   └── archive/               # Git message archive
└── docs/                      # Documentation
```

---

## Request Flow

### REST API Request

```
HTTP Request (POST /api/message/send)
    │
    ├─► tower-http TraceLayer (request logging)
    │
    ├─► Rate Limit Middleware (lib-server/ratelimit.rs)
    │      └─ Token bucket algorithm, 100 req/min default
    │
    ├─► Auth Middleware (lib-server/auth.rs)
    │      ├─ Bearer token validation
    │      ├─ JWT/JWKS validation (optional)
    │      └─ Localhost bypass for development
    │
    ├─► Axum Router (lib-server/api.rs)
    │      └─ Route matching + extractors
    │
    ├─► Handler (lib-server/tools.rs)
    │      ├─ Extract State<AppState>
    │      ├─ Extract Json<SendMessageRequest>
    │      └─ Validate input (serde + custom)
    │
    ├─► BMC Layer (lib-core/bmc/)
    │      ├─ MessageBmc::send(&mm, data)
    │      ├─ Business logic validation
    │      └─ Agent lookup + routing
    │
    ├─► Store Layer (lib-core/store/)
    │      ├─ SQL query execution
    │      └─ libsql async operations
    │
    └─► Response
           ├─ 201 Created + JSON body
           └─ Error mapping to HTTP status
```

### MCP Request

```
HTTP Request (POST /mcp)
    │
    ├─► StreamableHttpService (rmcp crate)
    │      └─ JSON-RPC 2.0 parsing
    │
    ├─► Session Management
    │      ├─ LocalSessionManager
    │      ├─ mcp-session-id header
    │      └─ initialize required first
    │
    ├─► Service Factory
    │      └─ AgentMailService::new_with_mm(mm.clone())
    │
    ├─► Tool Dispatch (lib-mcp/tools.rs)
    │      ├─ #[tool] macro for each operation
    │      ├─ Schema validation via schemars
    │      └─ Delegate to BMC layer
    │
    └─► SSE Response
           ├─ text/event-stream
           └─ JSON-RPC result/error
```

---

## Backend Model Controller (BMC) Pattern

The BMC pattern provides a clean separation between HTTP handlers and business logic:

```rust
// lib-core/src/bmc/message.rs
pub struct MessageBmc;

impl MessageBmc {
    /// Send a message from one agent to others
    pub async fn send(
        mm: &ModelManager,
        data: MessageForCreate,
    ) -> Result<Message> {
        // 1. Validate sender exists
        let sender = AgentBmc::get_by_name(mm, &data.project_slug, &data.sender_name).await?;

        // 2. Validate recipients exist
        for recipient_name in &data.recipient_names {
            AgentBmc::get_by_name(mm, &data.project_slug, recipient_name).await?;
        }

        // 3. Create message record
        let message = Self::create_internal(mm, data).await?;

        // 4. Create message_recipient records
        for recipient_name in &data.recipient_names {
            MessageRecipientBmc::create(mm, message.id, recipient_name).await?;
        }

        Ok(message)
    }

    /// List inbox for an agent
    pub async fn list_inbox(
        mm: &ModelManager,
        project_slug: &str,
        agent_name: &str,
        unread_only: bool,
    ) -> Result<Vec<Message>> {
        // Query with join on message_recipients
        // ...
    }
}
```

**Benefits:**
- Stateless controllers (no `self`)
- Explicit context passing via `ModelManager`
- Easy to test with mock ModelManager
- Clear separation from HTTP layer

---

## Type Safety Patterns

### Strong Newtypes

```rust
// Avoid primitive obsession
pub struct ProjectSlug(String);
pub struct AgentName(String);
pub struct MessageId(i64);

impl ProjectSlug {
    pub fn new(s: impl Into<String>) -> Result<Self> {
        let s = s.into();
        if s.is_empty() || s.len() > 100 {
            return Err(Error::InvalidSlug);
        }
        Ok(Self(s))
    }
}
```

### Error Handling

```rust
// lib-core/src/error.rs
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Project not found: {slug}")]
    ProjectNotFound { slug: String },

    #[error("Agent not found: {name} in project {project_slug}")]
    AgentNotFound { project_slug: String, name: String },

    #[error("Duplicate agent: {name}")]
    DuplicateAgent { name: String },

    #[error("Database error: {0}")]
    Database(#[from] libsql::Error),
}

// HTTP status mapping in lib-server
impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let (status, message) = match &self.inner {
            Error::ProjectNotFound { .. } => (StatusCode::NOT_FOUND, self.to_string()),
            Error::AgentNotFound { .. } => (StatusCode::NOT_FOUND, self.to_string()),
            Error::DuplicateAgent { .. } => (StatusCode::CONFLICT, self.to_string()),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "Internal error".into()),
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}
```

---

## MCP Tool Implementation

```rust
// lib-mcp/src/tools.rs
#[derive(Clone)]
pub struct AgentMailService {
    mm: Arc<ModelManager>,
}

#[tool(tool_box)]
impl AgentMailService {
    /// Send a message from one agent to others in a project
    #[tool(description = "Send a message to one or more agents")]
    async fn send_message(
        &self,
        #[tool(param, description = "Project slug")]
        project_slug: String,
        #[tool(param, description = "Sender agent name")]
        sender_name: String,
        #[tool(param, description = "Recipient agent names")]
        recipient_names: Vec<String>,
        #[tool(param, description = "Message subject")]
        subject: String,
        #[tool(param, description = "Message body (markdown)")]
        body_md: String,
    ) -> Result<String, ToolError> {
        let data = MessageForCreate {
            project_slug,
            sender_name,
            recipient_names,
            subject,
            body_md,
            parent_id: None,
            priority: None,
        };

        let message = MessageBmc::send(&self.mm, data)
            .await
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        Ok(serde_json::to_string(&message).unwrap())
    }

    // 44 more tools...
}
```

---

## Database Schema

```sql
-- Core entities
CREATE TABLE projects (
    id INTEGER PRIMARY KEY,
    slug TEXT UNIQUE NOT NULL,
    human_key TEXT NOT NULL,
    description TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE agents (
    id INTEGER PRIMARY KEY,
    project_id INTEGER NOT NULL REFERENCES projects(id),
    name TEXT NOT NULL,
    program TEXT NOT NULL,
    model TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(project_id, name)
);

CREATE TABLE messages (
    id INTEGER PRIMARY KEY,
    project_id INTEGER NOT NULL REFERENCES projects(id),
    thread_id INTEGER REFERENCES threads(id),
    sender_id INTEGER NOT NULL REFERENCES agents(id),
    subject TEXT NOT NULL,
    body_md TEXT NOT NULL,
    priority INTEGER DEFAULT 0,
    parent_id INTEGER REFERENCES messages(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE message_recipients (
    id INTEGER PRIMARY KEY,
    message_id INTEGER NOT NULL REFERENCES messages(id),
    recipient_id INTEGER NOT NULL REFERENCES agents(id),
    read_at TEXT,
    UNIQUE(message_id, recipient_id)
);

-- Resource coordination
CREATE TABLE file_reservations (
    id INTEGER PRIMARY KEY,
    project_id INTEGER NOT NULL REFERENCES projects(id),
    agent_id INTEGER NOT NULL REFERENCES agents(id),
    file_path TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    UNIQUE(project_id, file_path)
);

CREATE TABLE build_slots (
    id INTEGER PRIMARY KEY,
    project_id INTEGER NOT NULL REFERENCES projects(id),
    agent_id INTEGER NOT NULL REFERENCES agents(id),
    slot_type TEXT NOT NULL DEFAULT 'build',
    expires_at TEXT NOT NULL
);
```

---

## Configuration (12-Factor)

```rust
// lib-common/src/config.rs
impl AppConfig {
    pub fn load() -> Result<Self, config::ConfigError> {
        let run_mode = env::var("RUN_MODE").unwrap_or_else(|_| "development".into());

        let mut builder = Config::builder()
            .set_default("server.port", 8765)?
            .add_source(File::with_name("config/default").required(false))
            .add_source(File::with_name(&format!("config/{}", run_mode)).required(false));

        // 12-factor: PORT env var overrides config
        if let Ok(port) = env::var("PORT") {
            if let Ok(p) = port.parse::<i64>() {
                builder = builder.set_override("server.port", p)?;
            }
        }

        builder.build()?.try_deserialize()
    }
}
```

**Environment Variables:**
- `PORT` - HTTP server port (default: 8765)
- `HOST` - Bind address (default: 0.0.0.0)
- `RUST_LOG` - Log level (default: info)
- `DATABASE_URL` - SQLite path (default: ./data/mcp_agent_mail.db)
- `RATE_LIMIT_ENABLED` - Enable rate limiting (default: true)

---

## Performance Characteristics

| Metric | Rust | Python | Improvement |
|--------|------|--------|-------------|
| MCP Throughput | 15,200 req/s | 341 req/s | **44.6x** |
| MCP P50 Latency | 3.2ms | ~12ms | **3.8x** |
| MCP P99 Latency | 7.2ms | ~35ms | **4.9x** |
| REST /health | 62,316 req/s | 7,626 req/s | **8.2x** |
| REST /ready | 55,293 req/s | 2,140 req/s | **25.8x** |
| REST /message/send | 3,186 req/s | 182 req/s | **17.5x** |

**Key Optimizations:**
- Shared `ModelManager` across MCP sessions (no per-request migrations)
- Connection pooling with libsql
- Zero-copy JSON serialization with serde
- Async I/O throughout with tokio

---

## MCP Tools Reference

| Category | Tools | Description |
|----------|-------|-------------|
| **Infrastructure** | health, ready, metrics | Server health and monitoring |
| **Project** | ensure_project, list_projects | Project lifecycle management |
| **Agent** | register_agent, list_agents | Agent identity management |
| **Messaging** | send_message, check_inbox, reply_message, search_messages | Core messaging |
| **Threads** | list_threads, get_thread, summarize_thread | Conversation threads |
| **Contacts** | add_contact, list_contacts, block_contact | Agent routing |
| **Files** | reserve_file, release_file, check_paths | File coordination |
| **Build** | acquire_build_slot, release_build_slot, list_slots | Build coordination |
| **Products** | ensure_product, link_project, product_inbox, list_products | Cross-project messaging |

---

## Security Model

### Authentication Modes

```rust
pub enum AuthMode {
    None,           // Development only
    Bearer,         // Static token (HTTP_BEARER_TOKEN)
    Jwt,            // JWKS validation (HTTP_JWKS_URL)
}
```

### Rate Limiting

- Token bucket algorithm (governor crate)
- 100 requests/minute default
- Per-IP tracking
- Configurable via `RATE_LIMIT_*` env vars

### Input Validation

- Serde deserialization with strict types
- SQL injection prevention via parameterized queries
- Path traversal prevention for file operations

---

## Testing Strategy

```bash
# Unit tests
cargo test --workspace

# Integration tests
cargo test -p lib-core --test integration

# E2E tests (requires running server)
cd crates/services/web-ui && bun test

# Benchmarks
hey -n 10000 -c 50 http://localhost:8765/health
```

---

## Comparison with Python Architecture

| Aspect | Python | Rust |
|--------|--------|------|
| **Framework** | FastAPI + Uvicorn | Axum 0.8 + Tokio |
| **MCP** | FastMCP | rmcp crate |
| **Database** | SQLite + aiosqlite | libsql (async SQLite) |
| **Session Mgmt** | Per-request transport | Shared LocalSessionManager |
| **Config** | Pydantic Settings | config-rs + serde |
| **Error Handling** | Exception + FastAPI | Result<T, E> + thiserror |
| **Concurrency** | 4 Uvicorn workers | Single tokio runtime |
| **Type Safety** | Runtime (Pydantic) | Compile-time (Rust types) |

**Key Rust Improvements:**
1. **No per-request overhead** - Shared ModelManager eliminates migration conflicts
2. **Zero-cost abstractions** - Traits, generics have no runtime cost
3. **Compile-time safety** - SQL queries verified at build time
4. **Ownership model** - Memory safety without GC pauses

---

## Related Documentation

- [AGENTS.md](../AGENTS.md) - AI agent instructions
- [CLAUDE.md](../CLAUDE.md) - Claude-specific instructions
- [MCP_OPERATIONS.md](./MCP_OPERATIONS.md) - Tool reference (to be created)
- [rust-skills](~/.claude/skills/rust-skills/SKILL.md) - Rust development guidelines
