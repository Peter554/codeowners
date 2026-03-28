mod common;

use codeowners::get_owners;
use insta::assert_yaml_snapshot;
use serial_test::serial;

fn setup_repo(codeowners: &str, files: &[&str]) -> assert_fs::TempDir {
    let dir = common::setup_repo(codeowners, files);
    std::env::set_current_dir(dir.path()).unwrap();
    dir
}

// -- get_owners ---------------------------------------------------------------

#[test]
#[serial]
fn get_owners_basic() {
    let _dir = setup_repo(
        "\
* @global
src/* @src-team",
        &["README.md", "src/main.rs"],
    );
    let paths = vec!["README.md".into(), "src/main.rs".into()];

    let result = get_owners(&paths, true, &[]).unwrap();
    assert_yaml_snapshot!(result, @r#"
    - - README.md
      - line: 1
        pattern: "*"
        owners:
          - "@global"
        active: true
    - - src/main.rs
      - line: 2
        pattern: src/*
        owners:
          - "@src-team"
        active: true
    "#);
}

#[test]
#[serial]
fn get_owners_filter_by_team() {
    let _dir = setup_repo(
        "\
* @global
src/* @src-team",
        &["README.md", "src/main.rs"],
    );
    let paths = vec!["README.md".into(), "src/main.rs".into()];
    let filter = vec!["@src-team".into()];

    let result = get_owners(&paths, true, &filter).unwrap();
    assert_yaml_snapshot!(result, @r#"
    - - src/main.rs
      - line: 2
        pattern: src/*
        owners:
          - "@src-team"
        active: true
    "#);
}

#[test]
#[serial]
fn get_owners_filter_unowned() {
    let _dir = setup_repo("src/* @src-team", &["README.md", "src/main.rs"]);
    let paths = vec!["README.md".into(), "src/main.rs".into()];
    let filter = vec!["unowned".into()];

    let result = get_owners(&paths, true, &filter).unwrap();
    assert_yaml_snapshot!(result, @"
    - - README.md
      - ~
    ");
}

#[test]
#[serial]
fn get_owners_unowned_path() {
    let _dir = setup_repo("src/* @src-team", &["README.md", "src/main.rs"]);
    let paths = vec!["README.md".into()];

    let result = get_owners(&paths, true, &[]).unwrap();
    assert_yaml_snapshot!(result, @"
    - - README.md
      - ~
    ");
}

// -- get_owners path validation -----------------------------------------------

#[test]
#[serial]
fn get_owners_rejects_missing_path() {
    let _dir = setup_repo("* @global", &["README.md"]);
    let paths = vec!["does/not/exist.rs".to_string()];

    let err = get_owners(&paths, true, &[]).unwrap_err();
    assert!(err.to_string().contains("paths do not exist"));
}

#[test]
#[serial]
fn get_owners_no_check_path_skips_validation() {
    let _dir = setup_repo("* @global", &["README.md"]);
    let paths = vec!["does/not/exist.rs".to_string()];

    let result = get_owners(&paths, false, &[]).unwrap();
    assert_eq!(result.len(), 1);
}
