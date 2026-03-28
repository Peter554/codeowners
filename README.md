# codeowners

CLI tools for working with GitHub CODEOWNERS files.

## Install

```
cargo install --path .
```

## Commands

### owners (default)

Show the owners for one or more paths, based on the working tree CODEOWNERS. This is the default command — the `owners` subcommand can be omitted.

**Flags:**
- `--stdin` - Read paths from stdin (one per line)
- `--no-check-path` - Skip checking that paths exist
- `--filter` - Filter results by owner (comma-separated). Use `unowned` for unowned paths.
- `--check-unowned` - Error if any paths are unowned (after printing the table)

```
codeowners src/main.rs src/lib.rs
codeowners owners src/main.rs src/lib.rs  # equivalent

# Read paths from stdin
git ls-files | codeowners --stdin

# Skip path existence check
codeowners --no-check-path path/that/may/not/exist.rs

# Filter by owner
git ls-files | codeowners --stdin --filter @my-team
git ls-files | codeowners --stdin --filter @team-a,@team-b

# Show only unowned files
git ls-files | codeowners --stdin --filter unowned

# Error if any files are unowned
git ls-files | codeowners --stdin --check-unowned
```

### explain

Explain the CODEOWNERS assignment for a path. Shows all matching rules, which ones were superseded, and which rule is active.

**Flags:**
- `--no-check-path` - Skip checking that the path exists

```
codeowners explain src/main.rs
```

### diff

Show how code ownership changes between two git refs. Reports added files, removed files, and files whose ownership changed due to CODEOWNERS rule changes.

```
# Compare HEAD to the working tree (default)
codeowners diff

# Compare two refs
codeowners diff main feature-branch
```

## Acknowledgements

The `diff` subcommand is inspired by [codeowners-diff](https://github.com/samueljsb/codeowners-diff).
