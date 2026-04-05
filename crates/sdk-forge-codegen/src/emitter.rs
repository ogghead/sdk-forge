//! SDK crate emission orchestrator.
//!
//! Coordinates the full code generation pipeline: resolves types from the
//! API model, renders Tera templates, formats output, and writes a
//! complete Rust crate to disk.

use std::path::{Path, PathBuf};

use serde::Serialize;
use tera::Tera;

use sdk_forge_session::types::ApiModel;

use crate::auth::build_auth_info;
use crate::cargo_toml::build_cargo_toml_context;
use crate::client::build_client_context;
use crate::error::CodegenError;
use crate::errors::build_error_context;
use crate::formatter::format_rust_source;
use crate::types::resolve_all;

/// Configuration for the emitter.
#[derive(Debug, Clone)]
pub struct EmitConfig {
    /// Name for the generated crate (e.g. `"my-api-sdk"`).
    pub crate_name: String,
    /// Output directory. The crate is written to `{output_dir}/{crate_name}/`.
    pub output_dir: PathBuf,
    /// HTTP client crate (`"reqwest"` or `"rquest"`).
    pub http_client: String,
    /// Path to the Tera templates directory.
    pub templates_dir: PathBuf,
}

/// Result of a successful emission.
#[derive(Debug, Clone)]
pub struct EmitOutput {
    /// Path to the generated crate root.
    pub crate_dir: PathBuf,
    /// Number of files written.
    pub file_count: usize,
    /// Number of struct types generated.
    pub struct_count: usize,
    /// Number of endpoint methods generated.
    pub method_count: usize,
}

/// Generate a complete Rust SDK crate from an [`ApiModel`].
///
/// # Errors
///
/// Returns [`CodegenError`] variants for template, write, or formatting failures.
pub fn emit(model: &ApiModel, config: &EmitConfig) -> Result<EmitOutput, CodegenError> {
    // 1. Resolve types and methods from the API model.
    let (methods, registry) = resolve_all(model);
    let structs = registry.into_structs();

    // 2. Build template contexts.
    let auth_info = build_auth_info(&model.auth);
    let client_ctx = build_client_context(&model.base_url, &config.http_client, auth_info);
    let error_ctx = build_error_context(&config.http_client);
    let cargo_ctx =
        build_cargo_toml_context(&config.crate_name, &model.base_url, &config.http_client);

    // 3. Load Tera templates.
    let template_glob = config
        .templates_dir
        .join("**/*")
        .to_string_lossy()
        .into_owned();

    let tera = Tera::new(&template_glob).map_err(|err| CodegenError::TemplateError {
        reason: format!(
            "failed to load templates from {}: {err}",
            config.templates_dir.display()
        ),
    })?;

    // 4. Render each file.
    let crate_dir = config.output_dir.join(&config.crate_name);
    let src_dir = crate_dir.join("src");

    // Render Cargo.toml (not a Rust file — skip rustfmt).
    let cargo_toml = render_template(&tera, "cargo_toml.tera", &cargo_ctx)?;

    // Render Rust source files.
    let lib_rs = render_template(
        &tera,
        "lib.rs.tera",
        &LibContext {
            base_url: model.base_url.clone(),
        },
    )?;
    let types_rs = render_template(
        &tera,
        "types.rs.tera",
        &TypesContext {
            structs: structs.clone(),
        },
    )?;
    let client_rs = render_template(&tera, "client.rs.tera", &client_ctx)?;
    let endpoints_rs = render_template(
        &tera,
        "endpoints.rs.tera",
        &EndpointsContext {
            methods: methods.clone(),
        },
    )?;
    let error_rs = render_template(&tera, "error.rs.tera", &error_ctx)?;

    // 5. Format Rust files.
    let lib_rs_fmt = format_rust_source(&lib_rs)?;
    let types_rs_fmt = format_rust_source(&types_rs)?;
    let client_rs_fmt = format_rust_source(&client_rs)?;
    let endpoints_rs_fmt = format_rust_source(&endpoints_rs)?;
    let error_rs_fmt = format_rust_source(&error_rs)?;

    // 6. Write files to disk.
    create_dir(&crate_dir)?;
    create_dir(&src_dir)?;

    write_file(&crate_dir.join("Cargo.toml"), &cargo_toml)?;
    write_file(&src_dir.join("lib.rs"), &lib_rs_fmt)?;
    write_file(&src_dir.join("types.rs"), &types_rs_fmt)?;
    write_file(&src_dir.join("client.rs"), &client_rs_fmt)?;
    write_file(&src_dir.join("endpoints.rs"), &endpoints_rs_fmt)?;
    write_file(&src_dir.join("error.rs"), &error_rs_fmt)?;

    let file_count = 6;

    tracing::info!(
        crate_dir = %crate_dir.display(),
        structs = structs.len(),
        methods = methods.len(),
        "SDK crate generated"
    );

    Ok(EmitOutput {
        crate_dir,
        file_count,
        struct_count: structs.len(),
        method_count: methods.len(),
    })
}

// ── Template contexts ───────────────────────────────────────────────

/// Context for `lib.rs.tera`.
#[derive(Serialize)]
struct LibContext {
    /// Base URL for the doc comment.
    base_url: String,
}

/// Context for `types.rs.tera`.
#[derive(Serialize)]
struct TypesContext {
    /// All generated struct definitions.
    structs: Vec<crate::types::RustStruct>,
}

/// Context for `endpoints.rs.tera`.
#[derive(Serialize)]
struct EndpointsContext {
    /// All generated endpoint methods.
    methods: Vec<crate::types::EndpointMethod>,
}

// ── Helpers ─────────────────────────────────────────────────────────

/// Render a Tera template with the given context.
fn render_template<C: Serialize>(
    tera: &Tera,
    template_name: &str,
    context: &C,
) -> Result<String, CodegenError> {
    let tera_ctx =
        tera::Context::from_serialize(context).map_err(|err| CodegenError::TemplateError {
            reason: format!("failed to serialize context for {template_name}: {err}"),
        })?;

    tera.render(template_name, &tera_ctx)
        .map_err(|err| CodegenError::TemplateError {
            reason: format!("failed to render {template_name}: {err}"),
        })
}

/// Create a directory and all parent directories.
fn create_dir(path: &Path) -> Result<(), CodegenError> {
    std::fs::create_dir_all(path).map_err(|err| CodegenError::WriteError {
        reason: format!("failed to create directory {}: {err}", path.display()),
    })
}

/// Write content to a file, creating parent directories if needed.
fn write_file(path: &Path, content: &str) -> Result<(), CodegenError> {
    std::fs::write(path, content).map_err(|err| CodegenError::WriteError {
        reason: format!("failed to write {}: {err}", path.display()),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use std::collections::HashMap;
    use std::path::PathBuf;

    use sdk_forge_session::types::{
        ApiModel, AuthPattern, Endpoint, JsonSchema, PathParam, QueryParam, ResourceGroup,
    };

    use super::{EmitConfig, emit};

    /// Canonical test model.
    fn test_model() -> ApiModel {
        ApiModel {
            base_url: "https://api.example.com".to_owned(),
            auth: Some(AuthPattern::BearerToken),
            resources: vec![ResourceGroup {
                name: "Users".to_owned(),
                base_path: "/api/v1/users".to_owned(),
                endpoints: vec![
                    Endpoint {
                        method: "GET".to_owned(),
                        path_template: "/".to_owned(),
                        description: "List all users".to_owned(),
                        request_schema: None,
                        response_schema: Some(JsonSchema::Array {
                            items: Box::new(JsonSchema::Object {
                                properties: HashMap::from([
                                    ("id".to_owned(), JsonSchema::Integer),
                                    ("name".to_owned(), JsonSchema::String { format: None }),
                                ]),
                                required: vec!["id".to_owned(), "name".to_owned()],
                            }),
                        }),
                        requires_auth: true,
                        path_params: Vec::new(),
                        query_params: vec![QueryParam {
                            name: "page".to_owned(),
                            param_type: "integer".to_owned(),
                            required: false,
                            description: Some("Page number".to_owned()),
                        }],
                    },
                    Endpoint {
                        method: "GET".to_owned(),
                        path_template: "/{id}".to_owned(),
                        description: "Get user by ID".to_owned(),
                        request_schema: None,
                        response_schema: Some(JsonSchema::Object {
                            properties: HashMap::from([
                                ("id".to_owned(), JsonSchema::Integer),
                                ("name".to_owned(), JsonSchema::String { format: None }),
                            ]),
                            required: vec!["id".to_owned(), "name".to_owned()],
                        }),
                        requires_auth: true,
                        path_params: vec![PathParam {
                            name: "id".to_owned(),
                            param_type: "integer".to_owned(),
                            description: None,
                        }],
                        query_params: Vec::new(),
                    },
                    Endpoint {
                        method: "POST".to_owned(),
                        path_template: "/".to_owned(),
                        description: "Create a new user".to_owned(),
                        request_schema: Some(JsonSchema::Object {
                            properties: HashMap::from([
                                ("name".to_owned(), JsonSchema::String { format: None }),
                                ("email".to_owned(), JsonSchema::String { format: None }),
                            ]),
                            required: vec!["name".to_owned(), "email".to_owned()],
                        }),
                        response_schema: Some(JsonSchema::Object {
                            properties: HashMap::from([
                                ("id".to_owned(), JsonSchema::Integer),
                                ("name".to_owned(), JsonSchema::String { format: None }),
                            ]),
                            required: vec!["id".to_owned(), "name".to_owned()],
                        }),
                        requires_auth: true,
                        path_params: Vec::new(),
                        query_params: Vec::new(),
                    },
                ],
            }],
            pagination: None,
        }
    }

    /// Path to the templates directory relative to workspace root.
    fn templates_dir() -> PathBuf {
        // In tests, we're in the workspace root.
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest_dir
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join("templates"))
            .unwrap()
    }

    #[test]
    fn test_emit_generates_crate() {
        let model = test_model();
        let temp = std::env::temp_dir().join("sdk-forge-emit-test");
        let _ = std::fs::remove_dir_all(&temp);

        let config = EmitConfig {
            crate_name: "example-api".to_owned(),
            output_dir: temp.clone(),
            http_client: "reqwest".to_owned(),
            templates_dir: templates_dir(),
        };

        let result = emit(&model, &config);
        assert!(result.is_ok(), "emit should succeed: {result:?}");

        let output = result.unwrap();
        assert_eq!(output.file_count, 6, "should write 6 files");
        assert!(output.struct_count > 0, "should generate structs");
        assert_eq!(output.method_count, 3, "should generate 3 methods");

        // Verify files exist.
        let crate_dir = temp.join("example-api");
        assert!(
            crate_dir.join("Cargo.toml").exists(),
            "Cargo.toml should exist"
        );
        assert!(crate_dir.join("src/lib.rs").exists(), "lib.rs should exist");
        assert!(
            crate_dir.join("src/types.rs").exists(),
            "types.rs should exist"
        );
        assert!(
            crate_dir.join("src/client.rs").exists(),
            "client.rs should exist"
        );
        assert!(
            crate_dir.join("src/endpoints.rs").exists(),
            "endpoints.rs should exist"
        );
        assert!(
            crate_dir.join("src/error.rs").exists(),
            "error.rs should exist"
        );

        // Verify Cargo.toml content.
        let cargo_toml = std::fs::read_to_string(crate_dir.join("Cargo.toml")).unwrap();
        assert!(
            cargo_toml.contains("name = \"example-api\""),
            "Cargo.toml should have crate name"
        );
        assert!(
            cargo_toml.contains("reqwest"),
            "Cargo.toml should reference reqwest"
        );

        // Verify types.rs has struct definitions.
        let types_src = std::fs::read_to_string(crate_dir.join("src/types.rs")).unwrap();
        assert!(
            types_src.contains("pub struct User"),
            "should generate User struct"
        );
        assert!(
            types_src.contains("pub struct CreateUserRequest"),
            "should generate CreateUserRequest"
        );

        // Verify client.rs has auth.
        let client_src = std::fs::read_to_string(crate_dir.join("src/client.rs")).unwrap();
        assert!(
            client_src.contains("pub trait AuthStrategy"),
            "should have AuthStrategy trait"
        );
        assert!(
            client_src.contains("pub struct BearerAuth"),
            "should have BearerAuth struct"
        );

        // Verify endpoints.rs has methods.
        let endpoints_src = std::fs::read_to_string(crate_dir.join("src/endpoints.rs")).unwrap();
        assert!(
            endpoints_src.contains("async fn list_users"),
            "should have list_users method"
        );
        assert!(
            endpoints_src.contains("async fn get_user"),
            "should have get_user method"
        );
        assert!(
            endpoints_src.contains("async fn create_user"),
            "should have create_user method"
        );

        // Cleanup.
        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_emit_with_rquest() {
        let model = test_model();
        let temp = std::env::temp_dir().join("sdk-forge-emit-rquest-test");
        let _ = std::fs::remove_dir_all(&temp);

        let config = EmitConfig {
            crate_name: "rquest-api".to_owned(),
            output_dir: temp.clone(),
            http_client: "rquest".to_owned(),
            templates_dir: templates_dir(),
        };

        let result = emit(&model, &config);
        assert!(result.is_ok(), "emit with rquest should succeed");

        let crate_dir = temp.join("rquest-api");
        let cargo_toml = std::fs::read_to_string(crate_dir.join("Cargo.toml")).unwrap();
        assert!(
            cargo_toml.contains("rquest"),
            "Cargo.toml should reference rquest"
        );
        assert!(
            !cargo_toml.contains("reqwest"),
            "should not contain reqwest when using rquest"
        );

        let client_src = std::fs::read_to_string(crate_dir.join("src/client.rs")).unwrap();
        assert!(
            client_src.contains("rquest::RequestBuilder"),
            "client should use rquest types"
        );

        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_emit_no_auth_model() {
        let model = ApiModel {
            base_url: "https://public-api.example.com".to_owned(),
            auth: None,
            resources: vec![ResourceGroup {
                name: "Items".to_owned(),
                base_path: "/items".to_owned(),
                endpoints: vec![Endpoint {
                    method: "GET".to_owned(),
                    path_template: "/".to_owned(),
                    description: "List items".to_owned(),
                    request_schema: None,
                    response_schema: Some(JsonSchema::Array {
                        items: Box::new(JsonSchema::Integer),
                    }),
                    requires_auth: false,
                    path_params: Vec::new(),
                    query_params: Vec::new(),
                }],
            }],
            pagination: None,
        };

        let temp = std::env::temp_dir().join("sdk-forge-emit-noauth");
        let _ = std::fs::remove_dir_all(&temp);

        let config = EmitConfig {
            crate_name: "public-api".to_owned(),
            output_dir: temp.clone(),
            http_client: "reqwest".to_owned(),
            templates_dir: templates_dir(),
        };

        let result = emit(&model, &config);
        assert!(result.is_ok(), "emit without auth should succeed");

        let crate_dir = temp.join("public-api");
        let client_src = std::fs::read_to_string(crate_dir.join("src/client.rs")).unwrap();
        assert!(
            !client_src.contains("BearerAuth"),
            "no-auth model should not generate BearerAuth"
        );
        assert!(
            client_src.contains("NoAuth"),
            "should still have NoAuth struct"
        );

        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_emit_invalid_templates_dir() {
        let model = test_model();
        let config = EmitConfig {
            crate_name: "bad".to_owned(),
            output_dir: PathBuf::from("/tmp/sdk-forge-bad"),
            http_client: "reqwest".to_owned(),
            templates_dir: PathBuf::from("/nonexistent/templates"),
        };

        let result = emit(&model, &config);
        assert!(result.is_err(), "invalid templates dir should fail");
    }
}
