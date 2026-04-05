//! Library root for {{project-name}}.

pub mod error;

/// Initialize the application and run the main logic.
///
/// Sets up the miette error handler and tracing subscriber,
/// then executes the application.
pub fn run() -> miette::Result<()> {
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

    tracing::info!("{{project-name}} started");

    Ok(())
}
