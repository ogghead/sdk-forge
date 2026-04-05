//! Application error types.
//!
//! Add domain-specific [`Diagnostic`] variants here as the application grows.
//!
//! [`Diagnostic`]: miette::Diagnostic

use miette::Diagnostic;
use thiserror::Error;

/// Top-level application error.
#[derive(Debug, Error, Diagnostic)]
#[non_exhaustive]
pub enum AppError {
    /// An unexpected internal error.
    ///
    /// The inner string is a human-readable description of what went wrong.
    #[error("internal error: {0}")]
    #[diagnostic(code(app::internal), help("This is a bug; please file an issue"))]
    Internal(String),
}

#[cfg(test)]
mod tests {
    // wildcard_imports is denied crate-wide; allow here for idiomatic `use super::*` in tests.
    #[allow(clippy::wildcard_imports)]
    use super::*;

    #[test]
    fn test_app_error_internal_display() {
        let err = AppError::Internal("something broke".to_owned());
        assert_eq!(
            err.to_string(),
            "internal error: something broke",
            "error message format must be stable"
        );
    }
}
