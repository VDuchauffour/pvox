# Metron

A k9s-like terminal UI for Proxmox VE clusters.

## Features

- Real-time resource list (nodes, VMs, LXC, storage)
- Lifecycle actions: start, stop, reboot
- Color-coded status indicators
- Keyboard-driven navigation
- Live filter search
- Resource detail views

## Installation

```bash
cargo install --path .
```

## Usage

```bash
metron --host https://pve.local --token-id root@pam!metron --token abc123 --insecure
```

## Key Bindings

- `q` — Quit
- `?` — Help
- `/` — Filter
- `↑/↓` — Navigate
- `Enter` — View details
- `s` — Start VM/CT
- `S` — Stop VM/CT (confirm)
- `r` — Reboot VM/CT (confirm)
- `Esc` — Close modal / Cancel

## Configuration

Create `~/.metron/config.yaml`:

```yaml
host: https://pve.local
token_id: root@pam!metron
token: abc123
insecure: true
refresh_interval: 5
no_color: false
```

## CLI Flags

```bash
metron --help
```

## Environment

- `METRON_TOKEN` — API token secret fallback
