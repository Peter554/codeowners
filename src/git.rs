use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::GitRef;
use anyhow::{bail, Context, Result};

/// Discover the repository root.
pub(crate) fn discover_repo_root() -> Result<PathBuf> {
    let out = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("failed to run git rev-parse")?;
    if !out.status.success() {
        bail!(
            "git rev-parse failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(PathBuf::from(String::from_utf8(out.stdout)?.trim()))
}

/// Load `.github/CODEOWNERS` at a given ref.
pub(crate) fn load_codeowners(root: &Path, git_ref: &GitRef) -> Result<String> {
    match git_ref {
        GitRef::WorkingTree => {
            let path = root.join(".github/CODEOWNERS");
            std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))
        }
        GitRef::Ref(r) => {
            let blob = format!("{r}:.github/CODEOWNERS");
            let out = Command::new("git")
                .args(["cat-file", "blob", &blob])
                .current_dir(root)
                .output()
                .context("failed to run git cat-file")?;

            if !out.status.success() {
                bail!(
                    "git cat-file failed: {}",
                    String::from_utf8_lossy(&out.stderr)
                );
            }
            Ok(String::from_utf8(out.stdout)?)
        }
    }
}

/// List all tracked files.
pub(crate) fn list_files(root: &Path, git_ref: &GitRef) -> Result<HashSet<String>> {
    let out = match git_ref {
        GitRef::WorkingTree => Command::new("git")
            .arg("ls-files")
            .current_dir(root)
            .output()
            .context("git ls-files failed")?,
        GitRef::Ref(r) => Command::new("git")
            .args(["ls-tree", "-r", "--name-only", r])
            .current_dir(root)
            .output()
            .context("git ls-tree failed")?,
    };

    if !out.status.success() {
        bail!(
            "listing files failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    Ok(String::from_utf8(out.stdout)?
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect())
}
