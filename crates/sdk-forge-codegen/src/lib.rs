//! Wasm component code generation for SDK Forge.
//!
//! This crate takes an analyzed [`ApiModel`](sdk_forge_session::types::ApiModel)
//! and generates a complete Wasm component crate: a WIT spec describing the API
//! and a Rust implementation using `wasi:http/outgoing-handler` for HTTP requests.
//!
//! An optional [`openapi`] module can produce an `OpenAPI` v3.0 spec for
//! documentation purposes, but it is **not** part of the primary codegen path.

pub mod auth;
pub mod cargo_toml;
pub mod client;
pub mod emitter;
pub mod error;
pub mod errors;
pub mod formatter;
pub mod methods;
pub mod naming;
pub mod openapi;
pub mod types;
