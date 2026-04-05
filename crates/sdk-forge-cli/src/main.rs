//! SDK Forge CLI entry point.

/// Application entry point.
///
/// Delegates to the library crate for all logic so that
/// it can be tested and covered.
fn main() -> miette::Result<()> {
    sdk_forge_cli::run()
}
