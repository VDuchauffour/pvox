# AGENTS.md

## Project

- **pvox** — a k9s-like terminal UI for Proxmox VE clusters, written in Rust (edition 2024).
- Single binary, no workspace. Entrypoint: `src/main.rs` (tokio async), core loop in `src/lib.rs`.

## Build & Run

- **Task runner**: `just` (justfile). Use `just --list` for all commands.
- **Run dev build**: `just run [-- <ARGS>]` (e.g. `just run -- --endpoint https://pve.local`)
- **Run tests**: `just test` (alias: `cargo test --locked`)
- **Full CI check**: `just ci` (runs `fmt-check`, `lint-strict`, `test` in order)
- **Auto-fix issues**: `just ci-fix` (runs `fmt`, `lint-strict-fix`, `test`)

## Lint & Format

- **Formatter**: requires **nightly** toolchain: `cargo +nightly fmt`
- Check formatting: `just fmt-check`
- **Linter**: `cargo clippy --all-targets --all-features`
- Strict lint: `just lint-strict` (treats warnings as errors)
- **Pre-commit hooks** run `just check` and `just ci-fix` via local hooks.
- rustfmt config: `reorder_imports = true`, `group_imports = "StdExternalCrate"`, `reorder_modules = true`

## Architecture

- `src/api/` — HTTP client for Proxmox VE (`ProxmoxClient`), types, error handling.
- `src/app/` — App state (`App`), input handling (`input.rs`), command parsing (`command.rs`), modal system (`modal.rs`), sparkline data (`sparkline.rs`).
- `src/ui/` — Ratatui rendering: table list, header, dialogs, overlays, help panel, layout helpers.
- `src/tui.rs` — Crossterm terminal setup (raw mode, alternate screen, panic hook cleanup).
- `src/config.rs` — CLI args (`clap`) + YAML file config. CLI overrides file values. Validates against embedded JSON schema.
- `src/event.rs` — Event types (`AppEvent`, `LifecycleAction`, `ConfirmAction`).
- `src/theme.rs` — Color theming (supports `--no-color` via `ThemeKind`).
- Config file: `~/.config/pvox/config.yaml`. Schema: `schema/config.schema.json` (embedded in binary and validated at load time).

## Key Conventions

- **Resource IDs**: `qemu/<vmid>` for VMs, `lxc/<vmid>` for containers, `node/<name>` for nodes. `extract_vmid()` parses numeric ID from `type/id`.
- **Views**: `qemu` (default), `lxc`, `node`, `storage`. Command mode (`:`) accepts aliases like `vm`, `ct`, `container`.
- **Async model**: tokio mpsc unbounded channel. Spawns: polling task (every `refresh_interval`), tick task (33ms), crossterm event task, version/whoami one-shots.
- **No workspace / no Cargo workspace members.** Single crate.

## Testing

- Unit tests are co-located in `src/...` files (no separate `tests/` directory). `#[cfg(test)]` blocks in `config.rs`, `app/mod.rs`, `ui/format.rs`, `api/client.rs`.
- Tests use `tempfile` for config files and `tokio::sync::mpsc::unbounded_channel` for mock event channels.
- Coverage: CI runs `cargo tarpaulin` (not installed by default; install via `cargo binstall cargo-tarpaulin`).

## CI / Release

- CI: `.github/workflows/ci.yml` — runs on PR and main push. Uses `dtolnay/rust-toolchain@stable` with nightly rustfmt/clippy.
- Release drafter auto-drafts on main push. PRs must follow conventional commits (semantic title validator).
- Publish to crates.io happens on GitHub release publish (`cargo publish`).
- Branch naming: `feature/*`, `fix/*`, `chore/*`, `dependencies/*` → auto-labeled.

## Devcontainer

- `.devcontainer/` has Rust/Node/Python setup. Post-create installs rustfmt, clippy, nightly toolchain, just, cargo-tarpaulin, pre-commit.
- `CARGO_TARGET_DIR` is set to `target` (devcontainer) and `target/rust-analyzer` (VS Code rust-analyzer settings).

## Common Gotchas

- `cargo fmt` alone will fail or behave differently; always use `cargo +nightly fmt`.
- `just ci` is the fastest way to verify a change before pushing.
- `cargo check` is available as `just check` for quick compilation checks.
- The `insecure` CLI flag is a tri-bool: `--insecure` (true), `--insecure=true` (true), `--insecure=false` (false), omitted (file default).
- No integration tests requiring a live Proxmox cluster; all API tests are unit tests with mock data.
- Do not delete `Cargo.lock` — this is a binary crate.
