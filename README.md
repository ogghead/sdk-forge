# SDK Forge

<a href="https://github.com/ogghead/sdk-forge/actions/workflows/ci.yml"><img src="https://github.com/ogghead/sdk-forge/actions/workflows/ci.yml/badge.svg" alt="CI" /></a>
<a href="https://github.com/ogghead/sdk-forge"><img src="https://img.shields.io/badge/rust-1.93%2B-orange.svg?logo=rust" alt="MSRV 1.93+" /></a>
<a href="https://github.com/ogghead/sdk-forge/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License: MIT" /></a>

Record browser interactions, reverse-engineer APIs via Claude, and generate typed Rust SDKs.

SDK Forge watches you use a website, captures the HTTP traffic, uses Claude to figure out the API, and spits out a working Rust crate with typed request/response structs, async endpoint methods, and auth handling baked in.

## Usage

### 1. Record a session

Open an instrumented browser, interact with the site, then close the browser to save:

```sh
sdk-forge record https://api.example.com
```

This captures every HTTP exchange into a `.sdkforge` session file.

Options:

```
sdk-forge record <URL> [-o <path>]

  <URL>          Site to open in the instrumented browser
  -o, --output   Path for the session file (default: auto-generated)
```

### 2. Analyze the API

Feed the recorded session to Claude, which reverse-engineers the API structure:

```sh
sdk-forge analyze session.sdkforge
```

Claude identifies endpoints, groups them into resources, infers request/response schemas, detects auth patterns, and figures out pagination.

Options:

```
sdk-forge analyze <SESSION> [--model <model>]

  <SESSION>     Path to a .sdkforge file
  --model       Claude model to use (default: claude-sonnet-4-20250514)
```

### 3. Generate the SDK

Turn the analyzed session into a complete Rust crate:

```sh
sdk-forge generate session.sdkforge -o my-sdk
```

This creates a ready-to-use crate with:

- Typed structs for every request and response body
- Async methods for each discovered endpoint
- Auth handling (Bearer, API key, cookie, or basic auth)
- [`rquest`](https://github.com/penumbra-zone/rquest) as the HTTP client (TLS fingerprinting, browser impersonation)

Options:

```
sdk-forge generate <SESSION> [-o <dir>] [-n <name>] [--check]

  <SESSION>       Path to an analyzed .sdkforge file
  -o, --output    Output directory (default: output/)
  -n, --name      Crate name (default: derived from target URL)
  --check         Run cargo check on the generated crate
```

### 4. Inspect a session

View what was captured without running analysis:

```sh
sdk-forge inspect session.sdkforge --endpoints
```

Options:

```
sdk-forge inspect <SESSION> [--endpoints]

  <SESSION>       Path to a .sdkforge file
  --endpoints     Show only endpoint summaries (URLs, methods, status codes)
```

## Example

```sh
# Record yourself using an API
sdk-forge record https://jsonplaceholder.typicode.com

# Claude analyzes the traffic and builds an API model
sdk-forge analyze recording.sdkforge

# Generate a typed Rust SDK
sdk-forge generate recording.sdkforge -n jsonplaceholder-sdk -o .

# Use it
cd jsonplaceholder-sdk
cargo check
```

The generated crate looks like this:

```
jsonplaceholder-sdk/
  Cargo.toml          # rquest, serde, tokio, thiserror
  src/
    lib.rs            # Module declarations
    types.rs          # pub struct Post { id: i64, title: String, ... }
    client.rs         # ApiClient, AuthStrategy trait, BearerAuth, etc.
    endpoints.rs      # impl ApiClient { async fn list_posts(...) -> ... }
    error.rs          # ApiError enum
```

Using the generated SDK:

```rust
use jsonplaceholder_sdk::{ApiClient, NoAuth};

#[tokio::main]
async fn main() {
    let client = ApiClient::new(NoAuth);
    let posts = client.list_posts(None).await.unwrap();
    println!("{posts:?}");
}
```

For APIs that require auth:

```rust
use jsonplaceholder_sdk::{ApiClient, BearerAuth};

let client = ApiClient::new(BearerAuth::new("sk-..."));
```

## Install

```sh
cargo install --path crates/sdk-forge-cli
```

Or build from source:

```sh
cargo build --release
# Binary is at target/release/sdk-forge
```

## How it works

```
Browser ──capture──► .sdkforge file ──Claude──► API model ──codegen──► Rust crate
         (HAR-like)                  (analyze)  (JSON)      (Tera)     (src/*.rs)
```

1. **Record** — launches a Chromium browser via CDP, intercepts all network traffic, filters out static assets and analytics, saves raw HTTP exchanges
2. **Analyze** — sends the captured exchanges to Claude, which classifies endpoints, infers JSON schemas, detects auth patterns (Bearer, API key, cookie, basic), and identifies pagination
3. **Generate** — converts the API model into Rust IR types (`RustStruct`, `EndpointMethod`), renders Tera templates, formats with `rustfmt`, writes a complete crate to disk

## Architecture

```
sdk-forge-cli           CLI entry point (clap)
  ├── sdk-forge-session   Shared types + .sdkforge file format
  ├── sdk-forge-recorder  Browser orchestration + traffic capture
  ├── sdk-forge-analyzer  Claude-powered API reverse engineering
  └── sdk-forge-codegen   Rust SDK code generation (Tera templates)
```

## Development

Prerequisites: [Rust stable](https://rustup.rs/) (1.93+)

```sh
# Run all checks (what CI does)
cargo fmt && cargo clippy -- -D warnings && cargo nextest run

# Run tests for a specific crate
cargo nextest run -p sdk-forge-codegen

# Coverage (90% threshold enforced in CI)
cargo llvm-cov nextest --fail-under-lines 90
```

## License

MIT
