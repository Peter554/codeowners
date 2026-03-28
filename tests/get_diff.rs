mod common;

use assert_fs::prelude::*;
use codeowners::{get_diff, GitRef};
use insta::assert_yaml_snapshot;
use serial_test::serial;
use std::fs;
use std::process::Command;

fn setup_repo(codeowners: &str, files: &[&str]) -> assert_fs::TempDir {
    let dir = common::setup_repo(codeowners, files);
    std::env::set_current_dir(dir.path()).unwrap();
    dir
}

fn run_git(dir: &assert_fs::TempDir, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[serial]
fn get_diff_no_changes() {
    let _dir = setup_repo("* @global", &["README.md"]);

    let diff = get_diff(&GitRef::Ref("HEAD"), &GitRef::WorkingTree).unwrap();
    assert_yaml_snapshot!(diff, @"
    added: []
    removed: []
    changed: []
    ");
}

#[test]
#[serial]
fn get_diff_ownership_change() {
    let dir = setup_repo(
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

    let diff = get_diff(&GitRef::Ref("HEAD"), &GitRef::WorkingTree).unwrap();
    assert_yaml_snapshot!(diff, @r#"
    added: []
    removed: []
    changed:
      - - src/main.rs
        - - "@team-a"
        - - "@team-b"
    "#);
}

#[test]
#[serial]
fn get_diff_added_file() {
    let dir = setup_repo("* @global", &["README.md"]);

    dir.child("new_file.rs").write_str("").unwrap();
    run_git(&dir, &["add", "new_file.rs"]);

    let diff = get_diff(&GitRef::Ref("HEAD"), &GitRef::WorkingTree).unwrap();
    assert_yaml_snapshot!(diff, @r#"
    added:
      - - new_file.rs
        - - "@global"
    removed: []
    changed: []
    "#);
}

#[test]
#[serial]
fn get_diff_removed_file() {
    let dir = setup_repo("* @global", &["README.md", "to_delete.rs"]);

    std::fs::remove_file(dir.path().join("to_delete.rs")).unwrap();
    run_git(&dir, &["add", "to_delete.rs"]);

    let diff = get_diff(&GitRef::Ref("HEAD"), &GitRef::WorkingTree).unwrap();
    assert_yaml_snapshot!(diff, @r#"
    added: []
    removed:
      - - to_delete.rs
        - - "@global"
    changed: []
    "#);
}

#[test]
#[serial]
fn get_diff_feature_branch_against_master() {
    let dir = setup_repo(
        "\
* @global
src/* @team-a",
        &["README.md", "src/main.rs", "src/lib.rs"],
    );

    run_git(&dir, &["checkout", "-b", "feature"]);

    // Add a new file.
    dir.child("src/new.rs").write_str("").unwrap();

    // Delete a file.
    fs::remove_file(dir.child("src/lib.rs").path()).unwrap();

    // Change CODEOWNERS.
    dir.child(".github/CODEOWNERS")
        .write_str(
            "\
* @global
src/* @team-b",
        )
        .unwrap();

    run_git(&dir, &["add", "-A"]);
    run_git(&dir, &["commit", "-m", "feature changes"]);

    let diff = get_diff(&GitRef::Ref("master"), &GitRef::Ref("feature")).unwrap();
    assert_yaml_snapshot!(diff, @r#"
    added:
      - - src/new.rs
        - - "@team-b"
    removed:
      - - src/lib.rs
        - - "@team-a"
    changed:
      - - src/main.rs
        - - "@team-a"
        - - "@team-b"
    "#);
}
