//! Wasm component emission orchestrator.
//!
//! Coordinates the full code generation pipeline: resolves types from the
//! API model, renders Tera templates, formats Rust output, and writes a
//! complete Wasm component crate (with WIT spec) to disk.

use std::path::{Path, PathBuf};

use serde::Serialize;
use tera::Tera;

use sdk_forge_session::types::ApiModel;

use crate::auth::build_auth_info;
use crate::cargo_toml::build_cargo_toml_context;
use crate::client::build_component_context;
use crate::error::CodegenError;
use crate::formatter::format_rust_source;
use crate::types::{WitRecord, resolve_all};

/// Configuration for the emitter.
#[derive(Debug, Clone)]
pub struct EmitConfig {
    /// Name for the generated crate (e.g. `"my-api-sdk"`).
    pub crate_name: String,
    /// Output directory. The crate is written to `{output_dir}/{crate_name}/`.
    pub output_dir: PathBuf,
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
    /// Number of WIT record types generated.
    pub struct_count: usize,
    /// Number of endpoint functions generated.
    pub method_count: usize,
}

/// Generate a complete Wasm component crate from an [`ApiModel`].
///
/// Produces both the WIT spec and the Rust implementation.
///
/// # Errors
///
/// Returns [`CodegenError`] variants for template, write, or formatting failures.
pub fn emit(model: &ApiModel, config: &EmitConfig) -> Result<EmitOutput, CodegenError> {
    // 1. Resolve types and interfaces from the API model.
    let (interfaces, registry) = resolve_all(model);
    let records = registry.into_records();

    let method_count: usize = interfaces.iter().map(|i| i.functions.len()).sum();
    let struct_count = records.len();

    // 2. Build template contexts.
    let auth_info = build_auth_info(&model.auth);
    let cargo_ctx = build_cargo_toml_context(&config.crate_name, &model.base_url);

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
    let wit_dir = crate_dir.join("wit");

    // Render Cargo.toml (not Rust — skip rustfmt).
    let cargo_toml = render_template(&tera, "cargo_toml.tera", &cargo_ctx)?;

    // Render types.rs (needs records, consumed last).
    let types_rs = render_template(
        &tera,
        "types.rs.tera",
        &TypesContext {
            records: records.clone(),
        },
    )?;

    // Render WIT spec (not Rust — skip rustfmt).
    let world_wit = render_template(
        &tera,
        "world.wit.tera",
        &WitWorldContext {
            package_name: config.crate_name.clone(),
            base_url: model.base_url.clone(),
            auth: auth_info.clone(),
            has_auth: auth_info.is_some(),
            records,
            interfaces: interfaces.clone(),
        },
    )?;

    // Render http.rs (needs auth_info, clone for component context).
    let http_rs = render_template(
        &tera,
        "http.rs.tera",
        &HttpContext {
            has_auth: auth_info.is_some(),
            auth: auth_info.clone(),
        },
    )?;

    // Render lib.rs (uses component context which consumes auth_info + interfaces).
    let component_ctx =
        build_component_context(&model.base_url, &config.crate_name, auth_info, interfaces);
    let lib_rs = render_template(&tera, "lib.rs.tera", &component_ctx)?;

    // Render error.rs.
    let error_rs = render_template(
        &tera,
        "error.rs.tera",
        &ErrorRenderContext { _placeholder: true },
    )?;

    // 5. Format Rust files.
    let lib_rs_fmt = format_rust_source(&lib_rs)?;
    let types_rs_fmt = format_rust_source(&types_rs)?;
    let http_rs_fmt = format_rust_source(&http_rs)?;
    let error_rs_fmt = format_rust_source(&error_rs)?;

    // 6. Write files to disk.
    create_dir(&crate_dir)?;
    create_dir(&src_dir)?;
    create_dir(&wit_dir)?;

    write_file(&crate_dir.join("Cargo.toml"), &cargo_toml)?;
    write_file(&wit_dir.join("world.wit"), &world_wit)?;
    write_file(&src_dir.join("lib.rs"), &lib_rs_fmt)?;
    write_file(&src_dir.join("types.rs"), &types_rs_fmt)?;
    write_file(&src_dir.join("http.rs"), &http_rs_fmt)?;
    write_file(&src_dir.join("error.rs"), &error_rs_fmt)?;

    let file_count = 6;

    tracing::info!(
        crate_dir = %crate_dir.display(),
        records = struct_count,
        methods = method_count,
        "Wasm component crate generated"
    );

    Ok(EmitOutput {
        crate_dir,
        file_count,
        struct_count,
        method_count,
    })
}

// ── Template contexts ───────────────────────────────────────────────

/// Context for `world.wit.tera`.
#[derive(Serialize)]
struct WitWorldContext {
    /// Package name.
    package_name: String,
    /// Base URL for the doc comment.
    base_url: String,
    /// Auth info.
    auth: Option<crate::auth::AuthInfo>,
    /// Whether auth is configured.
    has_auth: bool,
    /// All generated records.
    records: Vec<WitRecord>,
    /// All interfaces.
    interfaces: Vec<crate::types::WitInterface>,
}

/// Context for `types.rs.tera`.
#[derive(Serialize)]
struct TypesContext {
    /// All generated record definitions.
    records: Vec<WitRecord>,
}

/// Context for `http.rs.tera`.
#[derive(Serialize)]
struct HttpContext {
    /// Whether auth is configured.
    has_auth: bool,
    /// Auth info.
    auth: Option<crate::auth::AuthInfo>,
}

/// Context for `error.rs.tera` (no variables needed, but Tera requires a JSON object).
#[derive(Serialize)]
struct ErrorRenderContext {
    /// Placeholder to make Tera happy (requires a JSON object).
    _placeholder: bool,
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
            templates_dir: templates_dir(),
        };

        let result = emit(&model, &config);
        assert!(result.is_ok(), "emit should succeed: {result:?}");

        let output = result.unwrap();
        assert_eq!(output.file_count, 6, "should write 6 files");
        assert!(output.struct_count > 0, "should generate records");
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
            crate_dir.join("src/http.rs").exists(),
            "http.rs should exist"
        );
        assert!(
            crate_dir.join("src/error.rs").exists(),
            "error.rs should exist"
        );
        assert!(
            crate_dir.join("wit/world.wit").exists(),
            "world.wit should exist"
        );

        // Verify Cargo.toml content.
        let cargo_toml = std::fs::read_to_string(crate_dir.join("Cargo.toml")).unwrap();
        assert!(
            cargo_toml.contains("name = \"example-api\""),
            "Cargo.toml should have crate name"
        );
        assert!(
            cargo_toml.contains("cdylib"),
            "Cargo.toml should specify cdylib crate type"
        );
        assert!(
            cargo_toml.contains("wit-bindgen"),
            "Cargo.toml should reference wit-bindgen"
        );

        // Verify world.wit has WIT records and interfaces.
        let world_wit = std::fs::read_to_string(crate_dir.join("wit/world.wit")).unwrap();
        assert!(
            world_wit.contains("package example-api:api"),
            "WIT should have package declaration"
        );
        assert!(
            world_wit.contains("record user"),
            "WIT should have user record"
        );
        assert!(
            world_wit.contains("interface users"),
            "WIT should have users interface"
        );
        assert!(
            world_wit.contains("list-users"),
            "WIT should have list-users function"
        );
        assert!(
            world_wit.contains("wasi:http/outgoing-handler"),
            "WIT should import wasi:http"
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

        // Cleanup.
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
            templates_dir: templates_dir(),
        };

        let result = emit(&model, &config);
        assert!(result.is_ok(), "emit without auth should succeed");

        let crate_dir = temp.join("public-api");
        let world_wit = std::fs::read_to_string(crate_dir.join("wit/world.wit")).unwrap();
        assert!(
            !world_wit.contains("bearer-auth"),
            "no-auth model should not have bearer-auth record"
        );

        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_emit_invalid_templates_dir() {
        let model = test_model();
        let config = EmitConfig {
            crate_name: "bad".to_owned(),
            output_dir: PathBuf::from("/tmp/sdk-forge-bad"),
            templates_dir: PathBuf::from("/nonexistent/templates"),
        };

        let result = emit(&model, &config);
        assert!(result.is_err(), "invalid templates dir should fail");
    }
}
