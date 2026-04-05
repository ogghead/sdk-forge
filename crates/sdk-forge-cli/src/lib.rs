//! SDK Forge — Record web interactions, reverse-engineer APIs, generate typed Rust SDKs.
//!
//! This is the library root for the CLI. All initialization and command
//! dispatch logic lives here so it can be tested and covered.

pub mod commands;
pub mod config;
pub mod error;

use clap::Parser;

use crate::commands::Cli;

/// Initialize global subsystems (error handler, tracing).
///
/// Safe to call multiple times — idempotent.
pub fn init() {
    // set_hook returns Err if already set (e.g. multiple tests in the same process); safe to ignore.
    drop(miette::set_hook(Box::new(|_| {
        Box::new(miette::MietteHandlerOpts::new().build())
    })));

    // try_init avoids a panic when called more than once (e.g. from multiple tests).
    drop(
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
            )
            .try_init(),
    );
}

/// Dispatch a parsed CLI command to its handler.
///
/// # Errors
///
/// Returns an error if the command handler fails.
pub fn dispatch(command: &commands::Command) -> miette::Result<()> {
    tracing::info!("sdk-forge started");

    match command {
        commands::Command::Record(args) => commands::record::execute(args),
        commands::Command::Analyze(args) => commands::analyze::execute(args),
        commands::Command::Generate(args) => commands::generate::execute(args),
        commands::Command::Inspect(args) => commands::inspect::execute(args),
    }
}

/// Initialize the application and run the CLI.
///
/// Parses CLI arguments from the process environment and dispatches
/// to the appropriate command handler.
pub fn run() -> miette::Result<()> {
    init();
    let cli = Cli::parse();
    dispatch(&cli.command)
}

#[cfg(test)]
mod tests {
    #[allow(clippy::wildcard_imports)]
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_init_is_idempotent() {
        init();
        init();
        // No panic means success.
    }

    #[test]
    fn test_dispatch_record_returns_not_implemented() {
        init();
        let cmd = commands::Command::Record(commands::RecordArgs {
            url: "https://example.com".to_owned(),
            output: None,
        });
        let result = dispatch(&cmd);
        assert!(
            result.is_err(),
            "record should return not-implemented error"
        );
    }

    #[test]
    fn test_dispatch_analyze_returns_not_implemented() {
        init();
        let cmd = commands::Command::Analyze(commands::AnalyzeArgs {
            session: PathBuf::from("test.sdkforge"),
            model: "claude-sonnet-4-20250514".to_owned(),
        });
        let result = dispatch(&cmd);
        assert!(
            result.is_err(),
            "analyze should return not-implemented error"
        );
    }

    #[test]
    fn test_dispatch_generate_returns_not_implemented() {
        init();
        let cmd = commands::Command::Generate(commands::GenerateArgs {
            session: PathBuf::from("test.sdkforge"),
            output: PathBuf::from("output"),
            name: None,
            check: false,
        });
        let result = dispatch(&cmd);
        assert!(
            result.is_err(),
            "generate should return not-implemented error"
        );
    }

    #[test]
    fn test_dispatch_inspect_returns_not_implemented() {
        init();
        let cmd = commands::Command::Inspect(commands::InspectArgs {
            session: PathBuf::from("test.sdkforge"),
            endpoints: false,
        });
        let result = dispatch(&cmd);
        assert!(
            result.is_err(),
            "inspect should return not-implemented error"
        );
    }
}
