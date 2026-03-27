_default:
    @just --list

# Run clippy fixes
[group('fix')]
fix-lint:
    cargo clippy --all-targets --all-features --fix --allow-staged

# Run formatting fixes
[group('fix')]
fix-fmt:
    cargo fmt

# Run all fixes
[group('fix')]
fix: fix-lint fix-fmt

# Run tests
[group('check')]
check-test:
    cargo test --all-features

# Run clippy checks
[group('check')]
check-lint:
    cargo clippy --all-targets --all-features -- -D warnings

# Run formatting checks
[group('check')]
check-fmt:
    cargo fmt -- --check

# Run all checks
[group('check')]
check: check-test check-lint check-fmt
