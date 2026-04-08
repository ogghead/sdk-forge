//! Implementation of the `sdk-forge generate` command.
//!
//! Takes an analyzed session and emits a Wasm component crate:
//! a WIT spec describing the API and a Rust implementation using WASI HTTP.

use std::path::PathBuf;

use miette::Context;

use sdk_forge_codegen::emitter::{EmitConfig, emit};
use sdk_forge_session::io::load_session;

use crate::commands::GenerateArgs;

/// Derive a crate name from the session's target URL.
///
/// Falls back to `"generated-sdk"` if the URL cannot be parsed.
fn crate_name_from_url(url: &str) -> String {
    url.split("://")
        .nth(1)
        .unwrap_or(url)
        .split('/')
        .next()
        .unwrap_or("generated-sdk")
        .replace(['.', ':'], "-")
}

/// Locate the templates directory.
///
/// Checks for a `templates/` directory next to the binary first,
/// then falls back to the workspace root (for development).
fn find_templates_dir() -> miette::Result<PathBuf> {
    // In development: look relative to CARGO_MANIFEST_DIR (compile-time).
    let dev_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("templates");
    if dev_path.is_dir() {
        return Ok(dev_path);
    }

    // Fallback: look next to the binary.
    if let Ok(exe) = std::env::current_exe()
        && let Some(parent) = exe.parent()
    {
        let beside_exe = parent.join("templates");
        if beside_exe.is_dir() {
            return Ok(beside_exe);
        }
    }

    miette::bail!("could not find templates directory")
}

/// Execute the generate command.
///
/// # Errors
///
/// Returns an error if the session cannot be loaded, the API model is missing,
/// or code generation fails.
pub fn execute(args: &GenerateArgs) -> miette::Result<()> {
    tracing::info!(
        session = %args.session.display(),
        output = %args.output.display(),
        "generating Wasm component SDK"
    );

    let session = load_session(&args.session).wrap_err("failed to load session file")?;

    let api_model = session.api_model.as_ref().ok_or_else(|| {
        miette::miette!("session has no API model; run `sdk-forge analyze` first")
    })?;

    let crate_name = args
        .name
        .clone()
        .unwrap_or_else(|| crate_name_from_url(&session.target_url));

    let templates_dir = find_templates_dir()?;

    let config = EmitConfig {
        crate_name,
        output_dir: args.output.clone(),
        templates_dir,
    };

    let output = emit(api_model, &config).map_err(|err| miette::miette!("{err}"))?;

    tracing::info!(
        crate_dir = %output.crate_dir.display(),
        files = output.file_count,
        records = output.struct_count,
        methods = output.method_count,
        "Wasm component SDK generated successfully"
    );

    if args.check {
        tracing::info!("running cargo component check on generated crate");
        let status = std::process::Command::new("cargo")
            .args(["component", "check"])
            .current_dir(&output.crate_dir)
            .status();
        match status {
            Ok(s) if s.success() => {
                tracing::info!("cargo component check passed");
            }
            Ok(s) => {
                miette::bail!("cargo component check failed with exit code: {}", s);
            }
            Err(err) => {
                miette::bail!(
                    "failed to run cargo component check (is cargo-component installed?): {err}"
                );
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    #[test]
    fn test_crate_name_from_url_simple() {
        assert_eq!(
            crate_name_from_url("https://api.example.com/v1"),
            "api-example-com",
            "dots and colons should become hyphens"
        );
    }

    #[test]
    fn test_crate_name_from_url_with_port() {
        assert_eq!(
            crate_name_from_url("http://localhost:3000/api"),
            "localhost-3000",
            "port separator should become a hyphen"
        );
    }

    #[test]
    fn test_crate_name_from_url_no_scheme() {
        assert_eq!(
            crate_name_from_url("example.com"),
            "example-com",
            "URL without scheme should still work"
        );
    }

    #[test]
    fn test_find_templates_dir_exists() {
        let result = find_templates_dir();
        assert!(
            result.is_ok(),
            "templates directory should be found in dev environment"
        );
    }
}
