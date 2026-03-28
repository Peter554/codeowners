# codeowners

CLI tools for working with GitHub CODEOWNERS files.

## Install

```
cargo install --git https://github.com/Peter554/codeowners
```

## Commands

- [owners](#owners-default) - Show the owners for one or more paths
- [explain](#explain) - Explain the codeowners assignment for a path
- [diff](#diff) - Show how code ownership changes between two git refs

### owners (default)

Show the owners for one or more paths, based on the working tree CODEOWNERS. This is the default command — the `owners` subcommand can be omitted.

```
Show the owners for one or more paths

Usage: codeowners owners [OPTIONS] [PATHS]...

Arguments:
  [PATHS]...  Paths to look up owners for

Options:
      --stdin            Read paths from stdin (one per line)
      --no-check-path    Skip checking that paths exist
      --filter <FILTER>  Filter results by owner (comma-separated). Use "unowned" for unowned paths
      --check-unowned    Error if any paths are unowned (after printing the table)
  -h, --help             Print help
```

**Examples:**

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

# Show unowned files added/modified between two refs (e.g. master and HEAD)
git diff --name-only --diff-filter=d master..HEAD | codeowners --stdin --filter unowned --check-unowned
```

### explain

Explain the CODEOWNERS assignment for a path. Shows all matching rules, which ones were superseded, and which rule is active.

```
Explain the CODEOWNERS assignment for a path

Usage: codeowners explain [OPTIONS] <PATH>

Arguments:
  <PATH>  Path to explain ownership for

Options:
      --no-check-path  Skip checking that the path exists
  -h, --help           Print help
```

**Examples:**

```
codeowners explain src/main.rs
```

### diff

Show how code ownership changes between two git refs. Reports added files, removed files, and files whose ownership changed due to CODEOWNERS rule changes.

```
Show how code ownership changes between two git refs

Usage: codeowners diff [BASE_REF] [HEAD_REF]

Arguments:
  [BASE_REF]  Base ref to compare from [default: HEAD]
  [HEAD_REF]  Head ref to compare to (default: the working tree)

Options:
  -h, --help  Print help
```

**Examples:**

```
# Compare HEAD to the working tree (default)
codeowners diff

# Compare master to HEAD
codeowners diff master HEAD

# Compare two refs
codeowners diff master feature-branch
```

## Acknowledgements

The `diff` subcommand is inspired by [codeowners-diff](https://github.com/samueljsb/codeowners-diff).
