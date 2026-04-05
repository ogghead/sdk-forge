//! Naming convention utilities for code generation.
//!
//! Centralizes all identifier transformations so that templates receive
//! pre-computed, valid Rust identifiers.

use heck::{ToSnakeCase, ToUpperCamelCase};

/// Rust reserved keywords that must be escaped with `r#` when used as identifiers.
const RESERVED_KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern",
    "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub",
    "ref", "return", "self", "Self", "static", "struct", "super", "trait", "true", "type", "union",
    "unsafe", "use", "where", "while", "yield",
];

/// The role a type plays relative to its endpoint, used for name generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeRole {
    /// Response body type.
    Response,
    /// Request body type.
    Request,
    /// Nested object inside a parent type.
    Nested,
}

/// Convert a string to `PascalCase` (upper camel case).
///
/// Uses the `heck` crate for robust word boundary detection.
pub fn to_pascal_case(s: &str) -> String {
    s.to_upper_camel_case()
}

/// Convert a string to `snake_case`.
///
/// Uses the `heck` crate for robust word boundary detection.
pub fn to_snake_case(s: &str) -> String {
    s.to_snake_case()
}

/// Naive singularization for English nouns.
///
/// Handles common suffixes: "ies" → "y", "ses" → "s", "s" → "".
/// Returns the input unchanged if it doesn't end in "s".
pub fn singularize(s: &str) -> String {
    if s.len() < 3 {
        return s.to_owned();
    }

    // "categories" → "category"
    if s.ends_with("ies") {
        let prefix_end = s.len().saturating_sub(3);
        if let Some(prefix) = s.get(..prefix_end) {
            return format!("{prefix}y");
        }
    }

    // "addresses" → "address", "statuses" → "status"
    if s.ends_with("ses") || s.ends_with("xes") || s.ends_with("zes") {
        let prefix_end = s.len().saturating_sub(2);
        if let Some(prefix) = s.get(..prefix_end) {
            return prefix.to_owned();
        }
    }

    // "users" → "user" (but not "status" → "statu")
    if s.ends_with('s') && !s.ends_with("ss") && !s.ends_with("us") {
        let prefix_end = s.len().saturating_sub(1);
        if let Some(prefix) = s.get(..prefix_end) {
            return prefix.to_owned();
        }
    }

    s.to_owned()
}

/// Escape a Rust reserved keyword with `r#` prefix.
///
/// Returns the input unchanged if it's not a reserved keyword.
pub fn escape_reserved(s: &str) -> String {
    if RESERVED_KEYWORDS.contains(&s) {
        format!("r#{s}")
    } else {
        s.to_owned()
    }
}

/// Build a `PascalCase` type name for a generated struct.
///
/// Derives the name from the resource group, HTTP method, and the type's role.
///
/// Examples:
/// - `("Users", "GET", Response)` → `"User"`
/// - `("Users", "POST", Request)` → `"CreateUserRequest"`
/// - `("Users", "PUT", Request)` → `"UpdateUserRequest"`
/// - `("Users", "GET", Response)` with list → same `"User"` (caller wraps in `Vec<>`)
pub fn build_type_name(resource: &str, method: &str, role: TypeRole) -> String {
    let singular = singularize(&to_snake_case(resource));
    let pascal = to_pascal_case(&singular);

    match role {
        TypeRole::Request => {
            let verb = match method.to_ascii_uppercase().as_str() {
                "POST" => "Create",
                "PUT" | "PATCH" => "Update",
                "DELETE" => "Delete",
                _ => "Send",
            };
            format!("{verb}{pascal}Request")
        }
        TypeRole::Response | TypeRole::Nested => pascal,
    }
}

/// Build a nested type name from a parent type name and a field name.
///
/// Example: `("User", "address")` → `"UserAddress"`
pub fn build_nested_type_name(parent: &str, field: &str) -> String {
    let field_pascal = to_pascal_case(field);
    format!("{parent}{field_pascal}")
}

/// Build a `snake_case` function name for an endpoint method.
///
/// Derives the name from the resource group, HTTP method, and path template.
///
/// Examples:
/// - `("Users", "GET", "/")` → `"list_users"`
/// - `("Users", "GET", "/{id}")` → `"get_user"`
/// - `("Users", "POST", "/")` → `"create_user"`
/// - `("Users", "DELETE", "/{id}")` → `"delete_user"`
pub fn build_fn_name(resource: &str, method: &str, path_template: &str) -> String {
    let resource_snake = to_snake_case(resource);
    let singular = singularize(&resource_snake);
    let has_path_param = path_template.contains('{');

    let method_upper = method.to_ascii_uppercase();
    let verb = match method_upper.as_str() {
        "GET" if has_path_param => "get",
        "GET" => "list",
        "POST" => "create",
        "PUT" | "PATCH" => "update",
        "DELETE" => "delete",
        "HEAD" => "head",
        "OPTIONS" => "options",
        other => return to_snake_case(&format!("{other}_{resource_snake}")),
    };

    // "list" uses plural, everything else uses singular.
    if verb == "list" {
        format!("{verb}_{resource_snake}")
    } else {
        format!("{verb}_{singular}")
    }
}

/// Convert a path template's parameter name to a Rust `format!()` placeholder.
///
/// `"/users/{id}/posts/{post_id}"` → `"/users/{}/posts/{}"`
pub fn path_template_to_format_str(path_template: &str) -> String {
    let mut result = String::with_capacity(path_template.len());
    let mut in_brace = false;

    for ch in path_template.chars() {
        match ch {
            '{' => {
                in_brace = true;
                result.push('{');
            }
            '}' => {
                in_brace = false;
                result.push('}');
            }
            _ => {
                if !in_brace {
                    result.push(ch);
                }
            }
        }
    }

    result
}

/// Convert a field name from JSON (potentially camelCase or other) to `snake_case`,
/// and return the original name as a serde rename if they differ.
pub fn field_name_and_rename(json_key: &str) -> (String, Option<String>) {
    let snake = to_snake_case(json_key);
    let escaped = escape_reserved(&snake);

    if escaped == json_key {
        (escaped, None)
    } else {
        (escaped, Some(json_key.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    // ── to_pascal_case ──────────────────────────────────────────

    #[test]
    fn test_pascal_case_from_snake() {
        assert_eq!(to_pascal_case("user_profiles"), "UserProfiles");
    }

    #[test]
    fn test_pascal_case_from_kebab() {
        assert_eq!(to_pascal_case("user-profiles"), "UserProfiles");
    }

    #[test]
    fn test_pascal_case_already_pascal() {
        assert_eq!(to_pascal_case("UserProfiles"), "UserProfiles");
    }

    // ── to_snake_case ───────────────────────────────────────────

    #[test]
    fn test_snake_case_from_pascal() {
        assert_eq!(to_snake_case("UserProfiles"), "user_profiles");
    }

    #[test]
    fn test_snake_case_from_camel() {
        assert_eq!(to_snake_case("userProfiles"), "user_profiles");
    }

    // ── singularize ─────────────────────────────────────────────

    #[test]
    fn test_singularize_regular() {
        assert_eq!(singularize("users"), "user");
        assert_eq!(singularize("posts"), "post");
        assert_eq!(singularize("items"), "item");
    }

    #[test]
    fn test_singularize_ies() {
        assert_eq!(singularize("categories"), "category");
        assert_eq!(singularize("entries"), "entry");
    }

    #[test]
    fn test_singularize_ses() {
        assert_eq!(singularize("addresses"), "address");
        assert_eq!(singularize("statuses"), "status");
    }

    #[test]
    fn test_singularize_already_singular() {
        assert_eq!(singularize("user"), "user");
        assert_eq!(singularize("status"), "status");
    }

    #[test]
    fn test_singularize_short() {
        assert_eq!(singularize("us"), "us");
        assert_eq!(singularize("a"), "a");
    }

    // ── escape_reserved ─────────────────────────────────────────

    #[test]
    fn test_escape_reserved_keyword() {
        assert_eq!(escape_reserved("type"), "r#type");
        assert_eq!(escape_reserved("self"), "r#self");
        assert_eq!(escape_reserved("async"), "r#async");
    }

    #[test]
    fn test_escape_non_reserved() {
        assert_eq!(escape_reserved("name"), "name");
        assert_eq!(escape_reserved("user_id"), "user_id");
    }

    // ── build_type_name ─────────────────────────────────────────

    #[test]
    fn test_type_name_response() {
        assert_eq!(
            build_type_name("Users", "GET", TypeRole::Response),
            "User",
            "GET response singularizes"
        );
    }

    #[test]
    fn test_type_name_post_request() {
        assert_eq!(
            build_type_name("Users", "POST", TypeRole::Request),
            "CreateUserRequest"
        );
    }

    #[test]
    fn test_type_name_put_request() {
        assert_eq!(
            build_type_name("Users", "PUT", TypeRole::Request),
            "UpdateUserRequest"
        );
    }

    #[test]
    fn test_type_name_patch_request() {
        assert_eq!(
            build_type_name("Users", "PATCH", TypeRole::Request),
            "UpdateUserRequest"
        );
    }

    #[test]
    fn test_type_name_delete_request() {
        assert_eq!(
            build_type_name("Users", "DELETE", TypeRole::Request),
            "DeleteUserRequest"
        );
    }

    // ── build_nested_type_name ──────────────────────────────────

    #[test]
    fn test_nested_type_name() {
        assert_eq!(build_nested_type_name("User", "address"), "UserAddress");
        assert_eq!(
            build_nested_type_name("User", "billing_info"),
            "UserBillingInfo"
        );
    }

    // ── build_fn_name ───────────────────────────────────────────

    #[test]
    fn test_fn_name_list() {
        assert_eq!(
            build_fn_name("Users", "GET", "/"),
            "list_users",
            "GET root should use list + plural"
        );
    }

    #[test]
    fn test_fn_name_get_by_id() {
        assert_eq!(
            build_fn_name("Users", "GET", "/{id}"),
            "get_user",
            "GET with param should use get + singular"
        );
    }

    #[test]
    fn test_fn_name_create() {
        assert_eq!(build_fn_name("Users", "POST", "/"), "create_user");
    }

    #[test]
    fn test_fn_name_update() {
        assert_eq!(build_fn_name("Users", "PUT", "/{id}"), "update_user");
    }

    #[test]
    fn test_fn_name_delete() {
        assert_eq!(build_fn_name("Users", "DELETE", "/{id}"), "delete_user");
    }

    // ── path_template_to_format_str ─────────────────────────────

    #[test]
    fn test_path_template_simple() {
        assert_eq!(path_template_to_format_str("/users/{id}"), "/users/{}");
    }

    #[test]
    fn test_path_template_multiple_params() {
        assert_eq!(
            path_template_to_format_str("/users/{user_id}/posts/{post_id}"),
            "/users/{}/posts/{}"
        );
    }

    #[test]
    fn test_path_template_no_params() {
        assert_eq!(path_template_to_format_str("/users"), "/users");
    }

    // ── field_name_and_rename ───────────────────────────────────

    #[test]
    fn test_field_name_no_rename_needed() {
        let (name, rename) = field_name_and_rename("user_id");
        assert_eq!(name, "user_id");
        assert!(rename.is_none(), "no rename for already-snake-case");
    }

    #[test]
    fn test_field_name_camel_case() {
        let (name, rename) = field_name_and_rename("userId");
        assert_eq!(name, "user_id");
        assert_eq!(rename, Some("userId".to_owned()));
    }

    #[test]
    fn test_field_name_reserved_keyword() {
        let (name, rename) = field_name_and_rename("type");
        assert_eq!(name, "r#type");
        assert_eq!(rename, Some("type".to_owned()));
    }
}
