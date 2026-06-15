# p9s

A k9s-like terminal UI for Proxmox VE clusters.

## Features

- Real-time cluster resource view (nodes, VMs, LXC, storage)
- Start, stop, and reboot actions with confirmation prompts
- CPU/memory sparkline history in detail view
- Color-coded status indicators (with `--no-color` support)
- Keyboard-driven navigation with filter search
- Async task tracking for lifecycle operations

## Install

```bash
cargo install p9s
```

## Configuration

p9s reads `~/.config/p9s/config.yaml`. CLI flags override file values:

```yaml
connection:
  endpoint: https://pve.local
  token_id: root@pam!p9s
  secret: abc123
  insecure: true
ui:
  theme: default
refresh_interval: 5
```

### CLI Flags

```
Usage: p9s [OPTIONS]

Options:
      --endpoint <ENDPOINT>  Proxmox endpoint URL
      --token-id <TOKEN_ID>  API token ID (e.g. root@pam!p9s)
      --secret <SECRET>        API token secret
      --insecure [<INSECURE>]  Allow insecure HTTPS (self-signed certs) [possible values: true, false]
      --config <CONFIG>      Path to config file
  -h, --help                 Print help


```

### Development

To ensure that you follow the development workflow, please setup the pre-commit hooks:

```sh
just pre-commit-install
```

> **Note:** This requires [`uv`](https://github.com/astral-sh/uv) to be installed, as the hooks are run via `uvx pre-commit`.

Common tasks:

```sh
just      # list all recipes
just run  # cargo run
just test # cargo test
just ci   # fmt-check + lint-strict + test
```
