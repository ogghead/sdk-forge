//! Shared session types and serialization for SDK Forge.
//!
//! This crate defines the core data structures used across all SDK Forge
//! components: the session file format, captured HTTP exchanges, and the
//! intermediate API model produced by analysis.

pub mod io;
pub mod redact;
pub mod types;

pub use types::{
    ApiModel, AuthPattern, CapturedRequest, CapturedResponse, Endpoint, EndpointClassification,
    Exchange, Initiator, PaginationPattern, PathParam, QueryParam, RequestBody, ResourceGroup,
    ResponseBody, Session, SessionMetadata, Timing,
};
