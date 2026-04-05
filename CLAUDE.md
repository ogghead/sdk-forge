# SDK Forge

Rust workspace (edition 2024, MSRV 1.93, stable channel).

Record web interactions, reverse-engineer APIs via Claude, and generate typed Rust SDKs.

## Commands

| Command | Purpose |
|---------|---------|
| `cargo check` | Fast type-checking (use first, before build) |
| `cargo build` | Compile debug binary |
| `cargo build --release` | Compile optimized binary |
| `cargo run -p sdk-forge-cli` | Build and run the CLI |
| `cargo nextest run` | Run unit/integration tests (preferred) |
| `cargo nextest run -p sdk-forge-session` | Run tests for a specific crate |
| `cargo nextest run -E 'test(name)'` | Run a specific test |
| `cargo test --doc` | Run doc tests (nextest can't run these) |
| `cargo clippy -- -D warnings` | Lint (warnings = errors) |
| `cargo fmt` | Format code |
| `cargo fmt -- --check` | Check formatting without modifying |
| `cargo doc --open` | Build and open docs |
| `cargo deny check` | Audit dependencies (licenses, advisories, bans) |
| `cargo machete` | Detect unused dependencies |
| `cargo llvm-cov nextest` | Run tests with coverage (text summary) |
| `cargo llvm-cov nextest --html --open` | Coverage report in browser |
| `cargo llvm-cov nextest --lcov --output-path lcov.info` | Generate LCOV for editors |
| `cargo llvm-cov nextest --fail-under-lines 90` | Enforce coverage threshold (matches CI) |
| `cargo llvm-cov clean` | Remove coverage artifacts |

## Workflow

Before committing, always run: `cargo fmt && cargo clippy -- -D warnings && cargo test`

A pre-commit hook automates this (format check, clippy, tests, and optionally `cargo deny check`). It is installed automatically by the Claude Code session start hook.

Coverage threshold is 90% line coverage, enforced in CI. Install `cargo-llvm-cov` to check locally (the `llvm-tools` component is already in `rust-toolchain.toml`).

## Architecture

```
Cargo.toml                       # Workspace root (shared deps, lints, profiles)
crates/
  sdk-forge-cli/                  # Binary crate — CLI entry point (clap)
    src/
      main.rs                     # Thin shim — calls lib::run()
      lib.rs                      # CLI initialization and command dispatch
      error.rs                    # CLI-specific error types
      config.rs                   # Configuration loading
      commands/
        mod.rs                    # CLI command definitions (clap derive)
        record.rs                 # `sdk-forge record <url>`
        analyze.rs                # `sdk-forge analyze <session>`
        generate.rs               # `sdk-forge generate <session>`
        inspect.rs                # `sdk-forge inspect <session>`
    tests/
      integration_test.rs         # CLI integration tests

  sdk-forge-session/              # Shared types — session file format
    src/
      lib.rs                      # Re-exports
      types.rs                    # Session, Exchange, ApiModel, etc.
      io.rs                       # Read/write .sdkforge files
      redact.rs                   # Redact sensitive data before analysis

  sdk-forge-recorder/             # Browser orchestration + traffic capture
    src/
      lib.rs                      # Re-exports
      browser.rs                  # CDP browser lifecycle
      interceptor.rs              # Network interception
      har.rs                      # HAR format support
      session.rs                  # Recording session management
      filters.rs                  # Filter noise from API traffic
      console.rs                  # Browser console capture
      error.rs                    # Recorder error types

  sdk-forge-analyzer/             # AI-powered API reverse engineering
    src/
      lib.rs                      # Re-exports
      claude.rs                   # Anthropic API client
      classifier.rs               # Endpoint classification
      schema.rs                   # Schema inference
      auth.rs                     # Auth pattern detection
      pagination.rs               # Pagination detection
      api_model.rs                # API model construction
      error.rs                    # Analyzer error types

  sdk-forge-codegen/              # Rust SDK code generation
    src/
      lib.rs                      # Re-exports
      emitter.rs                  # Crate generation orchestrator
      types.rs                    # Struct generation from schemas
      client.rs                   # HTTP client wrapper generation
      methods.rs                  # Per-endpoint method generation
      errors.rs                   # Error enum generation
      auth.rs                     # Auth module generation
      cargo_toml.rs               # Cargo.toml generation
      formatter.rs                # rustfmt integration
      error.rs                    # Codegen error types

prompts/                          # Claude prompt templates (version controlled)
templates/                        # Tera templates for code generation
tests/
  fixtures/                       # Recorded sessions for deterministic tests
scripts/
  pre-commit                      # Pre-commit hook (fmt, clippy, test, deny)
.claude/
  settings.json                   # Claude Code session hooks
  scripts/
    setup.sh                      # Auto-installs cargo tools at session start
.github/
  workflows/
    ci.yml                        # Main CI: lint, test, MSRV, deny, coverage
  dependabot.yml                  # Weekly updates for cargo & actions
```

### Dependency Graph

```
sdk-forge-cli
  ├── sdk-forge-session
  ├── sdk-forge-recorder ──► sdk-forge-session
  ├── sdk-forge-analyzer ──► sdk-forge-session
  └── sdk-forge-codegen  ──► sdk-forge-session
```

`main.rs` is a one-line shim that calls `sdk_forge_cli::run()`. All initialization and application logic lives in the library crate so it can be tested and covered.

Use file-per-module (mod.rs is legacy). Prefer `foo.rs` for flat modules and `foo/` directory with named files for nested modules.

## Dependencies

### Shared (workspace-level)
- `miette` (v7, features: `fancy`) — Pretty diagnostic error reporting
- `thiserror` (v2) — Derive `Error` trait
- `tracing` (v0.1) — Structured logging (replaces println/eprintln)
- `tracing-subscriber` (v0.3, features: `env-filter`) — Log filtering via `RUST_LOG`
- `serde` (v1, features: `derive`) + `serde_json` (v1) — Serialization
- `tokio` (v1, features: `full`) — Async runtime
- `reqwest` (v0.12, features: `json`) — HTTP client
- `clap` (v4, features: `derive`) — CLI framework
- `uuid` (v1, features: `v4`, `serde`) — Session/exchange IDs
- `chrono` (v0.4, features: `serde`) — Timestamps
- `tera` (v1) — Code generation templates

### Dev
- `pretty_assertions` (v1) — Colored diff output for `assert_eq!`
- `insta` (v1, features: `yaml`) — Snapshot testing

## Lint Configuration

Lints are configured in workspace root `Cargo.toml` under `[workspace.lints]`. Key policies:

- **Unsafe code**: `forbid` — zero unsafe code allowed
- **Clippy baseline**: `all`, `pedantic`, `nursery` at warn level
- **Restriction lints at deny level**:
  - Panic prevention: `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`, `indexing_slicing`
  - Debug artifacts: `dbg_macro`, `print_stdout`, `print_stderr`, `use_debug`
  - Shadowing: `shadow_reuse`, `shadow_same`, `shadow_unrelated`
  - Type safety: `as_conversions`, `lossy_float_literal`, `arithmetic_side_effects`
  - Documentation: `missing_docs_in_private_items`
  - Many more (see `Cargo.toml` for full list)
- **Allowed relaxations**: `module_name_repetitions`, `must_use_candidate`, `missing_errors_doc`, `missing_panics_doc`, `option_if_let_else`, `tests_outside_test_module`
- **Clippy thresholds** (in `clippy.toml`): cognitive-complexity = 20, type-complexity = 250

## CI Pipeline

CI runs on push to main and pull requests with 5 parallel jobs:

1. **Lint** — `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo machete`
2. **Test** — `cargo test --doc` (doc tests) + `cargo nextest run` (unit/integration)
3. **MSRV** — Verifies compilation on Rust 1.93
4. **Deny** — `cargo deny check` (license compliance, security advisories; see `deny.toml` for allowed licenses)
5. **Coverage** — `cargo llvm-cov nextest --fail-under-lines 90` (enforces 90% threshold)

## Error Handling

- Use `miette` for all error handling: `#[derive(Diagnostic, Error)]` for library error types, `miette::Result` for binary entry points
- Attach `#[diagnostic(code(app::error_kind), help("..."))]` to give users actionable hints
- Add `#[source_code]` + `#[label("...")]` fields when errors relate to source text
- Always use `?` operator for propagation; `.into_diagnostic()` adapts foreign errors into `miette::Report`
- NEVER use `.unwrap()`, `.expect()`, or `[]` indexing — these are denied by lint
- Use `.ok_or_else(|| miette!("..."))` to convert `Option` to `miette::Result`
- For output, use the `tracing` crate — `println!`/`eprintln!` are forbidden
- Define `Diagnostic` enums per crate in `error.rs`; re-export from `lib.rs` as needed

## Testing

- Unit tests: `#[cfg(test)] mod tests { use super::*; }` at bottom of each file
- Integration tests: `crates/<crate>/tests/` directory per crate
- Name tests descriptively: `test_parse_returns_error_on_empty_input`
- Use `assert_eq!` with context: `assert_eq!(result, expected, "failed for input: {input}")`
- Use `pretty_assertions` for complex comparisons, `insta` for snapshot testing
- Test error cases, not just happy paths

## Gotchas

- `cargo clippy` and `cargo check` share the build cache; running one after the other is fast
- `cargo test` compiles separately from `cargo build` (different cfg); first test run after build changes is slow
- Coverage (`cargo llvm-cov`) re-compiles with instrumentation; first run is slower than plain `cargo test`
- Restriction lints use `deny` not `forbid` because some derive macros (e.g. clap) emit `#[allow(clippy::restriction)]` which is incompatible with `forbid`
- This is a workspace — `cargo check -p <crate>` checks a single crate, plain `cargo check` checks all
