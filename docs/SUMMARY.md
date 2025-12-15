# MCP Agent Mail (Rust Implementation)

## Project Goal
To re-implement the `mcp_agent_mail` system ("Gmail for coding agents") using a "Rust Native" approach. This involves replacing the original Python stack (FastAPI, SQLite+FTS5, GitPython) with high-performance, safe, and idiomatic Rust alternatives (Axum, Libsql, Git2) while adding a modern SvelteKit frontend.

The project follows strict quality protocols (A+ code standards, TDD, zero technical debt) derived from the `Depyler` project's guidelines.

## Critical Pivot: Phase 1.5 - Accelerated Logic Porting
**Reasoning**: A pure manual rewrite was too slow (<10% feature parity).
**New Strategy**: We adopted a hybrid approach—using `depyler` to transpile Python logic as reference "pseudo-code," then manually integrating it into our idiomatic `lib-core`. This accelerates porting the complex logic of 46 MCP tools while maintaining Rust quality.

## Achievements So Far (Post-Pivot)

### 1. Architecture & Core (`lib-core`)
*   **Modular Workspace**: Established `lib-core` (domain logic), `mcp-server` (web API), and `mcp-cli` (command-line interface).
*   **Storage Layer**:
    *   **Database**: Fully integrated `libsql` (Turso/SQLite) with manual schema migrations (replacing `sqlx` due to compatibility needs).
    *   **Git**: Implemented `git_store` using `git2` for high-performance "mailbox" operations (atomic commits, file history).
*   **Domain Logic**: Implemented BMC (Backend Model Controller) pattern for:
    *   `Project`: Create, get by slug, get by human key, ensure archive.
    *   `Agent`: Create, get by ID/name, list all.
    *   `Message`: Create (with Git commit), list inbox, get by ID.
*   **Error Handling**: Robust `thiserror`-based `ServerError` system handling DB, Git, IO, and logic errors.

### 2. API Layer (`mcp-server`)
*   **Web Server**: Built an `axum` 0.6 web server (compatible with `libsql`/`tonic` dependencies).
*   **Endpoints**: Implemented core REST API endpoints mirroring BMC logic:
    *   `POST /api/project/ensure`
    *   `POST /api/agent/register`
    *   `POST /api/message/send`
    *   `POST /api/inbox`
    *   `GET /api/projects`
    *   `GET /api/projects/:slug/agents`
    *   `GET /api/messages/:id`
    *   `GET /api/health`

### 3. Verification (`mcp-cli`)
*   **CLI Tool**: Functional CLI for testing core flows without a frontend.
*   **End-to-End Test**: Successfully verified:
    *   Creating a project ("demo-project")
    *   Creating an agent ("researcher")
    *   Sending a message (persists to DB + Git repo)
    *   Retrieving data via API.

## Planned Work (Upcoming)

### Phase 2: Frontend Scaffolding (SvelteKit) - **IMMEDIATE NEXT STEP**
*   Initialize `crates/services/web-ui` with **SvelteKit** + **Bun**.
*   Configure **TailwindCSS** with Material Design 3 theming.
*   Set up `adapter-static` for embedding the frontend into the Rust binary.

### Phase 3: Full Feature Parity (The "46 Tools")
*   Systematically port the remaining logic for file reservations, agent links, and search using the `transpiled_reference` strategy.
*   Expose these features as API endpoints.

### Phase 4: MCP Protocol & Search
*   Integrate `mcp-protocol-sdk` to expose the API as a standard MCP server for AI agents.
*   Enable and optimize FTS5 search in `libsql`.

## Current Status
**Backend Core & API: STABLE & FUNCTIONAL.**
We have a working backend that persists to both SQL and Git, serves a JSON API, and handles errors correctly. We are perfectly positioned to build the frontend.