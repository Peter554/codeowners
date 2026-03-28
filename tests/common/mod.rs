use std::process::Command;

use assert_fs::prelude::*;
use assert_fs::TempDir;

/// Set up a temp git repo with a CODEOWNERS file and the given tracked files.
pub fn setup_repo(codeowners: &str, files: &[&str]) -> TempDir {
    let dir = TempDir::new().unwrap();

    Command::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    dir.child(".github/CODEOWNERS")
        .write_str(codeowners)
        .unwrap();

    for file in files {
        dir.child(file).write_str("").unwrap();
    }

    Command::new("git")
        .args(["add", "-A"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    dir
}
