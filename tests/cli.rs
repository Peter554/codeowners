mod common;

use std::process::Command;

use assert_cmd::cargo::CommandCargoExt;
use assert_fs::prelude::*;
use assert_fs::TempDir;
use insta::assert_snapshot;

fn codeowners_cmd(dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("codeowners").unwrap();
    cmd.current_dir(dir.path());
    cmd
}

#[test]
fn cli_owners() {
    let dir = common::setup_repo(
        "\
* @global
src/* @src-team",
        &["README.md", "src/main.rs"],
    );

    let output = codeowners_cmd(&dir)
        .args(["owners", "README.md", "src/main.rs"])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_snapshot!(String::from_utf8(output.stdout).unwrap(), @"
    | path          | owners    |
    |---------------|-----------|
    | `README.md`   | @global   |
    | `src/main.rs` | @src-team |
    ");
}

#[test]
fn cli_explain() {
    let dir = common::setup_repo(
        "\
* @global
src/* @src-team",
        &["src/main.rs"],
    );

    let output = codeowners_cmd(&dir)
        .args(["explain", "src/main.rs"])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_snapshot!(String::from_utf8(output.stdout).unwrap(), @"
    Owners: @src-team

    |   | line | pattern | owners    |
    |---|------|---------|-----------|
    |   | 1    | *       | @global   |
    | → | 2    | src/*   | @src-team |
    ");
}

#[test]
fn cli_diff_ownership_change() {
    let dir = common::setup_repo(
        "\
* @global
src/* @team-a",
        &["src/main.rs", "README.md"],
    );

    dir.child(".github/CODEOWNERS")
        .write_str(
            "\
* @global
src/* @team-b",
        )
        .unwrap();

    let output = codeowners_cmd(&dir)
        .args(["diff", "HEAD"])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_snapshot!(String::from_utf8(output.stdout).unwrap(), @"
    ## Changed ownership (1 files)

    | file          | HEAD    | working tree |
    |---------------|---------|--------------|
    | `src/main.rs` | @team-a | @team-b      |
    ");
}

#[test]
fn cli_diff_no_changes() {
    let dir = common::setup_repo("* @global", &["README.md"]);

    let output = codeowners_cmd(&dir)
        .args(["diff", "HEAD"])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_snapshot!(String::from_utf8(output.stdout).unwrap(), @"No ownership changes.");
}

#[test]
fn cli_check_unowned_fails() {
    let dir = common::setup_repo("src/* @src-team", &["README.md", "src/main.rs"]);

    let output = codeowners_cmd(&dir)
        .args(["owners", "README.md", "src/main.rs", "--check-unowned"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    // Table is still printed to stdout.
    assert_snapshot!(String::from_utf8(output.stdout).unwrap(), @"
    | path          | owners    |
    |---------------|-----------|
    | `README.md`   | (unowned) |
    | `src/main.rs` | @src-team |
    ");
    // Error message on stderr.
    assert_snapshot!(String::from_utf8(output.stderr).unwrap(), @"Error: 1 unowned path(s)");
}
