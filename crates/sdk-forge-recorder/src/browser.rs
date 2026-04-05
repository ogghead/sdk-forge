//! Browser lifecycle management via CDP.
//!
//! Handles launching Chromium, establishing a CDP connection,
//! and managing the browser process lifecycle.

/// Configuration for launching a browser session.
#[derive(Debug, Clone, Default)]
pub struct BrowserConfig {
    /// Whether to run in headless mode.
    pub headless: bool,
    /// Optional path to the Chromium executable.
    pub executable_path: Option<String>,
    /// Additional Chrome flags to pass.
    pub extra_args: Vec<String>,
}

#[cfg(test)]
mod tests {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    #[test]
    fn test_default_config_is_not_headless() {
        let config = BrowserConfig::default();
        assert!(
            !config.headless,
            "default config should not be headless (user needs to interact)"
        );
    }
}
