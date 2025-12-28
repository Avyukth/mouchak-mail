#![allow(clippy::unwrap_used, clippy::expect_used)]
use chrono::{Duration, Utc};
use lib_core::store::archive_lock::{ArchiveLock, LockOwner};
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_stale_lock_cleanup() {
    let dir = TempDir::new().unwrap();
    let lock = ArchiveLock::new(dir.path());

    // 1. Create fake stale lock with dead PID
    // We use a PID that is unlikely to exist (max PID is usually lower than i32::MAX)
    // But safely, we can just use a large number.
    let fake_owner = LockOwner {
        pid: 2147483647, // Max i32, unlikely to be us or active usually?
        // Better: Use a PID we know is dead? Hard to guarantee.
        // For test, we rely on is_process_alive returning false for this.
        timestamp: Utc::now() - Duration::hours(2),
        agent: None,
        hostname: hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".into()),
    };

    // Manually write the lock files
    let owner_path = dir.path().join(".archive.lock.owner");
    let lock_path = dir.path().join(".archive.lock");

    fs::write(&owner_path, serde_json::to_string(&fake_owner).unwrap()).unwrap();
    fs::write(&lock_path, "").unwrap();

    // 2. Attempt to acquire
    // Should clean up the stale lock and succeed
    let result = lock.acquire(None, std::time::Duration::from_secs(5)).await;
    assert!(
        result.is_ok(),
        "Should acquire lock after cleaning up stale one"
    );

    // Verify cleanup happened (files should be rewritten with OUR pid)
    let new_owner_content = fs::read_to_string(&owner_path).unwrap();
    let new_owner: LockOwner = serde_json::from_str(&new_owner_content).unwrap();
    assert_ne!(new_owner.pid, fake_owner.pid);
    assert!(new_owner.pid == std::process::id());
}

#[tokio::test]
async fn test_lock_contention() {
    let dir = TempDir::new().unwrap();
    let lock_path = dir.path().to_path_buf();

    // Process 1 acquires lock
    let lock1 = ArchiveLock::new(&lock_path);
    let guard1 = lock1
        .acquire(Some("agent1".into()), std::time::Duration::from_secs(1))
        .await
        .unwrap();

    // Process 2 tries to acquire (should fail/timeout)
    let lock2 = ArchiveLock::new(&lock_path);
    let result = lock2
        .acquire(Some("agent2".into()), std::time::Duration::from_secs(1))
        .await;

    assert!(result.is_err()); // Timeout

    // Guard1 dropped, releasing lock
    drop(guard1);

    // Process 2 tries again (should succeed)
    let guard2 = lock2
        .acquire(Some("agent2".into()), std::time::Duration::from_secs(1))
        .await;
    assert!(guard2.is_ok());
}
