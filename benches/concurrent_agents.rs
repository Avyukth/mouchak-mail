//! Full System Benchmark for MCP Agent Mail
//!
//! Replicates the load testing scenarios from `scripts/benchmark_concurrent_agents.sh`.
//! Supports multi-phase testing: Health, Readiness, MCP Tool Calls, and Agent Conversation.
//!
//! Usage:
//!   cargo run --release --bin concurrent-agents-bench -- [OPTIONS]

// Allow unwrap/expect/panic in benchmark code
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use anyhow::Result;
use reqwest::Client;
use serde::Serialize;
use tokio::sync::Semaphore;
use tokio::time::sleep;

// --- DTOs for MCP/Agent Requests ---

#[derive(Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: serde_json::Value,
    id: u64,
}

#[derive(Serialize)]
struct EnsureProjectRequest {
    human_key: String,
}

#[derive(Serialize)]
struct RegisterAgentRequest {
    project_slug: String,
    name: String,
    program: String,
    model: String,
}

#[derive(Serialize)]
struct SendMessageRequest {
    project_slug: String,
    sender_name: String,
    recipient_names: Vec<String>,
    subject: String,
    body_md: String,
    thread_id: Option<String>,
    importance: Option<String>,
}

#[derive(serde::Deserialize)]
struct EnsureProjectResponse {
    slug: String,
}

// --- Stats ---

#[derive(Debug)]
struct BenchmarkStats {
    label: String,
    #[allow(dead_code)]
    target_rate_str: String,
    actual_rate: f64,
    success_rate: f64,
    p99_latency_ms: u64,
    result_status: String,
}

struct AtomicStats {
    successful: AtomicU64,
    failed: AtomicU64,
    latencies: tokio::sync::Mutex<Vec<u64>>,
}

impl AtomicStats {
    fn new() -> Self {
        Self {
            successful: AtomicU64::new(0),
            failed: AtomicU64::new(0),
            latencies: tokio::sync::Mutex::new(Vec::with_capacity(10000)),
        }
    }

    async fn record(&self, latency_ms: u64, success: bool) {
        if success {
            self.successful.fetch_add(1, Ordering::Relaxed);
            // Optimization: Don't lock for every single request in ultra-high throughput if not needed,
            // but for P99 we need samples. storing all might be heavy.
            // For now, store all.
            let mut l = self.latencies.lock().await;
            l.push(latency_ms);
        } else {
            self.failed.fetch_add(1, Ordering::Relaxed);
        }
    }

    async fn finalize(
        &self,
        duration: Duration,
        label: &str,
        target_rate_str: &str,
    ) -> BenchmarkStats {
        let successful = self.successful.load(Ordering::Relaxed);
        let failed = self.failed.load(Ordering::Relaxed);
        let total = successful + failed;

        let success_rate = if total > 0 {
            (successful as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        let mut latencies = self.latencies.lock().await;
        latencies.sort_unstable();

        // P99
        let p99 = if !latencies.is_empty() {
            let idx = (latencies.len() as f64 * 0.99) as usize;
            latencies[idx.min(latencies.len() - 1)]
        } else {
            0
        };

        let duration_secs = duration.as_secs_f64();
        let actual_rate = if duration_secs > 0.0 {
            total as f64 / duration_secs
        } else {
            0.0
        };

        let result_status = if success_rate >= 100.0 {
            "OK".to_string()
        } else if success_rate >= 98.0 {
            "EDGE".to_string()
        } else {
            "FAIL".to_string()
        };

        BenchmarkStats {
            label: label.to_string(),
            target_rate_str: target_rate_str.to_string(),
            actual_rate,
            success_rate,
            p99_latency_ms: p99,
            result_status,
        }
    }
}

// --- Configuration ---

#[derive(Clone)]
struct Config {
    base_url: String,
    agents: usize,
    duration_secs: u64,
}

/// Parse command line arguments
fn parse_args() -> Option<(u16, usize, u64)> {
    let args: Vec<String> = std::env::args().collect();
    let mut port = 8765u16;
    let mut agents = 100usize;
    let mut duration = 10u64;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--port" => {
                if let Some(p) = args.get(i + 1) {
                    port = p.parse().unwrap_or(8765);
                }
                i += 2;
            }
            "--agents" => {
                if let Some(a) = args.get(i + 1) {
                    agents = a.parse().unwrap_or(100);
                }
                i += 2;
            }
            "--duration" => {
                if let Some(d) = args.get(i + 1) {
                    duration = d.parse().unwrap_or(10);
                }
                i += 2;
            }
            "--help" | "-h" => {
                println!("Usage: concurrent-agents-bench [--port P] [--agents N] [--duration S]");
                return None;
            }
            _ => i += 1,
        }
    }
    Some((port, agents, duration))
}

/// Wait for server to become ready
async fn wait_for_server(client: &Client, base_url: &str) -> Result<()> {
    println!("Waiting for server to be ready...");
    for _ in 0..60 {
        if let Ok(res) = client.get(format!("{}/health", base_url)).send().await
            && res.status().is_success()
        {
            println!("Server is ready!");
            return Ok(());
        }
        sleep(Duration::from_secs(1)).await;
    }
    anyhow::bail!("Server did not become ready at {}", base_url)
}

/// Write result to file
fn write_result(file: &mut std::fs::File, stats: &BenchmarkStats) -> std::io::Result<()> {
    writeln!(
        file,
        "| {} | {:.0} | {:.1}% | {}ms | {} |",
        stats.label,
        stats.actual_rate,
        stats.success_rate,
        stats.p99_latency_ms,
        stats.result_status
    )
}

// --- Benchmark Logic ---

async fn run_load_test(
    config: &Config,
    client: &Client,
    label: &str,
    target_rate: Option<u64>,
    task_fn: impl Fn(usize, Client, Arc<AtomicStats>) -> tokio::task::JoinHandle<()>
    + Send
    + Sync
    + Clone
    + 'static,
) -> Result<BenchmarkStats> {
    let target_rate_str = target_rate
        .map(|r| format!("{} req/s", r))
        .unwrap_or_else(|| "Full Speed".to_string());
    println!("\nTesting: {} (rate: {})", label, target_rate_str);
    println!("----------------------------------------");

    let stats = Arc::new(AtomicStats::new());

    // Rate setup
    // If target_rate is set, we need to throttle.
    // simplistic approach: rate per agent = target_rate / agents.
    // Interval between requests = 1 / rate_per_agent.
    let interval_per_agent = if let Some(rate) = target_rate {
        if rate == 0 {
            None
        } else {
            let r_per_agent = rate as f64 / config.agents as f64;
            if r_per_agent <= 0.0 {
                None
            } else {
                Some(Duration::from_secs_f64(1.0 / r_per_agent))
            }
        }
    } else {
        None
    };

    // We run for a fixed duration.
    // We can't just spawn N loops that check time, because we need to act like 'hey'.
    // 'hey' spawns N workers and they run until duration expires.

    let start_time = Instant::now();
    let duration_secs = config.duration_secs; // Extract Copy type to move into spawn

    // Shared flag for stopping
    let running = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let r_clone = running.clone();

    tokio::spawn(async move {
        sleep(Duration::from_secs(duration_secs)).await;
        r_clone.store(false, Ordering::Relaxed);
    });

    let mut handles = Vec::new();
    let semaphore = Arc::new(Semaphore::new(config.agents));

    // For the "Logic" function, we need to pass a generator that respects the interval

    for i in 0..config.agents {
        let client_clone = client.clone();
        let stats_clone = stats.clone();
        let running_clone = running.clone();
        let task_fn_clone = task_fn.clone();
        let _permit = semaphore.clone().acquire_owned().await?;

        // Wrapper to enforce loop and interval
        let h = tokio::spawn(async move {
            let mut tick_next = Instant::now();

            while running_clone.load(Ordering::Relaxed) {
                // Throttle
                if let Some(interval) = interval_per_agent {
                    let now = Instant::now();
                    if now < tick_next {
                        tokio::time::sleep_until(tokio::time::Instant::from_std(tick_next)).await;
                    }
                    tick_next += interval;
                }

                // Execute ONE iteration of the task
                let inner_h = task_fn_clone(i, client_clone.clone(), stats_clone.clone());
                inner_h.await.unwrap();
            }
            drop(_permit);
        });
        handles.push(h);
    }

    // Wait for all to finish
    // actually, they finish when time keeps up.
    for h in handles {
        let _ = h.await;
    }

    let actual_duration = start_time.elapsed();
    let result = stats
        .finalize(actual_duration, label, &target_rate_str)
        .await;

    // Print inline result
    let color_code = match result.result_status.as_str() {
        "OK" => "\x1b[0;32m",   // Green
        "EDGE" => "\x1b[1;33m", // Yellow
        "FAIL" => "\x1b[0;31m", // Red
        _ => "\x1b[0m",
    };
    let reset = "\x1b[0m";

    println!(
        "  Rate: {:.0} req/s | Success: {:.1}% | P99: {}ms | {}{}{}",
        result.actual_rate,
        result.success_rate,
        result.p99_latency_ms,
        color_code,
        result.result_status,
        reset
    );

    Ok(result)
}

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Parse Arguments
    let Some((port, agents, duration)) = parse_args() else {
        return Ok(());
    };

    let config = Config {
        base_url: format!("http://127.0.0.1:{}", port),
        agents,
        duration_secs: duration,
    };

    println!("==============================================");
    println!("MCP Agent Mail Benchmark - {} Concurrent Agents", agents);
    println!("==============================================");
    println!("Target: {}", config.base_url);
    println!("Duration per test: {}s", duration);
    println!();

    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .pool_max_idle_per_host(agents + 10)
        .build()?;

    // 2. Wait for Server
    wait_for_server(&client, &config.base_url).await?;

    // 3. Prepare Reporting
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let report_file = format!("benchmark_results_{}.md", timestamp);
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&report_file)?;

    writeln!(file, "# Benchmark Results: {} Concurrent Agents", agents)?;
    writeln!(file)?;
    writeln!(file, "**Date**: {}", chrono::Local::now())?;
    writeln!(file, "**Target**: {}", config.base_url)?;
    writeln!(file, "**Duration**: {}s per test", duration)?;
    writeln!(file, "**Concurrency**: {} agents", agents)?;
    writeln!(file)?;
    writeln!(file, "## Results")?;
    writeln!(file)?;
    writeln!(
        file,
        "| Test | Rate (req/s) | Success | P99 Latency | Result |"
    )?;
    writeln!(
        file,
        "|------|--------------|---------|-------------|--------|"
    )?;

    let mut results = Vec::new();

    // 4. Phase 1: Health Liveness
    println!("\n=== Phase 1: Health Endpoint (Raw HTTP throughput) ===");
    let url_liveness = format!("{}/health", config.base_url);
    let task_liveness = move |_, c: Client, s: Arc<AtomicStats>| {
        let u = url_liveness.clone();
        tokio::spawn(async move {
            let start = Instant::now();
            let res = c.get(&u).send().await;
            let lat = start.elapsed().as_millis() as u64;
            let success = matches!(res, Ok(r) if r.status().is_success());
            s.record(lat, success).await;
        })
    };

    let rates = [
        None,
        Some(5000),
        Some(3000),
        Some(2000),
        Some(1800),
        Some(1600),
        Some(1500),
        Some(1000),
    ];
    for r in rates {
        let stats = run_load_test(&config, &client, "/health", r, task_liveness.clone()).await?;
        write_result(&mut file, &stats)?;
        results.push(stats);
    }

    // 5. Phase 2: Readiness (DB)
    println!("\n=== Phase 2: Readiness Endpoint (DB connection test) ===");
    let url_readiness = format!("{}/ready", config.base_url);
    let task_readiness = move |_, c: Client, s: Arc<AtomicStats>| {
        let u = url_readiness.clone();
        tokio::spawn(async move {
            let start = Instant::now();
            let res = c.get(&u).send().await;
            let lat = start.elapsed().as_millis() as u64;
            let success = matches!(res, Ok(r) if r.status().is_success());
            s.record(lat, success).await;
        })
    };

    let rates_db = [None, Some(2000), Some(1600), Some(1000)];
    for r in rates_db {
        let stats = run_load_test(&config, &client, "/ready", r, task_readiness.clone()).await?;
        write_result(&mut file, &stats)?;
        results.push(stats);
    }

    // 6. Phase 3: MCP Tool Call
    println!("\n=== Phase 3: MCP Tool Calls (Realistic agent workload) ===");
    let url_mcp = format!("{}/mcp", config.base_url);
    let task_mcp = move |_, c: Client, s: Arc<AtomicStats>| {
        let u = url_mcp.clone();
        tokio::spawn(async move {
            let body = JsonRpcRequest {
                jsonrpc: "2.0".to_string(),
                method: "initialize".to_string(),
                params: serde_json::json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": {"name": "bench", "version": "1.0"}
                }),
                id: 1,
            };
            let start = Instant::now();
            let res = c
                .post(&u)
                .header("Accept", "application/json, text/event-stream")
                .json(&body)
                .send()
                .await;
            let lat = start.elapsed().as_millis() as u64;
            let success = matches!(res, Ok(r) if r.status().is_success());
            s.record(lat, success).await;
        })
    };

    let rates_mcp = [None, Some(2000), Some(1600), Some(1000), Some(500)];
    for r in rates_mcp {
        let label = match r {
            Some(_) => "MCP Tool Call",
            None => "MCP Tool Call (Full Speed)",
        };
        let stats = run_load_test(&config, &client, label, r, task_mcp.clone()).await?;
        write_result(&mut file, &stats)?;
        results.push(stats);
    }

    // 7. Phase 4: Full Agent Message Flow (Bonus/Verification)
    println!("\n=== Phase 4: Full Agent Message Flow (Bonus) ===");
    // Setup: Create project and agents first
    let human_key = format!("bench-rust-{}", chrono::Utc::now().timestamp());
    let ensure_resp = client
        .post(format!("{}/api/project/ensure", config.base_url))
        .json(&EnsureProjectRequest { human_key })
        .send()
        .await?;

    if ensure_resp.status().is_success() {
        let project: EnsureProjectResponse = ensure_resp.json().await?;
        let slug = project.slug;

        // Register agents
        print!("Registering {} agents...", agents);
        // Do this in parallel to be fast
        let mut reg_futs = Vec::new();
        for i in 0..agents {
            let c = client.clone();
            let u = config.base_url.clone();
            let s = slug.clone();
            let name = format!("agent-{:03}", i);
            reg_futs.push(tokio::spawn(async move {
                c.post(format!("{}/api/agent/register", u))
                    .json(&RegisterAgentRequest {
                        project_slug: s,
                        name,
                        program: "bench".into(),
                        model: "bench".into(),
                    })
                    .send()
                    .await
            }));
        }
        for f in reg_futs {
            let _ = f.await;
        }
        println!(" Done.");

        // Msg Task
        let url_send = format!("{}/api/message/send", config.base_url);
        let task_msg = move |idx, c: Client, s: Arc<AtomicStats>| {
            let u = url_send.clone();
            let p_slug = slug.clone();
            let sender = format!("agent-{:03}", idx); // Current agent is sender
            let recipient = format!("agent-{:03}", (idx + 1) % agents); // Next is receiver

            tokio::spawn(async move {
                let body = SendMessageRequest {
                    project_slug: p_slug,
                    sender_name: sender,
                    recipient_names: vec![recipient],
                    subject: "Bench".into(),
                    body_md: "Benchmark".into(),
                    thread_id: None,
                    importance: Some("normal".into()),
                };
                let start = Instant::now();
                let res = c.post(&u).json(&body).send().await;
                let lat = start.elapsed().as_millis() as u64;
                let success = matches!(res, Ok(r) if r.status().is_success());
                s.record(lat, success).await;
            })
        };

        // Run message bench
        let stats = run_load_test(
            &config,
            &client,
            "Full Agent Message (Full Speed)",
            None,
            task_msg,
        )
        .await?;
        write_result(&mut file, &stats)?;
    } else {
        println!("Skipping Phase 4: Could not create project.");
    }

    writeln!(file)?;
    writeln!(file, "## Analysis")?;
    writeln!(
        file,
        "See `scripts/benchmark_concurrent_agents.sh` for interpretation."
    )?;

    println!("\n==============================================");
    println!("Benchmark Complete!");
    println!("==============================================");
    println!("Results saved to: {}", report_file);

    Ok(())
}
