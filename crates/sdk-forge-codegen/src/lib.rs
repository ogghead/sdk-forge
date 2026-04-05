//! Rust SDK code generation for SDK Forge.
//!
//! This crate takes an analyzed API model and emits a complete, idiomatic
//! Rust crate with typed request/response structs, client methods, auth
//! handling, and error types. Code generation uses Tera templates for
//! deterministic output.

pub mod auth;
pub mod cargo_toml;
pub mod client;
pub mod emitter;
pub mod error;
pub mod errors;
pub mod formatter;
pub mod methods;
pub mod types;
