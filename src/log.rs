use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter};

/// Sets up tracing for the application using `tracing_subscriber`.
///
/// This function configures the tracing system with two layers:
///
/// - `fmt::layer()`: Provides structured formatting of tracing events, enabling
///   features such as ANSI-colored output and logging the target and log level.
/// - `EnvFilter`: Controls log filtering, allowing log levels to be set via environment variables.
///   If no environment variable is set, the default log level is `debug`.
///
/// This tracing setup is initialized by calling the `init()` method, which globally initializes
/// the subscriber.
///
/// # Layers
///
/// - `fmt::layer()`:
///   - Enables ANSI coloring for the log output.
///   - Logs the target of each log message (the module or crate where it originated).
///   - Logs the level (e.g., `info`, `debug`, `error`) of each message.
///
/// - `EnvFilter`:
///   - Attempts to read the log level from the environment using `RUST_LOG`.
///   - If no environment variable is found, it defaults to `debug`.
///
/// # Example
///
/// ```rust
/// setup_tracing();
/// tracing::info!("Application started");
/// ```
///
/// # Panics
///
/// - The function will panic if there is an error setting up the `EnvFilter` from the environment.
///
/// # Environment Variables
///
/// - You can control the logging level using the `RUST_LOG` environment variable, e.g.:
///   ```bash
///   RUST_LOG=info ./my_app
///   ```
///
pub fn setup_tracing() {
    let fmt_layer = fmt::layer()
        .with_target(true)
        .with_ansi(true)
        .with_level(true);

    let filter_layer =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug"));

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .init();
}
