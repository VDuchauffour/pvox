# Rust CI Skill

## CI Pipeline

The CI runs on every PR and main push via `.github/workflows/ci.yml`.

### Required Order

CI jobs run in this order:

1. `just fmt-check` — formatting with nightly toolchain
2. `just lint-strict` — clippy with `-D warnings` (warnings = errors)
3. `just test` — `cargo test --locked`

### Local Verification

Before pushing, always run:

```bash
just ci
```

To auto-fix issues:

```bash
just ci-fix
```

### Pre-commit Hooks

`.pre-commit-config.yaml` runs:

- `just check` — quick `cargo check`
- `just ci-fix` — format + lint fix + test

Install hooks locally:

```bash
pre-commit install
```

## Toolchain Requirements

- **Stable** Rust for building and testing
- **Nightly** Rust for `rustfmt` only
- `cargo +nightly fmt` — the ONLY correct way to format
- `cargo fmt` will fail or produce different output

## Lint Configuration

- `rustfmt.toml`: `reorder_imports = true`, `group_imports = "StdExternalCrate"`, `reorder_modules = true`
- Clippy: `--all-targets --all-features`
- Strict mode: `-- -D warnings` (treats ALL warnings as errors)

## Release Process

1. PRs merged to `main` → release-drafter auto-drafts
2. PRs must follow **conventional commits** (enforced by `pr-enhancement.yml`)
3. GitHub release publish → triggers `publish.yml` → `cargo publish`

## Branch Naming

- `feature/*` → `enhancement` label
- `fix/*` → `bug` label
- `chore/*` → `chore` label
- `dependencies/*` → `dependencies` label

## Common Mistakes

- Using `cargo fmt` instead of `cargo +nightly fmt` — will break CI
- Missing `--locked` in tests — should match `just test`
- Forgetting that warnings are errors in CI — run `just lint-strict` locally
- Deleting `Cargo.lock` — this is a binary crate, lockfile must be committed
