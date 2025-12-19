//! Stale Lock Cleanup (PORT-2.2)
//!
//! Advisory file lock with stale detection for archive operations.
//! NIST Control: AU-9 (Audit Log Protection)

use crate::error::{Error, Result};
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
    /// Create lock owner for current process
    #[allow(clippy::expect_used)] // hostname::get rarely fails
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
#[cfg(unix)]
fn is_process_alive(pid: u32) -> bool {
    // Check if /proc/{pid} exists (Linux) or use sysctl (macOS)
    #[cfg(target_os = "linux")]
    {
        std::path::Path::new(&format!("/proc/{}", pid)).exists()
    }

    #[cfg(target_os = "macos")]
    {
        // On macOS, check if /proc doesn't exist so we use a different method
        // Try to read process info via sysinfo or just assume alive
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(true)
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        // Conservative: assume alive on other Unix systems
        true
    }
}

#[cfg(not(unix))]
fn is_process_alive(_pid: u32) -> bool {
    // Conservative: assume alive if we can't check
    true
}

/// Advisory file lock with stale detection
pub struct ArchiveLock {
    lock_path: PathBuf,
    owner_path: PathBuf,
    inner: Mutex<()>, // Process-level mutex
}

impl ArchiveLock {
    /// Create new archive lock for given path
    pub fn new(archive_path: &Path) -> Self {
        Self {
            lock_path: archive_path.join(".archive.lock"),
            owner_path: archive_path.join(".archive.lock.owner"),
            inner: Mutex::new(()),
        }
    }

    /// Acquire lock with timeout and stale cleanup
    pub async fn acquire(
        &self,
        agent: Option<String>,
        timeout: std::time::Duration,
    ) -> Result<LockGuard<'_>> {
        let deadline = std::time::Instant::now() + timeout;
        let max_age = Duration::hours(1);

        loop {
            // Try to acquire process-level lock first
            let _inner = self.inner.lock().await;

            // Check for stale file lock
            if self.lock_path.exists() {
                if let Some(owner) = self.read_owner().await {
                    if owner.is_stale(max_age) {
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

            return Ok(LockGuard { lock: self });
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

    /// Release the lock
    pub async fn release(&self) -> Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_lock_acquire_release() {
        let dir = TempDir::new().expect("create temp dir");
        let lock = ArchiveLock::new(dir.path());

        // Acquire lock
        let guard = lock
            .acquire(Some("test-agent".into()), std::time::Duration::from_secs(5))
            .await
            .expect("acquire lock");

        // Lock files should exist
        assert!(lock.lock_path.exists());
        assert!(lock.owner_path.exists());

        // Read owner metadata
        let owner = lock.read_owner().await.expect("read owner");
        assert_eq!(owner.pid, std::process::id());
        assert_eq!(owner.agent, Some("test-agent".into()));

        // Release (drop guard)
        drop(guard);

        // Give spawned task time to clean up
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    #[tokio::test]
    async fn test_stale_lock_cleanup() {
        let dir = TempDir::new().expect("create temp dir");
        let lock = ArchiveLock::new(dir.path());

        // Create fake stale lock with dead PID
        let fake_owner = LockOwner {
            pid: 999999999, // Unlikely to exist
            timestamp: Utc::now() - Duration::hours(2),
            agent: None,
            hostname: "test".into(),
        };

        // Write fake lock files
        fs::write(&lock.lock_path, "").await.expect("write lock");
        fs::write(
            &lock.owner_path,
            serde_json::to_string(&fake_owner).expect("serialize"),
        )
        .await
        .expect("write owner");

        // Should succeed by cleaning stale lock
        let _guard = lock
            .acquire(Some("new-agent".into()), std::time::Duration::from_secs(1))
            .await
            .expect("acquire after stale cleanup");
    }

    #[tokio::test]
    async fn test_is_stale_dead_process() {
        let owner = LockOwner {
            pid: 999999999, // Unlikely to exist
            timestamp: Utc::now(),
            agent: None,
            hostname: "test".into(),
        };

        // Should be stale due to dead PID
        assert!(owner.is_stale(Duration::hours(24)));
    }

    #[tokio::test]
    async fn test_is_stale_old_timestamp() {
        let owner = LockOwner {
            pid: std::process::id(), // Current process (alive)
            timestamp: Utc::now() - Duration::hours(2),
            agent: None,
            hostname: "test".into(),
        };

        // Should be stale due to old timestamp
        assert!(owner.is_stale(Duration::hours(1)));
    }

    /// Integration test simulating crash recovery scenario.
    /// Models what ModelManager does on startup when it finds a stale lock.
    #[tokio::test]
    async fn test_crash_recovery_integration() {
        let dir = TempDir::new().expect("create temp dir");
        let lock = ArchiveLock::new(dir.path());

        // Step 1: Simulate a crashed process that left a stale lock
        // (dead PID from over an hour ago)
        let crashed_owner = LockOwner {
            pid: 999999999, // Dead PID
            timestamp: Utc::now() - Duration::hours(2),
            agent: Some("crashed-agent".into()),
            hostname: "crashed-host".into(),
        };

        fs::write(&lock.lock_path, "")
            .await
            .expect("write stale lock file");
        fs::write(
            &lock.owner_path,
            serde_json::to_string(&crashed_owner).expect("serialize"),
        )
        .await
        .expect("write stale owner file");

        // Step 2: New process starts up (simulating ModelManager::cleanup_stale_locks)
        // Quick check with short timeout - should clean stale and acquire
        let startup_timeout = std::time::Duration::from_millis(100);
        let guard = lock
            .acquire(Some("startup-cleanup".into()), startup_timeout)
            .await
            .expect("should acquire lock after cleaning stale");

        // Verify we now own the lock
        let owner = lock.read_owner().await.expect("read owner");
        assert_eq!(owner.pid, std::process::id());
        assert_eq!(owner.agent, Some("startup-cleanup".into()));

        // Step 3: Release lock (guard drop) - simulating successful startup
        drop(guard);
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Step 4: Normal operation can now acquire lock
        let normal_guard = lock
            .acquire(
                Some("normal-agent".into()),
                std::time::Duration::from_secs(1),
            )
            .await
            .expect("normal operation should acquire lock");

        let owner = lock.read_owner().await.expect("read owner");
        assert_eq!(owner.agent, Some("normal-agent".into()));

        drop(normal_guard);
    }
}
