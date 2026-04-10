//! AI-powered API reverse engineering for SDK Forge.
//!
//! This crate uses Claude to analyze captured HTTP exchanges and produce
//! a structured API model. Analysis happens in multiple focused passes:
//! classification, schema inference, auth detection, and pagination detection.

pub mod api_model;
pub mod auth;
pub mod classifier;
pub mod claude;
pub mod error;
pub mod pagination;
pub mod schema;
