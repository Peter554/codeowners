# codeowners

CLI tools for working with GitHub CODEOWNERS files.

## owners

Show the owners for one or more paths, based on the working tree CODEOWNERS.

```
codeowners owners src/main.rs src/lib.rs
```

## explain

Explain the CODEOWNERS assignment for a path. Shows all matching rules, which ones were superseded, and which rule is active.

```
codeowners explain src/main.rs
```

## diff

Show how code ownership changes between two git refs. Reports added files, removed files, and files whose ownership changed due to CODEOWNERS rule changes.

```
# Compare HEAD to the working tree (default)
codeowners diff

# Compare two refs
codeowners diff main feature-branch
```

## Acknowledgements

The `diff` subcommand is inspired by [codeowners-diff](https://github.com/samueljsb/codeowners-diff).
