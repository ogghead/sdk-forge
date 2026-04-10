//! Implementation of the `sdk-forge record` command.
//!
//! Launches an instrumented browser, captures network traffic,
//! and writes a session file on exit.

use crate::commands::RecordArgs;

/// Execute the record command.
///
/// # Errors
///
/// Returns an error if browser launch or session writing fails.
pub fn execute(args: &RecordArgs) -> miette::Result<()> {
    tracing::info!(url = %args.url, "starting recording session");
    miette::bail!("record command not yet implemented")
}
