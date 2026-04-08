//! CLI binary tests using `assert_cmd`.
//!
//! These tests invoke the `sdk-forge` binary as a subprocess to verify
//! argument parsing, exit codes, and error messages.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::missing_docs_in_private_items
)]

mod common;

use assert_cmd::Command;
use predicates::prelude::predicate;

// ── Help and version ───────────────────────────────────────────────

#[test]
fn test_cli_help_exits_zero() {
    Command::cargo_bin("sdk-forge")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("SDK Forge"));
}

#[test]
fn test_cli_version_exits_zero() {
    Command::cargo_bin("sdk-forge")
        .unwrap()
        .arg("--version")
        .assert()
        .success();
}

// ── Generate command ───────────────────────────────────────────────

#[test]
fn test_cli_generate_missing_session_file() {
    Command::cargo_bin("sdk-forge")
        .unwrap()
        .args(["generate", "/nonexistent/path.sdkforge"])
        .assert()
        .failure();
}

#[test]
fn test_cli_generate_unanalyzed_session_fails() {
    Command::cargo_bin("sdk-forge")
        .unwrap()
        .args(["generate", common::petstore_unanalyzed().to_str().unwrap()])
        .assert()
        .failure();
}

#[test]
fn test_cli_generate_analyzed_session_succeeds() {
    let temp = tempfile::tempdir().unwrap();
    Command::cargo_bin("sdk-forge")
        .unwrap()
        .args([
            "generate",
            common::petstore_analyzed().to_str().unwrap(),
            "-o",
            temp.path().to_str().unwrap(),
            "-n",
            "test-sdk",
        ])
        .assert()
        .success();

    // Verify the crate was actually generated (now a Wasm component).
    assert!(
        temp.path().join("test-sdk/Cargo.toml").exists(),
        "should generate Cargo.toml"
    );
    assert!(
        temp.path().join("test-sdk/src/lib.rs").exists(),
        "should generate lib.rs"
    );
    assert!(
        temp.path().join("test-sdk/wit/world.wit").exists(),
        "should generate world.wit"
    );
}

// ── Inspect command ────────────────────────────────────────────────

#[test]
fn test_cli_inspect_missing_file() {
    Command::cargo_bin("sdk-forge")
        .unwrap()
        .args(["inspect", "/nonexistent/path.sdkforge"])
        .assert()
        .failure();
}

// ── Record command ─────────────────────────────────────────────────

#[test]
fn test_cli_record_requires_url_argument() {
    Command::cargo_bin("sdk-forge")
        .unwrap()
        .arg("record")
        .assert()
        .failure();
}

// ── No subcommand ──────────────────────────────────────────────────

#[test]
fn test_cli_no_subcommand_shows_help() {
    Command::cargo_bin("sdk-forge")
        .unwrap()
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage"));
}
