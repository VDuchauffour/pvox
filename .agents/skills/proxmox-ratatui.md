# Proxmox + Ratatui Skill

## Domain Context

This repo is a **terminal UI** for **Proxmox VE** clusters using **ratatui** (Rust TUI framework).

## Key Patterns

### Resource Identifiers

- VMs: `qemu/<vmid>` (e.g. `qemu/100`)
- Containers: `lxc/<vmid>` (e.g. `lxc/200`)
- Nodes: `node/<name>` (e.g. `node/pve1`)
- Storage: `storage/<name>`
- Parse numeric ID: `extract_vmid("qemu/100")` → `Some(100)`

### View System

- Internal views: `qemu`, `lxc`, `node`, `storage`
- Default view: `qemu`
- Command aliases: `vm`/`vms` → `qemu`, `ct`/`container`/`containers` → `lxc`, `node`/`nodes` → `node`, `storage`/`storages` → `storage`
- Tab completion available via `view_completion()` in `src/app/command.rs`

### Async Event Loop

- Uses `tokio::sync::mpsc::unbounded_channel<AppEvent>`
- Spawns:
  1. **Polling task** — fetches cluster resources every `refresh_interval` seconds
  2. **Tick task** — 33ms redraw cycle
  3. **Event task** — reads `crossterm::event::EventStream`
  4. **Version/whoami** — one-shot fetches on startup
- All events dispatched in `tokio::select!` in `src/lib.rs`

### Modal System

Modal variants in `src/app/modal.rs`:

- `Help` — keybindings overlay
- `Filter` — live text filter (`/`)
- `Command` — view switcher (`:`)
- `CommandError(String)` — invalid command feedback
- `Confirm(ConfirmAction)` — stop/reboot confirmation
- `Details` — resource detail panel with sparkline

Rendering: `ModalRenderer` trait in `src/ui/mod.rs` decides whether to show list + overlay or just overlay.

### Sparkline Data

- `SparkLineData` in `src/app/sparkline.rs` tracks CPU/memory history
- Cleared when entering details modal (`sparkline_data.clear()`)
- Populated from `ClusterResource` fields: `cpu`, `maxcpu`, `mem`, `maxmem`

### Lifecycle Actions

- `s` — start VM/container (no confirm)
- `S` — stop VM/container (confirm modal)
- `r` — reboot VM/container (confirm modal)
- Actions send `LifecycleAction` via mpsc, handled by spawned async task in `src/lib.rs`
- Task polling: `check_task_status()` every 2s until `Completed` or `Error`

### Keyboard Navigation

- `q` — quit
- `?` — help
- `/` — filter
- `:` — command
- `↑`/`k`, `↓`/`j` — navigate
- `gg` — top, `G` — bottom
- `Enter` — details
- `Esc` — cancel/close
- `Ctrl+C` — force quit (always works)

## Color Theming

- `Theme` from `src/theme.rs` adapts based on `ThemeKind`
- `ThemeKind::Default` — full palette
- `ThemeKind::NoColor` — monochrome (`--no-color` or config)
- Always use `Theme::from_no_color(app.config.no_color())` in render functions

## Common Pitfalls

- **Never** block the main `tokio::select!` loop — all I/O must be in spawned tasks
- **Always** use `tx.send()` from spawned tasks; the main loop only `rx.recv()`
- `crossterm` events must be read via `EventStream` (not `read()` in async)
- Terminal cleanup is handled by `Tui::Drop` + panic hook — don't add manual cleanup
- Config schema (`schema/config.schema.json`) must stay in sync with `FileConfig` struct
