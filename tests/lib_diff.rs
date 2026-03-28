mod common;

use assert_fs::prelude::*;
use codeowners::{get_diff, GitRef};
use insta::assert_yaml_snapshot;
use serial_test::serial;
use std::process::Command;

fn in_repo(codeowners: &str, files: &[&str]) -> assert_fs::TempDir {
    let dir = common::setup_repo(codeowners, files);
    std::env::set_current_dir(dir.path()).unwrap();
    dir
}

#[test]
#[serial]
fn diff_no_changes() {
    let _dir = in_repo("* @global", &["README.md"]);

    let diff = get_diff(&GitRef::Ref("HEAD"), &GitRef::WorkingTree).unwrap();
    assert_yaml_snapshot!(diff, @"
    added: []
    removed: []
    changed: []
    ");
}

#[test]
#[serial]
fn diff_ownership_change() {
    let dir = in_repo(
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
fn diff_added_file() {
    let dir = in_repo("* @global", &["README.md"]);

    dir.child("new_file.rs").write_str("").unwrap();
    Command::new("git")
        .args(["add", "new_file.rs"])
        .current_dir(dir.path())
        .output()
        .unwrap();

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
fn diff_removed_file() {
    let dir = in_repo("* @global", &["README.md", "to_delete.rs"]);

    std::fs::remove_file(dir.path().join("to_delete.rs")).unwrap();
    Command::new("git")
        .args(["add", "to_delete.rs"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let diff = get_diff(&GitRef::Ref("HEAD"), &GitRef::WorkingTree).unwrap();
    assert_yaml_snapshot!(diff, @r#"
    added: []
    removed:
      - - to_delete.rs
        - - "@global"
    changed: []
    "#);
}
