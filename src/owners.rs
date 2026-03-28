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

#[cfg(test)]
mod tests {
    use super::*;

    fn ruleset(src: &str) -> RuleSet {
        parse(src).into_ruleset()
    }

    fn assert_index_entries(src: &str, pattern: &str, expected: &[(usize, &[&str])]) {
        let index = build_pattern_index(src);
        let entries = index
            .get(pattern)
            .unwrap_or_else(|| panic!("pattern {pattern:?} not found in index"));
        let expected: Vec<(usize, Vec<String>)> = expected
            .iter()
            .map(|(line, owners)| (*line, owners.iter().map(|s| s.to_string()).collect()))
            .collect();
        assert_eq!(entries, &expected);
    }

    // -- build_pattern_index --------------------------------------------------

    #[test]
    fn index_basic() {
        assert_index_entries(
            "\
* @global
src/* @src-team",
            "src/*",
            &[(2, &["@src-team"])],
        );
    }

    #[test]
    fn index_comments_and_blanks() {
        assert_index_entries(
            "\
# comment

* @global",
            "*",
            &[(3, &["@global"])],
        );
    }

    #[test]
    fn index_duplicate_patterns() {
        assert_index_entries(
            "\
*.rs @team-a
*.rs @team-b",
            "*.rs",
            &[(1, &["@team-a"]), (2, &["@team-b"])],
        );
    }

    #[test]
    fn index_no_owners() {
        assert_index_entries("*.log", "*.log", &[(1, &[])]);
    }

    // -- resolve_matched_rule -------------------------------------------------

    #[test]
    fn resolve_matches() {
        let src = "\
* @global
src/* @src-team";
        let rs = ruleset(src);
        let index = build_pattern_index(src);
        let rule = resolve_matched_rule(&rs, &index, "src/main.rs").expect("expected match");
        assert_eq!(rule.pattern, "src/*");
        assert_eq!(rule.line, 2);
        assert_eq!(rule.owners, vec!["@src-team"]);
        assert!(rule.active);
    }

    #[test]
    fn resolve_no_match() {
        let src = "src/* @src-team";
        let rs = ruleset(src);
        let index = build_pattern_index(src);
        assert!(resolve_matched_rule(&rs, &index, "README.md").is_none());
    }

    #[test]
    fn resolve_duplicate_last_wins() {
        let src = "\
*.rs @team-a
*.rs @team-b";
        let rs = ruleset(src);
        let index = build_pattern_index(src);
        let rule = resolve_matched_rule(&rs, &index, "lib.rs").expect("expected match");
        assert_eq!(rule.pattern, "*.rs");
        assert_eq!(rule.line, 2);
        assert_eq!(rule.owners, vec!["@team-b"]);
    }

    // -- explain_path ---------------------------------------------------------

    fn assert_explain(src: &str, path: &str, expected: &[(usize, &str, bool)]) {
        let rs = ruleset(src);
        let index = build_pattern_index(src);
        let rules = explain_path(&rs, &index, path);
        assert_eq!(rules.len(), expected.len(), "wrong number of rules");
        for (rule, (line, pattern, active)) in rules.iter().zip(expected.iter()) {
            assert_eq!(rule.line, *line);
            assert_eq!(rule.pattern, *pattern);
            assert_eq!(rule.active, *active);
        }
    }

    #[test]
    fn explain_single_rule() {
        assert_explain("* @global", "any/file.rs", &[(1, "*", true)]);
    }

    #[test]
    fn explain_multiple_rules() {
        assert_explain(
            "\
* @global
src/* @src-team",
            "src/main.rs",
            &[(1, "*", false), (2, "src/*", true)],
        );
    }

    #[test]
    fn explain_duplicate_pattern() {
        assert_explain(
            "\
*.rs @team-a
*.rs @team-b",
            "lib.rs",
            &[(1, "*.rs", false), (2, "*.rs", true)],
        );
    }

    #[test]
    fn explain_no_match() {
        assert_explain("src/* @src-team", "README.md", &[]);
    }

    // -- changed_codeowners_lines ---------------------------------------------

    fn assert_changed_lines(base: &str, head: &str, expected: &[&str]) {
        let mut result = changed_codeowners_lines(base, head);
        result.sort();
        let mut expected: Vec<String> = expected.iter().map(|s| s.to_string()).collect();
        expected.sort();
        assert_eq!(result, expected);
    }

    #[test]
    fn changed_identical() {
        assert_changed_lines("* @global", "* @global", &[]);
    }

    #[test]
    fn changed_added_line() {
        assert_changed_lines(
            "* @global",
            "\
* @global
src/* @src-team",
            &["src/* @src-team"],
        );
    }

    #[test]
    fn changed_removed_line() {
        assert_changed_lines(
            "\
* @global
src/* @src-team",
            "* @global",
            &["src/* @src-team"],
        );
    }

    #[test]
    fn changed_comments_ignored() {
        assert_changed_lines(
            "* @global",
            "\
# new comment
* @global",
            &[],
        );
    }

    #[test]
    fn changed_blanks_ignored() {
        assert_changed_lines(
            "* @global",
            "
* @global
",
            &[],
        );
    }
}
