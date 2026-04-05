//! Implementation of the `sdk-forge generate` command.
//!
//! Takes an analyzed session and emits a complete Rust SDK crate.

use crate::commands::GenerateArgs;

/// Execute the generate command.
///
/// # Errors
///
/// Returns an error if the API model is incomplete or code generation fails.
pub fn execute(args: &GenerateArgs) -> miette::Result<()> {
    tracing::info!(
        session = %args.session.display(),
        output = %args.output.display(),
        "generating SDK"
    );
    miette::bail!("generate command not yet implemented")
}
