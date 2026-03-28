use anyhow::{bail, Result};
use codeowners_rs::parse;
use itertools::Itertools;
use rayon::prelude::*;

mod git;
mod owners;

use git::{discover_repo_root, list_files, load_codeowners};
use owners::{build_filter_ruleset, changed_codeowners_lines, explain_path, resolve_owners};

pub use owners::MatchedRule;

/// Look up the owners for each path using the working tree CODEOWNERS.
///
/// Returns a list of (path, owners) sorted by path. An empty owner list means
/// the path is unowned. When `check_paths` is true, returns an error if any
/// path does not exist.
pub fn get_owners(paths: &[String], check_paths: bool) -> Result<Vec<(String, Vec<String>)>> {
    let root = discover_repo_root()?;

    if check_paths {
        let missing: Vec<_> = paths.iter().filter(|p| !root.join(p).exists()).collect();
        if !missing.is_empty() {
            bail!("paths do not exist:\n{}", missing.iter().join("\n"));
        }
    }

    let src = load_codeowners(&root, &GitRef::WorkingTree)?;
    let ruleset = parse(&src).into_ruleset();

    let results: Vec<_> = paths
        .par_iter()
        .map(|p| (p.clone(), resolve_owners(&ruleset, p)))
        .collect::<Vec<_>>()
        .into_iter()
        .sorted()
        .collect();

    Ok(results)
}

/// Explain the CODEOWNERS assignment for a single path.
///
/// Returns the active owners and all matching rules with line numbers.
/// When `check_path` is true, returns an error if the path does not exist.
pub fn get_explain(path: &str, check_path: bool) -> Result<(Vec<String>, Vec<MatchedRule>)> {
    let root = discover_repo_root()?;

    if check_path && !root.join(path).exists() {
        bail!("path does not exist: {path}");
    }

    let src = load_codeowners(&root, &GitRef::WorkingTree)?;
    let ruleset = parse(&src).into_ruleset();

    let owners = resolve_owners(&ruleset, path);
    let rules = explain_path(&ruleset, &src, path);

    Ok((owners, rules))
}

/// A git ref or the working tree.
pub enum GitRef<'a> {
    Ref(&'a str),
    WorkingTree,
}

/// The result of diffing ownership between two refs.
pub struct OwnersDiff {
    /// Files present in head but not base, with their head owners.
    pub added: Vec<(String, Vec<String>)>,
    /// Files present in base but not head, with their base owners.
    pub removed: Vec<(String, Vec<String>)>,
    /// Files with changed ownership: (path, base owners, head owners).
    pub changed: Vec<(String, Vec<String>, Vec<String>)>,
}

/// Diff code ownership between two git refs.
///
/// Compares CODEOWNERS rules and file lists at both refs to find:
/// - Files added/removed between the refs (with their owners).
/// - Files present in both refs whose ownership changed due to CODEOWNERS
///   rule changes. Only files matching a changed rule are evaluated, to
///   avoid re-resolving ownership for every file.
pub fn get_diff(base_ref: &GitRef, head_ref: &GitRef) -> Result<OwnersDiff> {
    let root = discover_repo_root()?;

    // Parse CODEOWNERS at both refs into rulesets.
    let base_src = load_codeowners(&root, base_ref)?;
    let head_src = load_codeowners(&root, head_ref)?;

    let base_ruleset = parse(&base_src).into_ruleset();
    let head_ruleset = parse(&head_src).into_ruleset();

    // Build a filter ruleset from lines that differ between the two
    // CODEOWNERS files. This lets us skip files unaffected by rule changes.
    let filter_ruleset = if base_src == head_src {
        None
    } else {
        let changed_lines = changed_codeowners_lines(&base_src, &head_src);
        if changed_lines.is_empty() {
            None
        } else {
            Some(build_filter_ruleset(&changed_lines))
        }
    };

    let base_files = list_files(&root, base_ref)?;
    let head_files = list_files(&root, head_ref)?;

    // New files get head owners.
    let added: Vec<_> = head_files
        .difference(&base_files)
        .map(|f| (f.clone(), resolve_owners(&head_ruleset, f)))
        .sorted()
        .collect();

    // Deleted files get base owners.
    let removed: Vec<_> = base_files
        .difference(&head_files)
        .map(|f| (f.clone(), resolve_owners(&base_ruleset, f)))
        .sorted()
        .collect();

    let common: Vec<_> = base_files.intersection(&head_files).cloned().collect();

    let changed: Vec<_> = match &filter_ruleset {
        None => vec![],
        Some(filter) => common
            .par_iter()
            // For files in both refs, check only those matching a changed rule.
            .filter(|file| filter.matching_rule(file.as_str()).is_some())
            .filter_map(|file| {
                let base = resolve_owners(&base_ruleset, file);
                let head = resolve_owners(&head_ruleset, file);
                if base != head {
                    Some((file.clone(), base, head))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .into_iter()
            .sorted()
            .collect(),
    };

    Ok(OwnersDiff {
        added,
        removed,
        changed,
    })
}
