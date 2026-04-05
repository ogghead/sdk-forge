//! `Cargo.toml` generation for the output SDK crate.
//!
//! Produces a complete `Cargo.toml` with the correct dependencies
//! for the generated SDK, using `rquest` as the HTTP client.

use serde::Serialize;

/// Context for rendering `cargo_toml.tera`.
#[derive(Debug, Clone, Serialize)]
pub struct CargoTomlContext {
    /// Crate name for the generated SDK (e.g. `"my-api-sdk"`).
    pub crate_name: String,
    /// Base URL of the API, used in the package description.
    pub base_url: String,
    /// HTTP client crate name (`"rquest"`).
    pub http_client_crate: String,
}

/// Build the `Cargo.toml` template context.
pub fn build_cargo_toml_context(
    crate_name: &str,
    base_url: &str,
    http_client_crate: &str,
) -> CargoTomlContext {
    CargoTomlContext {
        crate_name: crate_name.to_owned(),
        base_url: base_url.to_owned(),
        http_client_crate: http_client_crate.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::build_cargo_toml_context;

    #[test]
    fn test_cargo_toml_context() {
        let ctx = build_cargo_toml_context("my-api", "https://api.example.com", "rquest");
        assert_eq!(ctx.crate_name, "my-api");
        assert_eq!(ctx.base_url, "https://api.example.com");
        assert_eq!(ctx.http_client_crate, "rquest");
    }
}
