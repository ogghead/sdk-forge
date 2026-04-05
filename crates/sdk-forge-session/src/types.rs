//! Core data types for SDK Forge sessions.
//!
//! These types represent captured browser interactions, inferred API models,
//! and all metadata needed to generate typed Rust SDKs.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Session ──────────────────────────────────────────────────────────

/// A recorded browser session containing captured HTTP exchanges
/// and optional analysis results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique session identifier.
    pub id: Uuid,
    /// When the session was created.
    pub created_at: DateTime<Utc>,
    /// The URL the user navigated to.
    pub target_url: String,
    /// Browser version used for recording.
    pub browser_version: String,
    /// Raw captured HTTP exchanges.
    pub exchanges: Vec<Exchange>,
    /// Populated after the analysis phase.
    pub api_model: Option<ApiModel>,
    /// Additional session metadata.
    pub metadata: SessionMetadata,
}

/// Metadata about the recording session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    /// Duration of the recording in milliseconds.
    pub duration_ms: u64,
    /// Number of exchanges before filtering.
    pub raw_exchange_count: u64,
    /// User-supplied notes about the session.
    pub notes: Option<String>,
}

// ── Exchange ─────────────────────────────────────────────────────────

/// A single captured HTTP request/response pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Exchange {
    /// Unique exchange identifier.
    pub id: Uuid,
    /// When the exchange occurred.
    pub timestamp: DateTime<Utc>,
    /// The captured HTTP request.
    pub request: CapturedRequest,
    /// The captured HTTP response.
    pub response: CapturedResponse,
    /// What initiated this request.
    pub initiator: Initiator,
    /// Request timing information.
    pub timing: Timing,
    /// Set during the analysis phase.
    pub classification: Option<EndpointClassification>,
}

/// An HTTP request captured from the browser.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturedRequest {
    /// HTTP method (GET, POST, etc.).
    pub method: String,
    /// Full request URL.
    pub url: String,
    /// Request headers.
    pub headers: HashMap<String, String>,
    /// Parsed query parameters.
    pub query_params: HashMap<String, String>,
    /// Request body, if present.
    pub body: Option<RequestBody>,
}

/// An HTTP response captured from the browser.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturedResponse {
    /// HTTP status code.
    pub status: u16,
    /// Response headers.
    pub headers: HashMap<String, String>,
    /// Response body, if captured.
    pub body: Option<ResponseBody>,
    /// Content-Type header value.
    pub content_type: Option<String>,
}

/// The body of an HTTP request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RequestBody {
    /// JSON request body.
    Json {
        /// The JSON value.
        value: serde_json::Value,
    },
    /// URL-encoded form data.
    FormUrlEncoded {
        /// Form field key-value pairs.
        fields: HashMap<String, String>,
    },
    /// Multipart form data.
    Multipart {
        /// Multipart form fields.
        fields: Vec<MultipartField>,
    },
    /// Raw binary body.
    Raw {
        /// Base64-encoded raw bytes.
        data: String,
    },
}

/// A single field in a multipart form request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultipartField {
    /// Field name.
    pub name: String,
    /// Field value (text content or filename for file uploads).
    pub value: String,
    /// Content type of this field, if specified.
    pub content_type: Option<String>,
    /// Whether this field is a file upload.
    pub is_file: bool,
}

/// The body of an HTTP response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ResponseBody {
    /// JSON response body.
    Json {
        /// The JSON value.
        value: serde_json::Value,
    },
    /// Plain text response body.
    Text {
        /// The text content.
        content: String,
    },
    /// Binary response body (not captured in detail).
    Binary {
        /// Size in bytes.
        size: u64,
    },
}

/// What initiated an HTTP request in the browser.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Initiator {
    /// `XMLHttpRequest`.
    Xhr,
    /// Fetch API.
    Fetch,
    /// Page navigation.
    Navigation,
    /// Script-initiated (e.g., dynamic import).
    Script,
    /// Other or unknown initiator.
    Other,
}

/// Timing information for an HTTP exchange.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timing {
    /// When the request started (ms since session start).
    pub start_ms: u64,
    /// Time to first byte (ms since request start).
    pub ttfb_ms: Option<u64>,
    /// Total duration (ms).
    pub duration_ms: u64,
}

/// Classification assigned to an endpoint during analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointClassification {
    /// The resource group this endpoint belongs to.
    pub resource_group: String,
    /// Whether this endpoint should be ignored (noise).
    pub is_noise: bool,
    /// Brief description of what this endpoint does.
    pub description: Option<String>,
}

// ── API Model (populated by analysis) ────────────────────────────────

/// The intermediate representation of a discovered API.
///
/// Built by the analyzer from captured exchanges, this model
/// drives code generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiModel {
    /// Base URL for the API (e.g., `https://api.example.com`).
    pub base_url: String,
    /// Detected authentication pattern, if any.
    pub auth: Option<AuthPattern>,
    /// Logical groupings of related endpoints.
    pub resources: Vec<ResourceGroup>,
    /// Detected pagination pattern, if any.
    pub pagination: Option<PaginationPattern>,
}

/// A group of related API endpoints (e.g., "Users", "Posts").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceGroup {
    /// Human-readable name (e.g., "Users").
    pub name: String,
    /// Common path prefix (e.g., "/api/v1/users").
    pub base_path: String,
    /// Individual endpoints in this group.
    pub endpoints: Vec<Endpoint>,
}

/// A single API endpoint with its inferred schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Endpoint {
    /// HTTP method.
    pub method: String,
    /// Path template with parameters (e.g., "/users/{id}").
    pub path_template: String,
    /// Human-readable description.
    pub description: String,
    /// Inferred request body schema.
    pub request_schema: Option<JsonSchema>,
    /// Inferred response body schema.
    pub response_schema: Option<JsonSchema>,
    /// Whether this endpoint requires authentication.
    pub requires_auth: bool,
    /// Path parameters.
    pub path_params: Vec<PathParam>,
    /// Query string parameters.
    pub query_params: Vec<QueryParam>,
}

/// A path parameter in a URL template.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathParam {
    /// Parameter name (e.g., "id").
    pub name: String,
    /// Inferred type (e.g., "string", "integer").
    pub param_type: String,
    /// Description of this parameter.
    pub description: Option<String>,
}

/// A query string parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryParam {
    /// Parameter name.
    pub name: String,
    /// Inferred type.
    pub param_type: String,
    /// Whether this parameter is required.
    pub required: bool,
    /// Description of this parameter.
    pub description: Option<String>,
}

/// A simplified JSON Schema representation for request/response bodies.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum JsonSchema {
    /// A JSON object with named fields.
    Object {
        /// Object properties.
        properties: HashMap<String, Self>,
        /// Which properties are required.
        required: Vec<String>,
    },
    /// A JSON array with a uniform element type.
    Array {
        /// Schema for array elements.
        items: Box<Self>,
    },
    /// A string value.
    String {
        /// Optional format hint (e.g., "date-time", "email").
        format: Option<String>,
    },
    /// A numeric value (integer).
    Integer,
    /// A numeric value (floating point).
    Number,
    /// A boolean value.
    Boolean,
    /// A null value.
    Null,
}

/// Detected authentication pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthPattern {
    /// Bearer token in Authorization header.
    BearerToken,
    /// API key in a header or query parameter.
    ApiKey {
        /// Where the key is sent.
        location: ApiKeyLocation,
        /// Header or parameter name.
        name: String,
    },
    /// Cookie-based authentication.
    Cookie {
        /// Name of the auth cookie.
        cookie_name: String,
    },
    /// Basic HTTP authentication.
    BasicAuth,
}

/// Where an API key is transmitted.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ApiKeyLocation {
    /// In an HTTP header.
    Header,
    /// In a query parameter.
    Query,
}

/// Detected pagination pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaginationPattern {
    /// Cursor-based pagination.
    Cursor {
        /// Name of the cursor parameter.
        cursor_param: String,
    },
    /// Offset/limit pagination.
    OffsetLimit {
        /// Name of the offset parameter.
        offset_param: String,
        /// Name of the limit parameter.
        limit_param: String,
    },
    /// Page number pagination.
    PageNumber {
        /// Name of the page parameter.
        page_param: String,
    },
    /// Link header (RFC 8288) pagination.
    LinkHeader,
}

#[cfg(test)]
mod tests {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    #[test]
    fn test_session_round_trip_json() {
        let session = Session {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            target_url: "https://example.com".to_owned(),
            browser_version: "Chromium 120".to_owned(),
            exchanges: Vec::new(),
            api_model: None,
            metadata: SessionMetadata {
                duration_ms: 1000,
                raw_exchange_count: 0,
                notes: None,
            },
        };

        let json = serde_json::to_string(&session);
        assert!(json.is_ok(), "session should serialize to JSON");

        let json_str = json.unwrap_or_default();
        let parsed: Result<Session, _> = serde_json::from_str(&json_str);
        assert!(parsed.is_ok(), "session should deserialize from JSON");
    }

    #[test]
    fn test_request_body_json_variant() {
        let body = RequestBody::Json {
            value: serde_json::json!({"key": "value"}),
        };
        let json = serde_json::to_string(&body);
        assert!(json.is_ok(), "RequestBody::Json should serialize");
    }

    #[test]
    fn test_initiator_equality() {
        assert_eq!(
            Initiator::Xhr,
            Initiator::Xhr,
            "same variants should be equal"
        );
        assert_ne!(
            Initiator::Xhr,
            Initiator::Fetch,
            "different variants should not be equal"
        );
    }

    #[test]
    fn test_json_schema_nested() {
        let schema = JsonSchema::Object {
            properties: HashMap::from([("name".to_owned(), JsonSchema::String { format: None })]),
            required: vec!["name".to_owned()],
        };
        let json = serde_json::to_string(&schema);
        assert!(json.is_ok(), "nested JsonSchema should serialize");
    }
}
