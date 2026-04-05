//! Session file I/O.
//!
//! Read and write `.sdkforge` session files (JSON format).

use std::path::Path;

use miette::{Context, IntoDiagnostic};

use crate::Session;

/// Save a session to a `.sdkforge` file.
///
/// The file is written as pretty-printed JSON for human readability.
///
/// # Errors
///
/// Returns an error if the file cannot be created or serialization fails.
pub fn save_session(session: &Session, path: &Path) -> miette::Result<()> {
    let json = serde_json::to_string_pretty(session)
        .into_diagnostic()
        .wrap_err("failed to serialize session")?;
    std::fs::write(path, json)
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to write session file: {}", path.display()))
}

/// Load a session from a `.sdkforge` file.
///
/// # Errors
///
/// Returns an error if the file cannot be read or deserialization fails.
pub fn load_session(path: &Path) -> miette::Result<Session> {
    let contents = std::fs::read_to_string(path)
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to read session file: {}", path.display()))?;
    serde_json::from_str(&contents)
        .into_diagnostic()
        .wrap_err("failed to deserialize session file")
}

#[cfg(test)]
mod tests {
    #[allow(clippy::wildcard_imports)]
    use super::*;
    use crate::{Session, SessionMetadata};
    use chrono::Utc;
    use uuid::Uuid;

    /// Helper to create a minimal test session.
    fn test_session() -> Session {
        Session {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            target_url: "https://example.com".to_owned(),
            browser_version: "Chromium 120".to_owned(),
            exchanges: Vec::new(),
            api_model: None,
            metadata: SessionMetadata {
                duration_ms: 500,
                raw_exchange_count: 0,
                notes: None,
            },
        }
    }

    #[test]
    fn test_save_and_load_round_trip() {
        let dir = std::env::temp_dir().join("sdk-forge-test-io");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test.sdkforge");

        let session = test_session();
        let save_result = save_session(&session, &path);
        assert!(save_result.is_ok(), "save_session should succeed");

        let loaded = load_session(&path);
        assert!(loaded.is_ok(), "load_session should succeed");

        let loaded_session = loaded.unwrap_or_else(|_| test_session());
        assert_eq!(
            session.id, loaded_session.id,
            "loaded session should have the same id"
        );

        // Clean up
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn test_load_nonexistent_file() {
        let result = load_session(Path::new("/nonexistent/path.sdkforge"));
        assert!(result.is_err(), "loading nonexistent file should fail");
    }
}
