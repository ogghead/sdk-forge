# {{project-name}}

<a href="https://github.com/ogghead/{{project-name}}/actions/workflows/ci.yml"><img src="https://github.com/ogghead/{{project-name}}/actions/workflows/ci.yml/badge.svg" alt="CI" /></a>
<a href="https://github.com/ogghead/{{project-name}}"><img src="https://img.shields.io/badge/rust-1.93%2B-orange.svg?logo=rust" alt="MSRV 1.93+" /></a>
<a href="https://github.com/ogghead/{{project-name}}/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License: MIT" /></a>

> {{description}}

## Prerequisites

- [Rust stable](https://rustup.rs/) (see `rust-toolchain.toml` for pinned version)

## Quick Start

```sh
cargo run
```

## Commands

| Command | Purpose |
|---------|---------|
| `cargo check` | Fast type-check |
| `cargo build` | Compile debug binary |
| `cargo build --release` | Compile optimized binary |
| `cargo test --doc && cargo nextest run` | Run all tests |
| `cargo clippy -- -D warnings` | Lint (warnings = errors) |
| `cargo fmt` | Format code |
| `cargo deny check` | Audit dependencies |
| `cargo llvm-cov nextest` | Run tests with coverage |

## Before Committing

Install the pre-commit hook (once, after cloning):

```sh
./scripts/install-hooks.sh
```

This runs formatting, linting, tests, and dependency auditing automatically on each commit.

You can also run the checks manually:

```sh
cargo fmt && cargo clippy -- -D warnings && cargo test --doc && cargo nextest run
```
