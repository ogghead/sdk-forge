//! SDK error type generation context.
//!
//! Builds the template context for the generated `error.rs` file.
//! In a WASI component, errors map to WIT `result` types with a
//! custom `api-error` variant type.

use serde::Serialize;

/// Context for rendering the error template.
#[derive(Debug, Clone, Serialize)]
pub struct ErrorContext {
    /// Package name for the generated component.
    pub package_name: String,
}

/// Build the error template context.
pub fn build_error_context(package_name: &str) -> ErrorContext {
    ErrorContext {
        package_name: package_name.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::build_error_context;

    #[test]
    fn test_error_context() {
        let ctx = build_error_context("my-api");
        assert_eq!(ctx.package_name, "my-api");
    }
}
