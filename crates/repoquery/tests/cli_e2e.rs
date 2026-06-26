//! End-to-end CLI tests for the repoquery binary.
//!
//! Tests validate that CLI commands parse correctly, return expected
//! exit codes, and produce expected output formats.

use assert_cmd::Command;
use predicates::prelude::*;
use std::path::PathBuf;

/// Helper: get the repoquery binary path.
fn repoquery() -> Command {
    Command::cargo_bin("repoquery").unwrap()
}

/// Helper: path to a test data file for integration tests.
fn test_data_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests");
    p.push("test_data.yml");
    p
}

// ──── Help and Version ────

#[test]
fn test_help_contains_commands() {
    repoquery()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("query"))
        .stdout(predicate::str::contains("activity"))
        .stdout(predicate::str::contains("config"))
        .stdout(predicate::str::contains("sync"))
        .stdout(predicate::str::contains("parse"));
}

#[test]
fn test_version_output() {
    repoquery()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("v0.1.0"))
        .stdout(predicate::str::contains("security"));
}

// ──── Query Commands ────

#[test]
fn test_query_list_shows_table() {
    repoquery()
        .args(["query", "list", "--store"])
        .arg(test_data_path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Name"))
        .stdout(predicate::str::contains("Language"));
}

#[test]
fn test_query_list_json_output() {
    repoquery()
        .args(["query", "list", "--format", "json", "--store"])
        .arg(test_data_path())
        .assert()
        .success()
        .stdout(predicate::str::starts_with("["));
}

#[test]
fn test_query_list_csv_output() {
    repoquery()
        .args(["query", "list", "--format", "csv", "--store"])
        .arg(test_data_path())
        .assert()
        .success()
        .stdout(predicate::str::contains("name,owner,language"));
}

#[test]
fn test_query_list_markdown_output() {
    repoquery()
        .args(["query", "list", "--format", "md", "--store"])
        .arg(test_data_path())
        .assert()
        .success()
        .stdout(predicate::str::contains("| Name |"));
}

#[test]
fn test_query_search_finds_repos() {
    repoquery()
        .args(["query", "search", "rust", "--store"])
        .arg(test_data_path())
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
fn test_query_topics_shows_table() {
    repoquery()
        .args(["query", "topics", "--store"])
        .arg(test_data_path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Topic"));
}

#[test]
fn test_query_languages_shows_table() {
    repoquery()
        .args(["query", "languages", "--store"])
        .arg(test_data_path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Language"));
}

// ──── Config Commands ────

#[test]
fn test_config_help() {
    repoquery()
        .args(["config", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("init"))
        .stdout(predicate::str::contains("show"))
        .stdout(predicate::str::contains("set"));
}

// ──── Activity Commands ────

#[test]
fn test_activity_stale_shows_results() {
    repoquery()
        .args(["activity", "stale", "--store"])
        .arg(test_data_path())
        .assert()
        .success()
        .stdout(predicate::str::contains("stale"));
}

// ──── Error Handling ────

#[test]
fn test_missing_subcommand_shows_help() {
    repoquery()
        .arg("query")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage: repoquery query <COMMAND>"));
}

#[test]
fn test_nonexistent_store_fails_gracefully() {
    repoquery()
        .args(["query", "list", "--store", "/nonexistent/path/data.db"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("error").or(predicate::str::contains("Error")));
}

#[test]
fn test_invalid_flag_fails() {
    repoquery()
        .args(["query", "list", "--nonexistent-flag"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("error"));
}

// ──── Filter Combinations ────

#[test]
fn test_query_list_with_multiple_filters() {
    repoquery()
        .args([
            "query",
            "list",
            "--language",
            "Rust",
            "--min-stars",
            "100",
            "--sort",
            "stars",
            "--limit",
            "10",
            "--store",
        ])
        .arg(test_data_path())
        .assert()
        .success();
}

#[test]
fn test_query_list_sort_by_name_asc() {
    repoquery()
        .args([
            "query", "list", "--sort", "name", "--order", "asc", "--store",
        ])
        .arg(test_data_path())
        .assert()
        .success();
}
