//! CLI configuration loading.
//!
//! Handles configuration from environment variables, config files,
//! and CLI arguments.

/// Application configuration.
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Anthropic API key for Claude analysis.
    pub anthropic_api_key: Option<String>,
    /// Default output directory for generated SDKs.
    pub output_dir: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            anthropic_api_key: None,
            output_dir: "output".to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert!(
            config.anthropic_api_key.is_none(),
            "API key should not be set by default"
        );
        assert_eq!(
            config.output_dir, "output",
            "default output dir should be 'output'"
        );
    }
}
