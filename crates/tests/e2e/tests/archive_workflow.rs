//! Archive Workflow E2E Tests
//!
//! Tests the archive save/list/restore lifecycle including:
//! - Archive creation with labels
//! - Archive listing (human-readable and JSON)
//! - Archive restoration with confirmation bypass
//! - Data integrity verification (metadata, database, git storage)
//! - Clear and reset functionality
//!
//! Port of: test_archive_workflow.py
//!
//! Run tests:
//! ```bash
//! cargo test -p e2e-tests --test archive_workflow -- --test-threads=1
//! ```

#![allow(deprecated)] // cargo_bin is still valid for our use case
#![allow(clippy::unwrap_used, clippy::expect_used)] // expect/unwrap is fine in tests

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

// ============================================================================
// Test Helpers
// ============================================================================

/// Create a test environment with isolated data directory
fn setup_test_env() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create data directory structure
    let data_dir = temp_dir.path().join("data");
    fs::create_dir_all(&data_dir).expect("Failed to create data dir");

    temp_dir
}

/// Run CLI command with isolated data directory
fn run_cli_with_data_dir(temp_dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("mcp-agent-mail").unwrap();

    // Set working directory to temp dir so data/ paths resolve there
    cmd.current_dir(temp_dir.path());

    cmd
}

/// Create a dummy database file for testing
fn create_dummy_database(temp_dir: &TempDir) {
    let db_path = temp_dir.path().join("data/mcp_agent_mail.db");
    fs::create_dir_all(db_path.parent().unwrap()).expect("Failed to create data dir");
    fs::write(&db_path, b"SQLite format 3\x00dummy test data").expect("Failed to create db");
}

/// Create dummy git storage for testing
fn create_dummy_git_storage(temp_dir: &TempDir) {
    let git_path = temp_dir.path().join("data/archive");
    fs::create_dir_all(&git_path).expect("Failed to create git dir");

    // Create some git-like files
    fs::create_dir_all(git_path.join("objects")).expect("Failed to create objects dir");
    fs::write(git_path.join("HEAD"), "ref: refs/heads/main\n").expect("Failed to create HEAD");
    fs::write(
        git_path.join("objects/test-object"),
        "test git object content",
    )
    .expect("Failed to create test object");
}

// ============================================================================
// Archive Save Tests (PORT-1)
// ============================================================================

#[test]
fn test_archive_save_basic() {
    let temp_dir = setup_test_env();
    create_dummy_database(&temp_dir);

    let mut cmd = run_cli_with_data_dir(&temp_dir);
    cmd.args(["archive", "save"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Archive created"));
}

#[test]
fn test_archive_save_with_label() {
    let temp_dir = setup_test_env();
    create_dummy_database(&temp_dir);

    let mut cmd = run_cli_with_data_dir(&temp_dir);
    cmd.args(["archive", "save", "--label", "test-backup"])
        .assert()
        .success()
        .stdout(predicate::str::contains("test-backup"));
}

#[test]
fn test_archive_save_includes_metadata() {
    let temp_dir = setup_test_env();
    create_dummy_database(&temp_dir);

    // Create archive
    let mut cmd = run_cli_with_data_dir(&temp_dir);
    cmd.args(["archive", "save", "--label", "metadata-test"])
        .assert()
        .success();

    // Find the created archive
    let archives_dir = temp_dir.path().join("data/archives");
    let archive_path = fs::read_dir(&archives_dir)
        .expect("Failed to read archives dir")
        .find_map(|e| {
            let entry = e.ok()?;
            if entry
                .file_name()
                .to_string_lossy()
                .contains("metadata-test")
            {
                Some(entry.path())
            } else {
                None
            }
        })
        .expect("Archive file not found");

    // Verify archive contains metadata.json
    let file = fs::File::open(&archive_path).expect("Failed to open archive");
    let mut archive = zip::ZipArchive::new(file).expect("Failed to read zip");

    assert!(
        archive.by_name("metadata.json").is_ok(),
        "Archive should contain metadata.json"
    );

    // Verify metadata content
    let mut metadata_file = archive.by_name("metadata.json").unwrap();
    let mut content = String::new();
    std::io::Read::read_to_string(&mut metadata_file, &mut content).unwrap();

    let metadata: serde_json::Value =
        serde_json::from_str(&content).expect("metadata.json should be valid JSON");
    assert_eq!(
        metadata["label"].as_str(),
        Some("metadata-test"),
        "Label should match"
    );
    assert!(metadata["timestamp"].is_string(), "Should have timestamp");
    assert!(metadata["version"].is_string(), "Should have version");
}

#[test]
fn test_archive_save_includes_database() {
    let temp_dir = setup_test_env();
    create_dummy_database(&temp_dir);

    // Create archive
    let mut cmd = run_cli_with_data_dir(&temp_dir);
    cmd.args(["archive", "save", "--label", "db-test"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added database to archive"));

    // Verify archive contains database
    let archives_dir = temp_dir.path().join("data/archives");
    let archive_path = fs::read_dir(&archives_dir)
        .expect("Failed to read archives dir")
        .find_map(|e| {
            let entry = e.ok()?;
            if entry.file_name().to_string_lossy().contains("db-test") {
                Some(entry.path())
            } else {
                None
            }
        })
        .expect("Archive file not found");

    let file = fs::File::open(&archive_path).expect("Failed to open archive");
    let mut archive = zip::ZipArchive::new(file).expect("Failed to read zip");

    assert!(
        archive.by_name("mcp_agent_mail.db").is_ok(),
        "Archive should contain database file"
    );
}

#[test]
fn test_archive_save_includes_git_storage() {
    let temp_dir = setup_test_env();
    create_dummy_database(&temp_dir);
    create_dummy_git_storage(&temp_dir);

    // Create archive with git included (default)
    let mut cmd = run_cli_with_data_dir(&temp_dir);
    cmd.args(["archive", "save", "--label", "git-test"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added git storage to archive"));

    // Verify archive contains git storage
    let archives_dir = temp_dir.path().join("data/archives");
    let archive_path = fs::read_dir(&archives_dir)
        .expect("Failed to read archives dir")
        .find_map(|e| {
            let entry = e.ok()?;
            if entry.file_name().to_string_lossy().contains("git-test") {
                Some(entry.path())
            } else {
                None
            }
        })
        .expect("Archive file not found");

    let file = fs::File::open(&archive_path).expect("Failed to open archive");
    let mut archive = zip::ZipArchive::new(file).expect("Failed to read zip");

    // Check for git storage files
    let has_git_files = (0..archive.len()).any(|i| {
        archive
            .by_index(i)
            .map(|f| f.name().starts_with("git_storage/"))
            .unwrap_or(false)
    });

    assert!(has_git_files, "Archive should contain git storage files");
}

// ============================================================================
// Archive List Tests (PORT-2)
// ============================================================================

#[test]
fn test_archive_list_empty() {
    let temp_dir = setup_test_env();

    let mut cmd = run_cli_with_data_dir(&temp_dir);
    cmd.args(["archive", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No archives found"));
}

#[test]
fn test_archive_list_with_archives() {
    let temp_dir = setup_test_env();
    create_dummy_database(&temp_dir);

    // Create some archives
    run_cli_with_data_dir(&temp_dir)
        .args(["archive", "save", "--label", "backup-1"])
        .assert()
        .success();

    run_cli_with_data_dir(&temp_dir)
        .args(["archive", "save", "--label", "backup-2"])
        .assert()
        .success();

    // List archives
    let mut cmd = run_cli_with_data_dir(&temp_dir);
    cmd.args(["archive", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("backup-1"))
        .stdout(predicate::str::contains("backup-2"))
        .stdout(predicate::str::contains("Available restore points"));
}

#[test]
fn test_archive_list_json_format() {
    let temp_dir = setup_test_env();
    create_dummy_database(&temp_dir);

    // Create an archive
    run_cli_with_data_dir(&temp_dir)
        .args(["archive", "save", "--label", "json-test"])
        .assert()
        .success();

    // List in JSON format
    let output = run_cli_with_data_dir(&temp_dir)
        .args(["archive", "list", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value =
        serde_json::from_slice(&output).expect("Output should be valid JSON");

    assert!(json.is_array(), "JSON output should be an array");
    let archives = json.as_array().unwrap();
    assert!(!archives.is_empty(), "Should have at least one archive");

    // Verify archive structure
    let archive = &archives[0];
    assert!(archive["filename"].is_string(), "Should have filename");
    assert!(archive["path"].is_string(), "Should have path");
    assert!(archive["size_bytes"].is_number(), "Should have size");
    assert!(archive["label"].is_string(), "Should have label");
}

#[test]
fn test_archive_list_empty_json() {
    let temp_dir = setup_test_env();

    let output = run_cli_with_data_dir(&temp_dir)
        .args(["archive", "list", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value =
        serde_json::from_slice(&output).expect("Output should be valid JSON");

    assert!(json.is_array(), "JSON output should be an array");
    assert!(json.as_array().unwrap().is_empty(), "Should be empty array");
}

// ============================================================================
// Archive Restore Tests (PORT-3)
// ============================================================================

#[test]
fn test_archive_restore_not_found() {
    let temp_dir = setup_test_env();

    let mut cmd = run_cli_with_data_dir(&temp_dir);
    cmd.args(["archive", "restore", "nonexistent.zip", "--yes"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn test_archive_restore_success() {
    let temp_dir = setup_test_env();
    create_dummy_database(&temp_dir);

    // Create archive
    run_cli_with_data_dir(&temp_dir)
        .args(["archive", "save", "--label", "restore-test"])
        .assert()
        .success();

    // Find the archive path
    let archives_dir = temp_dir.path().join("data/archives");
    let archive_path = fs::read_dir(&archives_dir)
        .expect("Failed to read archives dir")
        .find_map(|e| {
            let entry = e.ok()?;
            if entry.file_name().to_string_lossy().contains("restore-test") {
                Some(entry.path())
            } else {
                None
            }
        })
        .expect("Archive file not found");

    // Delete the database to simulate data loss
    let db_path = temp_dir.path().join("data/mcp_agent_mail.db");
    fs::remove_file(&db_path).expect("Failed to remove db");
    assert!(!db_path.exists(), "Database should be deleted");

    // Restore from archive
    run_cli_with_data_dir(&temp_dir)
        .args([
            "archive",
            "restore",
            archive_path.to_string_lossy().as_ref(),
            "--yes",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Restored database"))
        .stdout(predicate::str::contains("Restore complete"));

    // Verify database is restored
    assert!(db_path.exists(), "Database should be restored");
}

#[test]
fn test_archive_restore_with_git_storage() {
    let temp_dir = setup_test_env();
    create_dummy_database(&temp_dir);
    create_dummy_git_storage(&temp_dir);

    // Create archive
    run_cli_with_data_dir(&temp_dir)
        .args(["archive", "save", "--label", "git-restore-test"])
        .assert()
        .success();

    // Find the archive path
    let archives_dir = temp_dir.path().join("data/archives");
    let archive_path = fs::read_dir(&archives_dir)
        .expect("Failed to read archives dir")
        .find_map(|e| {
            let entry = e.ok()?;
            if entry
                .file_name()
                .to_string_lossy()
                .contains("git-restore-test")
            {
                Some(entry.path())
            } else {
                None
            }
        })
        .expect("Archive file not found");

    // Delete git storage
    let git_path = temp_dir.path().join("data/archive");
    fs::remove_dir_all(&git_path).expect("Failed to remove git storage");
    assert!(!git_path.exists(), "Git storage should be deleted");

    // Restore from archive
    run_cli_with_data_dir(&temp_dir)
        .args([
            "archive",
            "restore",
            archive_path.to_string_lossy().as_ref(),
            "--yes",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Restored git storage"));

    // Verify git storage is restored
    assert!(git_path.exists(), "Git storage should be restored");
    assert!(
        git_path.join("HEAD").exists(),
        "Git HEAD should be restored"
    );
}

// ============================================================================
// Archive Clear-and-Reset Tests (PORT-4)
// ============================================================================

#[test]
fn test_archive_clear_and_reset_without_backup() {
    let temp_dir = setup_test_env();
    create_dummy_database(&temp_dir);
    create_dummy_git_storage(&temp_dir);

    let db_path = temp_dir.path().join("data/mcp_agent_mail.db");
    let git_path = temp_dir.path().join("data/archive");

    assert!(db_path.exists(), "Database should exist before reset");
    assert!(git_path.exists(), "Git storage should exist before reset");

    // Clear and reset without backup
    run_cli_with_data_dir(&temp_dir)
        .args(["archive", "clear-and-reset", "--yes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed database"))
        .stdout(predicate::str::contains("Removed git storage"))
        .stdout(predicate::str::contains("All data cleared"));

    assert!(!db_path.exists(), "Database should be removed");
    assert!(!git_path.exists(), "Git storage should be removed");
}

#[test]
fn test_archive_clear_and_reset_with_backup() {
    let temp_dir = setup_test_env();
    create_dummy_database(&temp_dir);
    create_dummy_git_storage(&temp_dir);

    let archives_dir = temp_dir.path().join("data/archives");

    // Clear and reset with backup
    run_cli_with_data_dir(&temp_dir)
        .args([
            "archive",
            "clear-and-reset",
            "--archive",
            "--label",
            "pre-wipe-backup",
            "--yes",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Creating backup archive"))
        .stdout(predicate::str::contains("All data cleared"));

    // Verify backup was created
    assert!(archives_dir.exists(), "Archives directory should exist");
    let has_backup = fs::read_dir(&archives_dir)
        .expect("Failed to read archives dir")
        .any(|e| {
            e.ok()
                .is_some_and(|entry| entry.file_name().to_string_lossy().contains("pre-wipe"))
        });
    assert!(has_backup, "Backup archive should be created");
}

// ============================================================================
// Data Integrity Tests (PORT-5)
// ============================================================================

#[test]
fn test_archive_roundtrip_data_integrity() {
    let temp_dir = setup_test_env();

    // Create database with specific content
    let db_path = temp_dir.path().join("data/mcp_agent_mail.db");
    fs::create_dir_all(db_path.parent().unwrap()).expect("Failed to create data dir");
    let original_content = b"SQLite format 3\x00specific test data 12345";
    fs::write(&db_path, original_content).expect("Failed to create db");

    // Create archive
    run_cli_with_data_dir(&temp_dir)
        .args(["archive", "save", "--label", "integrity-test"])
        .assert()
        .success();

    // Find the archive
    let archives_dir = temp_dir.path().join("data/archives");
    let archive_path = fs::read_dir(&archives_dir)
        .expect("Failed to read archives dir")
        .find_map(|e| {
            let entry = e.ok()?;
            if entry
                .file_name()
                .to_string_lossy()
                .contains("integrity-test")
            {
                Some(entry.path())
            } else {
                None
            }
        })
        .expect("Archive file not found");

    // Delete and restore
    fs::remove_file(&db_path).expect("Failed to remove db");
    run_cli_with_data_dir(&temp_dir)
        .args([
            "archive",
            "restore",
            archive_path.to_string_lossy().as_ref(),
            "--yes",
        ])
        .assert()
        .success();

    // Verify content matches
    let restored_content = fs::read(&db_path).expect("Failed to read restored db");
    assert_eq!(
        original_content.as_slice(),
        restored_content.as_slice(),
        "Restored content should match original"
    );
}

#[test]
fn test_archive_metadata_version_tracking() {
    let temp_dir = setup_test_env();
    create_dummy_database(&temp_dir);

    // Create archive
    run_cli_with_data_dir(&temp_dir)
        .args(["archive", "save", "--label", "version-test"])
        .assert()
        .success();

    // Verify version in metadata
    let archives_dir = temp_dir.path().join("data/archives");
    let archive_path = fs::read_dir(&archives_dir)
        .expect("Failed to read archives dir")
        .find_map(|e| {
            let entry = e.ok()?;
            if entry.file_name().to_string_lossy().contains("version-test") {
                Some(entry.path())
            } else {
                None
            }
        })
        .expect("Archive file not found");

    let file = fs::File::open(&archive_path).expect("Failed to open archive");
    let mut archive = zip::ZipArchive::new(file).expect("Failed to read zip");

    let mut metadata_file = archive.by_name("metadata.json").unwrap();
    let mut content = String::new();
    std::io::Read::read_to_string(&mut metadata_file, &mut content).unwrap();

    let metadata: serde_json::Value = serde_json::from_str(&content).unwrap();
    let version = metadata["version"].as_str().expect("Version should exist");

    // Version should be a semver string
    assert!(
        version.contains('.'),
        "Version should be semver format: {}",
        version
    );
}
