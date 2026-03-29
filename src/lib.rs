use std::collections::{HashMap, HashSet};

use anyhow::{bail, Result};
use codeowners_rs::{parse, RuleSet};
use itertools::Itertools;
use rayon::prelude::*;
use serde::Serialize;

mod git;

/// Look up the owners for each path using the working tree CODEOWNERS.
///
/// Returns a list of (path, matched rule) sorted by path. `None` means the
/// path is unowned. When `check_paths` is true, returns an error if any path
/// does not exist. When `filter` is non-empty, only paths whose owners
/// intersect the filter are returned; use "unowned" to match unowned paths.
pub fn get_owners(
    paths: &[String],
    check_paths: bool,
    filter: &[String],
) -> Result<Vec<(String, Option<MatchingRule>)>> {
    let root = git::discover_repo_root()?;

    if check_paths {
        let missing: Vec<_> = paths.iter().filter(|p| !root.join(p).exists()).collect();
        if !missing.is_empty() {
            bail!("paths do not exist:\n{}", missing.iter().join("\n"));
        }
    }

    let src = git::load_codeowners(&root, &GitRef::WorkingTree)?;
    let ruleset = parse(&src).into_ruleset();
    let index = build_pattern_index(&src);
    let filter: HashSet<&str> = filter.iter().map(|s| s.as_str()).collect();

    let results: Vec<_> = paths
        .par_iter()
        .map(|p| (p.clone(), get_active_matching_rule(&ruleset, &index, p)))
        .filter(|(_, rule)| matches_filter(rule, &filter))
        .collect::<Vec<_>>()
        .into_iter()
        .sorted_by(|(a, _), (b, _)| a.cmp(b))
        .collect();

    Ok(results)
}

/// Returns true if the rule matches the filter, or if the filter is empty.
fn matches_filter(rule: &Option<MatchingRule>, filter: &HashSet<&str>) -> bool {
    if filter.is_empty() {
        return true;
    }
    let owners = match rule {
        Some(r) => &r.owners,
        None => return filter.contains("unowned"),
    };
    if owners.is_empty() {
        return filter.contains("unowned");
    }
    owners.iter().any(|o| filter.contains(o.as_str()))
}

/// Explain the CODEOWNERS assignment for a single path.
///
/// Returns the active owners and all matching rules with line numbers.
/// When `check_path` is true, returns an error if the path does not exist.
pub fn explain_owners(path: &str, check_path: bool) -> Result<(Vec<String>, Vec<MatchingRule>)> {
    let root = git::discover_repo_root()?;

    if check_path && !root.join(path).exists() {
        bail!("path does not exist: {path}");
    }

    let src = git::load_codeowners(&root, &GitRef::WorkingTree)?;
    let ruleset = parse(&src).into_ruleset();
    let index = build_pattern_index(&src);

    let owners = get_owners_from_ruleset(&ruleset, path);
    let rules = get_all_matching_rules(&ruleset, &index, path);

    Ok((owners, rules))
}

/// A git ref or the working tree.
pub enum GitRef<'a> {
    Ref(&'a str),
    WorkingTree,
}

/// The result of diffing ownership between two refs.
#[derive(Debug, serde::Serialize)]
pub struct OwnersDiff {
    /// Files present in head but not base, with their head owners.
    pub added: Vec<(String, Vec<String>)>,
    /// Files present in base but not head, with their base owners.
    pub removed: Vec<(String, Vec<String>)>,
    /// Files with changed ownership: (path, base owners, head owners).
    pub changed: Vec<(String, Vec<String>, Vec<String>)>,
}

/// Diff code ownership between two git refs.
pub fn get_diff(base_ref: &GitRef, head_ref: &GitRef) -> Result<OwnersDiff> {
    let root = git::discover_repo_root()?;

    // Parse CODEOWNERS at both refs into rulesets.
    let base_src = git::load_codeowners(&root, base_ref)?;
    let head_src = git::load_codeowners(&root, head_ref)?;

    let base_ruleset = parse(&base_src).into_ruleset();
    let head_ruleset = parse(&head_src).into_ruleset();

    let base_files = git::list_files(&root, base_ref)?;
    let head_files = git::list_files(&root, head_ref)?;

    // New files get head owners.
    let added: Vec<_> = head_files
        .difference(&base_files)
        .map(|f| (f.clone(), get_owners_from_ruleset(&head_ruleset, f)))
        .sorted()
        .collect();

    // Deleted files get base owners.
    let removed: Vec<_> = base_files
        .difference(&head_files)
        .map(|f| (f.clone(), get_owners_from_ruleset(&base_ruleset, f)))
        .sorted()
        .collect();

    let common_files: Vec<_> = base_files.intersection(&head_files).cloned().collect();

    let changed: Vec<_> = common_files
        .par_iter()
        .filter_map(|file| {
            let base = get_owners_from_ruleset(&base_ruleset, file);
            let head = get_owners_from_ruleset(&head_ruleset, file);
            if base != head {
                Some((file.clone(), base, head))
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .into_iter()
        .sorted()
        .collect();

    Ok(OwnersDiff {
        added,
        removed,
        changed,
    })
}

/// Resolve the owners of a path, returning individual owner strings.
/// Returns an empty vec if no rule matches.
fn get_owners_from_ruleset(ruleset: &RuleSet, path: &str) -> Vec<String> {
    match ruleset.owners(path) {
        Some(owners) if !owners.is_empty() => owners.iter().map(|o| o.value.to_string()).collect(),
        _ => vec![],
    }
}

/// A CODEOWNERS rule that matched a path, with its source line number.
#[derive(Debug, Serialize)]
pub struct MatchingRule {
    pub line: usize,
    pub pattern: String,
    pub owners: Vec<String>,
    pub active: bool,
}

/// Resolve the active matching rule for a path using the prebuilt index.
/// Uses the last occurrence of the winning pattern (CODEOWNERS last-match-wins).
fn get_active_matching_rule(
    ruleset: &RuleSet,
    index: &PatternIndex,
    path: &str,
) -> Option<MatchingRule> {
    let rule = ruleset.matching_rule(path)?;
    let entries = index
        .get(rule.pattern.as_str())
        .expect("matching rule pattern must exist in index");
    let (line, owners) = entries.last().expect("pattern index entry cannot be empty");
    Some(MatchingRule {
        line: *line,
        pattern: rule.pattern.clone(),
        owners: owners.clone(),
        active: true,
    })
}

/// Find all CODEOWNERS rules matching a path, using the prebuilt pattern index.
///
/// Gets the set of matching patterns from the ruleset, then looks up all
/// occurrences in the index. Results are sorted by line number. The last
/// matching rule is marked active (CODEOWNERS uses last-match-wins semantics).
fn get_all_matching_rules(
    ruleset: &RuleSet,
    index: &PatternIndex,
    path: &str,
) -> Vec<MatchingRule> {
    let matching = ruleset.all_matching_rules(path);
    let matching_patterns: HashSet<&str> =
        matching.iter().map(|(_, r)| r.pattern.as_str()).collect();

    let mut matching_rules: Vec<MatchingRule> = matching_patterns
        .iter()
        .flat_map(|pattern| {
            index
                .get(*pattern)
                .into_iter()
                .flatten()
                .map(|(line, owners)| MatchingRule {
                    line: *line,
                    pattern: (*pattern).to_owned(),
                    owners: owners.clone(),
                    active: false,
                })
        })
        .collect();

    matching_rules.sort_by_key(|r| r.line);

    if let Some(last) = matching_rules.last_mut() {
        last.active = true;
    }

    matching_rules
}

/// Index mapping CODEOWNERS patterns to all their occurrences (line number, owners).
type PatternIndex = HashMap<String, Vec<(usize, Vec<String>)>>;

/// Build an index of pattern -> [(line number, owners)] by scanning the source once.
fn build_pattern_index(src: &str) -> PatternIndex {
    let mut index: PatternIndex = HashMap::new();
    for (i, line) in src.lines().enumerate() {
        let mut parts = line.split_whitespace();
        if let Some(pattern) = parts.next() {
            if pattern.starts_with('#') {
                continue;
            }
            let owners: Vec<String> = parts.map(String::from).collect();
            index
                .entry(pattern.to_owned())
                .or_default()
                .push((i + 1, owners));
        }
    }
    index
}
