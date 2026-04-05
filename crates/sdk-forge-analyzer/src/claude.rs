//! Anthropic Claude API client.
//!
//! A thin, hand-rolled client for the Anthropic Messages API.
//! Uses `tool_use` for structured output to ensure reliable parsing
//! of analysis results.

/// Configuration for the Claude API client.
#[derive(Debug, Clone)]
pub struct ClaudeConfig {
    /// Anthropic API key.
    pub api_key: String,
    /// Model identifier (e.g., "claude-sonnet-4-20250514").
    pub model: String,
    /// Maximum tokens to generate per request.
    pub max_tokens: u32,
}

impl ClaudeConfig {
    /// Default model used for analysis.
    pub const DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";
    /// Default max tokens per response.
    pub const DEFAULT_MAX_TOKENS: u32 = 4096;
}

#[cfg(test)]
mod tests {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    #[test]
    fn test_default_model_is_set() {
        assert!(
            !ClaudeConfig::DEFAULT_MODEL.is_empty(),
            "default model should not be empty"
        );
    }
}
