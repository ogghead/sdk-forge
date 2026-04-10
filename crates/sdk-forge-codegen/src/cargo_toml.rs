//! `Cargo.toml` generation for the output Wasm component crate.
//!
//! Produces a `Cargo.toml` for a `cdylib` crate that builds into a
//! Wasm component using `wit-bindgen` for WASI bindings.

use serde::Serialize;

/// Context for rendering `cargo_toml.tera`.
#[derive(Debug, Clone, Serialize)]
pub struct CargoTomlContext {
    /// Crate name for the generated component (e.g. `"my-api-sdk"`).
    pub crate_name: String,
    /// Base URL of the API, used in the package description.
    pub base_url: String,
}

/// Build the `Cargo.toml` template context.
pub fn build_cargo_toml_context(crate_name: &str, base_url: &str) -> CargoTomlContext {
    CargoTomlContext {
        crate_name: crate_name.to_owned(),
        base_url: base_url.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::build_cargo_toml_context;

    #[test]
    fn test_cargo_toml_context() {
        let ctx = build_cargo_toml_context("my-api", "https://api.example.com");
        assert_eq!(ctx.crate_name, "my-api");
        assert_eq!(ctx.base_url, "https://api.example.com");
    }
}
