# rust-starter

A Rust application starter template with strict lints, miette errors, tracing, CI, and coverage.

## What's Included

- **Error handling**: `miette` + `thiserror` with diagnostic codes and actionable help messages
- **Logging**: `tracing` + `tracing-subscriber` with `RUST_LOG` env filter
- **Strict lints**: Clippy pedantic + nursery + custom restriction lints (forbid unwrap/expect/panic/print/dbg/unsafe)
- **CI pipeline**: Format, clippy, tests (nextest), MSRV check, dependency audit (`cargo-deny`), unused dep detection (`cargo-machete`), code coverage (`cargo-llvm-cov`)
- **Coverage**: 60% threshold enforced in CI, HTML reports as artifacts
- **Profiles**: Optimized release (`lto`, `strip`, `panic=abort`) and dev (`split-debuginfo`)
- **Testing**: `pretty_assertions` + `insta` snapshot testing
- **Edition 2024** with MSRV 1.93

## Usage

### With cargo-generate

```sh
cargo install cargo-generate
cargo generate --git https://github.com/ogghead/rust-starter --name my-project
```

### With GitHub "Use this template"

1. Click **Use this template** on the GitHub repo page
2. Clone your new repo
3. Find-and-replace `{{project-name}}` with your project name (kebab-case)
4. Find-and-replace `{{crate_name}}` with your crate name (snake_case)
5. Find-and-replace `{{description}}` with your project description
6. Delete `cargo-generate.toml`, `TEMPLATE.md`, and `.github/workflows/template-test.yml`
7. Run `cargo generate-lockfile`

## After Generation

```sh
cd my-project
cargo fmt && cargo clippy -- -D warnings && cargo nextest run
```

See `CLAUDE.md` for full development conventions and `README.md` for project-specific docs.
