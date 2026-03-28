mod common;

use codeowners::explain_owners;
use insta::assert_yaml_snapshot;
use serial_test::serial;

fn setup_repo(codeowners: &str, files: &[&str]) -> assert_fs::TempDir {
    let dir = common::setup_repo(codeowners, files);
    std::env::set_current_dir(dir.path()).unwrap();
    dir
}

#[test]
#[serial]
fn explain_owners_multiple_matching_rules() {
    let _dir = setup_repo(
        "\
* @global
src/* @src-team",
        &["src/main.rs"],
    );

    let (owners, rules) = explain_owners("src/main.rs", true).unwrap();
    assert_yaml_snapshot!(owners, @r#"- "@src-team""#);
    assert_yaml_snapshot!(rules, @r#"
    - line: 1
      pattern: "*"
      owners:
        - "@global"
      active: false
    - line: 2
      pattern: src/*
      owners:
        - "@src-team"
      active: true
    "#);
}

#[test]
#[serial]
fn explain_owners_no_matching_rules() {
    let _dir = setup_repo("src/* @src-team", &["README.md"]);

    let (owners, rules) = explain_owners("README.md", true).unwrap();
    assert_yaml_snapshot!(owners, @"[]");
    assert_yaml_snapshot!(rules, @"[]");
}

#[test]
#[serial]
fn explain_owners_duplicate_pattern() {
    let _dir = setup_repo(
        "\
*.rs @team-a
*.rs @team-b",
        &["lib.rs"],
    );

    let (owners, rules) = explain_owners("lib.rs", true).unwrap();
    assert_yaml_snapshot!(owners, @r#"- "@team-b""#);
    assert_yaml_snapshot!(rules, @r#"
    - line: 1
      pattern: "*.rs"
      owners:
        - "@team-a"
      active: false
    - line: 2
      pattern: "*.rs"
      owners:
        - "@team-b"
      active: true
    "#);
}
