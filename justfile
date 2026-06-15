_default:
    @just --list

pre-commit-install:
    uvx pre-commit install

build:
    cargo build

release:
    cargo build --release

run *args:
    cargo run -- {{args}}

run-release *args:
    cargo run --release -- {{args}}

test:
    cargo test --locked

lint:
    cargo clippy --all-targets --all-features

lint-strict:
    cargo clippy --all-targets --all-features --locked -- -D warnings

lint-strict-fix:
    cargo clippy --fix --allow-dirty --all-targets --all-features --locked -- -D warnings

fmt:
    cargo +nightly fmt

fmt-check:
    cargo +nightly fmt --check

check:
    cargo check

ci: fmt-check lint-strict test

ci-fix: fmt lint-strict-fix test

install:
    cargo install --path .

clean:
    cargo clean

update:
    cargo update

setup-proxmox:
    uv run --with proxmoxer  scripts/setup-proxmox.py
