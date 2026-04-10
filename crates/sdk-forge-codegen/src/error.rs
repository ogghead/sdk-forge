//! Codegen error types.

use miette::Diagnostic;
use thiserror::Error;

/// Errors that can occur during SDK code generation.
#[derive(Debug, Error, Diagnostic)]
#[non_exhaustive]
pub enum CodegenError {
    /// Failed to render a template.
    #[error("template rendering failed: {reason}")]
    #[diagnostic(code(codegen::template))]
    TemplateError {
        /// Description of what went wrong.
        reason: String,
    },

    /// Failed to write generated files to disk.
    #[error("failed to write output: {reason}")]
    #[diagnostic(code(codegen::write))]
    WriteError {
        /// Description of what went wrong.
        reason: String,
    },

    /// The API model is missing required information for generation.
    #[error("incomplete API model: {reason}")]
    #[diagnostic(
        code(codegen::incomplete_model),
        help("Run `sdk-forge analyze` first to populate the API model")
    )]
    IncompleteModel {
        /// Description of what is missing.
        reason: String,
    },

    /// rustfmt failed to format the generated code.
    #[error("formatting failed: {reason}")]
    #[diagnostic(code(codegen::format), help("Ensure rustfmt is installed"))]
    FormatError {
        /// Description of what went wrong.
        reason: String,
    },

    /// Failed to convert the API model to an [`openapiv3::OpenAPI`] specification.
    #[error("OpenAPI conversion failed: {reason}")]
    #[diagnostic(
        code(codegen::openapi_conversion),
        help("Check that the API model has a valid base_url and at least one endpoint")
    )]
    OpenApiConversion {
        /// Description of what went wrong.
        reason: String,
    },

    /// Failed to serialize the [`openapiv3::OpenAPI`] specification.
    #[error("OpenAPI serialization failed: {reason}")]
    #[diagnostic(code(codegen::openapi_serialization))]
    OpenApiSerialization {
        /// Description of what went wrong.
        reason: String,
    },
}

#[cfg(test)]
mod tests {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    #[test]
    fn test_template_error_display() {
        let err = CodegenError::TemplateError {
            reason: "missing variable".to_owned(),
        };
        assert_eq!(
            err.to_string(),
            "template rendering failed: missing variable",
            "error message format must be stable"
        );
    }

    #[test]
    fn test_openapi_conversion_error_display() {
        let err = CodegenError::OpenApiConversion {
            reason: "no endpoints found".to_owned(),
        };
        assert_eq!(
            err.to_string(),
            "OpenAPI conversion failed: no endpoints found",
            "error message format must be stable"
        );
    }

    #[test]
    fn test_openapi_serialization_error_display() {
        let err = CodegenError::OpenApiSerialization {
            reason: "invalid JSON".to_owned(),
        };
        assert_eq!(
            err.to_string(),
            "OpenAPI serialization failed: invalid JSON",
            "error message format must be stable"
        );
    }
}
