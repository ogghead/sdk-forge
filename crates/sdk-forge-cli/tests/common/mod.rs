//! Shared test helpers for CLI integration tests.

use std::path::PathBuf;

/// Path to the workspace-level `tests/fixtures/` directory.
pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures")
}

/// Path to the workspace-level `templates/` directory.
pub fn templates_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../templates")
}

/// Path to the analyzed petstore fixture.
pub fn petstore_analyzed() -> PathBuf {
    fixtures_dir().join("petstore_analyzed.sdkforge")
}

/// Path to the unanalyzed petstore fixture.
pub fn petstore_unanalyzed() -> PathBuf {
    fixtures_dir().join("petstore_unanalyzed.sdkforge")
}

/// Path to the minimal no-auth fixture.
pub fn minimal_noauth() -> PathBuf {
    fixtures_dir().join("minimal_noauth.sdkforge")
}
