//! Intermediate representation types and `JsonSchema` → WIT/Rust type conversion.
//!
//! Defines the IR that sits between [`ApiModel`](sdk_forge_session::types::ApiModel)
//! and Tera templates. Templates receive pre-computed, valid WIT identifiers
//! and type strings — no naming logic belongs in templates.
//!
//! The generated output is a WIT spec + Rust Wasm component implementation.

use std::collections::HashMap;

use sdk_forge_session::types::{ApiModel, Endpoint, JsonSchema, ResourceGroup};
use serde::Serialize;

use crate::naming::{
    TypeRole, build_fn_name, build_nested_type_name, build_type_name, field_name_and_rename,
    path_template_to_format_str, to_kebab_case, to_snake_case,
};

/// Maximum nesting depth before falling back to a dynamic value type.
const MAX_DEPTH: usize = 10;

// ── WIT IR types ──────────────────────────────────────────────────

/// A WIT record ready for template rendering.
#[derive(Debug, Clone, Serialize)]
pub struct WitRecord {
    /// `kebab-case` record name for WIT.
    pub wit_name: String,
    /// `PascalCase` Rust struct name (for the implementation).
    pub rust_name: String,
    /// Doc comment for the record.
    pub doc_comment: String,
    /// Ordered list of fields.
    pub fields: Vec<WitField>,
}

/// A single field within a [`WitRecord`].
#[derive(Debug, Clone, Serialize)]
pub struct WitField {
    /// `kebab-case` field name for WIT.
    pub wit_name: String,
    /// `snake_case` Rust field name (for the implementation).
    pub rust_name: String,
    /// WIT type string (e.g. `"string"`, `"s64"`, `"option<string>"`).
    pub wit_type: String,
    /// Rust type string (e.g. `"String"`, `"i64"`, `"Option<String>"`).
    pub rust_type: String,
    /// Original JSON key if it differs from `rust_name` (for serde rename).
    pub serde_rename: Option<String>,
    /// Whether this field is optional.
    pub is_optional: bool,
}

/// Metadata for a generated WIT function / Rust endpoint method.
#[derive(Debug, Clone, Serialize)]
pub struct WitFunction {
    /// `kebab-case` function name for WIT.
    pub wit_name: String,
    /// `snake_case` Rust function name (for the implementation).
    pub rust_name: String,
    /// Doc comment describing the endpoint.
    pub doc_comment: String,
    /// Lowercase HTTP method (e.g. `"get"`, `"post"`).
    pub http_method: String,
    /// Rust `format!()` string for the URL path.
    pub path_format_str: String,
    /// Path parameters in order.
    pub path_params: Vec<WitParam>,
    /// Query string parameters.
    pub query_params: Vec<WitParam>,
    /// WIT record name for the request body, if any.
    pub request_body_wit: Option<String>,
    /// Rust type name for the request body, if any.
    pub request_body_rust: Option<String>,
    /// WIT type name for the response.
    pub response_wit: String,
    /// Rust type name for the response.
    pub response_rust: String,
    /// Whether this endpoint requires authentication.
    pub requires_auth: bool,
}

/// A parameter on a generated function signature.
#[derive(Debug, Clone, Serialize)]
pub struct WitParam {
    /// `kebab-case` WIT parameter name.
    pub wit_name: String,
    /// `snake_case` Rust parameter name.
    pub rust_name: String,
    /// WIT type string.
    pub wit_type: String,
    /// Rust type string.
    pub rust_type: String,
    /// Whether this param is optional.
    pub is_optional: bool,
}

/// A WIT interface grouping functions for a resource.
#[derive(Debug, Clone, Serialize)]
pub struct WitInterface {
    /// `kebab-case` interface name.
    pub wit_name: String,
    /// Doc comment.
    pub doc_comment: String,
    /// Functions in this interface.
    pub functions: Vec<WitFunction>,
}

/// Registry that maps struct names to their definitions, used for
/// deduplication and lookup.
#[derive(Debug, Default)]
pub struct TypeRegistry {
    /// Collected record definitions.
    records: Vec<WitRecord>,
    /// Track seen schema shapes to reuse names.
    seen: HashMap<String, String>,
}

impl TypeRegistry {
    /// Returns the collected records.
    pub fn into_records(self) -> Vec<WitRecord> {
        self.records
    }

    /// Returns a reference to the collected records.
    pub fn records(&self) -> &[WitRecord] {
        &self.records
    }

    /// Register a record definition. Returns the Rust name.
    fn register(&mut self, def: WitRecord) -> String {
        let name = def.rust_name.clone();
        self.records.push(def);
        name
    }
}

// ── Public API ──────────────────────────────────────────────────────

/// Resolve all types from an [`ApiModel`], producing IR records,
/// interfaces, and functions ready for template rendering.
///
/// Returns `(interfaces, registry)` where `registry` holds all generated records.
pub fn resolve_all(model: &ApiModel) -> (Vec<WitInterface>, TypeRegistry) {
    let mut registry = TypeRegistry::default();
    let mut interfaces = Vec::new();

    for resource in &model.resources {
        let iface = resolve_resource(resource, &mut registry);
        interfaces.push(iface);
    }

    (interfaces, registry)
}

// ── Resource resolution ────────────────────────────────────────────

/// Resolve a resource group into a [`WitInterface`].
fn resolve_resource(resource: &ResourceGroup, registry: &mut TypeRegistry) -> WitInterface {
    let mut functions = Vec::new();

    for endpoint in &resource.endpoints {
        let func = resolve_endpoint(resource, endpoint, registry);
        functions.push(func);
    }

    WitInterface {
        wit_name: to_kebab_case(&resource.name),
        doc_comment: format!("Endpoints under {}", resource.base_path),
        functions,
    }
}

// ── Endpoint resolution ─────────────────────────────────────────────

/// Resolve a single endpoint into a [`WitFunction`], registering any
/// new record types into the registry.
fn resolve_endpoint(
    resource: &ResourceGroup,
    endpoint: &Endpoint,
    registry: &mut TypeRegistry,
) -> WitFunction {
    let rust_fn = build_fn_name(&resource.name, &endpoint.method, &endpoint.path_template);
    let wit_fn = to_kebab_case(&rust_fn);

    // Resolve request body type.
    let (request_body_wit, request_body_rust) =
        endpoint
            .request_schema
            .as_ref()
            .map_or((None, None), |schema| {
                let type_name =
                    build_type_name(&resource.name, &endpoint.method, TypeRole::Request);
                let rust_name =
                    resolve_schema_as_named(schema, &type_name, &type_name, registry, 0);
                let wit_name = to_kebab_case(&rust_name);
                (Some(wit_name), Some(rust_name))
            });

    // Resolve response type.
    let (response_wit, response_rust) = endpoint.response_schema.as_ref().map_or_else(
        || ("unit".to_owned(), "()".to_owned()),
        |schema| {
            let base_name = build_type_name(&resource.name, &endpoint.method, TypeRole::Response);
            let rust_type = resolve_schema_type(schema, &base_name, &base_name, registry, 0);
            let wit_type = rust_type_to_wit(&rust_type);
            (wit_type, rust_type)
        },
    );

    // Path params.
    let path_params: Vec<WitParam> = endpoint
        .path_params
        .iter()
        .map(|p| {
            let rust_type = param_type_str(&p.param_type);
            let wit_type = rust_type_to_wit(&rust_type);
            WitParam {
                wit_name: to_kebab_case(&p.name),
                rust_name: to_snake_case(&p.name),
                wit_type,
                rust_type,
                is_optional: false,
            }
        })
        .collect();

    // Query params.
    let query_params: Vec<WitParam> = endpoint
        .query_params
        .iter()
        .map(|p| {
            let base_type = param_type_str(&p.param_type);
            let base_wit = rust_type_to_wit(&base_type);
            if p.required {
                WitParam {
                    wit_name: to_kebab_case(&p.name),
                    rust_name: to_snake_case(&p.name),
                    wit_type: base_wit,
                    rust_type: base_type,
                    is_optional: false,
                }
            } else {
                WitParam {
                    wit_name: to_kebab_case(&p.name),
                    rust_name: to_snake_case(&p.name),
                    wit_type: format!("option<{base_wit}>"),
                    rust_type: format!("Option<{base_type}>"),
                    is_optional: true,
                }
            }
        })
        .collect();

    // Path format string.
    let full_path = normalize_path(&resource.base_path, &endpoint.path_template);
    let path_fmt = path_template_to_format_str(&full_path);

    WitFunction {
        wit_name: wit_fn,
        rust_name: rust_fn,
        doc_comment: endpoint.description.clone(),
        http_method: endpoint.method.to_ascii_lowercase(),
        path_format_str: path_fmt,
        path_params,
        query_params,
        request_body_wit,
        request_body_rust,
        response_wit,
        response_rust,
        requires_auth: endpoint.requires_auth,
    }
}

// ── Schema resolution ───────────────────────────────────────────────

/// Resolve a `JsonSchema` into a Rust type string, potentially registering
/// new records into the registry.
fn resolve_schema_type(
    schema: &JsonSchema,
    parent_name: &str,
    context_name: &str,
    registry: &mut TypeRegistry,
    depth: usize,
) -> String {
    if depth >= MAX_DEPTH {
        return "serde_json::Value".to_owned();
    }

    match schema {
        JsonSchema::Object { .. } => {
            resolve_schema_as_named(schema, parent_name, context_name, registry, depth)
        }
        JsonSchema::Array { items } => {
            let inner = resolve_schema_type(
                items,
                parent_name,
                context_name,
                registry,
                depth.saturating_add(1),
            );
            format!("Vec<{inner}>")
        }
        JsonSchema::String { .. } => "String".to_owned(),
        JsonSchema::Integer => "i64".to_owned(),
        JsonSchema::Number => "f64".to_owned(),
        JsonSchema::Boolean => "bool".to_owned(),
        JsonSchema::Null => "Option<serde_json::Value>".to_owned(),
    }
}

/// Resolve an object `JsonSchema` as a named record, registering it and
/// returning the Rust struct name.
fn resolve_schema_as_named(
    schema: &JsonSchema,
    type_name: &str,
    context_name: &str,
    registry: &mut TypeRegistry,
    depth: usize,
) -> String {
    // For non-object schemas, just return the type string.
    let JsonSchema::Object {
        properties,
        required,
    } = schema
    else {
        return resolve_schema_type(schema, type_name, context_name, registry, depth);
    };

    // Check deduplication by schema fingerprint.
    let fingerprint = schema_fingerprint(schema);
    if let Some(existing) = registry.seen.get(&fingerprint) {
        return existing.clone();
    }

    // Build record fields sorted by name for deterministic output.
    let mut sorted_keys: Vec<&String> = properties.keys().collect();
    sorted_keys.sort();

    let fields: Vec<WitField> = sorted_keys
        .iter()
        .map(|key| {
            let child_schema = properties.get(*key);
            let is_required = required.contains(*key);

            let (rust_field_name, serde_rename) = field_name_and_rename(key);
            let wit_field_name = to_kebab_case(key);

            let nested_name = build_nested_type_name(type_name, key);
            let base_rust_type = child_schema.map_or_else(
                || "serde_json::Value".to_owned(),
                |s| {
                    resolve_schema_type(
                        s,
                        &nested_name,
                        &nested_name,
                        registry,
                        depth.saturating_add(1),
                    )
                },
            );
            let base_wit_type = rust_type_to_wit(&base_rust_type);

            let (final_rust_type, final_wit_type, is_optional) = if is_required {
                (base_rust_type, base_wit_type, false)
            } else {
                (
                    format!("Option<{base_rust_type}>"),
                    format!("option<{base_wit_type}>"),
                    true,
                )
            };

            WitField {
                wit_name: wit_field_name,
                rust_name: rust_field_name,
                wit_type: final_wit_type,
                rust_type: final_rust_type,
                serde_rename,
                is_optional,
            }
        })
        .collect();

    let def = WitRecord {
        wit_name: to_kebab_case(type_name),
        rust_name: type_name.to_owned(),
        doc_comment: format!("Auto-generated type for {context_name}."),
        fields,
    };

    let name = registry.register(def);
    registry.seen.insert(fingerprint, name.clone());
    name
}

/// Produce a deterministic fingerprint for a `JsonSchema` for deduplication.
fn schema_fingerprint(schema: &JsonSchema) -> String {
    serde_json::to_string(schema).unwrap_or_default()
}

/// Map a Rust type string to its WIT equivalent.
fn rust_type_to_wit(rust_type: &str) -> String {
    match rust_type {
        "String" | "serde_json::Value" | "Option<serde_json::Value>" => "string".to_owned(),
        "i64" => "s64".to_owned(),
        "f64" => "f64".to_owned(),
        "bool" => "bool".to_owned(),
        "()" => "unit".to_owned(),
        _ if rust_type.starts_with("Vec<") => {
            // Vec<T> → list<wit-t>
            let inner_end = rust_type.len().saturating_sub(1);
            let inner = rust_type.get(4..inner_end).unwrap_or("string");
            let wit_inner = rust_type_to_wit(inner);
            format!("list<{wit_inner}>")
        }
        _ if rust_type.starts_with("Option<") => {
            // Option<T> → option<wit-t>
            let inner_end = rust_type.len().saturating_sub(1);
            let inner = rust_type.get(7..inner_end).unwrap_or("string");
            let wit_inner = rust_type_to_wit(inner);
            format!("option<{wit_inner}>")
        }
        _ => {
            // Named types (PascalCase Rust) → kebab-case WIT
            to_kebab_case(rust_type)
        }
    }
}

/// Map a param type hint to a Rust type string.
fn param_type_str(param_type: &str) -> String {
    match param_type {
        "integer" | "int" => "i64".to_owned(),
        "number" | "float" | "double" => "f64".to_owned(),
        "boolean" | "bool" => "bool".to_owned(),
        _ => "String".to_owned(),
    }
}

/// Combine base path and path template, avoiding double slashes.
fn normalize_path(base_path: &str, path_template: &str) -> String {
    let base = base_path.trim_end_matches('/');
    let path = path_template.trim_start_matches('/');

    if base.is_empty() && path.is_empty() {
        return "/".to_owned();
    }
    if path.is_empty() {
        return base.to_owned();
    }
    if base.is_empty() {
        return format!("/{path}");
    }
    format!("{base}/{path}")
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use std::collections::HashMap;

    use sdk_forge_session::types::{
        ApiModel, AuthPattern, Endpoint, JsonSchema, PathParam, QueryParam, ResourceGroup,
    };

    use super::{TypeRegistry, resolve_all, resolve_schema_type, rust_type_to_wit};

    /// Minimal `ApiModel` for testing.
    fn sample_model() -> ApiModel {
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
                                    (
                                        "email".to_owned(),
                                        JsonSchema::String {
                                            format: Some("email".to_owned()),
                                        },
                                    ),
                                ]),
                                required: vec![
                                    "id".to_owned(),
                                    "name".to_owned(),
                                    "email".to_owned(),
                                ],
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
                                (
                                    "email".to_owned(),
                                    JsonSchema::String {
                                        format: Some("email".to_owned()),
                                    },
                                ),
                            ]),
                            required: vec!["id".to_owned(), "name".to_owned(), "email".to_owned()],
                        }),
                        requires_auth: true,
                        path_params: vec![PathParam {
                            name: "id".to_owned(),
                            param_type: "integer".to_owned(),
                            description: Some("User ID".to_owned()),
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
                                (
                                    "email".to_owned(),
                                    JsonSchema::String {
                                        format: Some("email".to_owned()),
                                    },
                                ),
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

    #[test]
    fn test_resolve_all_produces_interfaces() {
        let model = sample_model();
        let (interfaces, _registry) = resolve_all(&model);

        assert_eq!(interfaces.len(), 1, "should have 1 interface");
        assert_eq!(interfaces.first().unwrap().wit_name, "users");
        assert_eq!(
            interfaces.first().unwrap().functions.len(),
            3,
            "should have 3 functions"
        );
    }

    #[test]
    fn test_resolve_all_produces_records() {
        let model = sample_model();
        let (_interfaces, registry) = resolve_all(&model);
        let records = registry.into_records();

        assert!(!records.is_empty(), "should generate at least one record");

        let rust_names: Vec<&str> = records.iter().map(|r| r.rust_name.as_str()).collect();
        assert!(
            rust_names.contains(&"User"),
            "should have User record, got: {rust_names:?}"
        );
        assert!(
            rust_names.contains(&"CreateUserRequest"),
            "should have CreateUserRequest, got: {rust_names:?}"
        );
    }

    #[test]
    fn test_wit_names_are_kebab_case() {
        let model = sample_model();
        let (interfaces, registry) = resolve_all(&model);
        let records = registry.into_records();

        // Interface name.
        assert_eq!(interfaces.first().unwrap().wit_name, "users");

        // Function names.
        let fn_names: Vec<&str> = interfaces
            .first()
            .unwrap()
            .functions
            .iter()
            .map(|f| f.wit_name.as_str())
            .collect();
        assert!(fn_names.contains(&"list-users"));
        assert!(fn_names.contains(&"get-user"));
        assert!(fn_names.contains(&"create-user"));

        // Record names.
        let user_rec = records.iter().find(|r| r.rust_name == "User").unwrap();
        assert_eq!(user_rec.wit_name, "user");

        let create_rec = records
            .iter()
            .find(|r| r.rust_name == "CreateUserRequest")
            .unwrap();
        assert_eq!(create_rec.wit_name, "create-user-request");
    }

    #[test]
    fn test_user_record_fields() {
        let model = sample_model();
        let (_interfaces, registry) = resolve_all(&model);
        let records = registry.into_records();

        let user = records.iter().find(|r| r.rust_name == "User");
        assert!(user.is_some(), "User record should exist");

        let user_rec = user.unwrap();
        assert_eq!(user_rec.fields.len(), 3, "User should have 3 fields");

        let id_field = user_rec.fields.iter().find(|f| f.rust_name == "id");
        assert!(id_field.is_some(), "should have id field");
        assert_eq!(id_field.unwrap().rust_type, "i64");
        assert_eq!(id_field.unwrap().wit_type, "s64");
        assert!(!id_field.unwrap().is_optional, "id should be required");
    }

    #[test]
    fn test_optional_fields() {
        let schema = JsonSchema::Object {
            properties: HashMap::from([
                (
                    "required_field".to_owned(),
                    JsonSchema::String { format: None },
                ),
                ("optional_field".to_owned(), JsonSchema::Integer),
            ]),
            required: vec!["required_field".to_owned()],
        };

        let mut registry = TypeRegistry::default();
        let type_name =
            super::resolve_schema_as_named(&schema, "TestType", "TestType", &mut registry, 0);

        assert_eq!(type_name, "TestType");
        let records = registry.into_records();
        let test_rec = records.first().unwrap();

        let opt_field = test_rec
            .fields
            .iter()
            .find(|f| f.rust_name == "optional_field")
            .unwrap();
        assert!(
            opt_field.is_optional,
            "non-required field should be optional"
        );
        assert_eq!(opt_field.rust_type, "Option<i64>");
        assert_eq!(opt_field.wit_type, "option<s64>");

        let req_field = test_rec
            .fields
            .iter()
            .find(|f| f.rust_name == "required_field")
            .unwrap();
        assert!(
            !req_field.is_optional,
            "required field should not be optional"
        );
        assert_eq!(req_field.rust_type, "String");
        assert_eq!(req_field.wit_type, "string");
    }

    #[test]
    fn test_nested_object_creates_record() {
        let schema = JsonSchema::Object {
            properties: HashMap::from([(
                "address".to_owned(),
                JsonSchema::Object {
                    properties: HashMap::from([
                        ("street".to_owned(), JsonSchema::String { format: None }),
                        ("city".to_owned(), JsonSchema::String { format: None }),
                    ]),
                    required: vec!["street".to_owned(), "city".to_owned()],
                },
            )]),
            required: vec!["address".to_owned()],
        };

        let mut registry = TypeRegistry::default();
        super::resolve_schema_as_named(&schema, "User", "User", &mut registry, 0);
        let records = registry.into_records();

        let names: Vec<&str> = records.iter().map(|r| r.rust_name.as_str()).collect();
        assert!(
            names.contains(&"UserAddress"),
            "should create nested UserAddress record"
        );
        assert!(names.contains(&"User"), "should create User record");
    }

    #[test]
    fn test_array_type_resolution() {
        let schema = JsonSchema::Array {
            items: Box::new(JsonSchema::Integer),
        };

        let mut registry = TypeRegistry::default();
        let type_str = resolve_schema_type(&schema, "Items", "Items", &mut registry, 0);
        assert_eq!(type_str, "Vec<i64>");
    }

    #[test]
    fn test_primitive_types() {
        let mut registry = TypeRegistry::default();

        assert_eq!(
            resolve_schema_type(
                &JsonSchema::String { format: None },
                "T",
                "T",
                &mut registry,
                0
            ),
            "String"
        );
        assert_eq!(
            resolve_schema_type(&JsonSchema::Integer, "T", "T", &mut registry, 0),
            "i64"
        );
        assert_eq!(
            resolve_schema_type(&JsonSchema::Number, "T", "T", &mut registry, 0),
            "f64"
        );
        assert_eq!(
            resolve_schema_type(&JsonSchema::Boolean, "T", "T", &mut registry, 0),
            "bool"
        );
        assert_eq!(
            resolve_schema_type(&JsonSchema::Null, "T", "T", &mut registry, 0),
            "Option<serde_json::Value>"
        );
    }

    #[test]
    fn test_rust_type_to_wit() {
        assert_eq!(rust_type_to_wit("String"), "string");
        assert_eq!(rust_type_to_wit("i64"), "s64");
        assert_eq!(rust_type_to_wit("f64"), "f64");
        assert_eq!(rust_type_to_wit("bool"), "bool");
        assert_eq!(rust_type_to_wit("()"), "unit");
        assert_eq!(rust_type_to_wit("Vec<i64>"), "list<s64>");
        assert_eq!(rust_type_to_wit("Vec<String>"), "list<string>");
        assert_eq!(rust_type_to_wit("Option<String>"), "option<string>");
        assert_eq!(rust_type_to_wit("User"), "user");
        assert_eq!(rust_type_to_wit("CreateUserRequest"), "create-user-request");
        assert_eq!(rust_type_to_wit("Vec<User>"), "list<user>");
    }

    #[test]
    fn test_method_path_format() {
        let model = sample_model();
        let (interfaces, _) = resolve_all(&model);

        let funcs = &interfaces.first().unwrap().functions;
        let get_user = funcs.iter().find(|f| f.rust_name == "get_user").unwrap();
        assert_eq!(
            get_user.path_format_str, "/api/v1/users/{}",
            "path param should be replaced with {{}}"
        );
        assert_eq!(get_user.path_params.len(), 1, "should have 1 path param");
        assert_eq!(get_user.path_params.first().unwrap().rust_type, "i64");
        assert_eq!(get_user.path_params.first().unwrap().wit_type, "s64");
    }

    #[test]
    fn test_query_params_optional() {
        let model = sample_model();
        let (interfaces, _) = resolve_all(&model);

        let funcs = &interfaces.first().unwrap().functions;
        let list = funcs.iter().find(|f| f.rust_name == "list_users").unwrap();
        assert_eq!(list.query_params.len(), 1, "should have 1 query param");

        let page = list.query_params.first().unwrap();
        assert!(page.is_optional, "page should be optional");
        assert_eq!(page.rust_type, "Option<i64>");
        assert_eq!(page.wit_type, "option<s64>");
    }

    #[test]
    fn test_request_body_type() {
        let model = sample_model();
        let (interfaces, _) = resolve_all(&model);

        let funcs = &interfaces.first().unwrap().functions;
        let create = funcs.iter().find(|f| f.rust_name == "create_user").unwrap();
        assert_eq!(
            create.request_body_rust.as_deref(),
            Some("CreateUserRequest"),
            "POST should have request body type"
        );
        assert_eq!(
            create.request_body_wit.as_deref(),
            Some("create-user-request"),
            "POST should have WIT request body type"
        );
    }

    #[test]
    fn test_no_response_schema_returns_unit() {
        let model = ApiModel {
            base_url: "https://api.example.com".to_owned(),
            auth: None,
            resources: vec![ResourceGroup {
                name: "Items".to_owned(),
                base_path: "/items".to_owned(),
                endpoints: vec![Endpoint {
                    method: "DELETE".to_owned(),
                    path_template: "/{id}".to_owned(),
                    description: "Delete item".to_owned(),
                    request_schema: None,
                    response_schema: None,
                    requires_auth: false,
                    path_params: vec![PathParam {
                        name: "id".to_owned(),
                        param_type: "integer".to_owned(),
                        description: None,
                    }],
                    query_params: Vec::new(),
                }],
            }],
            pagination: None,
        };

        let (interfaces, _) = resolve_all(&model);
        let func = interfaces.first().unwrap().functions.first().unwrap();
        assert_eq!(func.response_rust, "()", "no schema means unit response");
        assert_eq!(func.response_wit, "unit", "no schema means unit WIT type");
    }

    #[test]
    fn test_deduplication_reuses_record() {
        let shared_schema = JsonSchema::Object {
            properties: HashMap::from([
                ("id".to_owned(), JsonSchema::Integer),
                ("name".to_owned(), JsonSchema::String { format: None }),
            ]),
            required: vec!["id".to_owned(), "name".to_owned()],
        };

        let model = ApiModel {
            base_url: "https://api.example.com".to_owned(),
            auth: None,
            resources: vec![ResourceGroup {
                name: "Items".to_owned(),
                base_path: "/items".to_owned(),
                endpoints: vec![
                    Endpoint {
                        method: "GET".to_owned(),
                        path_template: "/".to_owned(),
                        description: "List".to_owned(),
                        request_schema: None,
                        response_schema: Some(JsonSchema::Array {
                            items: Box::new(shared_schema.clone()),
                        }),
                        requires_auth: false,
                        path_params: Vec::new(),
                        query_params: Vec::new(),
                    },
                    Endpoint {
                        method: "GET".to_owned(),
                        path_template: "/{id}".to_owned(),
                        description: "Get".to_owned(),
                        request_schema: None,
                        response_schema: Some(shared_schema),
                        requires_auth: false,
                        path_params: vec![PathParam {
                            name: "id".to_owned(),
                            param_type: "integer".to_owned(),
                            description: None,
                        }],
                        query_params: Vec::new(),
                    },
                ],
            }],
            pagination: None,
        };

        let (interfaces, registry) = resolve_all(&model);
        let records = registry.into_records();

        // Both endpoints share the same schema; only one record should be generated.
        let item_records: Vec<_> = records.iter().filter(|r| r.rust_name == "Item").collect();
        assert_eq!(
            item_records.len(),
            1,
            "identical schemas should be deduplicated"
        );

        // Both functions should reference the same type.
        let funcs = &interfaces.first().unwrap().functions;
        assert_eq!(funcs.first().unwrap().response_rust, "Vec<Item>");
        assert_eq!(funcs.first().unwrap().response_wit, "list<item>");
        assert!(funcs.get(1).is_some());
    }

    #[test]
    fn test_normalize_path() {
        assert_eq!(super::normalize_path("/api/v1", "/users"), "/api/v1/users");
        assert_eq!(super::normalize_path("/api/v1/", "/users"), "/api/v1/users");
        assert_eq!(super::normalize_path("/api/v1", "users"), "/api/v1/users");
        assert_eq!(super::normalize_path("", "users"), "/users");
        assert_eq!(super::normalize_path("", ""), "/");
        assert_eq!(super::normalize_path("/api", ""), "/api");
    }

    #[test]
    fn test_param_type_str() {
        assert_eq!(super::param_type_str("integer"), "i64");
        assert_eq!(super::param_type_str("number"), "f64");
        assert_eq!(super::param_type_str("boolean"), "bool");
        assert_eq!(super::param_type_str("string"), "String");
        assert_eq!(super::param_type_str("unknown"), "String");
    }
}
