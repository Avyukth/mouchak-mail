# E2E Test Plan: WASM-Native Web UI Testing with Probar

> Comprehensive plan for browser automation E2E tests using jugar-probar's Playwright-parity features.

## Executive Summary

This plan outlines a WASM-native E2E testing strategy using jugar-probar to achieve full test coverage of the MCP Agent Mail web UI while maintaining zero JavaScript dependencies.

### Key Metrics

| Metric | Target | Current |
|--------|--------|---------|
| Page Coverage | 100% (6 routes) | 0% |
| User Flow Coverage | 100% (12 flows) | 0% |
| Accessibility (WCAG AA) | 100% | 0% |
| Visual Regression | All critical paths | 0% |

---

## 1. Architecture

### 1.1 Dual-Runtime Strategy

```
┌─────────────────────────────────────────────────────────────┐
│                    Probar Test Suite                        │
├─────────────────────────────┬───────────────────────────────┤
│     WasmRuntime             │      BrowserController        │
│  (Logic + Unit Tests)       │   (E2E + Visual Regression)   │
├─────────────────────────────┼───────────────────────────────┤
│  • Deterministic replay     │  • Chrome automation          │
│  • Fuzzing/invariants       │  • Screenshot comparison      │
│  • Fast CI execution        │  • Production parity          │
│  • State verification       │  • Accessibility audits       │
└─────────────────────────────┴───────────────────────────────┘
```

### 1.2 Test File Structure

```
crates/tests/e2e/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── config.rs              # TestConfig (env vars)
│   ├── fixtures.rs            # Test data generators
│   ├── locators.rs            # Page element selectors (NEW)
│   ├── pages/                 # Page Object Models (NEW)
│   │   ├── mod.rs
│   │   ├── dashboard.rs
│   │   ├── projects.rs
│   │   ├── inbox.rs
│   │   └── agents.rs
│   └── helpers/               # Test utilities (NEW)
│       ├── mod.rs
│       ├── browser.rs
│       ├── api_setup.rs
│       └── assertions.rs
└── tests/
    ├── api.rs                 # REST API tests (existing)
    ├── web_ui.rs              # Basic smoke tests (existing)
    ├── browser/               # Full browser E2E (NEW)
    │   ├── dashboard.rs
    │   ├── projects.rs
    │   ├── agents.rs
    │   ├── inbox.rs
    │   └── messaging.rs
    ├── accessibility.rs       # WCAG compliance (NEW)
    ├── visual_regression.rs   # Screenshot diffs (NEW)
    └── fuzzing.rs             # Input fuzzing (NEW)
```

---

## 2. Web UI Route Coverage

### 2.1 Dashboard (`/`)

| Test ID | Test Case | Priority | Assertions |
|---------|-----------|----------|------------|
| D-001 | Page loads successfully | P0 | Title visible, no errors |
| D-002 | Health status indicator shows "ok" | P0 | Green indicator, "ok" text |
| D-003 | Health status shows "offline" when API down | P1 | Red indicator, error message |
| D-004 | Project count displays correctly | P0 | Count matches API response |
| D-005 | Quick Actions navigate correctly | P0 | Links work, pages load |
| D-006 | Recent projects list shows max 5 | P1 | Count <= 5, sorted by date |
| D-007 | Project links navigate to detail | P1 | Click navigates, slug correct |

**Locators:**
```rust
mod dashboard {
    pub const HEALTH_INDICATOR: &str = "[data-testid='health-status']";
    pub const HEALTH_TEXT: &str = "h3:has-text('Backend Status') + p";
    pub const PROJECT_COUNT: &str = "[data-testid='project-count']";
    pub const QUICK_ACTION_PROJECTS: &str = "a[href='/projects']";
    pub const QUICK_ACTION_INBOX: &str = "a[href='/inbox']";
    pub const RECENT_PROJECTS_LIST: &str = "[data-testid='recent-projects'] li";
}
```

### 2.2 Projects (`/projects`)

| Test ID | Test Case | Priority | Assertions |
|---------|-----------|----------|------------|
| P-001 | Projects list loads | P0 | Table visible, no errors |
| P-002 | Empty state when no projects | P1 | Empty message, create button |
| P-003 | New Project form opens | P0 | Form visible on button click |
| P-004 | Create project with valid path | P0 | Success, project in list |
| P-005 | Create project validates empty | P1 | Button disabled |
| P-006 | Cancel form closes | P1 | Form hidden, state reset |
| P-007 | Project row shows slug, path, date | P1 | All columns populated |
| P-008 | Click project navigates to detail | P0 | URL changes, page loads |

**Locators:**
```rust
mod projects {
    pub const NEW_PROJECT_BTN: &str = "button:has-text('New Project')";
    pub const PROJECT_PATH_INPUT: &str = "#projectPath";
    pub const CREATE_BTN: &str = "button[type='submit']:has-text('Create')";
    pub const CANCEL_BTN: &str = "button:has-text('Cancel')";
    pub const PROJECTS_TABLE: &str = "table";
    pub const PROJECT_ROW: &str = "tbody tr";
    pub const EMPTY_STATE: &str = "[data-testid='empty-state']";
}
```

### 2.3 Project Detail (`/projects/[slug]`)

| Test ID | Test Case | Priority | Assertions |
|---------|-----------|----------|------------|
| PD-001 | Agent list loads for project | P0 | Agents displayed |
| PD-002 | Breadcrumb shows project slug | P1 | Correct path |
| PD-003 | Register Agent form opens | P0 | Form visible |
| PD-004 | Register agent with name | P0 | Success, agent in grid |
| PD-005 | Agent card shows all fields | P1 | Name, program, model, task |
| PD-006 | View Inbox link works | P0 | Navigates with params |
| PD-007 | Empty state when no agents | P1 | Message, register button |

**Locators:**
```rust
mod project_detail {
    pub const BREADCRUMB: &str = "nav";
    pub const REGISTER_AGENT_BTN: &str = "button:has-text('Register Agent')";
    pub const AGENT_NAME_INPUT: &str = "#agentName";
    pub const AGENT_PROGRAM_INPUT: &str = "#agentProgram";
    pub const AGENT_MODEL_INPUT: &str = "#agentModel";
    pub const AGENT_TASK_INPUT: &str = "#agentTask";
    pub const AGENT_CARDS: &str = "[data-testid='agent-card']";
    pub const VIEW_INBOX_LINK: &str = "a:has-text('View Inbox')";
}
```

### 2.4 All Agents (`/agents`)

| Test ID | Test Case | Priority | Assertions |
|---------|-----------|----------|------------|
| A-001 | Agents list loads across projects | P0 | All agents displayed |
| A-002 | Search filters by name | P1 | Results match query |
| A-003 | Search filters by program | P1 | Results match query |
| A-004 | Project filter works | P1 | Only selected project agents |
| A-005 | Agent card shows project link | P1 | Link navigates correctly |
| A-006 | Empty search results message | P2 | Appropriate message |

**Locators:**
```rust
mod agents {
    pub const SEARCH_INPUT: &str = "#search";
    pub const PROJECT_FILTER: &str = "#projectFilter";
    pub const AGENT_GRID: &str = "[data-testid='agents-grid']";
    pub const AGENT_CARD: &str = "[data-testid='agent-card']";
    pub const STATS_TEXT: &str = "[data-testid='agent-stats']";
}
```

### 2.5 Inbox (`/inbox`)

| Test ID | Test Case | Priority | Assertions |
|---------|-----------|----------|------------|
| I-001 | Inbox loads with selectors | P0 | Dropdowns visible |
| I-002 | Project selector populates | P0 | All projects in dropdown |
| I-003 | Agent selector populates after project | P0 | Agents for project |
| I-004 | Messages load for agent | P0 | Message list visible |
| I-005 | Empty inbox message | P1 | Appropriate message |
| I-006 | Message shows subject, preview, date | P1 | All fields visible |
| I-007 | Importance badge displays | P1 | Badge color correct |
| I-008 | ACK badge displays when required | P1 | Badge visible |
| I-009 | Thread indicator shows | P2 | Icon visible |
| I-010 | Refresh button reloads | P1 | Messages refresh |
| I-011 | Compose button opens modal | P0 | Modal visible |
| I-012 | URL params pre-select filters | P2 | Correct selection |

**Locators:**
```rust
mod inbox {
    pub const PROJECT_SELECT: &str = "#projectSelect";
    pub const AGENT_SELECT: &str = "#agentSelect";
    pub const REFRESH_BTN: &str = "button:has-text('Refresh')";
    pub const COMPOSE_BTN: &str = "button:has-text('Compose')";
    pub const MESSAGE_LIST: &str = "[data-testid='message-list']";
    pub const MESSAGE_ITEM: &str = "[data-testid='message-item']";
    pub const IMPORTANCE_BADGE: &str = "[data-testid='importance-badge']";
    pub const ACK_BADGE: &str = "[data-testid='ack-badge']";
}
```

### 2.6 Message Detail (`/inbox/[id]`)

| Test ID | Test Case | Priority | Assertions |
|---------|-----------|----------|------------|
| M-001 | Message loads by ID | P0 | Content visible |
| M-002 | Subject displays correctly | P0 | Matches message |
| M-003 | Body renders (markdown) | P0 | Content displayed |
| M-004 | Metadata section shows IDs | P1 | All IDs visible |
| M-005 | Reply button opens modal | P0 | Modal visible |
| M-006 | Back button navigates | P0 | Returns to inbox |
| M-007 | 404 for invalid ID | P1 | Not found message |

### 2.7 Compose Message Modal

| Test ID | Test Case | Priority | Assertions |
|---------|-----------|----------|------------|
| C-001 | Modal opens with sender pre-filled | P0 | Sender name visible |
| C-002 | Recipient selection works | P0 | Toggle adds/removes |
| C-003 | Subject required validation | P1 | Error on empty |
| C-004 | Body required validation | P1 | Error on empty |
| C-005 | Recipient required validation | P1 | Error on none selected |
| C-006 | Importance dropdown works | P2 | Value changes |
| C-007 | ACK checkbox toggles | P2 | State changes |
| C-008 | Send message succeeds | P0 | Modal closes, inbox refreshes |
| C-009 | Cancel closes modal | P1 | Modal hidden |
| C-010 | Escape key closes modal | P2 | Modal hidden |
| C-011 | Reply pre-fills thread ID | P1 | Thread ID set |
| C-012 | Reply pre-fills subject with Re: | P1 | Subject correct |

---

## 3. User Flow Tests

### 3.1 Complete User Journeys

| Flow ID | Flow Name | Steps | Priority |
|---------|-----------|-------|----------|
| UF-001 | Project Creation | Dashboard → Projects → Create → Verify | P0 |
| UF-002 | Agent Registration | Projects → Detail → Register → Verify | P0 |
| UF-003 | Send First Message | Create Project → Register 2 Agents → Send Message | P0 |
| UF-004 | View Message Thread | Send → Inbox → View → Reply → Verify Thread | P0 |
| UF-005 | Cross-Project Agent View | Create 2 Projects → Register Agents → Agents Page | P1 |
| UF-006 | Search Agents | Agents Page → Search → Filter → Verify Results | P1 |
| UF-007 | High Importance Message | Compose → Set High → Send → Verify Badge | P2 |
| UF-008 | ACK Required Message | Compose → Set ACK → Send → Verify Badge | P2 |
| UF-009 | Empty State Navigation | Empty Projects → Create CTA → Success | P1 |
| UF-010 | Error Recovery | API Down → Error Message → API Up → Refresh | P1 |
| UF-011 | Deep Link Access | Direct URL → Correct Page State | P2 |
| UF-012 | Multi-Recipient Message | Compose → Select Multiple → Send → All Receive | P1 |

---

## 4. Accessibility Testing (WCAG 2.1 AA)

### 4.1 Automated Checks

| Check ID | WCAG Criterion | Test Description |
|----------|----------------|------------------|
| ACC-001 | 1.4.3 | Contrast ratio >= 4.5:1 for text |
| ACC-002 | 1.4.11 | Non-text contrast >= 3:1 |
| ACC-003 | 2.1.1 | All interactive elements keyboard accessible |
| ACC-004 | 2.4.1 | Skip to main content link |
| ACC-005 | 2.4.7 | Focus indicators visible |
| ACC-006 | 3.2.1 | No unexpected context changes on focus |
| ACC-007 | 4.1.1 | Valid HTML (no duplicate IDs) |
| ACC-008 | 4.1.2 | All form inputs have labels |

### 4.2 Probar Accessibility API

```rust
use jugar_probar::accessibility::{
    check_contrast, check_keyboard_nav, check_focus_visible
};

#[test]
fn test_dashboard_accessibility() {
    let report = AccessibilityReport::for_page("/");

    assert!(report.contrast_ratio >= 4.5, "WCAG AA contrast");
    assert!(report.all_interactive_keyboard_accessible());
    assert!(report.no_flash_violations());
}
```

---

## 5. Visual Regression Testing

### 5.1 Screenshot Comparison Strategy

| Viewport | Width | Height | Device |
|----------|-------|--------|--------|
| Mobile | 375 | 667 | iPhone SE |
| Tablet | 768 | 1024 | iPad |
| Desktop | 1280 | 720 | Standard |
| Wide | 1920 | 1080 | Full HD |

### 5.2 Critical Paths for Visual Testing

1. Dashboard (all states: loading, ok, offline, error)
2. Projects list (empty, populated, form open)
3. Inbox (empty, messages, compose modal)
4. Dark mode variants of all above

---

## 6. Fuzzing & Edge Cases

### 6.1 Input Fuzzing

| Target | Fuzzing Strategy |
|--------|------------------|
| Project Path | Unicode, special chars, long strings, XSS payloads |
| Agent Name | Empty, whitespace, duplicates |
| Message Subject | Max length, markdown injection |
| Message Body | Large content, malformed markdown |
| URL Params | Invalid slugs, non-numeric IDs |

### 6.2 Invariant Checks

```rust
Invariant::new("no_console_errors", |state| {
    state.console_errors.is_empty()
});

Invariant::new("no_network_failures", |state| {
    state.failed_requests.is_empty()
});

Invariant::new("responsive_layout", |state| {
    !state.has_horizontal_scroll()
});
```

---

## 7. Test Data Fixtures

### 7.1 Fixture Functions

```rust
impl TestFixtures {
    // Project fixtures
    pub fn unique_project_slug() -> String;
    pub fn project_with_agents(agent_count: u32) -> ProjectFixture;

    // Agent fixtures
    pub fn unique_agent_name() -> String;
    pub fn agent_pair() -> (String, String);

    // Message fixtures
    pub fn simple_message() -> MessageFixture;
    pub fn threaded_conversation(depth: u32) -> Vec<MessageFixture>;
    pub fn high_importance_message() -> MessageFixture;
    pub fn ack_required_message() -> MessageFixture;
}
```

---

## 8. CI/CD Integration

### 8.1 Test Execution Modes

```bash
# Fast CI (no browser, WASM-only)
cargo test -p e2e-tests --features wasm-only

# Full E2E (with browser)
cargo test -p e2e-tests --features browser

# Visual regression (generate baselines)
cargo test -p e2e-tests --features visual -- --generate-baselines

# Accessibility audit
cargo test -p e2e-tests --test accessibility

# Fuzzing (extended duration)
FUZZ_ITERATIONS=10000 cargo test -p e2e-tests --test fuzzing
```

### 8.2 Makefile Targets

```makefile
test-e2e:
	cargo test -p e2e-tests

test-e2e-browser:
	cargo test -p e2e-tests --features browser

test-e2e-a11y:
	cargo test -p e2e-tests --test accessibility

test-e2e-visual:
	cargo test -p e2e-tests --features visual
```

---

## 9. Implementation Priority

### Phase 1: Foundation (P0)

1. BrowserController setup with Chrome
2. Page Object Models for all routes
3. Core user flows (UF-001 to UF-004)
4. Basic assertions for all pages

### Phase 2: Coverage (P1)

1. Complete page-level tests
2. Remaining user flows
3. Error state handling
4. Accessibility audit basics

### Phase 3: Robustness (P2)

1. Visual regression baselines
2. Input fuzzing suite
3. Extended accessibility checks
4. Performance benchmarks

---

## 10. Test Data IDs (data-testid)

Required `data-testid` attributes to add to web UI:

```typescript
// Dashboard
'health-status'
'project-count'
'recent-projects'

// Projects
'projects-table'
'empty-state'
'new-project-form'

// Agents
'agents-grid'
'agent-card'
'agent-stats'

// Inbox
'message-list'
'message-item'
'importance-badge'
'ack-badge'
'compose-modal'
```

---

## Appendix A: Probar API Reference

### Assertions

```rust
Assertion::equals(&a, &b)
Assertion::in_range(value, min, max)
Assertion::is_true(condition, message)
Assertion::approx_eq(actual, expected, tolerance)
Assertion::contains(collection, item)
Assertion::string_contains(haystack, needle)
Assertion::matches_regex(text, pattern)
```

### Locators

```rust
Locator::id("element-id")
Locator::css("button.primary")
Locator::text("Submit")
Locator::testid("submit-btn")
Locator::role("button").and(Locator::text("Submit"))
```

### Browser Actions

```rust
browser.navigate(url)
browser.click(locator)
browser.fill(locator, text)
browser.select(locator, value)
browser.wait_for(locator)
browser.screenshot()
```
