//! Component context generation for WIT/WASI templates.
//!
//! Builds the context for the generated Wasm component's world definition
//! and Rust implementation.

use serde::Serialize;

use crate::auth::AuthInfo;
use crate::types::WitInterface;

/// Context for rendering the WIT world and Rust component implementation.
#[derive(Debug, Clone, Serialize)]
pub struct ComponentContext {
    /// Base URL of the API.
    pub base_url: String,
    /// Component/package name in kebab-case.
    pub package_name: String,
    /// Auth information, if auth was detected.
    pub auth: Option<AuthInfo>,
    /// Whether any endpoint requires auth.
    pub has_auth: bool,
    /// WIT interfaces (one per resource group).
    pub interfaces: Vec<WitInterface>,
}

/// Build the component template context.
pub fn build_component_context(
    base_url: &str,
    package_name: &str,
    auth: Option<AuthInfo>,
    interfaces: Vec<WitInterface>,
) -> ComponentContext {
    let has_auth = auth.is_some();
    ComponentContext {
        base_url: base_url.to_owned(),
        package_name: package_name.to_owned(),
        auth,
        has_auth,
        interfaces,
    }
}

#[cfg(test)]
mod tests {
    use crate::auth::build_auth_info;

    use super::build_component_context;

    #[test]
    fn test_component_context_with_auth() {
        let auth = build_auth_info(&Some(sdk_forge_session::types::AuthPattern::BearerToken));
        let ctx = build_component_context("https://api.example.com", "my-api", auth, Vec::new());

        assert!(ctx.has_auth, "should have auth");
        assert!(ctx.auth.is_some());
        assert_eq!(ctx.base_url, "https://api.example.com");
        assert_eq!(ctx.package_name, "my-api");
    }

    #[test]
    fn test_component_context_without_auth() {
        let ctx = build_component_context("https://api.example.com", "my-api", None, Vec::new());
        assert!(!ctx.has_auth, "should not have auth");
        assert!(ctx.auth.is_none());
    }
}
