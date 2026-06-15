# Testing Patterns Skill

## Test Organization

- **No `tests/` directory** — all tests are co-located in `src/**/*.rs`
- Use `#[cfg(test)]` blocks at the bottom of the file they test
- Test files with tests: `config.rs`, `app/mod.rs`, `ui/format.rs`, `api/client.rs`

## Test Dependencies

- `tempfile` — for creating mock config files in `config.rs` tests
- `tokio::sync::mpsc::unbounded_channel` — for mocking the event channel in `app/mod.rs` tests
- `crossterm::event::{KeyCode, KeyEvent, KeyModifiers}` — for input simulation

## Common Test Patterns

### Config File Testing

```rust
use std::io::Write;
use tempfile::NamedTempFile;

let mut tmp = NamedTempFile::new().unwrap();
writeln!(tmp, "connection:\n  endpoint: https://pve.example.com").unwrap();

let args = Cli { config: Some(tmp.path().to_path_buf()), .. };
let cfg = args.load().unwrap();
```

### Event Channel Testing

```rust
let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();

// Trigger action
app.handle_key(key_event, &tx);

// Assert event sent
assert!(matches!(rx.try_recv(), Ok(AppEvent::LifecycleAction(_))));
```

### Mock Resources

```rust
fn mock_resource(name: &str, rtype: &str, node: Option<&str>) -> ClusterResource {
    ClusterResource {
        id: format!("{}/{}", rtype, name),
        r#type: rtype.to_string(),
        name: name.to_string(),
        node: node.map(|n| n.to_string()),
        status: "running".to_string(),
        cpu: None,
        maxcpu: None,
        mem: None,
        maxmem: None,
        disk: None,
        maxdisk: None,
        uptime: None,
    }
}
```

### Key Event Helpers

```rust
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn key(c: char) -> KeyEvent {
    KeyEvent::from(KeyCode::Char(c))
}

fn ctrl_c() -> KeyEvent {
    KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)
}
```

## Test Coverage

- CI runs `cargo tarpaulin` (install: `cargo binstall cargo-tarpaulin`)
- No integration tests requiring a live Proxmox cluster
- All API tests use mock data

## Writing Good Tests

- Test the **public API** (`App::handle_key`, `Config::load`, etc.)
- Test **edge cases**: empty resources, out-of-bounds indices, invalid input
- Test **modal transitions**: opening/closing, key dispatch with/without modals
- Test **filter behavior**: case-insensitive, partial matches, no matches
- Test **command resolution**: aliases, invalid commands, tab completion

## Running Tests

```bash
# All tests
just test

# Specific module
cargo test --locked app::

# Specific test
cargo test --locked test_filter_subset_by_name
```

## Testing Rules

- **Never** delete failing tests to make CI pass
- **Never** use `#[ignore]` without a documented reason
- Keep tests deterministic — no randomness, no timing-dependent assertions
- Use `assert!` + `matches!` for enum variant checks
- Use `unwrap_err()` for error path tests
