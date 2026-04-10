//! Rustfmt integration for generated source files.
//!
//! Runs `rustfmt` on generated Rust source to ensure consistent
//! formatting. Falls back to unformatted output if rustfmt is
//! unavailable.

use std::io::Write;
use std::process::{Command, Stdio};

use crate::error::CodegenError;

/// Format a Rust source string using `rustfmt`.
///
/// Returns the formatted source, or the original source with a warning
/// if `rustfmt` is not available or fails.
///
/// # Errors
///
/// Returns [`CodegenError::FormatError`] only if rustfmt produces
/// invalid output. Missing rustfmt results in a warning, not an error.
pub fn format_rust_source(source: &str) -> Result<String, CodegenError> {
    let result = Command::new("rustfmt")
        .arg("--edition")
        .arg("2021")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();

    let Ok(mut child) = result else {
        tracing::warn!("rustfmt not found; skipping formatting");
        return Ok(source.to_owned());
    };

    // Write source to stdin.
    if let Some(mut stdin) = child.stdin.take() {
        // Ignore write errors — rustfmt may close stdin early on invalid input.
        let _result = stdin.write_all(source.as_bytes());
    }

    let output = child
        .wait_with_output()
        .map_err(|err| CodegenError::FormatError {
            reason: format!("failed to wait for rustfmt: {err}"),
        })?;

    if output.status.success() {
        String::from_utf8(output.stdout).map_err(|err| CodegenError::FormatError {
            reason: format!("rustfmt produced invalid UTF-8: {err}"),
        })
    } else {
        // rustfmt failed (e.g. syntax error in template output).
        // Return the original source with a warning.
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::warn!(%stderr, "rustfmt failed; using unformatted output");
        Ok(source.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::format_rust_source;

    #[test]
    fn test_format_valid_rust() {
        let source = "fn main(){let x=1;let y=2;}";
        let result = format_rust_source(source);
        assert!(result.is_ok(), "formatting should succeed");

        let formatted = result.unwrap_or_default();
        // rustfmt should add whitespace.
        assert!(
            formatted.contains("let x = 1;"),
            "should be formatted, got: {formatted}"
        );
    }

    #[test]
    fn test_format_invalid_rust_falls_back() {
        let source = "this is not valid rust at all {{{{ }}}}";
        let result = format_rust_source(source);
        assert!(result.is_ok(), "should fall back without error");

        let output = result.unwrap_or_default();
        assert_eq!(output, source, "should return original source on failure");
    }
}
