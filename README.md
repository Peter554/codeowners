# codeowners

CLI tools for working with GitHub CODEOWNERS files.

## owners

Show the owners for one or more paths, based on the working tree CODEOWNERS.

```
codeowners owners src/main.rs src/lib.rs
```

## diff

Show how code ownership changes between two git refs. Reports added files, removed files, and files whose ownership changed due to CODEOWNERS rule changes.

```
# Compare HEAD to the working tree (default)
codeowners diff

# Compare two refs
codeowners diff main feature-branch
```
