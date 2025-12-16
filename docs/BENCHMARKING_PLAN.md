# MCP Agent Mail - Comprehensive Benchmarking Plan

## Executive Summary

This document outlines a complete benchmarking strategy for the MCP Agent Mail system - a multi-agent communication platform built with Rust/Axum, libsql (SQLite), and Git-backed storage.

---

## 1. System Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        LOAD GENERATOR                           │
│                    (k6 / wrk / criterion)                       │
└─────────────────────────┬───────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│                      RATE LIMITER                               │
│              governor (100 RPS / 200 burst)                     │
└─────────────────────────┬───────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│                      AXUM SERVER                                │
│                  (Tokio async runtime)                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │  /api/inbox │  │/api/message │  │  /api/messages/search   │  │
│  │   (POST)    │  │   /send     │  │       (FTS5)            │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
└─────────────────────────┬───────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│                     lib-core (BMC)                              │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │  MessageBmc │  │  AgentBmc   │  │      ProjectBmc         │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
└─────────────────────────┬───────────────────────────────────────┘
                          │
          ┌───────────────┴───────────────┐
          ▼                               ▼
┌─────────────────────┐       ┌─────────────────────┐
│      libsql         │       │       Git2          │
│   (SQLite + WAL)    │       │  (Archive Storage)  │
│   ┌─────────────┐   │       │   ┌─────────────┐   │
│   │   FTS5      │   │       │   │  .md files  │   │
│   │   Index     │   │       │   │  per msg    │   │
│   └─────────────┘   │       │   └─────────────┘   │
└─────────────────────┘       └─────────────────────┘
```

---

## 2. Identified Bottlenecks

### 2.1 Critical Path Bottlenecks

| Bottleneck | Severity | Component | Impact |
|------------|----------|-----------|--------|
| **Git I/O per message** | HIGH | Git2 | Each message creates 3+ files + commit |
| **N+1 recipient queries** | MEDIUM | MessageBmc | Separate query per recipient |
| **Single SQLite writer** | MEDIUM | libsql | Write serialization under load |
| **FTS5 indexing** | LOW | SQLite | Automatic on INSERT, adds latency |
| **JSON deserialization** | LOW | serde | Per-row attachment parsing |

### 2.2 Resource Bottlenecks

| Resource | Constraint | Expected Limit |
|----------|------------|----------------|
| File descriptors | Git + SQLite | ~1000 concurrent |
| Memory | Message buffering | ~100MB baseline |
| Disk I/O | Git archive writes | ~500 msg/s sustained |
| CPU | JSON parsing, hashing | ~10k req/s single core |

---

## 3. Key Metrics to Measure

### 3.1 Latency Metrics (HDR Histogram)

| Metric | Target | SLA |
|--------|--------|-----|
| `message_send_p50` | < 10ms | < 50ms |
| `message_send_p99` | < 100ms | < 500ms |
| `inbox_list_p50` | < 5ms | < 20ms |
| `inbox_list_p99` | < 50ms | < 200ms |
| `fts_search_p50` | < 20ms | < 100ms |
| `fts_search_p99` | < 100ms | < 500ms |

### 3.2 Throughput Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| `messages_per_second` | > 1000 | Sustained write throughput |
| `reads_per_second` | > 5000 | Inbox/outbox reads |
| `searches_per_second` | > 500 | FTS5 queries |
| `concurrent_agents` | > 100 | Active connections |

### 3.3 Resource Metrics

| Metric | Collection Method |
|--------|-------------------|
| `cpu_utilization` | /proc/stat or macOS equivalent |
| `memory_rss` | Process memory |
| `disk_iops` | iostat |
| `open_file_handles` | lsof count |
| `sqlite_page_cache_hit` | PRAGMA cache_size queries |
| `git_object_count` | Git repo stats |

### 3.4 Error Metrics

| Metric | Threshold |
|--------|-----------|
| `error_rate_5xx` | < 0.1% |
| `rate_limit_429s` | Tracked (not error) |
| `sqlite_busy_errors` | 0 (critical) |
| `git_lock_conflicts` | < 1% |

---

## 4. Benchmark Scenarios

### 4.1 Micro-benchmarks (criterion.rs)

```rust
// benches/message_ops.rs
use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};

fn bench_message_create(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mm = rt.block_on(ModelManager::new()).unwrap();

    c.bench_function("message_create_db_only", |b| {
        b.iter(|| {
            rt.block_on(async {
                MessageBmc::create(&mm, /* params */).await
            })
        })
    });
}

fn bench_inbox_list(c: &mut Criterion) {
    let mut group = c.benchmark_group("inbox_list");
    for size in [10, 100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            size,
            |b, &size| { /* benchmark with size messages */ }
        );
    }
    group.finish();
}

fn bench_fts_search(c: &mut Criterion) {
    // Pre-populate with 10k messages
    c.bench_function("fts_search_1k_corpus", |b| {
        b.iter(|| {
            rt.block_on(MessageBmc::search(&mm, project_id, "important meeting"))
        })
    });
}

criterion_group!(benches, bench_message_create, bench_inbox_list, bench_fts_search);
criterion_main!(benches);
```

### 4.2 Load Testing (k6)

```javascript
// k6/scenarios/message_flow.js
import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend } from 'k6/metrics';

const messageLatency = new Trend('message_latency');
const errorRate = new Rate('error_rate');

export const options = {
  scenarios: {
    // Scenario 1: Sustained load
    sustained_load: {
      executor: 'constant-arrival-rate',
      rate: 100,
      timeUnit: '1s',
      duration: '5m',
      preAllocatedVUs: 50,
    },
    // Scenario 2: Spike test
    spike_test: {
      executor: 'ramping-arrival-rate',
      startRate: 10,
      stages: [
        { target: 500, duration: '30s' },
        { target: 500, duration: '1m' },
        { target: 10, duration: '30s' },
      ],
      preAllocatedVUs: 100,
    },
    // Scenario 3: Soak test
    soak_test: {
      executor: 'constant-arrival-rate',
      rate: 50,
      duration: '1h',
      preAllocatedVUs: 30,
    },
  },
  thresholds: {
    'message_latency': ['p(99)<500'],
    'error_rate': ['rate<0.01'],
  },
};

export default function() {
  // Setup
  const baseUrl = 'http://localhost:8765';
  const projectSlug = 'benchmark-project';

  // Send message
  const sendStart = Date.now();
  const sendRes = http.post(`${baseUrl}/api/message/send`, JSON.stringify({
    project_slug: projectSlug,
    sender_name: `agent-${__VU}`,
    recipients: ['agent-receiver'],
    subject: `Test message ${Date.now()}`,
    body_md: 'Benchmark test message content',
  }), { headers: { 'Content-Type': 'application/json' } });

  messageLatency.add(Date.now() - sendStart);
  errorRate.add(sendRes.status >= 400);

  check(sendRes, {
    'message sent': (r) => r.status === 200,
  });

  // List inbox
  const inboxRes = http.post(`${baseUrl}/api/inbox`, JSON.stringify({
    project_slug: projectSlug,
    agent_name: 'agent-receiver',
  }), { headers: { 'Content-Type': 'application/json' } });

  check(inboxRes, {
    'inbox listed': (r) => r.status === 200,
  });

  sleep(0.1);
}
```

### 4.3 Stress Testing (wrk)

```bash
# High concurrency test
wrk -t12 -c400 -d30s \
  -s scripts/send_message.lua \
  http://localhost:8765/api/message/send

# Sustained throughput test
wrk -t4 -c100 -d5m --latency \
  -s scripts/inbox_list.lua \
  http://localhost:8765/api/inbox
```

### 4.4 Database-Specific Benchmarks

```rust
// benches/database_ops.rs

fn bench_sqlite_write_throughput(c: &mut Criterion) {
    // Test raw INSERT performance without Git
    c.bench_function("raw_insert_1000", |b| {
        b.iter(|| {
            for _ in 0..1000 {
                // INSERT message without Git write
            }
        })
    });
}

fn bench_concurrent_readers(c: &mut Criterion) {
    // Test WAL mode with concurrent reads
    let mut group = c.benchmark_group("concurrent_reads");
    for readers in [1, 4, 8, 16].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(readers),
            readers,
            |b, &n| {
                // Spawn n reader tasks
            }
        );
    }
}

fn bench_fts5_index_build(c: &mut Criterion) {
    // Measure FTS5 indexing overhead
    c.bench_function("fts5_rebuild_10k", |b| {
        b.iter(|| {
            // INSERT INTO messages_fts(messages_fts) VALUES('rebuild')
        })
    });
}
```

---

## 5. Comparison Benchmarks (Similar Systems)

### 5.1 Message Queue Comparison

| System | Throughput | P99 Latency | Use Case |
|--------|------------|-------------|----------|
| **MCP Agent Mail** | ~1k msg/s (target) | ~100ms | Agent coordination |
| RabbitMQ | ~30k msg/s | ~10ms | General messaging |
| Kafka | ~100k+ msg/s | ~5ms | Event streaming |
| Redis Streams | ~50k msg/s | ~1ms | Real-time |
| NATS | ~10M msg/s | <1ms | High-frequency |

### 5.2 SQLite Performance Reference

| Operation | Expected | Source |
|-----------|----------|--------|
| Simple SELECT | 200ns | libsql benchmarks |
| INSERT (WAL) | 50-100μs | SQLite docs |
| FTS5 search (1k docs) | 1-10ms | SQLite docs |
| Connection open | 40μs | Turso benchmarks |

### 5.3 Axum Framework Reference

| Metric | Value | Source |
|--------|-------|--------|
| Hello World RPS | ~45k | Sharkbench 2025 |
| JSON response RPS | ~40k | TechEmpower |
| Memory per conn | ~10KB | Tokio benchmarks |

---

## 6. Implementation Plan

### Phase 1: Setup Infrastructure (Day 1-2)

```bash
# 1. Add criterion to workspace
cargo add criterion --dev --features html_reports

# 2. Create benchmark directory structure
mkdir -p benches
mkdir -p k6/scenarios
mkdir -p scripts

# 3. Add to Cargo.toml
[[bench]]
name = "message_ops"
harness = false

[[bench]]
name = "database_ops"
harness = false
```

### Phase 2: Micro-benchmarks (Day 3-5)

| Task | File | Priority |
|------|------|----------|
| Message create benchmark | `benches/message_ops.rs` | P0 |
| Inbox list benchmark | `benches/message_ops.rs` | P0 |
| FTS search benchmark | `benches/message_ops.rs` | P0 |
| Agent lookup benchmark | `benches/agent_ops.rs` | P1 |
| Git write benchmark | `benches/git_ops.rs` | P1 |

### Phase 3: Load Testing (Day 6-8)

| Task | Tool | Scenario |
|------|------|----------|
| Sustained load (100 RPS) | k6 | 5 minute run |
| Spike test (10→500→10) | k6 | Ramp pattern |
| Concurrent agents (100) | wrk | High concurrency |
| Soak test (1 hour) | k6 | Memory leaks |

### Phase 4: Analysis & Optimization (Day 9-10)

1. Generate flamegraphs with `cargo flamegraph`
2. Identify hot paths
3. Profile memory with `heaptrack`
4. Optimize critical paths
5. Re-run benchmarks

---

## 7. Benchmark Execution

### 7.1 Environment Setup

```bash
# Isolate CPU cores for benchmarking
sudo cpuset -c 0-3 -p $$

# Disable frequency scaling
echo performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor

# Clean database
rm -rf data/mcp_agent_mail.db data/archive

# Build release
cargo build --release -p mcp-server

# Start server
PORT=8765 ./target/release/mcp-server
```

### 7.2 Run Benchmarks

```bash
# Micro-benchmarks
cargo bench --bench message_ops -- --save-baseline main

# Load test
k6 run k6/scenarios/message_flow.js --out json=results/k6_output.json

# Stress test
wrk -t12 -c400 -d30s -s scripts/send_message.lua http://localhost:8765/api/message/send

# Generate report
cargo bench -- --load-baseline main
```

### 7.3 Collect Metrics

```bash
# During benchmark run
watch -n1 'curl -s localhost:8765/metrics'

# SQLite stats
sqlite3 data/mcp_agent_mail.db "PRAGMA page_count; PRAGMA freelist_count;"

# Git stats
git -C data/archive count-objects -v
```

---

## 8. Success Criteria

### 8.1 Performance Targets

| Metric | Target | Stretch Goal |
|--------|--------|--------------|
| Message send P99 | < 100ms | < 50ms |
| Inbox list P99 | < 50ms | < 20ms |
| Throughput (write) | > 500/s | > 1000/s |
| Throughput (read) | > 2000/s | > 5000/s |
| Error rate | < 0.1% | < 0.01% |

### 8.2 Scalability Targets

| Dimension | Target |
|-----------|--------|
| Messages in DB | 1M+ |
| Concurrent agents | 100+ |
| Projects | 1000+ |
| Message size | 1MB |

---

## 9. Optimization Roadmap (Post-Benchmark)

### High Impact / Low Effort
- [ ] Batch recipient resolution (eliminate N+1)
- [ ] Add database indexes on hot paths
- [ ] Enable prepared statement caching

### High Impact / High Effort
- [ ] Async Git operations (background writer)
- [ ] Connection pooling for libsql
- [ ] Enable libsql MVCC (experimental)

### Medium Impact
- [ ] Pre-serialize JSON attachments
- [ ] Add response caching (inbox TTL)
- [ ] Implement bulk message send

### Monitoring
- [ ] Add Prometheus metrics endpoint
- [ ] Track P50/P99/P999 latencies
- [ ] Alert on error rate spikes

---

## 10. References

### Tools
- [Criterion.rs](https://bheisler.github.io/criterion.rs/book/)
- [k6 Load Testing](https://k6.io/docs/)
- [wrk HTTP Benchmarking](https://github.com/wg/wrk)
- [Flamegraph](https://github.com/flamegraph-rs/flamegraph)

### Research
- [Benchmarking Message Queues (MDPI)](https://www.mdpi.com/2673-4001/4/2/18)
- [Turso Performance](https://turso.tech/blog/how-turso-made-connections-faster)
- [Axum Benchmarks (Sharkbench)](https://sharkbench.dev/web/rust-axum)
- [OpenMessaging Benchmark](https://openmessaging.cloud/docs/benchmarks/)

### Similar Systems
- [Kafka Performance](https://www.confluent.io/blog/kafka-fastest-messaging-system/)
- [RabbitMQ Benchmarks](https://www.rabbitmq.com/blog/2020/06/04/how-to-run-benchmarks)
- [NATS Performance](https://docs.nats.io/nats-concepts/overview#performance)

---

## Appendix A: Benchmark File Templates

### A.1 Cargo.toml additions

```toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports", "async_tokio"] }
tokio-test = "0.4"

[[bench]]
name = "message_ops"
harness = false

[[bench]]
name = "database_ops"
harness = false
```

### A.2 Benchmark skeleton

```rust
// benches/message_ops.rs
use criterion::{criterion_group, criterion_main, Criterion};
use lib_core::ModelManager;
use tokio::runtime::Runtime;

fn message_benchmarks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("placeholder", |b| {
        b.iter(|| {
            // TODO: Implement
        })
    });
}

criterion_group!(benches, message_benchmarks);
criterion_main!(benches);
```

---

*Document Version: 1.0*
*Created: 2025-12-16*
*Author: Claude Code*
