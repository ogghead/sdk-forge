//! Rust SDK code generation for SDK Forge.
//!
//! This crate takes an analyzed API model and produces either an `OpenAPI` v3.0
//! specification or a complete, idiomatic Rust SDK crate. The core pipeline
//! converts [`ApiModel`] to [`openapiv3::OpenAPI`], which can then be serialized
//! directly or fed into progenitor for Rust code generation.

pub mod auth;
pub mod cargo_toml;
pub mod client;
pub mod emitter;
pub mod error;
pub mod errors;
pub mod formatter;
pub mod methods;
pub mod openapi;
pub mod types;
