use std::collections::{HashMap, HashSet};

use codeowners_rs::{parse, RuleSet};
use serde::Serialize;

/// A CODEOWNERS rule that matched a path, with its source line number.
#[derive(Debug, Serialize)]
pub struct MatchedRule {
    pub line: usize,
    pub pattern: String,
    pub owners: Vec<String>,
    pub active: bool,
}

/// Find all CODEOWNERS rules matching a path, using the prebuilt pattern index.
///
/// Gets the set of matching patterns from the ruleset, then looks up all
/// occurrences in the index. Results are sorted by line number. The last
/// matching rule is marked active (CODEOWNERS uses last-match-wins semantics).
pub fn explain_path(ruleset: &RuleSet, index: &PatternIndex, path: &str) -> Vec<MatchedRule> {
    let matched = ruleset.all_matching_rules(path);
    let matched_patterns: HashSet<&str> = matched.iter().map(|(_, r)| r.pattern.as_str()).collect();

    let mut matched_rules: Vec<MatchedRule> = matched_patterns
        .iter()
        .flat_map(|pattern| {
            index
                .get(*pattern)
                .into_iter()
                .flatten()
                .map(|(line, owners)| MatchedRule {
                    line: *line,
                    pattern: (*pattern).to_owned(),
                    owners: owners.clone(),
                    active: false,
                })
        })
        .collect();

    matched_rules.sort_by_key(|r| r.line);

    if let Some(last) = matched_rules.last_mut() {
        last.active = true;
    }

    matched_rules
}

/// Index mapping CODEOWNERS patterns to all their occurrences (line number, owners).
pub type PatternIndex = HashMap<String, Vec<(usize, Vec<String>)>>;

/// Build an index of pattern -> [(line number, owners)] by scanning the source once.
pub fn build_pattern_index(src: &str) -> PatternIndex {
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

/// Resolve the active matching rule for a path using the prebuilt index.
/// Uses the last occurrence of the winning pattern (CODEOWNERS last-match-wins).
pub fn resolve_matched_rule(
    ruleset: &RuleSet,
    index: &PatternIndex,
    path: &str,
) -> Option<MatchedRule> {
    let rule = ruleset.matching_rule(path)?;
    let entries = index
        .get(rule.pattern.as_str())
        .expect("matching rule pattern must exist in index");
    let (line, owners) = entries.last().expect("pattern index entry cannot be empty");
    Some(MatchedRule {
        line: *line,
        pattern: rule.pattern.clone(),
        owners: owners.clone(),
        active: true,
    })
}

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
