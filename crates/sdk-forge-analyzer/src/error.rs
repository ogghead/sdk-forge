//! Analyzer error types.

use miette::Diagnostic;
use thiserror::Error;

/// Errors that can occur during API analysis.
#[derive(Debug, Error, Diagnostic)]
#[non_exhaustive]
pub enum AnalyzerError {
    /// Failed to communicate with the Claude API.
    #[error("Claude API error: {reason}")]
    #[diagnostic(code(analyzer::api), help("Check your API key and network connection"))]
    ApiError {
        /// Description of what went wrong.
        reason: String,
    },

    /// The analysis response could not be parsed.
    #[error("failed to parse analysis response: {reason}")]
    #[diagnostic(code(analyzer::parse))]
    ParseError {
        /// Description of what went wrong.
        reason: String,
    },

    /// The session has no exchanges to analyze.
    #[error("session contains no exchanges to analyze")]
    #[diagnostic(
        code(analyzer::empty_session),
        help("Record a session first with `sdk-forge record`")
    )]
    EmptySession,
}

#[cfg(test)]
mod tests {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    #[test]
    fn test_api_error_display() {
        let err = AnalyzerError::ApiError {
            reason: "rate limited".to_owned(),
        };
        assert_eq!(
            err.to_string(),
            "Claude API error: rate limited",
            "error message format must be stable"
        );
    }

    #[test]
    fn test_empty_session_display() {
        let err = AnalyzerError::EmptySession;
        assert_eq!(
            err.to_string(),
            "session contains no exchanges to analyze",
            "error message format must be stable"
        );
    }
}
