# Web UI Transformation Plan: Leptos → SvelteKit

> **Goal**: Replace Leptos WASM frontend with SvelteKit while maintaining single-binary deployment.
> **Reference**: Python mcp_agent_mail implementation (Alpine.js + Jinja2 + Tailwind)
> **Created**: 2025-12-22
> **Tracking**: Use `bd` for issue tracking per AGENTS.md

---

## Executive Summary

| Aspect | Current (Leptos) | Target (SvelteKit) | Reference (Python) |
|--------|------------------|-------------------|-------------------|
| **Framework** | Leptos 0.8.14 (Rust/WASM) | SvelteKit 2.0 + Svelte 5 | Alpine.js + Jinja2 |
| **Build Output** | `web-ui-leptos/dist/` | `web-ui/build/` | Server-side rendered |
| **Embedding** | rust-embed → binary | rust-embed → binary | N/A (Python server) |
| **Pages** | 12 complete pages | Port all 12 | Reference templates |
| **Components** | 34+ (incl. Magic UI) | shadcn-svelte + Magic UI | CDN-based libs |
| **Bundle Size** | ~2MB WASM | ~200KB JS (target) | N/A |
| **Design** | Indigo/Purple theme | Match Python UI | Gmail-inspired |

---

## Design System Reference

### Color System (from Python base.html)

```javascript
// Tailwind Config - Match Python Implementation
colors: {
  primary: {
    50: '#eef2ff',   // Indigo-50
    100: '#e0e7ff',
    200: '#c7d2fe',
    300: '#a5b4fc',
    400: '#818cf8',
    500: '#6366f1',  // Primary brand
    600: '#4f46e5',  // Hover state
    700: '#4338ca',
    800: '#3730a3',
    900: '#312e81',
    950: '#1e1b4b',
  },
  success: { 50: '#f0fdf4', 100: '#dcfce7', 500: '#10b981', 600: '#059669', 700: '#047857' },
  warning: { 50: '#fffbeb', 100: '#fef3c7', 500: '#f59e0b', 600: '#d97706', 700: '#b45309' },
  danger: { 50: '#fef2f2', 100: '#fee2e2', 500: '#ef4444', 600: '#dc2626', 700: '#b91c1c' },
}
```

### Typography

```css
font-family: 'Inter', system-ui, -apple-system, sans-serif;

/* Scale */
--text-xs: 12px;   /* Labels, badges */
--text-sm: 14px;   /* Body text */
--text-base: 16px; /* Default */
--text-lg: 18px;   /* Subheadings */
--text-xl: 20px;   /* Brand name */
--text-2xl: 24px;  /* Section headings */
--text-3xl: 30px;  /* Modal titles */
```

### Shadows & Animations

```javascript
boxShadow: {
  soft: '0 2px 8px 0 rgba(0, 0, 0, 0.05)',
  medium: '0 4px 16px 0 rgba(0, 0, 0, 0.08)',
  large: '0 8px 32px 0 rgba(0, 0, 0, 0.12)',
  glow: '0 0 20px rgba(99, 102, 241, 0.3)',
},
animation: {
  'fade-in': 'fadeIn 0.2s ease-in-out',
  'slide-up': 'slideUp 0.3s ease-out',
  'slide-down': 'slideDown 0.3s ease-out',
  'scale-in': 'scaleIn 0.2s ease-out',
  'shimmer': 'shimmer 2s infinite',
}
```

---

## Phase 0: Infrastructure Preparation

### 0.1 Create Feature Branch

```bash
git checkout -b feature/sveltekit-web-ui
```

### 0.2 Update Embedding Configuration

**File**: `crates/libs/lib-server/src/embedded.rs`

```rust
// CHANGE: Point to SvelteKit build output
#[derive(Embed)]
#[folder = "../../services/web-ui/build"]
pub struct Assets;
```

### 0.3 Update Static File Handler

**File**: `crates/libs/lib-server/src/static_files.rs`

```rust
// Update cache strategy for SvelteKit output structure
let cache_control = if path.contains("/_app/immutable/") {
    // SvelteKit immutable assets - cache forever
    "public, max-age=31536000, immutable"
} else if path.ends_with(".js") || path.ends_with(".css") {
    "public, max-age=86400"
} else if path == "index.html" {
    "public, max-age=0, must-revalidate"
} else {
    "public, max-age=86400"
};
```

### 0.4 Update SvelteKit Configuration

**File**: `crates/services/web-ui/svelte.config.js`

```javascript
import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

const config = {
    preprocess: vitePreprocess(),
    kit: {
        adapter: adapter({
            pages: 'build',
            assets: 'build',
            fallback: 'index.html',  // SPA mode for Rust embedding
            precompress: true,
            strict: true
        }),
        paths: { base: '' },
        alias: {
            $components: 'src/lib/components',
            $api: 'src/lib/api',
            $stores: 'src/lib/stores'
        }
    }
};

export default config;
```

---

## Phase 1: Dependencies & shadcn-svelte Setup

### 1.1 Install Core Dependencies

```bash
cd crates/services/web-ui

# Initialize shadcn-svelte
bunx shadcn-svelte@latest init

# Add core dependencies
bun add bits-ui clsx tailwind-merge tailwind-variants
bun add lucide-svelte          # Icons (match Python's Lucide)
bun add mode-watcher           # Dark mode
bun add cmdk-sv                # Command palette (Cmd+K)
bun add marked dompurify       # Markdown rendering
bun add prismjs                # Code syntax highlighting
bun add dayjs                  # Date formatting
bun add @floating-ui/dom       # Tooltips
bun add fuse.js                # Fuzzy search
```

### 1.2 Configure Tailwind (Match Python Theme)

**File**: `tailwind.config.js`

```javascript
import { fontFamily } from 'tailwindcss/defaultTheme';

/** @type {import('tailwindcss').Config} */
export default {
    darkMode: 'class',
    content: ['./src/**/*.{html,js,svelte,ts}'],
    theme: {
        extend: {
            fontFamily: {
                sans: ['Inter', ...fontFamily.sans],
            },
            colors: {
                // Match Python implementation exactly
                primary: {
                    50: '#eef2ff', 100: '#e0e7ff', 200: '#c7d2fe',
                    300: '#a5b4fc', 400: '#818cf8', 500: '#6366f1',
                    600: '#4f46e5', 700: '#4338ca', 800: '#3730a3',
                    900: '#312e81', 950: '#1e1b4b',
                },
                success: {
                    50: '#f0fdf4', 100: '#dcfce7', 500: '#10b981',
                    600: '#059669', 700: '#047857', 900: '#064e3b',
                },
                warning: {
                    50: '#fffbeb', 100: '#fef3c7', 500: '#f59e0b',
                    600: '#d97706', 700: '#b45309', 900: '#78350f',
                },
                danger: {
                    50: '#fef2f2', 100: '#fee2e2', 500: '#ef4444',
                    600: '#dc2626', 700: '#b91c1c', 900: '#7f1d1d',
                },
            },
            boxShadow: {
                soft: '0 2px 8px 0 rgba(0, 0, 0, 0.05)',
                medium: '0 4px 16px 0 rgba(0, 0, 0, 0.08)',
                large: '0 8px 32px 0 rgba(0, 0, 0, 0.12)',
                glow: '0 0 20px rgba(99, 102, 241, 0.3)',
            },
            animation: {
                'fade-in': 'fadeIn 0.2s ease-in-out',
                'slide-up': 'slideUp 0.3s ease-out',
                'slide-down': 'slideDown 0.3s ease-out',
                'scale-in': 'scaleIn 0.2s ease-out',
                'shimmer': 'shimmer 2s infinite',
                'slide-in': 'slideIn 0.2s ease-out',
            },
            keyframes: {
                fadeIn: { '0%': { opacity: '0' }, '100%': { opacity: '1' } },
                slideUp: { '0%': { transform: 'translateY(10px)', opacity: '0' }, '100%': { transform: 'translateY(0)', opacity: '1' } },
                slideDown: { '0%': { transform: 'translateY(-10px)', opacity: '0' }, '100%': { transform: 'translateY(0)', opacity: '1' } },
                scaleIn: { '0%': { transform: 'scale(0.95)', opacity: '0' }, '100%': { transform: 'scale(1)', opacity: '1' } },
                shimmer: { '0%': { backgroundPosition: '-1000px 0' }, '100%': { backgroundPosition: '1000px 0' } },
                slideIn: { '0%': { transform: 'translateX(-10px)', opacity: '0' }, '100%': { transform: 'translateX(0)', opacity: '1' } },
            },
        },
    },
};
```

### 1.3 Add shadcn Components

```bash
# Core primitives
bunx shadcn-svelte@latest add button card input textarea label
bunx shadcn-svelte@latest add checkbox switch select dialog alert
bunx shadcn-svelte@latest add badge avatar tabs tooltip progress
bunx shadcn-svelte@latest add skeleton separator breadcrumb pagination
bunx shadcn-svelte@latest add sonner  # Toast notifications
bunx shadcn-svelte@latest add command # Command palette
bunx shadcn-svelte@latest add popover dropdown-menu scroll-area
bunx shadcn-svelte@latest add toggle-group resizable
```

---

## Phase 2: Port Core Components

### 2.1 Component Structure

```
src/lib/components/
├── ui/                           # shadcn components (auto-generated)
│   ├── button/
│   ├── card/
│   ├── input/
│   ├── command/                  # Cmd+K palette
│   └── ...
├── layout/
│   ├── Layout.svelte             # Main layout (from Python base.html)
│   ├── Sidebar.svelte            # Navigation sidebar
│   ├── Header.svelte             # Sticky header with glassmorphism
│   └── Footer.svelte
├── mail/
│   ├── UnifiedInbox.svelte       # Gmail-style split view
│   ├── MessageList.svelte        # Left pane: message list
│   ├── MessageDetail.svelte      # Right pane: message detail
│   ├── MessageItem.svelte        # Individual message row
│   ├── FilterBar.svelte          # Collapsible filter panel
│   ├── SearchBar.svelte          # Global search with Cmd+K hint
│   ├── ComposeMessage.svelte     # Message composer
│   ├── ThreadView.svelte         # Thread conversation view
│   └── BulkActions.svelte        # Select all, mark read, etc.
├── projects/
│   ├── ProjectCard.svelte
│   ├── ProjectList.svelte
│   └── FileReservationTable.svelte
├── agents/
│   ├── AgentCard.svelte
│   └── AgentList.svelte
├── common/
│   ├── EmptyState.svelte         # Unified empty state component
│   ├── KeyboardShortcut.svelte   # <kbd> badge component
│   ├── ProjectBadge.svelte       # Color-coded project badge
│   ├── ImportanceBadge.svelte    # Urgent/High/Normal/Low
│   └── SortDropdown.svelte       # Newest/Oldest/Sender/Longest
└── magic/                        # Animated components (port from Leptos)
    ├── TypingText.svelte
    ├── NumberCounter.svelte
    ├── AnimatedGradient.svelte
    ├── ShimmerText.svelte
    ├── GridPattern.svelte
    └── BlurFade.svelte
```

### 2.2 Key Component: UnifiedInboxManager (Svelte Store)

**File**: `src/lib/stores/unifiedInbox.ts`

Port the Python `unifiedInboxManager()` Alpine.js component to Svelte stores:

```typescript
import { writable, derived, get } from 'svelte/store';
import type { Message, Filter } from '$api/types';

// State
export const allMessages = writable<Message[]>([]);
export const searchQuery = writable('');
export const showFilters = writable(false);
export const sortBy = writable<'newest' | 'oldest' | 'sender' | 'longest'>('newest');
export const viewMode = writable<'split' | 'list'>('split');
export const isFullscreen = writable(false);
export const selectedMessage = writable<Message | null>(null);
export const selectedMessages = writable<number[]>([]);

// Filters
export const filters = writable<Filter>({
    project: '',
    sender: '',
    recipient: '',
    importance: '',
    hasThread: ''
});

// Auto-refresh
export const autoRefreshEnabled = writable(true);
export const autoRefreshSeconds = writable(45);
export const isRefreshing = writable(false);
export const lastRefreshTime = writable<Date | null>(null);
export const refreshError = writable<string | null>(null);

// Derived: Unique values for filter dropdowns
export const uniqueProjects = derived(allMessages, $messages =>
    [...new Set($messages.map(m => m.project_name))].sort()
);
export const uniqueSenders = derived(allMessages, $messages =>
    [...new Set($messages.map(m => m.sender))].sort()
);
export const uniqueRecipients = derived(allMessages, $messages => {
    const all = $messages.flatMap(m => m.recipients.split(',').map(r => r.trim()));
    return [...new Set(all)].filter(Boolean).sort();
});

// Derived: Filtered & sorted messages
export const filteredMessages = derived(
    [allMessages, searchQuery, filters, sortBy],
    ([$messages, $query, $filters, $sort]) => {
        let result = [...$messages];

        // Search filter
        if ($query) {
            const q = $query.toLowerCase();
            result = result.filter(m =>
                m.subject.toLowerCase().includes(q) ||
                m.sender.toLowerCase().includes(q) ||
                m.recipients.toLowerCase().includes(q) ||
                m.excerpt.toLowerCase().includes(q) ||
                m.project_name.toLowerCase().includes(q)
            );
        }

        // Apply filters
        if ($filters.project) result = result.filter(m => m.project_name === $filters.project);
        if ($filters.sender) result = result.filter(m => m.sender === $filters.sender);
        if ($filters.recipient) result = result.filter(m => m.recipients.includes($filters.recipient));
        if ($filters.importance) result = result.filter(m => m.importance === $filters.importance);
        if ($filters.hasThread === 'true') result = result.filter(m => m.thread_id);
        if ($filters.hasThread === 'false') result = result.filter(m => !m.thread_id);

        // Sort
        switch ($sort) {
            case 'newest': result.sort((a, b) => new Date(b.created_ts).getTime() - new Date(a.created_ts).getTime()); break;
            case 'oldest': result.sort((a, b) => new Date(a.created_ts).getTime() - new Date(b.created_ts).getTime()); break;
            case 'sender': result.sort((a, b) => a.sender.localeCompare(b.sender)); break;
            case 'longest': result.sort((a, b) => b.body_length - a.body_length); break;
        }

        return result;
    }
);

// Derived: Check if any filters are active
export const filtersActive = derived(filters, $f =>
    Boolean($f.project || $f.sender || $f.recipient || $f.importance || $f.hasThread)
);

// Actions
export function clearFilters() {
    filters.set({ project: '', sender: '', recipient: '', importance: '', hasThread: '' });
}

export function toggleSelectAll() {
    const current = get(selectedMessages);
    const filtered = get(filteredMessages);
    if (current.length === filtered.length) {
        selectedMessages.set([]);
    } else {
        selectedMessages.set(filtered.map(m => m.id));
    }
}

export function selectNextMessage() {
    const messages = get(filteredMessages);
    const current = get(selectedMessage);
    if (!current) {
        selectedMessage.set(messages[0] || null);
        return;
    }
    const idx = messages.findIndex(m => m.id === current.id);
    if (idx < messages.length - 1) {
        selectedMessage.set(messages[idx + 1]);
    }
}

export function selectPreviousMessage() {
    const messages = get(filteredMessages);
    const current = get(selectedMessage);
    if (!current) return;
    const idx = messages.findIndex(m => m.id === current.id);
    if (idx > 0) {
        selectedMessage.set(messages[idx - 1]);
    }
}
```

### 2.3 Key Component: UnifiedInbox Layout

**File**: `src/lib/components/mail/UnifiedInbox.svelte`

```svelte
<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { browser } from '$app/environment';
    import * as Command from '$lib/components/ui/command';
    import { Button } from '$lib/components/ui/button';
    import { Input } from '$lib/components/ui/input';
    import { Checkbox } from '$lib/components/ui/checkbox';
    import { Search, SlidersHorizontal, Columns2, List, Maximize2, Minimize2, RefreshCw } from 'lucide-svelte';

    import {
        allMessages, searchQuery, showFilters, sortBy, viewMode,
        isFullscreen, selectedMessage, filteredMessages, filtersActive,
        autoRefreshEnabled, isRefreshing, lastRefreshTime,
        selectNextMessage, selectPreviousMessage
    } from '$stores/unifiedInbox';

    import MessageList from './MessageList.svelte';
    import MessageDetail from './MessageDetail.svelte';
    import FilterBar from './FilterBar.svelte';
    import { fetchUnifiedInbox } from '$api/client';

    let refreshInterval: NodeJS.Timeout | null = null;

    onMount(() => {
        loadMessages();
        setupKeyboardShortcuts();
        if ($autoRefreshEnabled) {
            startAutoRefresh();
        }
    });

    onDestroy(() => {
        if (refreshInterval) clearInterval(refreshInterval);
    });

    async function loadMessages() {
        $isRefreshing = true;
        try {
            const data = await fetchUnifiedInbox();
            $allMessages = data.messages;
            $lastRefreshTime = new Date();
            // Auto-select first message in split view
            if ($viewMode === 'split' && !$selectedMessage && data.messages.length > 0) {
                $selectedMessage = data.messages[0];
            }
        } catch (e) {
            console.error('Failed to load messages:', e);
        } finally {
            $isRefreshing = false;
        }
    }

    function startAutoRefresh() {
        refreshInterval = setInterval(loadMessages, 45000);
    }

    function setupKeyboardShortcuts() {
        if (!browser) return;

        function handleKeydown(e: KeyboardEvent) {
            // Cmd+K or / to focus search
            if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
                e.preventDefault();
                document.getElementById('unified-search')?.focus();
            }
            if (e.key === '/' && document.activeElement?.tagName !== 'INPUT') {
                e.preventDefault();
                document.getElementById('unified-search')?.focus();
            }
            // j/k for navigation (vim style)
            if (e.key === 'j' && document.activeElement?.tagName !== 'INPUT') {
                e.preventDefault();
                selectNextMessage();
            }
            if (e.key === 'k' && document.activeElement?.tagName !== 'INPUT') {
                e.preventDefault();
                selectPreviousMessage();
            }
            // f for fullscreen
            if (e.key === 'f' && document.activeElement?.tagName !== 'INPUT') {
                e.preventDefault();
                $isFullscreen = !$isFullscreen;
            }
            // Escape to deselect
            if (e.key === 'Escape') {
                $selectedMessage = null;
                $isFullscreen = false;
            }
        }

        window.addEventListener('keydown', handleKeydown);
        return () => window.removeEventListener('keydown', handleKeydown);
    }
</script>

<div class="flex flex-col" class:fixed={$isFullscreen} class:inset-0={$isFullscreen} class:z-50={$isFullscreen}>
    <!-- Fullscreen Backdrop -->
    {#if $isFullscreen}
        <div
            class="fixed inset-0 bg-slate-900/80 backdrop-blur-sm z-40"
            on:click={() => $isFullscreen = false}
            transition:fade={{ duration: 200 }}
        />
    {/if}

    <!-- Search Header (sticky) -->
    <div class="bg-white dark:bg-slate-800 border-b border-slate-200 dark:border-slate-700
                px-6 py-4 flex items-center gap-4 sticky top-16 shadow-sm z-10"
         class:z-50={$isFullscreen}>

        <!-- Search Bar -->
        <div class="flex-1 relative group">
            <Search class="absolute left-4 top-1/2 -translate-y-1/2 w-5 h-5 text-slate-400
                          group-focus-within:text-primary-500 transition-colors" />
            <Input
                id="unified-search"
                type="search"
                placeholder="Search all messages across all projects and agents..."
                class="w-full pl-12 pr-16 py-3 bg-slate-50 dark:bg-slate-900 rounded-xl"
                bind:value={$searchQuery}
                on:keydown={(e) => e.key === 'Escape' && ($searchQuery = '')}
            />
            <kbd class="absolute right-3 top-1/2 -translate-y-1/2 px-2 py-1
                       bg-slate-200 dark:bg-slate-700 text-slate-600 dark:text-slate-400
                       text-xs font-mono rounded hidden sm:inline-flex items-center gap-1">
                <span>⌘</span><span>K</span>
            </kbd>
        </div>

        <!-- Filters Toggle -->
        <Button
            variant={$filtersActive ? 'default' : 'outline'}
            class="gap-2"
            on:click={() => $showFilters = !$showFilters}
        >
            <SlidersHorizontal class="w-4 h-4" />
            <span class="hidden sm:inline">Filters</span>
            {#if $filtersActive}
                <span class="w-2 h-2 bg-primary-500 rounded-full animate-pulse" />
            {/if}
        </Button>

        <!-- View Toggle -->
        <div class="flex items-center gap-1 p-1 bg-slate-100 dark:bg-slate-900 rounded-lg border">
            <button
                class="p-2 rounded transition-colors"
                class:bg-white={$viewMode === 'split'}
                class:dark:bg-slate-800={$viewMode === 'split'}
                class:shadow-sm={$viewMode === 'split'}
                on:click={() => $viewMode = 'split'}
            >
                <Columns2 class="w-4 h-4 text-slate-700 dark:text-slate-300" />
            </button>
            <button
                class="p-2 rounded transition-colors"
                class:bg-white={$viewMode === 'list'}
                class:dark:bg-slate-800={$viewMode === 'list'}
                class:shadow-sm={$viewMode === 'list'}
                on:click={() => $viewMode = 'list'}
            >
                <List class="w-4 h-4 text-slate-700 dark:text-slate-300" />
            </button>
        </div>

        <!-- Fullscreen Toggle -->
        <Button variant="outline" class="gap-2" on:click={() => $isFullscreen = !$isFullscreen}>
            {#if $isFullscreen}
                <Minimize2 class="w-4 h-4" />
            {:else}
                <Maximize2 class="w-4 h-4" />
            {/if}
            <span class="hidden lg:inline">{$isFullscreen ? 'Exit' : 'Fullscreen'}</span>
        </Button>

        <!-- Refresh -->
        <Button variant="outline" class="gap-2" on:click={loadMessages} disabled={$isRefreshing}>
            <RefreshCw class="w-4 h-4" class:animate-spin={$isRefreshing} />
            <span class="hidden xl:inline">Refresh</span>
        </Button>

        <!-- Message Count -->
        <div class="hidden md:flex items-center gap-2 text-sm text-slate-600 dark:text-slate-400">
            <span class="font-semibold text-slate-900 dark:text-white">{$filteredMessages.length}</span>
            <span>{$filteredMessages.length === 1 ? 'message' : 'messages'}</span>
        </div>
    </div>

    <!-- Advanced Filters (Collapsible) -->
    {#if $showFilters}
        <div transition:slide={{ duration: 200 }}>
            <FilterBar />
        </div>
    {/if}

    <!-- Split Pane Layout -->
    <div class="flex overflow-hidden"
         class:max-h-[750px]={!$isFullscreen}
         class:min-h-[650px]={!$isFullscreen}
         class:flex-1={$isFullscreen}>

        <!-- Left: Message List -->
        <aside class={$viewMode === 'split'
            ? 'w-full md:w-2/5 lg:w-1/3 border-r border-slate-200 dark:border-slate-700'
            : 'w-full'}>
            <MessageList />
        </aside>

        <!-- Right: Message Detail (split view only) -->
        {#if $viewMode === 'split'}
            <main class="hidden md:flex flex-1 flex-col bg-white dark:bg-slate-800">
                <MessageDetail />
            </main>
        {/if}
    </div>
</div>
```

---

## Phase 3: Port All Routes

### 3.1 Route Structure

| Route | File | Status | Priority |
|-------|------|--------|----------|
| `/` | `+page.svelte` | Exists (basic) | P1 - Enhance |
| `/projects` | `/projects/+page.svelte` | Exists (basic) | P1 - Enhance |
| `/projects/[slug]` | `/projects/[slug]/+page.svelte` | Exists (basic) | P1 - Enhance |
| `/projects/[slug]/file-reservations` | `/projects/[slug]/file-reservations/+page.svelte` | **Missing** | P2 |
| `/agents` | `/agents/+page.svelte` | Exists (basic) | P1 - Enhance |
| `/attachments` | `/attachments/+page.svelte` | **Missing** | P2 |
| `/inbox` | `/inbox/+page.svelte` | Exists (basic) | P1 - Enhance |
| `/inbox/[id]` | `/inbox/[id]/+page.svelte` | Exists (basic) | P1 - Enhance |
| `/mail` | `/mail/+page.svelte` | **Missing** | **P0** (Primary) |
| `/mail/unified` | `/mail/unified/+page.svelte` | **Missing** | P3 (Redirect) |
| `/mail/unified-inbox` | `/mail/unified-inbox/+page.svelte` | **Missing** | P3 (Redirect) |
| `/thread/[id]` | `/thread/[id]/+page.svelte` | **Missing** | P1 |
| `/search` | `/search/+page.svelte` | **Missing** | P1 |
| `/archive` | `/archive/+page.svelte` | **Missing** | P2 |

### 3.2 Implementation Priority

**P0 - Critical Path** (Unified Inbox is the main feature):
1. `/mail` - Full UnifiedInbox with split view

**P1 - Core Routes**:
2. `/thread/[id]` - Thread conversation view
3. `/search` - Global search page
4. Dashboard (`/`) - Enhanced with stats
5. Existing pages - Polish and enhance

**P2 - Secondary Routes**:
6. `/attachments` - Attachment browser
7. `/projects/[slug]/file-reservations` - File locks
8. `/archive` - Archive browser

**P3 - Redirects**:
9. `/mail/unified` → `/mail`
10. `/mail/unified-inbox` → `/mail`

---

## Phase 4: Expand API Client

### 4.1 API Types

**File**: `src/lib/api/types.ts`

```typescript
export interface Message {
    id: number;
    project_id: number;
    project_slug: string;
    project_name: string;
    sender_id: number;
    sender: string;
    recipients: string;
    thread_id: string | null;
    subject: string;
    body_md: string;
    body_length: number;
    excerpt: string;
    importance: 'urgent' | 'high' | 'normal' | 'low';
    ack_required: boolean;
    created_ts: string;
    created_relative: string;
    read?: boolean;
}

export interface Thread {
    id: string;
    subject: string;
    messages: Message[];
    participants: string[];
    project_slug: string;
    created_at: string;
    updated_at: string;
}

export interface FileReservation {
    id: number;
    project_id: number;
    agent_id: number;
    agent_name: string;
    path_pattern: string;
    exclusive: boolean;
    reason: string;
    expires_ts: string;
    created_ts: string;
}

export interface Attachment {
    id: number;
    message_id: number;
    filename: string;
    mime_type: string;
    size_bytes: number;
    created_at: string;
}

export interface Filter {
    project: string;
    sender: string;
    recipient: string;
    importance: string;
    hasThread: string;
}

export interface UnifiedInboxResponse {
    messages: Message[];
    projects: Project[];
}
```

### 4.2 API Client Extensions

**File**: `src/lib/api/client.ts` - Add missing endpoints:

```typescript
// Unified Inbox
export async function fetchUnifiedInbox(limit = 1000): Promise<UnifiedInboxResponse> {
    return request<UnifiedInboxResponse>(`/mail/api/unified-inbox?limit=${limit}&include_projects=true`);
}

// Threads
export async function getThread(projectSlug: string, threadId: string): Promise<Thread> {
    return request<Thread>('/thread', {
        method: 'POST',
        body: JSON.stringify({ project_slug: projectSlug, thread_id: threadId })
    });
}

// Search
export async function searchMessages(query: string, projectSlug?: string): Promise<Message[]> {
    return request<Message[]>('/message/search', {
        method: 'POST',
        body: JSON.stringify({ query, project_slug: projectSlug })
    });
}

// File Reservations
export async function listFileReservations(projectSlug: string): Promise<FileReservation[]> {
    return request<FileReservation[]>('/file_reservations/list', {
        method: 'POST',
        body: JSON.stringify({ project_slug: projectSlug })
    });
}

// Attachments
export async function listAttachments(projectSlug: string): Promise<Attachment[]> {
    return request<Attachment[]>(`/projects/${projectSlug}/attachments`);
}

// Mark messages as read
export async function markMessagesRead(messageIds: number[], agentName: string): Promise<void> {
    await Promise.all(messageIds.map(id =>
        request('/message/read', {
            method: 'POST',
            body: JSON.stringify({ message_id: id, agent_name: agentName })
        })
    ));
}
```

---

## Phase 5: Magic UI Components (Port from Leptos)

### 5.1 TypingText.svelte

```svelte
<script lang="ts">
    import { onMount } from 'svelte';

    export let text: string;
    export let speed: number = 50;
    export let delay: number = 0;

    let displayText = '';
    let currentIndex = 0;

    onMount(() => {
        const timeout = setTimeout(() => {
            const interval = setInterval(() => {
                if (currentIndex < text.length) {
                    displayText = text.slice(0, currentIndex + 1);
                    currentIndex++;
                } else {
                    clearInterval(interval);
                }
            }, speed);
            return () => clearInterval(interval);
        }, delay);
        return () => clearTimeout(timeout);
    });
</script>

<span class="typing-text">{displayText}<span class="animate-pulse">|</span></span>
```

### 5.2 NumberCounter.svelte

```svelte
<script lang="ts">
    import { tweened } from 'svelte/motion';
    import { cubicOut } from 'svelte/easing';

    export let value: number;
    export let duration: number = 1000;
    export let format: (n: number) => string = (n) => Math.round(n).toLocaleString();

    const displayed = tweened(0, { duration, easing: cubicOut });
    $: displayed.set(value);
</script>

<span class="tabular-nums font-semibold">{format($displayed)}</span>
```

### 5.3 AnimatedGradient.svelte

```svelte
<script lang="ts">
    export let direction: 'horizontal' | 'vertical' | 'diagonal' = 'horizontal';
    export let colors: string[] = ['#6366f1', '#8b5cf6', '#ec4899'];
</script>

<div
    class="animated-gradient"
    style="
        background: linear-gradient(
            {direction === 'horizontal' ? '90deg' : direction === 'vertical' ? '180deg' : '135deg'},
            {colors.join(', ')}
        );
        background-size: 200% 200%;
        animation: gradient-shift 3s ease infinite;
    "
>
    <slot />
</div>

<style>
    @keyframes gradient-shift {
        0%, 100% { background-position: 0% 50%; }
        50% { background-position: 100% 50%; }
    }
</style>
```

---

## Phase 6: Build Integration

### 6.1 Build Script

**File**: `scripts/build-web-ui.sh`

```bash
#!/bin/bash
set -e

echo "Building SvelteKit Web UI..."
cd crates/services/web-ui

# Install dependencies
bun install

# Build
bun run build

# Verify output
if [ ! -f "build/index.html" ]; then
    echo "ERROR: build/index.html not found"
    exit 1
fi

echo "Build complete!"
echo "Output: $(ls -la build/ | wc -l) files"
du -sh build/
```

### 6.2 Update Vite Config

**File**: `vite.config.ts`

```typescript
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
    plugins: [sveltekit()],
    server: {
        proxy: {
            '/api': 'http://127.0.0.1:8080',
            '/mail/api': 'http://127.0.0.1:8080'
        }
    },
    build: {
        rollupOptions: {
            output: {
                manualChunks: {
                    vendor: ['svelte', 'bits-ui'],
                    markdown: ['marked', 'dompurify', 'prismjs']
                }
            }
        }
    }
});
```

---

## Git-Based Implementation Plan

### Branch Strategy

```
main
└── feature/sveltekit-web-ui (long-running)
    ├── svelte/phase-0-infrastructure
    ├── svelte/phase-1-dependencies
    ├── svelte/phase-2-components
    ├── svelte/phase-3-mail-unified    ← Primary feature
    ├── svelte/phase-4-routes
    ├── svelte/phase-5-magic-ui
    └── svelte/phase-6-build
```

### Beads Issue Breakdown

```bash
# Create Epic
bd create "EPIC: SvelteKit Web UI Transformation" \
  --description="Replace Leptos WASM with SvelteKit. Match Python UI design. Maintain single-binary." \
  -t epic -p 1

# Phase 0: Infrastructure (2 hours)
bd create "Update rust-embed to use web-ui/build" -t task -p 1
bd create "Update static_files.rs cache strategy for SvelteKit" -t task -p 1
bd create "Configure SvelteKit static adapter" -t task -p 1

# Phase 1: Dependencies (3 hours)
bd create "Initialize shadcn-svelte" -t task -p 1
bd create "Configure Tailwind to match Python theme" -t task -p 1
bd create "Add all shadcn components" -t task -p 1

# Phase 2: Core Components (8 hours)
bd create "Create UnifiedInbox Svelte store" -t task -p 0
bd create "Create MessageList component" -t task -p 1
bd create "Create MessageDetail component" -t task -p 1
bd create "Create FilterBar component" -t task -p 1
bd create "Create SearchBar with Cmd+K" -t task -p 1
bd create "Add keyboard shortcuts (j/k/f/Esc)" -t task -p 2

# Phase 3: Mail Route (12 hours) - CRITICAL
bd create "Implement /mail unified inbox page" -t feature -p 0
bd create "Implement /thread/[id] page" -t feature -p 1
bd create "Implement /search page" -t feature -p 1

# Phase 4: API (3 hours)
bd create "Expand API client with all endpoints" -t task -p 1
bd create "Add TypeScript types for API" -t task -p 2

# Phase 5: Magic UI (4 hours)
bd create "Port TypingText to Svelte" -t task -p 3
bd create "Port NumberCounter to Svelte" -t task -p 3
bd create "Port AnimatedGradient to Svelte" -t task -p 3

# Phase 6: Build & Polish (4 hours)
bd create "Create build integration script" -t task -p 2
bd create "Add Playwright E2E tests" -t task -p 2
bd create "Visual regression testing" -t task -p 3

# Phase 7: Cleanup (2 hours)
bd create "Archive Leptos codebase" -t chore -p 4
bd create "Update documentation" -t chore -p 3
```

---

## Success Criteria

- [ ] All 12 pages functional and visually match Python reference
- [ ] Single binary builds with embedded SvelteKit assets
- [ ] Bundle size < 200KB (gzip)
- [ ] Gmail-style split view with keyboard navigation (j/k/f/Esc)
- [ ] Command palette (Cmd+K) working
- [ ] Auto-refresh with 45s interval
- [ ] Dark mode support
- [ ] Lighthouse score > 90
- [ ] E2E tests passing

---

## Appendix: Component Mapping

### Leptos → Svelte Component Mapping

| Leptos Component | Svelte Equivalent | Source |
|------------------|-------------------|--------|
| `Button` | `$lib/components/ui/button` | shadcn-svelte |
| `Card`, `CardHeader`, etc. | `$lib/components/ui/card` | shadcn-svelte |
| `Input` | `$lib/components/ui/input` | shadcn-svelte |
| `Select` | `$lib/components/ui/select` | shadcn-svelte |
| `Dialog` | `$lib/components/ui/dialog` | shadcn-svelte |
| `Tabs` | `$lib/components/ui/tabs` | shadcn-svelte |
| `Toast` | `$lib/components/ui/sonner` | shadcn-svelte |
| `Layout` | `src/routes/+layout.svelte` | Custom |
| `SplitViewLayout` | `$lib/components/mail/UnifiedInbox` | Custom |
| `FilterBar` | `$lib/components/mail/FilterBar` | Custom |
| `ComposeMessage` | `$lib/components/mail/ComposeMessage` | Exists |
| `ProjectCard` | `$lib/components/projects/ProjectCard` | Custom |
| Magic UI components | `$lib/components/magic/*` | Port from Leptos |

### Python Alpine.js → Svelte Store Mapping

| Alpine.js State | Svelte Store |
|----------------|--------------|
| `allMessages` | `allMessages` writable |
| `filteredMessages` | `filteredMessages` derived |
| `selectedMessage` | `selectedMessage` writable |
| `searchQuery` | `searchQuery` writable |
| `filters` | `filters` writable |
| `viewMode` | `viewMode` writable |
| `isFullscreen` | `isFullscreen` writable |
| `autoRefreshEnabled` | `autoRefreshEnabled` writable |
