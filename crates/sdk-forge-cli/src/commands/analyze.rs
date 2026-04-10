//! Implementation of the `sdk-forge analyze` command.
//!
//! Loads a recorded session, sends it to Claude for multi-pass
//! analysis, and writes the enriched session back.

use crate::commands::AnalyzeArgs;

/// Execute the analyze command.
///
/// # Errors
///
/// Returns an error if session loading, analysis, or writing fails.
pub fn execute(args: &AnalyzeArgs) -> miette::Result<()> {
    tracing::info!(session = %args.session.display(), "analyzing session");
    miette::bail!("analyze command not yet implemented")
}
