//! Browser orchestration and traffic capture for SDK Forge.
//!
//! This crate manages the browser lifecycle via CDP (Chrome `DevTools` Protocol),
//! intercepts network traffic, filters out noise, and produces captured
//! exchanges for the session file.

pub mod browser;
pub mod console;
pub mod error;
pub mod filters;
pub mod har;
pub mod interceptor;
pub mod session;
