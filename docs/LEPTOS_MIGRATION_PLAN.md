# Leptos Migration Plan: SvelteKit → Rust/WASM

> Port MCP Agent Mail web UI from SvelteKit to Leptos for zero-JS, full-stack Rust.

## Executive Summary

| Aspect | SvelteKit (Current) | Leptos (Target) |
|--------|---------------------|-----------------|
| Language | TypeScript | Rust |
| Runtime | JavaScript | WASM |
| Signals | `$state`, `$derived` | `signal()`, `Memo` |
| Routing | File-based | `leptos_router` |
| SSR | Built-in | Built-in |
| Styling | Tailwind CSS | Tailwind CSS (same) |
| Bundle | ~150KB JS | ~200KB WASM + 5KB shim |

---

## 1. Architecture Mapping

### 1.1 Project Structure

```
crates/services/web-ui-leptos/    # NEW
├── Cargo.toml
├── src/
│   ├── main.rs                   # Hydration entry
│   ├── lib.rs                    # App component
│   ├── app.rs                    # Root app + router
│   ├── api/
│   │   ├── mod.rs
│   │   └── client.rs             # Server functions
│   ├── components/
│   │   ├── mod.rs
│   │   ├── layout.rs             # Main layout
│   │   ├── compose_message.rs
│   │   ├── status_card.rs
│   │   └── agent_card.rs
│   └── pages/
│       ├── mod.rs
│       ├── dashboard.rs          # /
│       ├── projects.rs           # /projects
│       ├── project_detail.rs     # /projects/:slug
│       ├── agents.rs             # /agents
│       ├── inbox.rs              # /inbox
│       └── message_detail.rs     # /inbox/:id
├── style/
│   └── tailwind.css
├── public/
│   └── (static assets)
└── Trunk.toml                    # Build config
```

### 1.2 Route Mapping

| SvelteKit Route | Leptos Route | Component |
|-----------------|--------------|-----------|
| `/+page.svelte` | `/` | `Dashboard` |
| `/projects/+page.svelte` | `/projects` | `Projects` |
| `/projects/[slug]/+page.svelte` | `/projects/:slug` | `ProjectDetail` |
| `/agents/+page.svelte` | `/agents` | `Agents` |
| `/inbox/+page.svelte` | `/inbox` | `Inbox` |
| `/inbox/[id]/+page.svelte` | `/inbox/:id` | `MessageDetail` |
| `/+layout.svelte` | `App` wrapper | `Layout` |

---

## 2. Syntax Translation Guide

### 2.1 Reactive State

**SvelteKit (Svelte 5 Runes)**
```svelte
<script lang="ts">
  let count = $state(0);
  let doubled = $derived(count * 2);

  $effect(() => {
    console.log('Count changed:', count);
  });
</script>
```

**Leptos**
```rust
#[component]
fn Counter() -> impl IntoView {
    let (count, set_count) = signal(0);
    let doubled = Memo::new(move |_| count.get() * 2);

    Effect::new(move |_| {
        logging::log!("Count changed: {}", count.get());
    });

    view! { /* ... */ }
}
```

### 2.2 Event Handlers

**SvelteKit**
```svelte
<button onclick={() => count++}>Click</button>
<input bind:value={name} />
<form onsubmit={(e) => { e.preventDefault(); submit(); }}>
```

**Leptos**
```rust
view! {
    <button on:click=move |_| set_count.update(|n| *n += 1)>"Click"</button>
    <input prop:value=name on:input=move |ev| set_name.set(event_target_value(&ev)) />
    <form on:submit=move |ev| { ev.prevent_default(); submit(); }>
}
```

### 2.3 Conditional Rendering

**SvelteKit**
```svelte
{#if loading}
  <Spinner />
{:else if error}
  <Error message={error} />
{:else}
  <Content {data} />
{/if}
```

**Leptos**
```rust
view! {
    <Show
        when=move || !loading.get()
        fallback=|| view! { <Spinner /> }
    >
        <Show
            when=move || error.get().is_none()
            fallback=move || view! { <Error message=error.get().unwrap() /> }
        >
            <Content data=data />
        </Show>
    </Show>
}
```

### 2.4 List Rendering

**SvelteKit**
```svelte
{#each projects as project (project.id)}
  <ProjectCard {project} />
{/each}
```

**Leptos**
```rust
view! {
    <For
        each=move || projects.get()
        key=|project| project.id
        children=move |project| view! { <ProjectCard project=project /> }
    />
}
```

### 2.5 Component Props

**SvelteKit**
```svelte
<script lang="ts">
  interface Props {
    title: string;
    onClose: () => void;
  }
  let { title, onClose }: Props = $props();
</script>
```

**Leptos**
```rust
#[component]
fn Modal(
    title: String,
    #[prop(into)] on_close: Callback<()>,
) -> impl IntoView {
    view! {
        <div>
            <h2>{title}</h2>
            <button on:click=move |_| on_close.call(())>"Close"</button>
        </div>
    }
}
```

---

## 3. API Client Migration

### 3.1 Current TypeScript Client

```typescript
// $lib/api/client.ts
export async function getProjects(): Promise<Project[]> {
  return request<Project[]>('/projects');
}
```

### 3.2 Leptos Server Functions

```rust
// src/api/client.rs
use leptos::*;
use lib_core::models::Project;  // Share types with backend!

#[server(GetProjects)]
pub async fn get_projects() -> Result<Vec<Project>, ServerFnError> {
    let client = reqwest::Client::new();
    let projects = client
        .get("http://localhost:8765/api/projects")
        .send()
        .await?
        .json::<Vec<Project>>()
        .await?;
    Ok(projects)
}
```

### 3.3 Type Sharing with Backend

```toml
# crates/services/web-ui-leptos/Cargo.toml
[dependencies]
lib-core = { path = "../../libs/lib-core", features = ["serde"] }
```

```rust
// Direct import of backend types - no duplication!
use lib_core::models::{Project, Agent, Message};
```

---

## 4. Component Migration

### 4.1 Dashboard (`/`)

**Priority:** P0
**Complexity:** Medium

| SvelteKit Feature | Leptos Equivalent |
|-------------------|-------------------|
| `$effect` for data loading | `Resource` + `Suspense` |
| Health status indicator | Derived signal + CSS class |
| Project count card | `Memo` computed value |
| Recent projects list | `For` iteration |

```rust
#[component]
pub fn Dashboard() -> impl IntoView {
    let health = Resource::new(|| (), |_| check_health());
    let projects = Resource::new(|| (), |_| get_projects());

    view! {
        <div class="space-y-6">
            <h1 class="text-2xl font-bold">"Dashboard"</h1>

            // Health Status Card
            <Suspense fallback=|| view! { <Skeleton /> }>
                {move || health.get().map(|h| view! {
                    <StatusCard
                        status=h.status
                        class=if h.status == "ok" { "bg-green-500" } else { "bg-red-500" }
                    />
                })}
            </Suspense>

            // Projects Card
            <Suspense fallback=|| view! { <Skeleton /> }>
                {move || projects.get().map(|p| view! {
                    <Card title="Projects" count=p.len() />
                })}
            </Suspense>
        </div>
    }
}
```

### 4.2 Projects Page (`/projects`)

**Priority:** P0
**Complexity:** High (form handling)

| Feature | Implementation |
|---------|----------------|
| Project list table | `For` + `Resource` |
| Create project form | `ActionForm` + server function |
| Form validation | Derive from signals |
| Loading states | `Suspense` + `Transition` |

### 4.3 Inbox Page (`/inbox`)

**Priority:** P0
**Complexity:** High (cascading selects)

| Feature | Implementation |
|---------|----------------|
| Project selector | `<select>` + `on:change` |
| Agent selector (dependent) | `Resource` with dependency |
| Message list | `For` + `Resource` |
| Compose modal | Portal + `Show` |
| URL params | `use_query_map()` |

### 4.4 ComposeMessage Modal

**Priority:** P0
**Complexity:** High (complex form)

| Feature | Implementation |
|---------|----------------|
| Recipient toggle | `RwSignal<HashSet<String>>` |
| Form validation | Computed signals |
| Submit handling | `Action` + `ActionForm` |
| Close on Escape | `window_event_listener` |

---

## 5. Styling Strategy

### 5.1 Tailwind CSS Integration

```toml
# Trunk.toml
[[hooks]]
stage = "build"
command = "sh"
command_arguments = ["-c", "npx tailwindcss -i ./style/tailwind.css -o ./style/output.css"]
```

```rust
// tailwind.config.js
module.exports = {
  content: ["./src/**/*.rs"],
  // ... same config as SvelteKit
}
```

### 5.2 Dark Mode

```rust
#[component]
fn App() -> impl IntoView {
    let (dark_mode, set_dark_mode) = signal(false);

    view! {
        <div class=move || if dark_mode.get() { "dark" } else { "" }>
            <Router>
                // ...
            </Router>
        </div>
    }
}
```

---

## 6. Build & Deploy

### 6.1 Development

```bash
# Install Trunk
cargo install trunk

# Install wasm target
rustup target add wasm32-unknown-unknown

# Development server with hot reload
trunk serve --open
```

### 6.2 Production Build

```bash
# Optimized WASM build
trunk build --release

# Output in dist/
# - index.html
# - web-ui-leptos-*.wasm (brotli compressed)
# - web-ui-leptos-*.js (thin shim)
```

### 6.3 Cargo.toml

```toml
[package]
name = "web-ui-leptos"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
leptos = { version = "0.7", features = ["csr"] }
leptos_router = "0.7"
leptos_meta = "0.7"
console_error_panic_hook = "0.1"
wasm-bindgen = "0.2"
web-sys = { version = "0.3", features = ["Window", "Document"] }

# Shared types with backend
lib-core = { path = "../../libs/lib-core", default-features = false, features = ["serde"] }

[profile.release]
lto = true
opt-level = 'z'
codegen-units = 1
```

---

## 7. Migration Phases

### Phase 1: Foundation (P0)

1. Create `web-ui-leptos` crate scaffold
2. Setup Trunk build pipeline
3. Configure Tailwind CSS
4. Create `App` with router skeleton
5. Implement `Layout` component
6. Port `Dashboard` page

### Phase 2: Core Pages (P0)

7. Port `Projects` page + create form
8. Port `ProjectDetail` page + agent registration
9. Port `Inbox` page with cascading selects
10. Port `ComposeMessage` modal
11. Port `MessageDetail` page

### Phase 3: Polish (P1)

12. Port `Agents` page with search/filter
13. Add loading skeletons
14. Implement error boundaries
15. Add dark mode toggle
16. URL state synchronization

### Phase 4: Production (P2)

17. SSR setup (optional)
18. Performance optimization
19. Accessibility audit
20. E2E tests with Probar

---

## 8. Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Learning curve | Start with simple Dashboard page |
| WASM bundle size | Enable LTO, opt-level='z', brotli compression |
| Debug experience | `console_error_panic_hook`, browser devtools |
| CSS class strings | Keep existing Tailwind classes verbatim |
| API compatibility | Share types via lib-core, use server functions |

---

## 9. Success Criteria

- [ ] All 6 routes functional
- [ ] All user flows working (UF-001 to UF-012)
- [ ] Zero JavaScript in production bundle
- [ ] Lighthouse score >= 90
- [ ] WCAG AA compliant
- [ ] Bundle size < 500KB (compressed)
- [ ] E2E tests passing with Probar

---

## References

- [Leptos Book](https://book.leptos.dev)
- [Leptos GitHub](https://github.com/leptos-rs/leptos)
- [Leptos Router Docs](https://docs.rs/leptos_router/latest/leptos_router/)
- [Trunk Build Tool](https://trunkrs.dev/)
- [Tailwind + Leptos](https://book.leptos.dev/interlude_styling.html)
