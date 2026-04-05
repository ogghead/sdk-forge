//! CLI command definitions and dispatch.

pub mod analyze;
pub mod generate;
pub mod inspect;
pub mod record;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// SDK Forge — Record web interactions, reverse-engineer APIs, generate typed Rust SDKs.
#[derive(Debug, Parser)]
#[command(name = "sdk-forge", version, about)]
pub struct Cli {
    /// The subcommand to run.
    #[command(subcommand)]
    pub command: Command,
}

/// Available CLI commands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Launch an instrumented browser and record API interactions.
    Record(RecordArgs),
    /// Analyze a recorded session using Claude to reverse-engineer the API.
    Analyze(AnalyzeArgs),
    /// Generate a typed Rust SDK from an analyzed session.
    Generate(GenerateArgs),
    /// Inspect a recorded session (view captured endpoints).
    Inspect(InspectArgs),
}

/// Arguments for the `record` command.
#[derive(Debug, clap::Args)]
pub struct RecordArgs {
    /// URL to navigate to in the instrumented browser.
    pub url: String,
    /// Output path for the session file.
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

/// Arguments for the `analyze` command.
#[derive(Debug, clap::Args)]
pub struct AnalyzeArgs {
    /// Path to the `.sdkforge` session file.
    pub session: PathBuf,
    /// Claude model to use for analysis.
    #[arg(long, default_value = "claude-sonnet-4-20250514")]
    pub model: String,
}

/// Arguments for the `generate` command.
#[derive(Debug, clap::Args)]
pub struct GenerateArgs {
    /// Path to the analyzed `.sdkforge` session file.
    pub session: PathBuf,
    /// Output directory for the generated SDK crate.
    #[arg(short, long, default_value = "output")]
    pub output: PathBuf,
    /// Name for the generated SDK crate.
    #[arg(short, long)]
    pub name: Option<String>,
    /// Run `cargo check` on the generated crate.
    #[arg(long)]
    pub check: bool,
}

/// Arguments for the `inspect` command.
#[derive(Debug, clap::Args)]
pub struct InspectArgs {
    /// Path to the `.sdkforge` session file.
    pub session: PathBuf,
    /// Show only endpoint summaries (URLs, methods, status codes).
    #[arg(long)]
    pub endpoints: bool,
}
