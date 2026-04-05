//! Session data redaction.
//!
//! Strips sensitive values from sessions before sending to external
//! analysis services. Preserves structure so that auth patterns,
//! header names, and field names remain visible.

use crate::Session;

/// Placeholder token for redacted bearer tokens.
pub const REDACTED_BEARER: &str = "<REDACTED_BEARER_TOKEN>";
/// Placeholder token for redacted cookies.
pub const REDACTED_COOKIE: &str = "<REDACTED_COOKIE>";
/// Placeholder token for redacted sensitive values.
pub const REDACTED_VALUE: &str = "<REDACTED>";

/// Create a redacted copy of a session suitable for external analysis.
///
/// This replaces sensitive values (tokens, passwords, cookies) with
/// placeholder strings while preserving the overall structure so
/// that auth detection and schema inference still work.
///
/// # Errors
///
/// Returns an error if the session cannot be cloned or processed.
pub fn redact_session(session: &Session) -> miette::Result<Session> {
    // TODO: Implement redaction logic for:
    // - Authorization header values → REDACTED_BEARER
    // - Cookie header values → REDACTED_COOKIE
    // - Request body fields matching sensitive patterns → REDACTED_VALUE
    // - JWT tokens → REDACTED_JWT
    // - API key patterns → REDACTED_API_KEY
    Ok(session.clone())
}

#[cfg(test)]
mod tests {
    #[allow(clippy::wildcard_imports)]
    use super::*;
    use crate::{Session, SessionMetadata};
    use chrono::Utc;
    use uuid::Uuid;

    #[test]
    fn test_redact_session_preserves_structure() {
        let session = Session {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            target_url: "https://example.com".to_owned(),
            browser_version: "Chromium 120".to_owned(),
            exchanges: Vec::new(),
            api_model: None,
            metadata: SessionMetadata {
                duration_ms: 100,
                raw_exchange_count: 0,
                notes: None,
            },
        };

        let redacted = redact_session(&session);
        assert!(redacted.is_ok(), "redaction should succeed");

        let redacted_session = redacted.unwrap_or_else(|_| session.clone());
        assert_eq!(
            session.id, redacted_session.id,
            "redaction should preserve session id"
        );
        assert_eq!(
            session.target_url, redacted_session.target_url,
            "redaction should preserve target URL"
        );
    }

    #[test]
    fn test_redaction_constants_are_recognizable() {
        assert!(
            REDACTED_BEARER.contains("REDACTED"),
            "bearer placeholder should contain REDACTED"
        );
        assert!(
            REDACTED_COOKIE.contains("REDACTED"),
            "cookie placeholder should contain REDACTED"
        );
        assert!(
            REDACTED_VALUE.contains("REDACTED"),
            "value placeholder should contain REDACTED"
        );
    }
}
