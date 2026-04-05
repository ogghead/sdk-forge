//! Auth context generation for templates.
//!
//! Produces data that drives the `client.rs.tera` template to generate
//! working auth implementations — not just a trait. The generated SDK
//! includes a default auth struct that handles the detected pattern
//! (bearer, API key, cookie, basic) out of the box.

use sdk_forge_session::types::{ApiKeyLocation, AuthPattern};
use serde::Serialize;

/// Auth information passed to Tera templates.
#[derive(Debug, Clone, Serialize)]
pub struct AuthInfo {
    /// Machine-readable variant name (e.g. `"bearer"`, `"api_key"`, `"cookie"`, `"basic"`).
    pub variant: String,
    /// Human-readable description for doc comments.
    pub description: String,
    /// Name of the generated default auth struct (e.g. `"BearerAuth"`).
    pub struct_name: String,
    /// Fields for the generated auth struct.
    pub fields: Vec<AuthField>,
    /// Where the credential is injected (e.g. `"header"`, `"query"`, `"cookie"`).
    pub location: String,
    /// Header/param/cookie name used for injection (e.g. `"Authorization"`, `"X-API-Key"`).
    pub credential_name: String,
}

/// A field on the generated auth struct.
#[derive(Debug, Clone, Serialize)]
pub struct AuthField {
    /// Field name.
    pub name: String,
    /// Rust type.
    pub rust_type: String,
    /// Doc comment.
    pub doc_comment: String,
}

/// Build auth template context from a detected [`AuthPattern`].
///
/// Returns `None` if no auth pattern was detected.
pub fn build_auth_info(pattern: &Option<AuthPattern>) -> Option<AuthInfo> {
    let pat = pattern.as_ref()?;

    let info = match pat {
        AuthPattern::BearerToken => AuthInfo {
            variant: "bearer".to_owned(),
            description: "Bearer token authentication".to_owned(),
            struct_name: "BearerAuth".to_owned(),
            fields: vec![AuthField {
                name: "token".to_owned(),
                rust_type: "String".to_owned(),
                doc_comment: "The bearer token value.".to_owned(),
            }],
            location: "header".to_owned(),
            credential_name: "Authorization".to_owned(),
        },
        AuthPattern::ApiKey { location, name } => {
            let loc_str = match location {
                ApiKeyLocation::Header => "header",
                ApiKeyLocation::Query => "query",
            };
            AuthInfo {
                variant: "api_key".to_owned(),
                description: format!("API key authentication via {loc_str} `{name}`"),
                struct_name: "ApiKeyAuth".to_owned(),
                fields: vec![AuthField {
                    name: "key".to_owned(),
                    rust_type: "String".to_owned(),
                    doc_comment: "The API key value.".to_owned(),
                }],
                location: loc_str.to_owned(),
                credential_name: name.clone(),
            }
        }
        AuthPattern::Cookie { cookie_name } => AuthInfo {
            variant: "cookie".to_owned(),
            description: format!("Cookie-based authentication using `{cookie_name}`"),
            struct_name: "CookieAuth".to_owned(),
            fields: vec![AuthField {
                name: "value".to_owned(),
                rust_type: "String".to_owned(),
                doc_comment: format!("The `{cookie_name}` cookie value."),
            }],
            location: "cookie".to_owned(),
            credential_name: cookie_name.clone(),
        },
        AuthPattern::BasicAuth => AuthInfo {
            variant: "basic".to_owned(),
            description: "HTTP Basic authentication".to_owned(),
            struct_name: "BasicAuth".to_owned(),
            fields: vec![
                AuthField {
                    name: "username".to_owned(),
                    rust_type: "String".to_owned(),
                    doc_comment: "The username.".to_owned(),
                },
                AuthField {
                    name: "password".to_owned(),
                    rust_type: "String".to_owned(),
                    doc_comment: "The password.".to_owned(),
                },
            ],
            location: "header".to_owned(),
            credential_name: "Authorization".to_owned(),
        },
    };

    Some(info)
}

#[cfg(test)]
mod tests {
    use sdk_forge_session::types::{ApiKeyLocation, AuthPattern};

    use super::build_auth_info;

    #[test]
    fn test_bearer_auth_info() {
        let info = build_auth_info(&Some(AuthPattern::BearerToken));
        assert!(info.is_some());

        let auth = info.unwrap_or_else(|| unreachable!());
        assert_eq!(auth.variant, "bearer");
        assert_eq!(auth.struct_name, "BearerAuth");
        assert_eq!(auth.fields.len(), 1, "bearer has one field: token");
        assert_eq!(auth.location, "header");
    }

    #[test]
    fn test_api_key_header_info() {
        let info = build_auth_info(&Some(AuthPattern::ApiKey {
            location: ApiKeyLocation::Header,
            name: "X-API-Key".to_owned(),
        }));
        assert!(info.is_some());

        let auth = info.unwrap_or_else(|| unreachable!());
        assert_eq!(auth.variant, "api_key");
        assert_eq!(auth.struct_name, "ApiKeyAuth");
        assert_eq!(auth.location, "header");
        assert_eq!(auth.credential_name, "X-API-Key");
    }

    #[test]
    fn test_api_key_query_info() {
        let info = build_auth_info(&Some(AuthPattern::ApiKey {
            location: ApiKeyLocation::Query,
            name: "api_key".to_owned(),
        }));
        assert!(info.is_some());

        let auth = info.unwrap_or_else(|| unreachable!());
        assert_eq!(auth.location, "query");
        assert_eq!(auth.credential_name, "api_key");
    }

    #[test]
    fn test_cookie_auth_info() {
        let info = build_auth_info(&Some(AuthPattern::Cookie {
            cookie_name: "session_id".to_owned(),
        }));
        assert!(info.is_some());

        let auth = info.unwrap_or_else(|| unreachable!());
        assert_eq!(auth.variant, "cookie");
        assert_eq!(auth.struct_name, "CookieAuth");
        assert_eq!(auth.credential_name, "session_id");
    }

    #[test]
    fn test_basic_auth_info() {
        let info = build_auth_info(&Some(AuthPattern::BasicAuth));
        assert!(info.is_some());

        let auth = info.unwrap_or_else(|| unreachable!());
        assert_eq!(auth.variant, "basic");
        assert_eq!(auth.struct_name, "BasicAuth");
        assert_eq!(auth.fields.len(), 2, "basic auth has username + password");
    }

    #[test]
    fn test_no_auth() {
        let info = build_auth_info(&None);
        assert!(info.is_none());
    }
}
