//! CLI-specific error types.
//!
//! Add CLI-level [`Diagnostic`] variants here as the application grows.
//!
//! [`Diagnostic`]: miette::Diagnostic

use miette::Diagnostic;
use thiserror::Error;

/// CLI-level application error.
#[derive(Debug, Error, Diagnostic)]
#[non_exhaustive]
pub enum CliError {
    /// An unexpected internal error.
    #[error("internal error: {0}")]
    #[diagnostic(code(cli::internal), help("This is a bug; please file an issue"))]
    Internal(String),

    /// A command is not yet implemented.
    #[error("command not yet implemented: {command}")]
    #[diagnostic(code(cli::not_implemented), help("This feature is coming soon"))]
    NotImplemented {
        /// The command that was attempted.
        command: String,
    },
}

#[cfg(test)]
mod tests {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    #[test]
    fn test_cli_error_internal_display() {
        let err = CliError::Internal("something broke".to_owned());
        assert_eq!(
            err.to_string(),
            "internal error: something broke",
            "error message format must be stable"
        );
    }

    #[test]
    fn test_cli_error_not_implemented_display() {
        let err = CliError::NotImplemented {
            command: "record".to_owned(),
        };
        assert_eq!(
            err.to_string(),
            "command not yet implemented: record",
            "error message format must be stable"
        );
    }
}
