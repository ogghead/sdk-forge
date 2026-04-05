//! HTTP client context generation for templates.
//!
//! Builds the context for `client.rs.tera` — the generated SDK's main
//! client struct, `AuthStrategy` trait, and default auth implementations.

use serde::Serialize;

use crate::auth::AuthInfo;

/// Context for rendering `client.rs.tera`.
#[derive(Debug, Clone, Serialize)]
pub struct ClientContext {
    /// Base URL of the API.
    pub base_url: String,
    /// HTTP client crate name (`"rquest"`).
    pub http_client_crate: String,
    /// Auth information, if auth was detected.
    pub auth: Option<AuthInfo>,
    /// Whether any endpoint requires auth.
    pub has_auth: bool,
}

/// Build the client template context.
pub fn build_client_context(
    base_url: &str,
    http_client_crate: &str,
    auth: Option<AuthInfo>,
) -> ClientContext {
    let has_auth = auth.is_some();
    ClientContext {
        base_url: base_url.to_owned(),
        http_client_crate: http_client_crate.to_owned(),
        auth,
        has_auth,
    }
}

#[cfg(test)]
mod tests {
    use crate::auth::build_auth_info;

    use super::build_client_context;

    #[test]
    fn test_client_context_with_auth() {
        let auth = build_auth_info(&Some(sdk_forge_session::types::AuthPattern::BearerToken));
        let ctx = build_client_context("https://api.example.com", "rquest", auth);

        assert!(ctx.has_auth, "should have auth");
        assert!(ctx.auth.is_some());
        assert_eq!(ctx.base_url, "https://api.example.com");
    }

    #[test]
    fn test_client_context_without_auth() {
        let ctx = build_client_context("https://api.example.com", "rquest", None);
        assert!(!ctx.has_auth, "should not have auth");
        assert!(ctx.auth.is_none());
    }
}
