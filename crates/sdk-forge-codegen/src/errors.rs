//! SDK error enum generation context.
//!
//! Builds the template context for the generated `error.rs` file.
//! The error module is mostly static — the only variable is the
//! HTTP client crate name (`reqwest` or `rquest`).

use serde::Serialize;

/// Context for rendering `error.rs.tera`.
#[derive(Debug, Clone, Serialize)]
pub struct ErrorContext {
    /// HTTP client crate name (e.g. `"reqwest"` or `"rquest"`).
    pub http_client_crate: String,
}

/// Build the error template context.
pub fn build_error_context(http_client_crate: &str) -> ErrorContext {
    ErrorContext {
        http_client_crate: http_client_crate.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::build_error_context;

    #[test]
    fn test_default_error_context() {
        let ctx = build_error_context("reqwest");
        assert_eq!(ctx.http_client_crate, "reqwest");
    }

    #[test]
    fn test_rquest_error_context() {
        let ctx = build_error_context("rquest");
        assert_eq!(ctx.http_client_crate, "rquest");
    }
}
