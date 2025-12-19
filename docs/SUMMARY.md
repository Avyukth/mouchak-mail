# MCP Agent Mail (Rust) - Summary

## Project Goal
Port `mcp_agent_mail` ("Gmail for coding agents") from Python to a **native Rust** implementation that preserves API behavior while improving performance, safety, and operability. The Rust stack replaces FastAPI/SQLite/GitPython with **Axum**, **libsql**, and **git2**, and exposes both REST and MCP transports with a Rust-native UI strategy.

## Current Architecture (Rust-Native)
- **Multi-crate workspace**: `lib-core` (BMC domain logic), `lib-mcp` (MCP tools), `lib-server` (Axum HTTP), services (`mcp-server`, `mcp-stdio`, `mcp-cli`, `web-ui`, `web-ui-leptos`).
- **Storage**: libsql for primary data + git2-backed archive for durable message history.
- **Transport**: REST endpoints + MCP JSON-RPC (rmcp) + STDIO service.
- **UI**: SvelteKit implementation exists; Leptos migration is the target for a Rust/WASM web UI.

## Porting Strategy (Python → Rust)
- **Parity-first** with **BMC pattern** and strong types. Python logic is used as a behavioral reference, but Rust implementation remains idiomatic and safe.
- **Tool parity tracking** is documented in `docs/PROJECT_PLAN.md` and `docs/PYTHON_PORT_PLAN_v2.md`, with gaps and sequencing in `docs/GAP_ANALYSIS.md`.
- **Quality gates** follow Rust best practices: explicit error handling, no `unwrap` in production, strong newtypes, and TDD discipline (see `.tmp/skills/rust-skills/SKILL.md`).

## Major Workstreams
- **Worktree Integration (opt-in)**: Identity canonicalization, guard/hook composition, Git pathspec matching, and product-wide coordination to support multi-agent worktrees without breaking existing behavior. Reference: `PLAN_TO_NON_DISRUPTIVELY_INTEGRATE_WITH_THE_GIT_WORKTREE_APPROACH.md`.
- **Secure Mailbox Sharing**: Export a static, read-only mailbox bundle with scrubbing, integrity manifests, and optional encryption. Reference: `PLAN_TO_ENABLE_EASY_AND_SECURE_SHARING_OF_AGENT_MAILBOX.md`.
- **Production Hardening**: Auth/RBAC, rate limiting, observability, panic hooks, container hardening, and CI quality gates. Reference: `docs/PRODUCTION_HARDENING_PLAN.md` and `.tmp/skills/production-hardening-backend/SKILL.md`.
- **Web UI Migration**: Move from SvelteKit to Leptos (Rust/WASM) for a unified Rust stack. Reference: `docs/LEPTOS_MIGRATION_PLAN.md`.

## Verification & Performance
- **Testing**: E2E and integration plans in `docs/E2E_TEST_PLAN.md`, plus gap-driven test coverage goals in `docs/PRODUCTION_HARDENING_PLAN.md`.
- **Benchmarking**: Baseline and head-to-head plans in `docs/BENCHMARKING_PLAN.md` and `docs/UNIFIED_BENCHMARKING_PLAN.md`.
- **Architecture**: System design, request flow, and security model in `docs/ARCHITECTURE.md`.

## Current Status (High Level)
- Core Rust backend, data layer, REST + MCP transports exist and are functional.
- Feature parity and production readiness are **in progress**, guided by the port plan and gap analysis.
- UI is transitioning toward **Leptos**, with SvelteKit retained for continuity.

## Reading Order (Quick Start)
1. `docs/ARCHITECTURE.md` - System overview and crate boundaries.
2. `docs/PROJECT_PLAN.md` + `docs/PYTHON_PORT_PLAN_v2.md` - Parity roadmap.
3. `docs/GAP_ANALYSIS.md` - What’s missing vs Python.
4. Worktree + Sharing plans (links above) - multi-agent workflows and exports.
5. `docs/PRODUCTION_HARDENING_PLAN.md` - hardening and release readiness.
