//! Recorder error types.

use miette::Diagnostic;
use thiserror::Error;

/// Errors that can occur during browser recording.
#[derive(Debug, Error, Diagnostic)]
#[non_exhaustive]
pub enum RecorderError {
    /// Failed to launch or connect to the browser.
    #[error("browser launch failed: {reason}")]
    #[diagnostic(
        code(recorder::browser_launch),
        help("Ensure Chromium is installed and accessible")
    )]
    BrowserLaunch {
        /// Description of what went wrong.
        reason: String,
    },

    /// Failed to capture network traffic.
    #[error("network capture error: {reason}")]
    #[diagnostic(code(recorder::capture))]
    Capture {
        /// Description of what went wrong.
        reason: String,
    },

    /// The recording session was interrupted.
    #[error("recording interrupted: {reason}")]
    #[diagnostic(code(recorder::interrupted))]
    Interrupted {
        /// Description of what went wrong.
        reason: String,
    },
}

#[cfg(test)]
mod tests {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    #[test]
    fn test_browser_launch_error_display() {
        let err = RecorderError::BrowserLaunch {
            reason: "not found".to_owned(),
        };
        assert_eq!(
            err.to_string(),
            "browser launch failed: not found",
            "error message format must be stable"
        );
    }
}
