# Python to Rust Port Plan v2.0 — Production-Grade Implementation

> **Reference Commit**: `f2b563dad55aa03fcb2a3563b773089f6f03ef50` (Python)
> **Target**: `mcp-agent-mail-rs` (Rust)
> **Analysis Date**: 2025-12-17
> **NIST SP 800-53 Compliance**: High-impact baseline

---

## Executive Summary

This document details the porting plan for changes made to the Python `mcp_agent_mail` codebase since the reference commit. The plan follows **Extreme TDD workflow** (RED-GREEN-REFACTOR), **Toyota Way quality gates** (Jidoka), and **NIST SP 800-53** security controls.

### Change Summary (40 commits, ~3,000 LoC)

| Category | Python Files | Lines Changed | Priority | Complexity |
|----------|--------------|---------------|----------|------------|
| Tool Consolidation | app.py | +2040/-984 | P0 | High (8/10) |
| Storage Robustness | storage.py | +454/-180 | P0 | High (9/10) |
| Guard System | guard.py | +28/-10 | P1 | Medium (6/10) |
| HTTP Layer | http.py | +67/-30 | P1 | Medium (5/10) |
| CLI Enhancements | cli.py | +257/-100 | P1 | Medium (5/10) |
| Database/FTS | db.py | +18/-8 | P2 | Low (3/10) |
| Config/Misc | config.py, llm.py, etc. | +100/-50 | P3 | Low (2/10) |

---

## Architecture Alignment

### Rust10x BMC Pattern Compliance

All ported code follows the Backend Model Controller pattern:

```rust
// ✅ Stateless controller
pub struct MessageBmc;

impl MessageBmc {
    // ✅ Explicit context + ModelManager
    pub async fn send(
        ctx: &Ctx,
        mm: &ModelManager,
        data: MessageForCreate,
    ) -> Result<Message> {
        // ✅ Validate via other BMCs
        // ✅ Execute business logic
        // ✅ Store via mm.db()
    }
}
```

### Production Hardening Controls

| Control | Implementation | NIST Control |
|---------|----------------|--------------|
| Input Validation | `ValidationError` enum with suggestions | SI-10 |
| Error Handling | `thiserror` with context preservation | AU-3 |
| Rate Limiting | Per-tool token bucket with JWT identity | SC-5 |
| Audit Logging | Structured tracing with correlation IDs | AU-2, AU-3 |
| Secret Management | Environment variables, no hardcoded secrets | SC-28 |
| Access Control | RBAC with capability checking | AC-3, AC-6 |

---

## Epic 1: Tool Consolidation & Input Validation (P0)

**Complexity Score**: 8/10
**Recommended Subtasks**: 6
**NIST Controls**: SI-10 (Input Validation), AU-3 (Audit)

### Background

Python consolidated `summarize_thread` tools and added comprehensive input validation with intelligent error suggestions for AI agents. This significantly improves agent UX and reduces retry loops.

---

### Task 1.1: Consolidate summarize_thread Tools

**ID**: PORT-1.1
**Complexity**: 6/10
**Files**: `lib-mcp/src/tools.rs`, `lib-core/src/model/message.rs`
**Dependencies**: None

**Problem Statement**:
Two separate tools (`summarize_thread`, `summarize_threads`) create API redundancy and confusion.

**Implementation**:

```rust
// lib-core/src/model/message.rs

/// Unified input type accepting single or multiple thread IDs
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ThreadIds {
    Single(String),
    Multiple(Vec<String>),
}

impl ThreadIds {
    pub fn into_vec(self) -> Vec<String> {
        match self {
            Self::Single(id) => vec![id],
            Self::Multiple(ids) => ids,
        }
    }
}

/// Unified summary result
#[derive(Debug, Serialize, JsonSchema)]
pub struct SummarizeResult {
    pub summaries: Vec<ThreadSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<ThreadError>>,
}

impl MessageBmc {
    /// Summarize one or more threads
    ///
    /// # Arguments
    /// * `thread_ids` - Single thread ID or array of thread IDs
    /// * `include_messages` - Include full messages in summary
    /// * `max_length` - Maximum summary length per thread
    pub async fn summarize_threads(
        ctx: &Ctx,
        mm: &ModelManager,
        thread_ids: ThreadIds,
        include_messages: Option<bool>,
        max_length: Option<usize>,
    ) -> Result<SummarizeResult> {
        let ids = thread_ids.into_vec();
        let mut summaries = Vec::with_capacity(ids.len());
        let mut errors = Vec::new();

        for thread_id in ids {
            match Self::summarize_single_thread(ctx, mm, &thread_id, include_messages, max_length).await {
                Ok(summary) => summaries.push(summary),
                Err(e) => errors.push(ThreadError {
                    thread_id,
                    error: e.to_string(),
                }),
            }
        }

        Ok(SummarizeResult {
            summaries,
            errors: if errors.is_empty() { None } else { Some(errors) },
        })
    }
}
```

**MCP Tool Definition**:

```rust
// lib-mcp/src/tools.rs

#[tool(
    name = "summarize_thread",
    description = "Summarize one or more message threads. Accepts single thread_id or array of thread_ids."
)]
pub async fn summarize_thread(
    #[arg(description = "Single thread ID (string) or multiple thread IDs (array)")]
    thread_ids: ThreadIds,
    #[arg(description = "Include full message bodies", default = false)]
    include_messages: Option<bool>,
    #[arg(description = "Maximum summary length in characters", default = 500)]
    max_length: Option<usize>,
) -> Result<SummarizeResult> {
    // ...
}
```

**Test Strategy** (RED phase first):

```rust
#[tokio::test]
async fn test_summarize_single_thread() {
    // Given: A thread with 3 messages
    // When: summarize_thread called with single ID
    // Then: Returns single summary
}

#[tokio::test]
async fn test_summarize_multiple_threads() {
    // Given: 3 threads with messages
    // When: summarize_thread called with array of IDs
    // Then: Returns 3 summaries
}

#[tokio::test]
async fn test_summarize_partial_failure() {
    // Given: 2 valid threads, 1 invalid
    // When: summarize_thread called with all 3
    // Then: Returns 2 summaries + 1 error, no panic
}
```

**Acceptance Criteria**:
- [ ] Single tool accepts both `String` and `Vec<String>` input
- [ ] JSON schema validates both input types
- [ ] Partial failures don't break entire operation
- [ ] Backward compatible with existing callers
- [ ] Deprecation notice for old `summarize_threads` tool

---

### Task 1.2: Implement Production-Grade Input Validation

**ID**: PORT-1.2
**Complexity**: 7/10
**Files**: `lib-core/src/error.rs`, `lib-core/src/utils/validation.rs` (new)
**Dependencies**: None
**NIST Controls**: SI-10, AU-3

**Problem Statement**:
AI agents frequently make input mistakes (wrong ID types, invalid paths, etc.). Clear, actionable error messages reduce retry loops.

**Implementation** (following `thiserror` patterns):

```rust
// lib-core/src/utils/validation.rs

use derive_more::From;
use serde::Serialize;

/// Validation error with actionable suggestion
#[derive(Debug, Clone, Serialize)]
pub struct ValidationFailure {
    pub field: String,
    pub provided: String,
    pub reason: String,
    pub suggestion: Option<String>,
    pub pattern: Option<String>,
}

/// Input validation errors with recovery hints
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Invalid {field}: {reason}")]
    InvalidField {
        field: String,
        provided: String,
        reason: String,
        suggestion: Option<String>,
    },

    #[error("Project key must be absolute path or human_key, got: {provided}")]
    InvalidProjectKey {
        provided: String,
        suggestion: String,
    },

    #[error("Agent name must match ^[a-zA-Z0-9_]{{1,64}}$, got: {provided}")]
    InvalidAgentName {
        provided: String,
        suggestion: String,
    },

    #[error("File path must be relative (no leading /), got: {provided}")]
    AbsolutePathNotAllowed {
        provided: String,
        suggestion: String,
    },

    #[error("TTL must be between {min}s and {max}s, got: {provided}s")]
    InvalidTtl {
        provided: u64,
        min: u64,
        max: u64,
        suggestion: u64,
    },

    #[error("Entity not found: {entity_type} with {identifier}")]
    NotFound {
        entity_type: String,
        identifier: String,
        similar: Vec<String>,
    },
}

impl ValidationError {
    /// Convert to MCP-compatible error response
    pub fn to_tool_error(&self) -> ToolError {
        ToolError {
            error_type: self.error_type(),
            message: self.to_string(),
            recoverable: true,  // All validation errors are recoverable
            context: self.context(),
        }
    }

    fn error_type(&self) -> &'static str {
        match self {
            Self::InvalidField { .. } => "InvalidInput",
            Self::InvalidProjectKey { .. } => "InvalidProjectKey",
            Self::InvalidAgentName { .. } => "InvalidAgentName",
            Self::AbsolutePathNotAllowed { .. } => "InvalidPath",
            Self::InvalidTtl { .. } => "InvalidTtl",
            Self::NotFound { .. } => "NotFound",
        }
    }

    fn context(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }
}
```

**Validation Functions**:

```rust
// lib-core/src/utils/validation.rs

use regex::Regex;
use lazy_static::lazy_static;

lazy_static! {
    static ref AGENT_NAME_RE: Regex = Regex::new(r"^[a-zA-Z0-9_]{1,64}$").unwrap();
    static ref HUMAN_KEY_RE: Regex = Regex::new(r"^[a-zA-Z0-9_-]{1,64}$").unwrap();
}

/// Validate and potentially sanitize agent name
pub fn validate_agent_name(name: &str) -> Result<(), ValidationError> {
    if AGENT_NAME_RE.is_match(name) {
        return Ok(());
    }

    Err(ValidationError::InvalidAgentName {
        provided: name.to_string(),
        suggestion: sanitize_agent_name(name),
    })
}

/// Sanitize agent name for suggestion
pub fn sanitize_agent_name(input: &str) -> String {
    input
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .take(64)
        .collect::<String>()
        .to_lowercase()
}

/// Validate project key (absolute path or human_key)
pub fn validate_project_key(key: &str) -> Result<(), ValidationError> {
    // Check if it's an absolute path
    if key.starts_with('/') {
        if std::path::Path::new(key).exists() {
            return Ok(());
        }
        // Path format but doesn't exist - might be valid
        return Ok(());
    }

    // Check if it's a valid human_key
    if HUMAN_KEY_RE.is_match(key) {
        return Ok(());
    }

    // Invalid - provide suggestion
    let suggestion = if key.contains('/') && !key.starts_with('/') {
        format!("/{}", key)  // Suggest making it absolute
    } else {
        sanitize_agent_name(key)  // Suggest as human_key
    };

    Err(ValidationError::InvalidProjectKey {
        provided: key.to_string(),
        suggestion,
    })
}

/// Validate file reservation path (must be relative)
pub fn validate_reservation_path(path: &str) -> Result<(), ValidationError> {
    if path.starts_with('/') {
        let suggestion = path.trim_start_matches('/').to_string();
        return Err(ValidationError::AbsolutePathNotAllowed {
            provided: path.to_string(),
            suggestion,
        });
    }
    Ok(())
}

/// Validate TTL within bounds
pub fn validate_ttl(ttl_seconds: u64) -> Result<(), ValidationError> {
    const MIN_TTL: u64 = 60;        // 1 minute
    const MAX_TTL: u64 = 604_800;   // 7 days
    const DEFAULT_TTL: u64 = 3600;  // 1 hour

    if ttl_seconds >= MIN_TTL && ttl_seconds <= MAX_TTL {
        return Ok(());
    }

    let suggestion = ttl_seconds.clamp(MIN_TTL, MAX_TTL);

    Err(ValidationError::InvalidTtl {
        provided: ttl_seconds,
        min: MIN_TTL,
        max: MAX_TTL,
        suggestion,
    })
}
```

**Test Strategy**:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_agent_names() {
        assert!(validate_agent_name("claude_1").is_ok());
        assert!(validate_agent_name("AGENT123").is_ok());
        assert!(validate_agent_name("a").is_ok());
    }

    #[test]
    fn test_invalid_agent_names_with_suggestions() {
        let err = validate_agent_name("claude-1").unwrap_err();
        if let ValidationError::InvalidAgentName { suggestion, .. } = err {
            assert_eq!(suggestion, "claude1");
        }

        let err = validate_agent_name("my agent!").unwrap_err();
        if let ValidationError::InvalidAgentName { suggestion, .. } = err {
            assert_eq!(suggestion, "myagent");
        }
    }

    #[test]
    fn test_absolute_path_rejection() {
        let err = validate_reservation_path("/src/lib.rs").unwrap_err();
        if let ValidationError::AbsolutePathNotAllowed { suggestion, .. } = err {
            assert_eq!(suggestion, "src/lib.rs");
        }
    }

    #[test]
    fn test_ttl_clamping() {
        assert!(validate_ttl(3600).is_ok());  // Valid

        let err = validate_ttl(30).unwrap_err();  // Too short
        if let ValidationError::InvalidTtl { suggestion, .. } = err {
            assert_eq!(suggestion, 60);
        }
    }
}
```

**Acceptance Criteria**:
- [ ] All MCP tools validate inputs before processing
- [ ] Errors include: field, provided value, reason, suggestion
- [ ] All validation errors marked `recoverable: true`
- [ ] Suggestions are actionable (not just "invalid input")
- [ ] 100% test coverage on validation module
- [ ] Integrated with tracing for audit log (AU-3)

---

### Task 1.3: Implement Agent Mistake Detection Helpers

**ID**: PORT-1.3
**Complexity**: 5/10
**Files**: `lib-core/src/utils/mistake_detection.rs` (new)
**Dependencies**: PORT-1.2

**Problem Statement**:
AI agents make predictable mistakes (confusing IDs, wrong field types). Proactive detection improves UX.

**Implementation**:

```rust
// lib-core/src/utils/mistake_detection.rs

use strsim::levenshtein;

/// Suggestion for detected mistake
#[derive(Debug, Clone)]
pub struct MistakeSuggestion {
    pub detected_issue: String,
    pub suggestion: String,
    pub confidence: f64,  // 0.0 - 1.0
}

/// Detect if input looks like an absolute path used as project_key
pub fn detect_path_as_project_key(input: &str) -> Option<MistakeSuggestion> {
    if input.contains('/') && !input.starts_with('/') {
        // Relative path used where absolute expected
        return Some(MistakeSuggestion {
            detected_issue: "Relative path provided, expected absolute path or human_key".into(),
            suggestion: format!("Use absolute path: /{}", input),
            confidence: 0.8,
        });
    }
    None
}

/// Detect if agent_name looks like a file path
pub fn detect_path_as_agent_name(input: &str) -> Option<MistakeSuggestion> {
    if input.contains('/') || input.contains('.') {
        let sanitized = input
            .split(&['/', '.'][..])
            .last()
            .unwrap_or(input)
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .collect::<String>();

        return Some(MistakeSuggestion {
            detected_issue: "Agent name contains path characters".into(),
            suggestion: format!("Use agent name: {}", sanitized),
            confidence: 0.9,
        });
    }
    None
}

/// Detect thread_id vs message_id confusion
#[derive(Debug, Clone, Copy)]
pub enum IdType {
    ThreadId,
    MessageId,
}

pub fn detect_id_confusion(input: &str, expected: IdType) -> Option<MistakeSuggestion> {
    // Thread IDs are typically user-defined strings
    // Message IDs are typically numeric
    let is_numeric = input.parse::<i64>().is_ok();

    match (expected, is_numeric) {
        (IdType::ThreadId, true) => Some(MistakeSuggestion {
            detected_issue: "Numeric ID provided where thread_id expected".into(),
            suggestion: "thread_id is a user-defined string (e.g., 'FEAT-123'), not a numeric message_id".into(),
            confidence: 0.7,
        }),
        (IdType::MessageId, false) if !input.parse::<i64>().is_ok() => Some(MistakeSuggestion {
            detected_issue: "Non-numeric value provided where message_id expected".into(),
            suggestion: "message_id must be numeric (e.g., 42)".into(),
            confidence: 0.7,
        }),
        _ => None,
    }
}

/// Find similar strings using Levenshtein distance
pub fn suggest_similar<'a>(
    input: &str,
    candidates: &'a [&str],
    max_distance: usize,
) -> Vec<&'a str> {
    let mut matches: Vec<_> = candidates
        .iter()
        .map(|c| (*c, levenshtein(input, c)))
        .filter(|(_, d)| *d <= max_distance)
        .collect();

    matches.sort_by_key(|(_, d)| *d);
    matches.into_iter().map(|(c, _)| c).take(3).collect()
}

/// Comprehensive mistake detection for tool inputs
pub fn analyze_input_mistakes(
    field: &str,
    value: &str,
    context: &MistakeContext,
) -> Vec<MistakeSuggestion> {
    let mut suggestions = Vec::new();

    match field {
        "project_key" => {
            if let Some(s) = detect_path_as_project_key(value) {
                suggestions.push(s);
            }
        }
        "agent_name" => {
            if let Some(s) = detect_path_as_agent_name(value) {
                suggestions.push(s);
            }
        }
        "thread_id" => {
            if let Some(s) = detect_id_confusion(value, IdType::ThreadId) {
                suggestions.push(s);
            }
        }
        "message_id" => {
            if let Some(s) = detect_id_confusion(value, IdType::MessageId) {
                suggestions.push(s);
            }
        }
        _ => {}
    }

    // Check for similar existing entities
    if let Some(existing) = context.get_existing_values(field) {
        let similar = suggest_similar(value, &existing, 3);
        if !similar.is_empty() {
            suggestions.push(MistakeSuggestion {
                detected_issue: format!("'{}' not found", value),
                suggestion: format!("Did you mean: {}?", similar.join(", ")),
                confidence: 0.8,
            });
        }
    }

    suggestions
}
```

**Test Strategy**:

```rust
#[test]
fn test_detect_path_as_agent_name() {
    let result = detect_path_as_agent_name("src/main.rs");
    assert!(result.is_some());
    assert!(result.unwrap().suggestion.contains("rs"));
}

#[test]
fn test_similar_suggestions() {
    let candidates = &["claude_1", "claude_2", "gemini_1"];
    let similar = suggest_similar("claued_1", candidates, 3);
    assert_eq!(similar.first(), Some(&"claude_1"));
}
```

**Acceptance Criteria**:
- [ ] Detects 5+ common mistake patterns
- [ ] Levenshtein similarity for entity suggestions
- [ ] Confidence scores for filtering low-quality suggestions
- [ ] Integrated with validation error responses

---

### Task 1.4: Conditional Build Slot Tool Registration

**ID**: PORT-1.4
**Complexity**: 3/10
**Files**: `lib-mcp/src/lib.rs`, `lib-common/src/config.rs`
**Dependencies**: None

**Problem Statement**:
Build slot tools should only register when `WORKTREES_ENABLED=true`.

**Implementation**:

```rust
// lib-common/src/config.rs

#[derive(Debug, Clone)]
pub struct McpConfig {
    /// Enable worktree-related features (build slots, pre-commit guard)
    pub worktrees_enabled: bool,

    /// Enable git identity features
    pub git_identity_enabled: bool,
}

impl McpConfig {
    pub fn from_env() -> Self {
        Self {
            worktrees_enabled: parse_bool_env("WORKTREES_ENABLED"),
            git_identity_enabled: parse_bool_env("GIT_IDENTITY_ENABLED"),
        }
    }

    /// Check if worktree features should be active
    pub fn worktrees_active(&self) -> bool {
        self.worktrees_enabled || self.git_identity_enabled
    }
}

fn parse_bool_env(key: &str) -> bool {
    std::env::var(key)
        .map(|v| matches!(v.to_lowercase().as_str(), "1" | "true" | "yes" | "t" | "y"))
        .unwrap_or(false)
}

// lib-mcp/src/lib.rs

pub fn register_tools(router: &mut ToolRouter, config: &McpConfig) {
    // Core tools - always registered
    router.register(ensure_project);
    router.register(register_agent);
    router.register(send_message);
    router.register(file_reservation_paths);
    // ... 40+ core tools

    // Build slot tools - conditional
    if config.worktrees_active() {
        tracing::info!("Registering build slot tools (WORKTREES_ENABLED=true)");
        router.register(acquire_build_slot);
        router.register(renew_build_slot);
        router.register(release_build_slot);
    } else {
        tracing::debug!("Build slot tools disabled (WORKTREES_ENABLED not set)");
    }
}
```

**Test Strategy**:

```rust
#[test]
fn test_tools_list_excludes_build_slots_when_disabled() {
    std::env::remove_var("WORKTREES_ENABLED");
    let config = McpConfig::from_env();
    let router = create_router(&config);

    let tools = router.list_tools();
    assert!(!tools.iter().any(|t| t.name == "acquire_build_slot"));
}

#[test]
fn test_tools_list_includes_build_slots_when_enabled() {
    std::env::set_var("WORKTREES_ENABLED", "1");
    let config = McpConfig::from_env();
    let router = create_router(&config);

    let tools = router.list_tools();
    assert!(tools.iter().any(|t| t.name == "acquire_build_slot"));
}
```

**Acceptance Criteria**:
- [ ] Build slot tools only appear when WORKTREES_ENABLED=true
- [ ] tools/list reflects actual available tools
- [ ] Truthy values: "1", "true", "yes", "t", "y" (case-insensitive)
- [ ] GIT_IDENTITY_ENABLED as alternative gate
- [ ] Startup logs indicate which mode is active

---

## Epic 2: Storage Robustness & File Handle Management (P0)

**Complexity Score**: 9/10
**Recommended Subtasks**: 5
**NIST Controls**: SC-5 (DoS Protection), AU-9 (Audit Protection)

### Background

Python made critical improvements to prevent file descriptor exhaustion and improve concurrent access to git repositories. This is essential for long-running production servers.

---

### Task 2.1: Implement LRU Repository Cache

**ID**: PORT-2.1
**Complexity**: 8/10
**Files**: `lib-core/src/store/repo_cache.rs` (new), `lib-core/src/store/git_archive.rs`
**Dependencies**: None
**NIST Controls**: SC-5

**Problem Statement**:
Opening git repositories leaks file descriptors. Need bounded cache with automatic eviction.

**Implementation** (following Rust ownership patterns):

```rust
// lib-core/src/store/repo_cache.rs

use git2::Repository;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, warn};

/// Thread-safe LRU cache for git repositories
///
/// Limits open file descriptors by evicting least-recently-used repos.
/// Default capacity: 8 repos (each repo can use 10-50 FDs)
pub struct RepoCache {
    cache: Arc<Mutex<LruCache<PathBuf, Arc<Mutex<Repository>>>>>,
    capacity: usize,
}

impl RepoCache {
    /// Create cache with specified capacity
    ///
    /// # Arguments
    /// * `capacity` - Max repos to cache (default 8, each uses ~10-50 FDs)
    pub fn new(capacity: usize) -> Self {
        let cap = NonZeroUsize::new(capacity)
            .expect("cache capacity must be > 0");

        Self {
            cache: Arc::new(Mutex::new(LruCache::new(cap))),
            capacity,
        }
    }

    /// Get or open repository at path
    ///
    /// Thread-safe with interior mutability via Arc<Mutex<Repository>>
    pub async fn get(&self, path: &Path) -> Result<Arc<Mutex<Repository>>> {
        let canonical = path.canonicalize()
            .map_err(|e| Error::InvalidPath {
                path: path.display().to_string(),
                source: e,
            })?;

        let mut cache = self.cache.lock().await;

        // Check cache first (updates LRU order)
        if let Some(repo) = cache.get(&canonical) {
            debug!(path = %canonical.display(), "Cache hit");
            return Ok(Arc::clone(repo));
        }

        // Open new repository
        debug!(path = %canonical.display(), "Cache miss, opening repo");
        let repo = Repository::open(&canonical)
            .map_err(|e| Error::GitOpen {
                path: canonical.display().to_string(),
                source: e,
            })?;

        let repo = Arc::new(Mutex::new(repo));

        // Insert into cache (may evict LRU entry)
        if cache.len() >= self.capacity {
            if let Some((evicted_path, _evicted_repo)) = cache.pop_lru() {
                debug!(path = %evicted_path.display(), "Evicted repo from cache");
                // Repository dropped here, releasing file handles
            }
        }

        cache.put(canonical.clone(), Arc::clone(&repo));

        Ok(repo)
    }

    /// Non-blocking check if path is cached
    ///
    /// Returns `None` if cache lock is held (doesn't block)
    pub fn peek(&self, path: &Path) -> Option<bool> {
        let canonical = path.canonicalize().ok()?;

        // Try non-blocking lock
        match self.cache.try_lock() {
            Ok(cache) => Some(cache.contains(&canonical)),
            Err(_) => None,  // Lock held, can't check
        }
    }

    /// Get cached repo without opening (for fast paths)
    pub async fn get_if_cached(&self, path: &Path) -> Option<Arc<Mutex<Repository>>> {
        let canonical = path.canonicalize().ok()?;
        let cache = self.cache.lock().await;
        cache.peek(&canonical).map(Arc::clone)
    }

    /// Current cache size
    pub async fn len(&self) -> usize {
        self.cache.lock().await.len()
    }

    /// Clear all cached repos (for testing/shutdown)
    pub async fn clear(&self) {
        let mut cache = self.cache.lock().await;
        cache.clear();
        debug!("Repo cache cleared");
    }
}

impl Default for RepoCache {
    fn default() -> Self {
        Self::new(8)  // 8 repos * ~50 FDs = ~400 FDs (well under ulimit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_repo() -> (TempDir, PathBuf) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().to_path_buf();
        Repository::init(&path).unwrap();
        (dir, path)
    }

    #[tokio::test]
    async fn test_cache_hit() {
        let cache = RepoCache::new(2);
        let (_dir, path) = create_test_repo().await;

        // First access - cache miss
        let _repo1 = cache.get(&path).await.unwrap();

        // Second access - cache hit
        let _repo2 = cache.get(&path).await.unwrap();

        assert_eq!(cache.len().await, 1);
    }

    #[tokio::test]
    async fn test_lru_eviction() {
        let cache = RepoCache::new(2);  // Capacity 2

        let (_dir1, path1) = create_test_repo().await;
        let (_dir2, path2) = create_test_repo().await;
        let (_dir3, path3) = create_test_repo().await;

        // Fill cache
        cache.get(&path1).await.unwrap();
        cache.get(&path2).await.unwrap();
        assert_eq!(cache.len().await, 2);

        // Add third - should evict path1 (LRU)
        cache.get(&path3).await.unwrap();
        assert_eq!(cache.len().await, 2);

        // path1 should be evicted
        assert!(cache.get_if_cached(&path1).await.is_none());
        assert!(cache.get_if_cached(&path2).await.is_some());
        assert!(cache.get_if_cached(&path3).await.is_some());
    }

    #[tokio::test]
    async fn test_peek_nonblocking() {
        let cache = RepoCache::new(2);
        let (_dir, path) = create_test_repo().await;

        // Not cached yet
        assert_eq!(cache.peek(&path), Some(false));

        // Add to cache
        cache.get(&path).await.unwrap();

        // Now cached
        assert_eq!(cache.peek(&path), Some(true));
    }
}
```

**Acceptance Criteria**:
- [ ] LRU cache with configurable capacity (default 8)
- [ ] Thread-safe access via `Arc<Mutex<_>>`
- [ ] `peek()` is non-blocking for hot paths
- [ ] Evicted repos properly dropped (FDs released)
- [ ] Benchmark shows no FD growth under sustained load
- [ ] Integration test with >8 concurrent projects

---

### Task 2.2: Implement Stale Lock Cleanup

**ID**: PORT-2.2
**Complexity**: 7/10
**Files**: `lib-core/src/store/archive_lock.rs` (new)
**Dependencies**: PORT-2.1
**NIST Controls**: AU-9

**Problem Statement**:
If process crashes while holding lock, stale locks block all future operations.

**Implementation**:

```rust
// lib-core/src/store/archive_lock.rs

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

/// Lock owner metadata for stale detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockOwner {
    pub pid: u32,
    pub timestamp: DateTime<Utc>,
    pub agent: Option<String>,
    pub hostname: String,
}

impl LockOwner {
    pub fn current(agent: Option<String>) -> Self {
        Self {
            pid: std::process::id(),
            timestamp: Utc::now(),
            agent,
            hostname: hostname::get()
                .map(|h| h.to_string_lossy().to_string())
                .unwrap_or_else(|_| "unknown".into()),
        }
    }

    /// Check if lock is stale (owner dead or too old)
    pub fn is_stale(&self, max_age: Duration) -> bool {
        // Check age
        if Utc::now() - self.timestamp > max_age {
            return true;
        }

        // Check if owner process is still alive
        if !is_process_alive(self.pid) {
            return true;
        }

        false
    }
}

/// Check if process with given PID is alive
fn is_process_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        // kill(pid, 0) checks existence without sending signal
        unsafe { libc::kill(pid as libc::pid_t, 0) == 0 }
    }

    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
        use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};

        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if handle == 0 {
                return false;
            }
            CloseHandle(handle);
            true
        }
    }

    #[cfg(not(any(unix, windows)))]
    {
        // Conservative: assume alive if we can't check
        true
    }
}

/// Advisory file lock with stale detection
pub struct ArchiveLock {
    lock_path: PathBuf,
    owner_path: PathBuf,
    inner: Mutex<()>,  // Process-level mutex
}

impl ArchiveLock {
    pub fn new(archive_path: &Path) -> Self {
        Self {
            lock_path: archive_path.join(".archive.lock"),
            owner_path: archive_path.join(".archive.lock.owner"),
            inner: Mutex::new(()),
        }
    }

    /// Acquire lock with timeout and stale cleanup
    pub async fn acquire(&self, agent: Option<String>, timeout: std::time::Duration) -> Result<LockGuard<'_>> {
        let deadline = std::time::Instant::now() + timeout;

        loop {
            // Try to acquire process-level lock first
            let _inner = self.inner.lock().await;

            // Check for stale file lock
            if self.lock_path.exists() {
                if let Some(owner) = self.read_owner().await {
                    if owner.is_stale(Duration::hours(1)) {
                        info!(
                            pid = owner.pid,
                            age = %owner.timestamp,
                            "Cleaning up stale lock"
                        );
                        self.force_cleanup().await?;
                    } else {
                        // Lock held by live process
                        if std::time::Instant::now() > deadline {
                            return Err(Error::LockTimeout {
                                path: self.lock_path.display().to_string(),
                                owner_pid: owner.pid,
                            });
                        }

                        // Brief sleep before retry
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                        continue;
                    }
                } else {
                    // Lock file exists but no owner metadata - assume stale
                    warn!("Lock file exists without owner metadata, forcing cleanup");
                    self.force_cleanup().await?;
                }
            }

            // Create lock
            fs::write(&self.lock_path, "").await?;

            // Write owner metadata
            let owner = LockOwner::current(agent);
            let owner_json = serde_json::to_string_pretty(&owner)?;
            fs::write(&self.owner_path, owner_json).await?;

            debug!(pid = owner.pid, "Lock acquired");

            return Ok(LockGuard {
                lock: self,
            });
        }
    }

    async fn read_owner(&self) -> Option<LockOwner> {
        let content = fs::read_to_string(&self.owner_path).await.ok()?;
        serde_json::from_str(&content).ok()
    }

    async fn force_cleanup(&self) -> Result<()> {
        let _ = fs::remove_file(&self.lock_path).await;
        let _ = fs::remove_file(&self.owner_path).await;
        Ok(())
    }

    async fn release(&self) -> Result<()> {
        fs::remove_file(&self.lock_path).await?;
        let _ = fs::remove_file(&self.owner_path).await;
        debug!("Lock released");
        Ok(())
    }
}

/// RAII guard for automatic lock release
pub struct LockGuard<'a> {
    lock: &'a ArchiveLock,
}

impl<'a> Drop for LockGuard<'a> {
    fn drop(&mut self) {
        // Spawn cleanup task (can't await in drop)
        let lock_path = self.lock.lock_path.clone();
        let owner_path = self.lock.owner_path.clone();

        tokio::spawn(async move {
            let _ = fs::remove_file(&lock_path).await;
            let _ = fs::remove_file(&owner_path).await;
        });
    }
}
```

**Test Strategy**:

```rust
#[tokio::test]
async fn test_stale_lock_cleanup() {
    let dir = TempDir::new().unwrap();
    let lock = ArchiveLock::new(dir.path());

    // Create fake stale lock with dead PID
    let fake_owner = LockOwner {
        pid: 999999999,  // Unlikely to exist
        timestamp: Utc::now() - Duration::hours(2),
        agent: None,
        hostname: "test".into(),
    };
    fs::write(
        dir.path().join(".archive.lock.owner"),
        serde_json::to_string(&fake_owner).unwrap()
    ).await.unwrap();
    fs::write(dir.path().join(".archive.lock"), "").await.unwrap();

    // Should clean up stale lock and acquire
    let guard = lock.acquire(None, std::time::Duration::from_secs(5)).await;
    assert!(guard.is_ok());
}
```

**Acceptance Criteria**:
- [ ] Stale locks from dead processes auto-cleaned
- [ ] Age-based cleanup for abandoned locks (>1 hour)
- [ ] Cross-platform PID liveness check (Unix + Windows)
- [ ] RAII guard ensures release on drop
- [ ] No data corruption on forced cleanup
- [ ] Tests simulate crash recovery scenarios

---

### Task 2.3: Fix Potential File Handle Leaks

**ID**: PORT-2.3
**Complexity**: 4/10
**Files**: `lib-core/src/store/attachments.rs`, `lib-core/src/store/git_archive.rs`
**Dependencies**: PORT-2.1

**Problem Statement**:
Audit existing code for potential file handle leaks, ensure proper cleanup.

**Implementation Checklist**:

```rust
// ❌ AVOID: Implicit drops that may delay cleanup
fn process_file(path: &Path) -> Result<Data> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    // ... process
    Ok(data)
    // File closed here, but timing is implicit
}

// ✅ PREFER: Explicit scope management
fn process_file(path: &Path) -> Result<Data> {
    let data = {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        // ... process into data
        data
    };  // File explicitly dropped here

    Ok(data)
}

// ✅ PREFER: Drop guard for critical resources
fn process_image(path: &Path) -> Result<ImageData> {
    struct ImageGuard(DynamicImage);
    impl Drop for ImageGuard {
        fn drop(&mut self) {
            // Ensure memory freed immediately
            debug!("Image buffer released");
        }
    }

    let img = ImageGuard(image::open(path)?);
    let processed = process(&img.0)?;
    Ok(processed)
}
```

**Audit Areas**:
1. `attachments.rs` - Image processing with `image` crate
2. `git_archive.rs` - Repository file operations
3. `precommit_guard.rs` - Script file generation

**Acceptance Criteria**:
- [ ] Audit all file operations in lib-core/src/store/
- [ ] Add explicit drops where FD lifetime matters
- [ ] Stress test: process 1000 attachments, verify FD count stable
- [ ] Document ownership patterns in code comments

---

## Epic 3: Guard System & Worktree Support (P1)

**Complexity Score**: 6/10
**Recommended Subtasks**: 4
**NIST Controls**: CM-7 (Least Functionality)

---

### Task 3.1: Honor WORKTREES_ENABLED Gate

**ID**: PORT-3.1
**Complexity**: 3/10
**Files**: `lib-core/src/model/precommit_guard.rs`
**Dependencies**: PORT-1.4

**Implementation**:

```rust
// lib-core/src/model/precommit_guard.rs

impl PrecommitGuard {
    /// Check if guard checks should run
    fn should_check(&self) -> bool {
        // Check primary gate
        if parse_bool_env("WORKTREES_ENABLED") {
            return true;
        }

        // Check alternative gate
        if parse_bool_env("GIT_IDENTITY_ENABLED") {
            return true;
        }

        debug!("Pre-commit guard disabled (WORKTREES_ENABLED not set)");
        false
    }

    pub fn check_reservations(&self, staged_files: &[PathBuf]) -> Result<CheckResult> {
        if !self.should_check() {
            return Ok(CheckResult::Skipped { reason: "WORKTREES_ENABLED not set" });
        }

        // ... actual checking logic
    }
}

fn parse_bool_env(key: &str) -> bool {
    std::env::var(key)
        .map(|v| matches!(
            v.to_lowercase().as_str(),
            "1" | "true" | "yes" | "t" | "y"
        ))
        .unwrap_or(false)
}
```

---

### Task 3.2: Add Advisory/Bypass Modes

**ID**: PORT-3.2
**Complexity**: 4/10
**Files**: `lib-core/src/model/precommit_guard.rs`
**Dependencies**: PORT-3.1

**Implementation**:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardMode {
    /// Block commits on conflict (default)
    Enforce,
    /// Log warning but allow commit
    Warn,
    /// Skip all checks
    Bypass,
}

impl GuardMode {
    pub fn from_env() -> Self {
        // Bypass takes precedence
        if std::env::var("AGENT_MAIL_BYPASS")
            .map(|v| v == "1")
            .unwrap_or(false)
        {
            return Self::Bypass;
        }

        // Then check mode
        match std::env::var("AGENT_MAIL_GUARD_MODE")
            .map(|v| v.to_lowercase())
            .as_deref()
        {
            Ok("warn") | Ok("advisory") => Self::Warn,
            Ok("enforce") | Ok("block") => Self::Enforce,
            _ => Self::Enforce,  // Default
        }
    }
}

impl PrecommitGuard {
    pub fn check_reservations(&self, staged_files: &[PathBuf]) -> Result<CheckResult> {
        let mode = GuardMode::from_env();

        if mode == GuardMode::Bypass {
            info!("Pre-commit guard bypassed (AGENT_MAIL_BYPASS=1)");
            return Ok(CheckResult::Bypassed);
        }

        // ... check for conflicts ...

        if !conflicts.is_empty() {
            match mode {
                GuardMode::Enforce => {
                    return Err(Error::ReservationConflict { conflicts });
                }
                GuardMode::Warn => {
                    warn!(
                        conflicts = ?conflicts,
                        "File reservation conflicts detected (advisory mode)"
                    );
                    return Ok(CheckResult::WarningsOnly { conflicts });
                }
                GuardMode::Bypass => unreachable!(),
            }
        }

        Ok(CheckResult::Clean)
    }
}
```

---

### Task 3.3: Add Pre-push Guard Support

**ID**: PORT-3.3
**Complexity**: 5/10
**Files**: `lib-core/src/model/precommit_guard.rs`
**Dependencies**: PORT-3.2

**Implementation**:

```rust
/// Render pre-push hook script
pub fn render_prepush_script(config: &GuardConfig) -> String {
    format!(r#"#!/bin/sh
# MCP Agent Mail Pre-Push Guard
# Generated: {timestamp}

set -e

SERVER_URL="{server_url}"
PROJECT_KEY="{project_key}"

# Exit early if disabled
if [ "${{AGENT_MAIL_BYPASS:-}}" = "1" ]; then
    exit 0
fi

# Read remote refs from stdin
while read local_ref local_sha remote_ref remote_sha; do
    # Skip if no local ref (delete)
    if [ "$local_sha" = "0000000000000000000000000000000000000000" ]; then
        continue
    fi

    # Check for conflicts
    response=$(curl -sf -X POST "$SERVER_URL/api/guard/check-push" \
        -H "Content-Type: application/json" \
        -d '{{"project_key":"'"$PROJECT_KEY"'","local_sha":"'"$local_sha"'","remote_sha":"'"$remote_sha"'"}}' \
        2>/dev/null) || {{
        echo "Warning: Could not reach agent mail server, allowing push" >&2
        continue
    }}

    # Parse conflicts
    conflicts=$(echo "$response" | grep -o '"conflicts":\[[^]]*\]' | grep -o '\[[^]]*\]')
    if [ -n "$conflicts" ] && [ "$conflicts" != "[]" ]; then
        echo "ERROR: Push blocked due to file reservation conflicts:" >&2
        echo "$conflicts" >&2
        exit 1
    fi
done

exit 0
"#,
        timestamp = Utc::now().to_rfc3339(),
        server_url = config.server_url,
        project_key = config.project_key,
    )
}
```

---

### Task 3.4: Support Custom core.hooksPath

**ID**: PORT-3.4
**Complexity**: 3/10
**Files**: `lib-core/src/model/precommit_guard.rs`
**Dependencies**: None

**Implementation**:

```rust
/// Get hooks directory respecting git config
fn get_hooks_dir(repo_path: &Path) -> Result<PathBuf> {
    let repo = Repository::open(repo_path)?;

    // Check for custom hooks path
    if let Ok(config) = repo.config() {
        if let Ok(hooks_path) = config.get_string("core.hooksPath") {
            let path = PathBuf::from(&hooks_path);

            // Handle relative paths (resolved from repo root)
            let resolved = if path.is_relative() {
                repo_path.join(&path)
            } else {
                path
            };

            debug!(hooks_path = %resolved.display(), "Using custom hooks path");
            return Ok(resolved);
        }
    }

    // Default: .git/hooks
    Ok(repo_path.join(".git/hooks"))
}
```

---

## Epic 4: HTTP Layer & Rate Limiting (P1)

**Complexity Score**: 5/10
**Recommended Subtasks**: 2
**NIST Controls**: SC-5 (DoS Protection)

---

### Task 4.1: Fix JWT Identity in Rate Limiting

**ID**: PORT-4.1
**Complexity**: 4/10
**Files**: `lib-server/src/ratelimit.rs`
**Dependencies**: None

**Implementation**:

```rust
// lib-server/src/ratelimit.rs

/// Generate rate limit bucket key
fn get_bucket_key(
    req: &Request<Body>,
    claims: Option<&Claims>,
) -> String {
    let ip = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ci| ci.0.ip().to_string())
        .unwrap_or_else(|| "unknown".into());

    // Include JWT subject for per-user limits
    match claims {
        Some(c) => format!("{}:{}", c.sub, ip),
        None => ip,
    }
}
```

---

### Task 4.2: Add Per-Tool Rate Limiting

**ID**: PORT-4.2
**Complexity**: 5/10
**Files**: `lib-server/src/ratelimit.rs`
**Dependencies**: PORT-4.1

**Implementation**:

```rust
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub default_rps: u32,
    pub default_burst: u32,
    pub tool_limits: HashMap<String, ToolLimit>,
}

#[derive(Debug, Clone)]
pub struct ToolLimit {
    pub rps: u32,
    pub burst: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        let mut tool_limits = HashMap::new();

        // Write operations - lower limits
        for tool in ["send_message", "reply_message", "file_reservation_paths"] {
            tool_limits.insert(tool.into(), ToolLimit { rps: 10, burst: 20 });
        }

        // Read operations - higher limits
        for tool in ["fetch_inbox", "search_messages", "list_agents"] {
            tool_limits.insert(tool.into(), ToolLimit { rps: 100, burst: 200 });
        }

        Self {
            default_rps: 50,
            default_burst: 100,
            tool_limits,
        }
    }
}
```

---

## Epic 5: Database & FTS Improvements (P2)

**Complexity Score**: 3/10
**Recommended Subtasks**: 2

---

### Task 5.1: Handle FTS5 Leading Wildcards

**ID**: PORT-5.1
**Complexity**: 3/10
**Files**: `lib-core/src/model/message.rs`

**Implementation**:

```rust
/// Sanitize FTS5 query - handle leading wildcards and special chars
fn sanitize_fts_query(query: &str) -> Result<String> {
    let trimmed = query.trim();

    if trimmed.is_empty() {
        return Err(Error::InvalidQuery("Query cannot be empty".into()));
    }

    // FTS5 doesn't support leading wildcards
    if trimmed.starts_with('*') {
        let without_leading = trimmed.trim_start_matches('*');
        if without_leading.is_empty() {
            return Err(Error::InvalidQuery("Query cannot be only wildcards".into()));
        }
        // Convert to prefix search
        return Ok(format!("{}*", without_leading));
    }

    // Escape special FTS5 characters
    let escaped = trimmed
        .replace('"', "\"\"")  // Escape quotes
        .replace('\\', "\\\\"); // Escape backslashes

    Ok(escaped)
}
```

---

### Task 5.2: Graceful FTS5 Error Handling

**ID**: PORT-5.2
**Complexity**: 3/10
**Files**: `lib-core/src/model/message.rs`
**Dependencies**: PORT-5.1

**Implementation**:

```rust
pub async fn search_messages(
    mm: &ModelManager,
    query: &str,
    limit: Option<i32>,
) -> Result<Vec<MessageSearchResult>> {
    let sanitized = sanitize_fts_query(query)?;

    let result = sqlx::query_as::<_, MessageSearchResult>(
        "SELECT * FROM messages_fts WHERE messages_fts MATCH ? ORDER BY rank LIMIT ?"
    )
    .bind(&sanitized)
    .bind(limit.unwrap_or(50))
    .fetch_all(mm.db())
    .await;

    match result {
        Ok(rows) => Ok(rows),
        Err(e) if is_fts_syntax_error(&e) => {
            warn!(query = %query, error = %e, "FTS5 query syntax error");
            // Return empty with explanation rather than error
            Ok(vec![])
        }
        Err(e) => Err(e.into()),
    }
}

fn is_fts_syntax_error(e: &sqlx::Error) -> bool {
    match e {
        sqlx::Error::Database(db_err) => {
            db_err.message().contains("fts5") ||
            db_err.message().contains("syntax error")
        }
        _ => false,
    }
}
```

---

## Epic 6: CLI Enhancements (P1)

**Complexity Score**: 5/10
**Recommended Subtasks**: 3

---

### Task 6.1: Add 'am' Shell Alias

**ID**: PORT-6.1
**Complexity**: 2/10
**Files**: `scripts/install.sh`

---

### Task 6.2: Improve Installer

**ID**: PORT-6.2
**Complexity**: 4/10
**Files**: `scripts/install.sh`

---

### Task 6.3: Port Validation

**ID**: PORT-6.3
**Complexity**: 3/10
**Files**: `mcp-agent-mail/src/main.rs`

---

## Epic 7: Testing Infrastructure (P1)

**Complexity Score**: 6/10
**Recommended Subtasks**: 3

### Task 7.1: Add Concurrency Tests

**ID**: PORT-7.1
**Complexity**: 6/10
**Files**: `lib-core/tests/concurrent_*.rs`

Port 12 concurrency tests covering parallel MCP operations.

### Task 7.2: Add Guard/Worktree Tests

**ID**: PORT-7.2
**Complexity**: 5/10
**Files**: `lib-core/tests/guard_worktree_tests.rs`

Port 18 guard worktree tests.

### Task 7.3: Add Image Edge Case Tests

**ID**: PORT-7.3
**Complexity**: 5/10
**Files**: `lib-core/tests/image_edge_tests.rs`

Port 26 image processing edge case tests.

---

## Beads Structure Summary

```
Epic: Python Port v2 (PORT)
├── PORT-1.1: Consolidate summarize_thread (P0, 6/10)
├── PORT-1.2: Input validation with suggestions (P0, 7/10)
├── PORT-1.3: Agent mistake detection (P0, 5/10)
├── PORT-1.4: Conditional build slot registration (P0, 3/10)
├── PORT-2.1: LRU repository cache (P0, 8/10)
├── PORT-2.2: Stale lock cleanup (P0, 7/10)
├── PORT-2.3: File handle leak audit (P0, 4/10)
├── PORT-3.1: WORKTREES_ENABLED gate (P1, 3/10)
├── PORT-3.2: Advisory/bypass modes (P1, 4/10)
├── PORT-3.3: Pre-push guard support (P1, 5/10)
├── PORT-3.4: Custom core.hooksPath (P1, 3/10)
├── PORT-4.1: JWT identity rate limiting (P1, 4/10)
├── PORT-4.2: Per-tool rate limiting (P1, 5/10)
├── PORT-5.1: FTS5 leading wildcards (P2, 3/10)
├── PORT-5.2: FTS5 error handling (P2, 3/10)
├── PORT-6.1: 'am' shell alias (P1, 2/10)
├── PORT-6.2: Installer improvements (P1, 4/10)
├── PORT-6.3: Port validation (P1, 3/10)
├── PORT-7.1: Concurrency tests (P1, 6/10)
├── PORT-7.2: Guard/worktree tests (P1, 5/10)
└── PORT-7.3: Image edge case tests (P1, 5/10)
```

**Total Tasks**: 21
**P0 (Critical)**: 7 tasks
**P1 (High)**: 12 tasks
**P2 (Medium)**: 2 tasks

---

## Quality Gates (Per Task)

Each task must pass before merge:

```bash
# 1. Compile
cargo check --all-targets

# 2. Format
cargo fmt --check

# 3. Lint
cargo clippy --all-targets -- -D warnings

# 4. Tests (specific to task)
cargo test -p lib-core <task_tests>

# 5. Coverage (>= 85%)
cargo llvm-cov --package lib-core

# 6. TDG Grade (>= B)
pmat analyze tdg --fail-on-violation --min-grade B
```

---

## Commit Convention

Follow Extreme TDD workflow:

```bash
# RED Phase
git commit -m "[RED] PORT-1.2: Add failing tests for input validation"

# GREEN Phase
git commit -m "[GREEN] PORT-1.2: Implement validation with suggestions"

# REFACTOR Phase
git commit -m "[REFACTOR] PORT-1.2: Extract common patterns, add docs

Coverage: 92% ✅
Mutation: 87% ✅
Grade: A ✅"
```

---

## Awaiting Approval

Ready to create beads in `/Users/amrit/Documents/Projects/Rust/mouchak/mcp-agent-mail-rs/.beads/` with this structure.

**Approve to proceed with `bd create` commands.**
