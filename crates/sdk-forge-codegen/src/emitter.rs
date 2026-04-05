//! SDK crate emission orchestrator.
//!
//! Coordinates the full code generation pipeline: converts the API model
//! to an [`openapiv3::OpenAPI`] specification, optionally serializes it,
//! and writes the output to disk. Future versions will integrate progenitor
//! for direct Rust SDK generation from the spec.

use std::path::Path;

use crate::error::CodegenError;
use crate::openapi;

/// Output format for the generated SDK.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// Emit an `OpenAPI` v3.0 JSON specification file.
    OpenApiJson,
}

/// Generate SDK output from an [`ApiModel`](sdk_forge_session::types::ApiModel).
///
/// Currently emits an `OpenAPI` v3.0 JSON spec. Future versions will support
/// direct Rust crate generation via progenitor.
///
/// # Errors
///
/// Returns [`CodegenError`] variants for conversion, serialization, or I/O failures.
pub fn emit(
    model: &sdk_forge_session::types::ApiModel,
    output_dir: &Path,
    format: OutputFormat,
) -> Result<String, CodegenError> {
    let spec = openapi::api_model_to_openapi(model)?;

    match format {
        OutputFormat::OpenApiJson => {
            let json = openapi::openapi_to_json(&spec)?;

            let output_path = output_dir.join("openapi.json");
            std::fs::create_dir_all(output_dir).map_err(|err| CodegenError::WriteError {
                reason: format!("failed to create output directory: {err}"),
            })?;
            std::fs::write(&output_path, &json).map_err(|err| CodegenError::WriteError {
                reason: format!("failed to write {}: {err}", output_path.display()),
            })?;

            Ok(output_path.display().to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    use std::collections::HashMap;

    use sdk_forge_session::types::{ApiModel, AuthPattern, Endpoint, JsonSchema, ResourceGroup};

    /// Build a minimal valid [`ApiModel`] for emitter tests.
    fn minimal_model() -> ApiModel {
        ApiModel {
            base_url: "https://api.test.com".to_owned(),
            auth: Some(AuthPattern::BearerToken),
            resources: vec![ResourceGroup {
                name: "Items".to_owned(),
                base_path: "/items".to_owned(),
                endpoints: vec![Endpoint {
                    method: "GET".to_owned(),
                    path_template: "/".to_owned(),
                    description: "List items".to_owned(),
                    request_schema: None,
                    response_schema: Some(JsonSchema::Array {
                        items: Box::new(JsonSchema::Object {
                            properties: HashMap::from([("id".to_owned(), JsonSchema::Integer)]),
                            required: vec!["id".to_owned()],
                        }),
                    }),
                    requires_auth: true,
                    path_params: vec![],
                    query_params: vec![],
                }],
            }],
            pagination: None,
        }
    }

    #[test]
    fn test_emit_openapi_json() {
        let model = minimal_model();
        let temp_dir = std::env::temp_dir().join("sdk-forge-test-emit");
        let _ = std::fs::remove_dir_all(&temp_dir);

        let result = emit(&model, &temp_dir, OutputFormat::OpenApiJson);
        assert!(result.is_ok(), "emit should succeed: {result:?}");

        let output_path = result.unwrap_or_default();
        assert!(
            output_path.ends_with("openapi.json"),
            "should output openapi.json"
        );

        // Verify the file exists and is valid JSON.
        let contents = std::fs::read_to_string(&output_path).unwrap_or_default();
        assert!(
            contents.contains("\"openapi\""),
            "output should be valid OpenAPI JSON"
        );

        // Cleanup.
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_emit_fails_for_invalid_model() {
        let model = ApiModel {
            base_url: String::new(),
            auth: None,
            resources: vec![],
            pagination: None,
        };
        let temp_dir = std::env::temp_dir().join("sdk-forge-test-emit-fail");
        let result = emit(&model, &temp_dir, OutputFormat::OpenApiJson);
        assert!(result.is_err(), "should fail for invalid model");
    }
}
