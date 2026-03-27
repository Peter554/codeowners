use std::collections::{HashMap, HashSet};

use codeowners_rs::{parse, RuleSet};

/// Resolve the owners of a path, returning individual owner strings.
/// Returns an empty vec if no rule matches.
pub fn resolve_owners(ruleset: &RuleSet, path: &str) -> Vec<String> {
    match ruleset.owners(path) {
        Some(owners) if !owners.is_empty() => owners.iter().map(|o| o.value.to_string()).collect(),
        _ => vec![],
    }
}

/// Given the raw CODEOWNERS sources at two refs, return only the lines that
/// were added or removed (i.e. lines unique to either side).  These are the
/// only patterns that can cause an ownership change.
///
/// We do a simple symmetric-difference on non-blank, non-comment lines rather
/// than a positional diff, because the *set* of active rules is what matters
/// for ownership — two files with the same rules in different order do differ
/// semantically, and a rule that moved position *is* a change.
pub fn changed_codeowners_lines(base_src: &str, head_src: &str) -> Vec<String> {
    let meaningful = |src: &str| -> Vec<String> {
        src.lines()
            .map(|l| l.trim().to_owned())
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .collect()
    };

    let base_lines = meaningful(base_src);
    let head_lines = meaningful(head_src);

    let mut base_counts: HashMap<String, usize> = HashMap::new();
    for line in &base_lines {
        *base_counts.entry(line.clone()).or_default() += 1;
    }
    let mut head_counts: HashMap<String, usize> = HashMap::new();
    for line in &head_lines {
        *head_counts.entry(line.clone()).or_default() += 1;
    }

    let all_keys: HashSet<_> = base_counts
        .keys()
        .chain(head_counts.keys())
        .cloned()
        .collect();

    let mut changed = Vec::new();
    for key in all_keys {
        let b = base_counts.get(&key).copied().unwrap_or(0);
        let h = head_counts.get(&key).copied().unwrap_or(0);
        if b != h {
            changed.push(key);
        }
    }
    changed
}

/// Build a small `RuleSet` that matches any file affected by the changed
/// CODEOWNERS lines.  We give every line a dummy owner so that
/// `matching_rule` returns `Some` for affected paths.
pub fn build_filter_ruleset(changed_lines: &[String]) -> RuleSet {
    let filter_src: String = changed_lines
        .iter()
        .map(|line| {
            let pattern = line.split_whitespace().next().unwrap_or("");
            format!("{pattern} @__filter__\n")
        })
        .collect();

    parse(&filter_src).into_ruleset()
}
