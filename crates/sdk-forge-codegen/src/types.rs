//! Intermediate representation types and `JsonSchema` → Rust type conversion.
//!
//! Defines the IR that sits between [`ApiModel`](sdk_forge_session::types::ApiModel)
//! and Tera templates. Templates receive pre-computed, valid Rust identifiers
//! and type strings — no naming logic belongs in templates.

use std::collections::HashMap;

use sdk_forge_session::types::{ApiModel, Endpoint, JsonSchema, ResourceGroup};
use serde::Serialize;

use crate::naming::{
    TypeRole, build_fn_name, build_nested_type_name, build_type_name, field_name_and_rename,
    path_template_to_format_str, to_snake_case,
};

/// Maximum nesting depth before falling back to `serde_json::Value`.
const MAX_DEPTH: usize = 10;

// ── IR types ────────────────────────────────────────────────────────

/// A Rust struct ready for template rendering.
#[derive(Debug, Clone, Serialize)]
pub struct RustStruct {
    /// `PascalCase` struct name.
    pub name: String,
    /// Doc comment for the struct.
    pub doc_comment: String,
    /// Ordered list of fields.
    pub fields: Vec<RustField>,
}

/// A single field within a [`RustStruct`].
#[derive(Debug, Clone, Serialize)]
pub struct RustField {
    /// `snake_case` field name (may be `r#`-escaped).
    pub name: String,
    /// Fully resolved Rust type string (e.g. `"Option<String>"`).
    pub rust_type: String,
    /// Original JSON key if it differs from `name` (for `#[serde(rename)]`).
    pub serde_rename: Option<String>,
    /// Whether this field wraps `Option<T>`.
    pub is_optional: bool,
}

/// Metadata for a generated endpoint method.
#[derive(Debug, Clone, Serialize)]
pub struct EndpointMethod {
    /// `snake_case` function name.
    pub fn_name: String,
    /// Doc comment describing the endpoint.
    pub doc_comment: String,
    /// Lowercase HTTP method (e.g. `"get"`, `"post"`).
    pub http_method: String,
    /// Rust `format!()` string for the URL path.
    pub path_format_str: String,
    /// Path parameters in order.
    pub path_params: Vec<MethodParam>,
    /// Query string parameters.
    pub query_params: Vec<MethodParam>,
    /// Type name for the request body, if any.
    pub request_body_type: Option<String>,
    /// Type name for the response (e.g. `"User"`, `"Vec<User>"`, `"()"`).
    pub response_type: String,
    /// Whether this endpoint requires authentication.
    pub requires_auth: bool,
}

/// A parameter on a generated method signature.
#[derive(Debug, Clone, Serialize)]
pub struct MethodParam {
    /// `snake_case` parameter name.
    pub name: String,
    /// Rust type string.
    pub rust_type: String,
    /// Whether this param is optional (wraps `Option<T>`).
    pub is_optional: bool,
}

/// Registry that maps struct names to their definitions, used for
/// deduplication and lookup.
#[derive(Debug, Default)]
pub struct TypeRegistry {
    /// Map from struct name to its definition.
    structs: Vec<RustStruct>,
    /// Track seen schema shapes to reuse names.
    seen: HashMap<String, String>,
}

impl TypeRegistry {
    /// Returns the collected structs.
    pub fn into_structs(self) -> Vec<RustStruct> {
        self.structs
    }

    /// Returns a reference to the collected structs.
    pub fn structs(&self) -> &[RustStruct] {
        &self.structs
    }

    /// Register a struct definition. Returns the name.
    fn register(&mut self, def: RustStruct) -> String {
        let name = def.name.clone();
        self.structs.push(def);
        name
    }
}

// ── Public API ──────────────────────────────────────────────────────

/// Resolve all types from an [`ApiModel`], producing IR structs and
/// endpoint methods ready for template rendering.
///
/// Returns `(methods, registry)` where `registry` holds all generated structs.
pub fn resolve_all(model: &ApiModel) -> (Vec<EndpointMethod>, TypeRegistry) {
    let mut registry = TypeRegistry::default();
    let mut methods = Vec::new();

    for resource in &model.resources {
        for endpoint in &resource.endpoints {
            let method = resolve_endpoint(resource, endpoint, &mut registry);
            methods.push(method);
        }
    }

    (methods, registry)
}

// ── Endpoint resolution ─────────────────────────────────────────────

/// Resolve a single endpoint into an [`EndpointMethod`], registering any
/// new struct types into the registry.
fn resolve_endpoint(
    resource: &ResourceGroup,
    endpoint: &Endpoint,
    registry: &mut TypeRegistry,
) -> EndpointMethod {
    let fn_name = build_fn_name(&resource.name, &endpoint.method, &endpoint.path_template);

    // Resolve request body type.
    let request_body_type = endpoint.request_schema.as_ref().map(|schema| {
        let type_name = build_type_name(&resource.name, &endpoint.method, TypeRole::Request);
        resolve_schema_as_named(schema, &type_name, &type_name, registry, 0)
    });

    // Resolve response type.
    let response_type = endpoint.response_schema.as_ref().map_or_else(
        || "()".to_owned(),
        |schema| {
            let base_name = build_type_name(&resource.name, &endpoint.method, TypeRole::Response);
            resolve_schema_type(schema, &base_name, &base_name, registry, 0)
        },
    );

    // Path params.
    let path_params: Vec<MethodParam> = endpoint
        .path_params
        .iter()
        .map(|p| MethodParam {
            name: to_snake_case(&p.name),
            rust_type: param_type_str(&p.param_type),
            is_optional: false,
        })
        .collect();

    // Query params.
    let query_params: Vec<MethodParam> = endpoint
        .query_params
        .iter()
        .map(|p| {
            let base_type = param_type_str(&p.param_type);
            if p.required {
                MethodParam {
                    name: to_snake_case(&p.name),
                    rust_type: base_type,
                    is_optional: false,
                }
            } else {
                MethodParam {
                    name: to_snake_case(&p.name),
                    rust_type: format!("Option<{base_type}>"),
                    is_optional: true,
                }
            }
        })
        .collect();

    // Path format string.
    let full_path = normalize_path(&resource.base_path, &endpoint.path_template);
    let path_fmt = path_template_to_format_str(&full_path);

    EndpointMethod {
        fn_name,
        doc_comment: endpoint.description.clone(),
        http_method: endpoint.method.to_ascii_lowercase(),
        path_format_str: path_fmt,
        path_params,
        query_params,
        request_body_type,
        response_type,
        requires_auth: endpoint.requires_auth,
    }
}

// ── Schema resolution ───────────────────────────────────────────────

/// Resolve a `JsonSchema` into a Rust type string, potentially registering
/// new structs into the registry.
///
/// For objects, this creates a named struct. For arrays, wraps in `Vec<>`.
/// For primitives, returns the type string directly.
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

/// Resolve an object `JsonSchema` as a named struct, registering it and
/// returning the struct name.
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

    // Build struct fields sorted by name for deterministic output.
    let mut sorted_keys: Vec<&String> = properties.keys().collect();
    sorted_keys.sort();

    let fields: Vec<RustField> = sorted_keys
        .iter()
        .map(|key| {
            let child_schema = properties.get(*key);
            let is_required = required.contains(*key);

            let (field_name, serde_rename) = field_name_and_rename(key);

            let nested_name = build_nested_type_name(type_name, key);
            let base_type = child_schema.map_or_else(
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

            let (final_type, is_optional) = if is_required {
                (base_type, false)
            } else {
                (format!("Option<{base_type}>"), true)
            };

            RustField {
                name: field_name,
                rust_type: final_type,
                serde_rename,
                is_optional,
            }
        })
        .collect();

    let def = RustStruct {
        name: type_name.to_owned(),
        doc_comment: format!("Auto-generated type for {context_name}."),
        fields,
    };

    let name = registry.register(def);
    registry.seen.insert(fingerprint, name.clone());
    name
}

/// Produce a deterministic fingerprint for a `JsonSchema` for deduplication.
///
/// Uses JSON serialization of the schema as the fingerprint.
fn schema_fingerprint(schema: &JsonSchema) -> String {
    serde_json::to_string(schema).unwrap_or_default()
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

    use super::{TypeRegistry, resolve_all, resolve_schema_type};

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
    fn test_resolve_all_produces_methods() {
        let model = sample_model();
        let (methods, _registry) = resolve_all(&model);

        assert_eq!(methods.len(), 3, "should have 3 methods");

        let names: Vec<&str> = methods.iter().map(|m| m.fn_name.as_str()).collect();
        assert!(names.contains(&"list_users"), "should have list_users");
        assert!(names.contains(&"get_user"), "should have get_user");
        assert!(names.contains(&"create_user"), "should have create_user");
    }

    #[test]
    fn test_resolve_all_produces_structs() {
        let model = sample_model();
        let (_methods, registry) = resolve_all(&model);
        let structs = registry.into_structs();

        assert!(!structs.is_empty(), "should generate at least one struct");

        let struct_names: Vec<&str> = structs.iter().map(|s| s.name.as_str()).collect();
        assert!(
            struct_names.contains(&"User"),
            "should have User struct, got: {struct_names:?}"
        );
        assert!(
            struct_names.contains(&"CreateUserRequest"),
            "should have CreateUserRequest, got: {struct_names:?}"
        );
    }

    #[test]
    fn test_user_struct_fields() {
        let model = sample_model();
        let (_methods, registry) = resolve_all(&model);
        let structs = registry.into_structs();

        let user = structs.iter().find(|s| s.name == "User");
        assert!(user.is_some(), "User struct should exist");

        let user_struct = user.unwrap();
        assert_eq!(user_struct.fields.len(), 3, "User should have 3 fields");

        let id_field = user_struct.fields.iter().find(|f| f.name == "id");
        assert!(id_field.is_some(), "should have id field");
        assert_eq!(id_field.unwrap().rust_type, "i64");
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
        let structs = registry.into_structs();
        let test_struct = structs.first().unwrap();

        let opt_field = test_struct
            .fields
            .iter()
            .find(|f| f.name == "optional_field")
            .unwrap();
        assert!(
            opt_field.is_optional,
            "non-required field should be optional"
        );
        assert_eq!(opt_field.rust_type, "Option<i64>");

        let req_field = test_struct
            .fields
            .iter()
            .find(|f| f.name == "required_field")
            .unwrap();
        assert!(
            !req_field.is_optional,
            "required field should not be optional"
        );
        assert_eq!(req_field.rust_type, "String");
    }

    #[test]
    fn test_nested_object_creates_struct() {
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
        let structs = registry.into_structs();

        let names: Vec<&str> = structs.iter().map(|s| s.name.as_str()).collect();
        assert!(
            names.contains(&"UserAddress"),
            "should create nested UserAddress struct"
        );
        assert!(names.contains(&"User"), "should create User struct");
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
    fn test_method_path_format() {
        let model = sample_model();
        let (methods, _) = resolve_all(&model);

        let get_user = methods.iter().find(|m| m.fn_name == "get_user").unwrap();
        assert_eq!(
            get_user.path_format_str, "/api/v1/users/{}",
            "path param should be replaced with {{}}"
        );
        assert_eq!(get_user.path_params.len(), 1, "should have 1 path param");
        assert_eq!(get_user.path_params.first().unwrap().rust_type, "i64");
    }

    #[test]
    fn test_query_params_optional() {
        let model = sample_model();
        let (methods, _) = resolve_all(&model);

        let list = methods.iter().find(|m| m.fn_name == "list_users").unwrap();
        assert_eq!(list.query_params.len(), 1, "should have 1 query param");

        let page = list.query_params.first().unwrap();
        assert!(page.is_optional, "page should be optional");
        assert_eq!(page.rust_type, "Option<i64>");
    }

    #[test]
    fn test_request_body_type() {
        let model = sample_model();
        let (methods, _) = resolve_all(&model);

        let create = methods.iter().find(|m| m.fn_name == "create_user").unwrap();
        assert_eq!(
            create.request_body_type.as_deref(),
            Some("CreateUserRequest"),
            "POST should have request body type"
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

        let (methods, _) = resolve_all(&model);
        let delete = methods.first().unwrap();
        assert_eq!(delete.response_type, "()", "no schema means unit response");
    }

    #[test]
    fn test_deduplication_reuses_struct() {
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

        let (methods, registry) = resolve_all(&model);
        let structs = registry.into_structs();

        // Both endpoints share the same schema; only one struct should be generated.
        let item_structs: Vec<_> = structs.iter().filter(|s| s.name == "Item").collect();
        assert_eq!(
            item_structs.len(),
            1,
            "identical schemas should be deduplicated"
        );

        // Both methods should reference the same type.
        assert_eq!(methods.first().unwrap().response_type, "Vec<Item>");
        assert!(methods.get(1).is_some());
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
