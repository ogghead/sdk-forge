//! Implementation of the `sdk-forge inspect` command.
//!
//! Displays a summary of a recorded session's captured exchanges
//! without performing any analysis.

use crate::commands::InspectArgs;

/// Execute the inspect command.
///
/// # Errors
///
/// Returns an error if the session file cannot be loaded.
pub fn execute(args: &InspectArgs) -> miette::Result<()> {
    tracing::info!(session = %args.session.display(), "inspecting session");
    miette::bail!("inspect command not yet implemented")
}
